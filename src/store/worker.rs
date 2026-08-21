//! Serialized background access to the `SQLite` repository.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use async_channel::{Receiver, Sender};
use jiff::Timestamp;
use serde::Serialize;

use super::{PersistenceSnapshot, SqliteRepository, StorageError};

/// Snapshot returned by the storage worker.
pub type StorageSnapshot = PersistenceSnapshot;

/// Versioned, human-readable backup envelope.
#[derive(Debug, Serialize)]
pub struct BackupFile {
    /// Backup format version.
    pub format_version: u32,
    /// Cadence application version that produced the file.
    pub application_version: &'static str,
    /// UTC export timestamp.
    pub exported_at: String,
    /// Persisted timetable data.
    pub data: PersistenceSnapshot,
}

enum Command {
    Load {
        reply: Sender<Result<StorageSnapshot, StorageError>>,
    },
    Replace {
        snapshot: StorageSnapshot,
        reply: Sender<Result<(), StorageError>>,
    },
    Export {
        reply: Sender<Result<String, StorageError>>,
    },
    ExportToPath {
        path: PathBuf,
        reply: Sender<Result<(), StorageError>>,
    },
    ArchiveAndStartFresh {
        reply: Sender<Result<StorageSnapshot, StorageError>>,
    },
}

/// Cloneable client for the single-owner storage worker.
#[derive(Clone)]
pub struct StorageClient {
    sender: Sender<Command>,
    path: PathBuf,
}

impl std::fmt::Debug for StorageClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageClient")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl StorageClient {
    /// Starts a storage worker for `path`.
    ///
    /// # Parameters
    ///
    /// - `path`: `SQLite` database path owned by the worker.
    ///
    /// # Returns
    ///
    /// A client whose commands are serialized by one background thread.
    ///
    /// # Panics
    ///
    /// Panics when:
    ///
    /// - The operating system cannot create the worker thread.
    #[must_use]
    pub fn spawn(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let (sender, receiver) = async_channel::unbounded();
        let worker_path = path.clone();
        thread::Builder::new()
            .name("cadence-storage".to_owned())
            .spawn(move || run_worker(&worker_path, &receiver))
            .expect("Cadence storage worker must start");
        Self { sender, path }
    }

    /// Returns the database path handled by this client.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Loads the complete persisted snapshot asynchronously.
    #[must_use]
    pub fn load(&self) -> Receiver<Result<StorageSnapshot, StorageError>> {
        let (reply, receiver) = async_channel::bounded(1);
        self.send(Command::Load { reply });
        receiver
    }

    /// Commits a complete snapshot atomically.
    #[must_use]
    pub fn replace(&self, snapshot: StorageSnapshot) -> Receiver<Result<(), StorageError>> {
        let (reply, receiver) = async_channel::bounded(1);
        self.send(Command::Replace { snapshot, reply });
        receiver
    }

    /// Serializes one consistent snapshot as a versioned JSON backup.
    #[must_use]
    pub fn export_json(&self) -> Receiver<Result<String, StorageError>> {
        let (reply, receiver) = async_channel::bounded(1);
        self.send(Command::Export { reply });
        receiver
    }

    /// Serializes one consistent snapshot and atomically writes a JSON backup.
    ///
    /// The complete export, including file creation, syncing, and replacement, runs on the
    /// storage worker so the application executor remains responsive while a backup is written.
    #[must_use]
    pub fn export_to_path(&self, path: impl Into<PathBuf>) -> Receiver<Result<(), StorageError>> {
        let (reply, receiver) = async_channel::bounded(1);
        self.send(Command::ExportToPath {
            path: path.into(),
            reply,
        });
        receiver
    }

    /// Archives the unreadable database and initializes a new one.
    #[must_use]
    pub fn archive_and_start_fresh(&self) -> Receiver<Result<StorageSnapshot, StorageError>> {
        let (reply, receiver) = async_channel::bounded(1);
        self.send(Command::ArchiveAndStartFresh { reply });
        receiver
    }

    fn send(&self, command: Command) {
        let _ = self.sender.send_blocking(command);
    }
}

fn run_worker(path: &Path, receiver: &Receiver<Command>) {
    let mut repository = None::<SqliteRepository>;
    while let Ok(command) = receiver.recv_blocking() {
        match command {
            Command::Load { reply } => {
                let result = repository_for(path, &mut repository)
                    .and_then(|repository| repository.load_snapshot());
                let _ = reply.send_blocking(result);
            }
            Command::Replace { snapshot, reply } => {
                let result = repository_for(path, &mut repository)
                    .and_then(|repository| repository.replace_snapshot(&snapshot));
                let _ = reply.send_blocking(result);
            }
            Command::Export { reply } => {
                let result = repository_for(path, &mut repository)
                    .and_then(|repository| repository.load_snapshot())
                    .and_then(serialize_backup);
                let _ = reply.send_blocking(result);
            }
            Command::ExportToPath {
                path: destination,
                reply,
            } => {
                let result = repository_for(path, &mut repository)
                    .and_then(|repository| repository.load_snapshot())
                    .and_then(serialize_backup)
                    .and_then(|contents| write_backup_atomically(&destination, &contents));
                let _ = reply.send_blocking(result);
            }
            Command::ArchiveAndStartFresh { reply } => {
                let result = archive_and_start_fresh(path, &mut repository);
                let _ = reply.send_blocking(result);
            }
        }
    }
}

fn serialize_backup(data: StorageSnapshot) -> Result<String, StorageError> {
    serde_json::to_string_pretty(&BackupFile {
        format_version: 3,
        application_version: env!("CARGO_PKG_VERSION"),
        exported_at: Timestamp::now().to_string(),
        data,
    })
    .map_err(|error| StorageError::Io(error.to_string()))
}

fn write_backup_atomically(path: &Path, contents: &str) -> Result<(), StorageError> {
    let temporary = path.with_file_name(format!(
        ".{}.tmp-{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("cadence-backup.json"),
        std::process::id(),
    ));
    let result = (|| {
        let mut file =
            fs::File::create(&temporary).map_err(|error| StorageError::Io(error.to_string()))?;
        file.write_all(contents.as_bytes())
            .map_err(|error| StorageError::Io(error.to_string()))?;
        file.sync_all()
            .map_err(|error| StorageError::Io(error.to_string()))?;
        fs::rename(&temporary, path).map_err(|error| StorageError::Io(error.to_string()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn repository_for<'a>(
    path: &Path,
    repository: &'a mut Option<SqliteRepository>,
) -> Result<&'a mut SqliteRepository, StorageError> {
    if repository.is_none() {
        *repository = Some(SqliteRepository::open(path.to_owned())?);
    }
    repository
        .as_mut()
        .ok_or_else(|| StorageError::Io("storage worker did not open the database".to_owned()))
}

fn archive_and_start_fresh(
    path: &Path,
    repository: &mut Option<SqliteRepository>,
) -> Result<StorageSnapshot, StorageError> {
    repository.take();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| StorageError::Io(error.to_string()))?
        .as_secs();
    let archive = parent.join(format!("cadence-recovery-{stamp}"));
    fs::create_dir_all(&archive).map_err(|error| StorageError::Io(error.to_string()))?;
    move_if_exists(path, &archive.join("cadence.sqlite3"))?;
    move_if_exists(
        &path.with_extension("sqlite3-journal"),
        &archive.join("cadence.sqlite3-journal"),
    )?;
    let opened = SqliteRepository::open(path.to_owned())?;
    let snapshot = opened.load_snapshot()?;
    *repository = Some(opened);
    Ok(snapshot)
}

fn move_if_exists(source: &Path, destination: &Path) -> Result<(), StorageError> {
    if source.exists() {
        fs::rename(source, destination).map_err(|error| StorageError::Io(error.to_string()))?;
    }
    Ok(())
}
