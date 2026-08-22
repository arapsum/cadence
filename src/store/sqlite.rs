//! SQLite-backed persistence, migrations, and local data-path resolution.

use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

use jiff::{
    Timestamp,
    civil::{Date, Time},
};
use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

use crate::domain::{
    Category, CategoryColor, CategoryId, ClockFormat, DateRange, Event, EventDraft, EventId,
    RecurrenceException, RecurrenceExceptionKind, RecurrenceSeries, RecurrenceSeriesId,
    ReminderOffset, RepositoryError, Settings, SnapInterval, TimeZoneId, WeekStart,
};

use super::{
    AppPreferences, AppearanceMode, AppearancePreferences, CalendarViewModePreference,
    InMemoryRepository, PersistenceSnapshot, TimetableRepository,
};

/// Latest schema version understood by Cadence.
pub const CURRENT_SCHEMA_VERSION: u32 = 5;

/// Error raised while opening, migrating, validating, or using local storage.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StorageError {
    /// The data directory or database could not be created.
    Io(String),
    /// `SQLite` rejected an operation.
    Sqlite(String),
    /// A migration could not be applied.
    Migration(String),
    /// The database failed an integrity or entity validation check.
    Corrupt(String),
    /// The database was written by a newer Cadence version.
    IncompatibleSchema { found: u32, supported: u32 },
    /// A persisted value does not satisfy the domain model.
    InvalidEntity(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(f, "storage I/O failed: {message}"),
            Self::Sqlite(message) => write!(f, "SQLite failed: {message}"),
            Self::Migration(message) => write!(f, "database migration failed: {message}"),
            Self::Corrupt(message) => write!(f, "the timetable database is corrupt: {message}"),
            Self::IncompatibleSchema { found, supported } => write!(
                f,
                "this database uses schema {found}, but this Cadence build supports through schema {supported}."
            ),
            Self::InvalidEntity(message) => {
                write!(f, "stored timetable data is invalid: {message}")
            }
        }
    }
}

impl std::error::Error for StorageError {}

impl StorageError {
    /// Returns a concise message suitable for the recovery surface.
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::Io(_) => "Cadence could not access the local data folder.".to_owned(),
            Self::Sqlite(_) => "Cadence could not read the local timetable database.".to_owned(),
            Self::Migration(_) => "Cadence could not upgrade the timetable database.".to_owned(),
            Self::Corrupt(_) => "The timetable database failed its integrity checks.".to_owned(),
            Self::IncompatibleSchema { found, supported } => {
                format!(
                    "This database uses schema {found}; this build supports through schema {supported}."
                )
            }
            Self::InvalidEntity(_) => "The database contains invalid timetable data.".to_owned(),
        }
    }
}

impl From<StorageError> for RepositoryError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error.to_string())
    }
}

/// Returns Cadence's platform data directory.
///
/// The `CADENCE_DATA_DIR` override is intended for tests and portable installs.
/// Linux follows `XDG_DATA_HOME`, then `$HOME/.local/share`.
///
/// # Returns
///
/// The directory in which Cadence stores its database.
///
/// # Errors
///
/// Returns an error when:
///
/// - No usable data directory environment variable exists.
pub fn data_directory() -> Result<PathBuf, StorageError> {
    if let Some(path) = std::env::var_os("CADENCE_DATA_DIR") {
        return Ok(PathBuf::from(path));
    }
    if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(path).join("cadence"));
    }
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".local/share/cadence"))
        .ok_or_else(|| StorageError::Io("HOME is not set; choose CADENCE_DATA_DIR".to_owned()))
}

/// Returns the default Cadence database path.
///
/// # Returns
///
/// A path ending in `cadence.sqlite3` inside [`data_directory`].
///
/// # Errors
///
/// Returns an error when:
///
/// - The platform data directory cannot be resolved.
pub fn database_path() -> Result<PathBuf, StorageError> {
    Ok(data_directory()?.join("cadence.sqlite3"))
}

/// `SQLite` repository that stores all timetable entities in one local database.
pub struct SqliteRepository {
    connection: Connection,
    path: PathBuf,
}

impl fmt::Debug for SqliteRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SqliteRepository")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl SqliteRepository {
    /// Opens or creates a database and applies all supported migrations.
    ///
    /// # Parameters
    ///
    /// - `path`: Database file to open.
    ///
    /// # Returns
    ///
    /// An initialized repository.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - The parent directory cannot be created.
    /// - `SQLite` cannot open or migrate the file.
    /// - The file is corrupt or uses a newer schema.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let path = path.into();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|error| StorageError::Io(error.to_string()))?;
        }
        let connection = Connection::open(&path).map_err(sqlite_error)?;
        configure_connection(&connection)?;
        migrate(&connection)?;
        validate_database(&connection)?;
        let repository = Self { connection, path };
        repository
            .load_snapshot()
            .map_err(|error| StorageError::Corrupt(error.to_string()))?;
        Ok(repository)
    }

    /// Returns the path used by this repository.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn ensure_unique_category_name(&self, category: &Category) -> Result<(), RepositoryError> {
        if self.categories()?.iter().any(|existing| {
            existing.id() != category.id()
                && existing.name().to_lowercase() == category.name().to_lowercase()
        }) {
            return Err(RepositoryError::DuplicateCategoryName);
        }
        Ok(())
    }

    /// Reads all persisted entities in a deterministic order.
    ///
    /// # Returns
    ///
    /// A complete persistence snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - `SQLite` returns an invalid or unreadable row.
    pub fn load_snapshot(&self) -> Result<PersistenceSnapshot, StorageError> {
        let settings = self.load_settings()?;
        let preferences = self.load_preferences()?;
        let categories = self.load_categories()?;
        let events = self.load_all_events()?;
        let recurrence_series = self.load_all_series()?;
        let recurrence_exceptions = self.load_all_exceptions()?;
        Ok(PersistenceSnapshot {
            settings,
            preferences,
            categories,
            events,
            recurrence_series,
            recurrence_exceptions,
        })
    }

    /// Replaces every persisted table atomically.
    ///
    /// # Parameters
    ///
    /// - `snapshot`: Complete state to commit.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the transaction commits.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - A snapshot entity is invalid or violates a foreign key.
    /// - The transaction cannot commit.
    pub fn replace_snapshot(&mut self, snapshot: &PersistenceSnapshot) -> Result<(), StorageError> {
        InMemoryRepository::from_snapshot(snapshot)
            .map_err(|error| StorageError::InvalidEntity(error.to_string()))?;
        let tx = self.connection.transaction().map_err(sqlite_error)?;
        tx.execute("DELETE FROM recurrence_exceptions", [])
            .map_err(sqlite_error)?;
        tx.execute("DELETE FROM recurrence_series", [])
            .map_err(sqlite_error)?;
        tx.execute("DELETE FROM events", []).map_err(sqlite_error)?;
        tx.execute("DELETE FROM categories", [])
            .map_err(sqlite_error)?;
        insert_categories(&tx, &snapshot.categories)?;
        insert_events(&tx, &snapshot.events)?;
        insert_series(&tx, &snapshot.recurrence_series)?;
        insert_exceptions(&tx, &snapshot.recurrence_exceptions)?;
        update_settings(&tx, &snapshot.settings)?;
        update_preferences(&tx, &snapshot.preferences)?;
        tx.commit().map_err(sqlite_error)
    }

    fn load_settings(&self) -> Result<Settings, StorageError> {
        self.connection
            .query_row(
                "SELECT week_start, clock_format, timezone, snap_minutes, day_start, day_end FROM settings WHERE id = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .map_err(sqlite_error)
            .and_then(|(week_start, clock_format, timezone, snap_minutes, day_start, day_end)| {
                settings_from_values(&week_start, &clock_format, &timezone, snap_minutes, &day_start, &day_end)
            })
    }

    fn load_preferences(&self) -> Result<AppPreferences, StorageError> {
        let row = self
            .connection
            .query_row(
                "SELECT view_mode, category_filter_id, notifications_enabled, reduce_motion, appearance_mode, light_theme, dark_theme, font_family, font_size FROM app_preferences WHERE id = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, i64>(8)?,
                    ))
                },
            )
            .optional()
            .map_err(sqlite_error)?;
        let Some((
            view_mode,
            category_filter_id,
            notifications_enabled,
            reduce_motion,
            appearance_mode,
            light_theme,
            dark_theme,
            font_family,
            font_size,
        )) = row
        else {
            return Ok(AppPreferences::default());
        };
        let view_mode = match view_mode.as_str() {
            "day" => CalendarViewModePreference::Day,
            "week" => CalendarViewModePreference::Week,
            value => {
                return Err(StorageError::InvalidEntity(format!(
                    "unknown view mode '{value}'"
                )));
            }
        };
        let category_filter = category_filter_id
            .map(|value| value.parse::<Uuid>().map(CategoryId::from_uuid))
            .transpose()
            .map_err(|error| {
                StorageError::InvalidEntity(format!("invalid category filter: {error}"))
            })?;
        let appearance = appearance_from_values(
            &appearance_mode,
            &light_theme,
            &dark_theme,
            &font_family,
            font_size,
        )?;
        Ok(AppPreferences {
            view_mode,
            category_filter,
            notifications_enabled: notifications_enabled != 0,
            reduce_motion: reduce_motion != 0,
            appearance,
        })
    }

    fn load_categories(&self) -> Result<Vec<Category>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT id, name, color, is_visible FROM categories ORDER BY lower(name), id")
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(sqlite_error)?;
        rows.map(|row| {
            row.map_err(sqlite_error)
                .and_then(|(id, name, color, visible)| {
                    category_from_values(&id, &name, &color, visible != 0)
                })
        })
        .collect()
    }

    fn load_all_events(&self) -> Result<Vec<Event>, StorageError> {
        let mut statement = self.connection
            .prepare("SELECT id, category_id, title, date, start_time, end_time, notes, reminder_minutes, created_at, updated_at FROM events ORDER BY date, start_time, end_time, id")
            .map_err(sqlite_error)?;
        let rows = statement.query_map([], event_row).map_err(sqlite_error)?;
        rows.map(|row| row.map_err(sqlite_error).and_then(event_from_values))
            .collect()
    }

    fn load_all_series(&self) -> Result<Vec<RecurrenceSeries>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT data FROM recurrence_series ORDER BY id")
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(sqlite_error)?;
        rows.map(|row| {
            row.map_err(sqlite_error).and_then(|data| {
                serde_json::from_str(&data).map_err(|error| {
                    StorageError::InvalidEntity(format!("invalid recurrence series: {error}"))
                })
            })
        })
        .collect()
    }

    fn load_all_exceptions(&self) -> Result<Vec<RecurrenceException>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT data FROM recurrence_exceptions ORDER BY series_id, original_date")
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(sqlite_error)?;
        rows.map(|row| {
            row.map_err(sqlite_error).and_then(|data| {
                serde_json::from_str(&data).map_err(|error| {
                    StorageError::InvalidEntity(format!("invalid recurrence exception: {error}"))
                })
            })
        })
        .collect()
    }
}

impl TimetableRepository for SqliteRepository {
    fn event(&self, id: EventId) -> Result<Option<Event>, RepositoryError> {
        self.connection
            .query_row(
                "SELECT id, category_id, title, date, start_time, end_time, notes, reminder_minutes, created_at, updated_at FROM events WHERE id = ?1",
                [id.to_string()],
                event_row,
            )
            .optional()
            .map_err(sqlite_error)
            .and_then(|row| row.map(event_from_values).transpose())
            .map_err(Into::into)
    }

    fn events(&self, range: DateRange) -> Result<Vec<Event>, RepositoryError> {
        let mut statement = self.connection
            .prepare("SELECT id, category_id, title, date, start_time, end_time, notes, reminder_minutes, created_at, updated_at FROM events WHERE date >= ?1 AND date < ?2 ORDER BY date, start_time, end_time, id")
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map(
                params![range.start().to_string(), range.end().to_string()],
                event_row,
            )
            .map_err(sqlite_error)?;
        rows.map(|row| row.map_err(sqlite_error).and_then(event_from_values))
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn recurrence_series(&self) -> Result<Vec<RecurrenceSeries>, RepositoryError> {
        self.load_all_series().map_err(Into::into)
    }

    fn series(&self, id: RecurrenceSeriesId) -> Result<Option<RecurrenceSeries>, RepositoryError> {
        self.connection
            .query_row(
                "SELECT data FROM recurrence_series WHERE id = ?1",
                [id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sqlite_error)
            .and_then(|data| {
                data.map(|data| {
                    serde_json::from_str(&data)
                        .map_err(|error| StorageError::InvalidEntity(error.to_string()))
                })
                .transpose()
            })
            .map_err(Into::into)
    }

    fn recurrence_exceptions(&self) -> Result<Vec<RecurrenceException>, RepositoryError> {
        self.load_all_exceptions().map_err(Into::into)
    }

    fn create_series(&mut self, series: RecurrenceSeries) -> Result<(), RepositoryError> {
        let tx = self.connection.transaction().map_err(sqlite_error)?;
        insert_series(&tx, std::slice::from_ref(&series)).map_err(map_repository_error)?;
        tx.commit().map_err(sqlite_error).map_err(Into::into)
    }

    fn update_series(&mut self, series: RecurrenceSeries) -> Result<(), RepositoryError> {
        let data = serde_json::to_string(&series)
            .map_err(|error| RepositoryError::InvalidEntity(error.to_string()))?;
        let changed = self
            .connection
            .execute(
                "UPDATE recurrence_series SET category_id = ?2, start_date = ?3, data = ?4 WHERE id = ?1",
                params![
                    series.id().to_string(),
                    series.template().category_id.to_string(),
                    series.template().date.to_string(),
                    data,
                ],
            )
            .map_err(sqlite_error)?;
        if changed == 0 {
            return Err(RepositoryError::SeriesNotFound);
        }
        Ok(())
    }

    fn delete_series(
        &mut self,
        id: RecurrenceSeriesId,
    ) -> Result<RecurrenceSeries, RepositoryError> {
        let Some(series) = self.series(id)? else {
            return Err(RepositoryError::SeriesNotFound);
        };
        self.connection
            .execute(
                "DELETE FROM recurrence_series WHERE id = ?1",
                [id.to_string()],
            )
            .map_err(sqlite_error)?;
        Ok(series)
    }

    fn upsert_exception(&mut self, exception: RecurrenceException) -> Result<(), RepositoryError> {
        let data = serde_json::to_string(&exception)
            .map_err(|error| RepositoryError::InvalidEntity(error.to_string()))?;
        let Some(_series) = self.series(exception.series_id())? else {
            return Err(RepositoryError::SeriesNotFound);
        };
        if let RecurrenceExceptionKind::Modified(draft) = exception.kind()
            && self.category(draft.category_id)?.is_none()
        {
            return Err(RepositoryError::CategoryNotFound);
        }
        self.connection
            .execute(
                "INSERT INTO recurrence_exceptions (series_id, original_date, data) VALUES (?1, ?2, ?3) ON CONFLICT(series_id, original_date) DO UPDATE SET data = excluded.data",
                params![exception.series_id().to_string(), exception.original_date().to_string(), data],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    fn delete_exception(
        &mut self,
        series_id: RecurrenceSeriesId,
        original_date: Date,
    ) -> Result<Option<RecurrenceException>, RepositoryError> {
        let existing = self.recurrence_exceptions()?.into_iter().find(|exception| {
            exception.series_id() == series_id && exception.original_date() == original_date
        });
        self.connection
            .execute(
                "DELETE FROM recurrence_exceptions WHERE series_id = ?1 AND original_date = ?2",
                params![series_id.to_string(), original_date.to_string()],
            )
            .map_err(sqlite_error)?;
        Ok(existing)
    }

    fn create_event(&mut self, event: Event) -> Result<(), RepositoryError> {
        let tx = self.connection.transaction().map_err(sqlite_error)?;
        insert_event(&tx, &event).map_err(map_repository_error)?;
        tx.commit().map_err(sqlite_error).map_err(Into::into)
    }

    fn update_event(&mut self, event: Event) -> Result<(), RepositoryError> {
        let tx = self.connection.transaction().map_err(sqlite_error)?;
        let changed = tx
            .execute(
                "UPDATE events SET category_id = ?2, title = ?3, date = ?4, start_time = ?5, end_time = ?6, notes = ?7, reminder_minutes = ?8, created_at = ?9, updated_at = ?10 WHERE id = ?1",
                params![event.id().to_string(), event.category_id().to_string(), event.title(), event.date().to_string(), event.start_time().to_string(), event.end_time().to_string(), event.notes(), event.reminder().map(|reminder| i64::from(reminder.minutes())), event.created_at().to_string(), event.updated_at().to_string()],
            )
            .map_err(|error| map_repository_error(sqlite_error(error)))?;
        if changed == 0 {
            return Err(RepositoryError::EventNotFound);
        }
        tx.commit().map_err(sqlite_error).map_err(Into::into)
    }

    fn delete_event(&mut self, id: EventId) -> Result<Event, RepositoryError> {
        let Some(event) = self.event(id)? else {
            return Err(RepositoryError::EventNotFound);
        };
        let tx = self.connection.transaction().map_err(sqlite_error)?;
        tx.execute("DELETE FROM events WHERE id = ?1", [id.to_string()])
            .map_err(sqlite_error)?;
        tx.commit()
            .map_err(sqlite_error)
            .map_err(RepositoryError::from)?;
        Ok(event)
    }

    fn categories(&self) -> Result<Vec<Category>, RepositoryError> {
        self.load_categories().map_err(Into::into)
    }

    fn category(&self, id: CategoryId) -> Result<Option<Category>, RepositoryError> {
        self.connection
            .query_row(
                "SELECT id, name, color, is_visible FROM categories WHERE id = ?1",
                [id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(sqlite_error)
            .and_then(|row| {
                row.map(|(id, name, color, visible)| {
                    category_from_values(&id, &name, &color, visible != 0)
                })
                .transpose()
            })
            .map_err(Into::into)
    }

    fn create_category(&mut self, category: Category) -> Result<(), RepositoryError> {
        self.ensure_unique_category_name(&category)?;
        let tx = self.connection.transaction().map_err(sqlite_error)?;
        insert_category(&tx, &category).map_err(map_repository_error)?;
        tx.commit().map_err(sqlite_error).map_err(Into::into)
    }

    fn update_category(&mut self, category: Category) -> Result<(), RepositoryError> {
        self.ensure_unique_category_name(&category)?;
        let tx = self.connection.transaction().map_err(sqlite_error)?;
        let changed = tx
            .execute(
                "UPDATE categories SET name = ?2, color = ?3, is_visible = ?4 WHERE id = ?1",
                params![
                    category.id().to_string(),
                    category.name(),
                    color_name(category.color_token()),
                    i64::from(category.is_visible())
                ],
            )
            .map_err(sqlite_error)?;
        if changed == 0 {
            return Err(RepositoryError::CategoryNotFound);
        }
        tx.commit().map_err(sqlite_error).map_err(Into::into)
    }

    fn delete_category(&mut self, id: CategoryId) -> Result<Category, RepositoryError> {
        let categories = self.categories()?;
        let Some(category) = categories
            .iter()
            .find(|category| category.id() == id)
            .cloned()
        else {
            return Err(RepositoryError::CategoryNotFound);
        };
        if categories.len() == 1 {
            return Err(RepositoryError::LastCategory);
        }
        if self.recurrence_exceptions()?.iter().any(|exception| {
            matches!(
                exception.kind(),
                RecurrenceExceptionKind::Modified(draft) if draft.category_id == id
            )
        }) {
            return Err(RepositoryError::CategoryInUse);
        }
        let tx = self.connection.transaction().map_err(sqlite_error)?;
        match tx.execute("DELETE FROM categories WHERE id = ?1", [id.to_string()]) {
            Ok(_) => tx
                .commit()
                .map_err(sqlite_error)
                .map_err(Into::into)
                .map(|()| category),
            Err(rusqlite::Error::SqliteFailure(code, _))
                if code.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err(RepositoryError::CategoryInUse)
            }
            Err(error) => Err(RepositoryError::from(sqlite_error(error))),
        }
    }

    fn settings(&self) -> Result<Settings, RepositoryError> {
        self.load_settings().map_err(Into::into)
    }

    fn replace_settings(&mut self, settings: Settings) -> Result<(), RepositoryError> {
        let tx = self.connection.transaction().map_err(sqlite_error)?;
        update_settings(&tx, &settings).map_err(RepositoryError::from)?;
        tx.commit().map_err(sqlite_error).map_err(Into::into)
    }

    fn preferences(&self) -> Result<AppPreferences, RepositoryError> {
        self.load_preferences().map_err(Into::into)
    }

    fn replace_preferences(&mut self, preferences: AppPreferences) -> Result<(), RepositoryError> {
        let tx = self.connection.transaction().map_err(sqlite_error)?;
        update_preferences(&tx, &preferences).map_err(RepositoryError::from)?;
        tx.commit().map_err(sqlite_error).map_err(Into::into)
    }

    fn snapshot(&self) -> Result<PersistenceSnapshot, RepositoryError> {
        self.load_snapshot().map_err(Into::into)
    }
}

fn configure_connection(connection: &Connection) -> Result<(), StorageError> {
    connection
        .execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = DELETE; PRAGMA synchronous = FULL; PRAGMA busy_timeout = 5000;")
        .map_err(sqlite_error)
}

fn migrate(connection: &Connection) -> Result<(), StorageError> {
    let current: u32 = connection
        .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
        .map_err(sqlite_error)?;
    if current > CURRENT_SCHEMA_VERSION {
        return Err(StorageError::IncompatibleSchema {
            found: current,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }
    if current == CURRENT_SCHEMA_VERSION {
        return Ok(());
    }
    let tx = connection.unchecked_transaction().map_err(sqlite_error)?;
    if current < 1 {
        tx.execute_batch(
            "CREATE TABLE categories (id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL, color TEXT NOT NULL, is_visible INTEGER NOT NULL CHECK (is_visible IN (0, 1)));
             CREATE TABLE events (id TEXT PRIMARY KEY NOT NULL, category_id TEXT NOT NULL REFERENCES categories(id) ON DELETE RESTRICT, title TEXT NOT NULL, date TEXT NOT NULL, start_time TEXT NOT NULL, end_time TEXT NOT NULL, notes TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, CHECK (end_time > start_time));
             CREATE INDEX events_date_time_idx ON events(date, start_time, end_time);
             CREATE INDEX events_category_date_idx ON events(category_id, date);
             CREATE TABLE settings (id INTEGER PRIMARY KEY CHECK (id = 1), week_start TEXT NOT NULL, clock_format TEXT NOT NULL, timezone TEXT NOT NULL, snap_minutes INTEGER NOT NULL, day_start TEXT NOT NULL, day_end TEXT NOT NULL);",
        ).map_err(|error| StorageError::Migration(error.to_string()))?;
        let defaults = Settings::default();
        tx.execute(
            "INSERT INTO settings (id, week_start, clock_format, timezone, snap_minutes, day_start, day_end) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                week_start_name(defaults.week_starts_on()),
                clock_format_name(defaults.clock_format()),
                defaults.time_zone().as_str(),
                i64::from(defaults.snap_interval().minutes()),
                defaults.day_start().to_string(),
                defaults.day_end().to_string(),
            ],
        )
        .map_err(|error| StorageError::Migration(error.to_string()))?;
        for category in super::default_categories() {
            insert_category(&tx, &category)?;
        }
        tx.pragma_update(None, "user_version", 1_u32)
            .map_err(|error| StorageError::Migration(error.to_string()))?;
    }
    if current < 2 {
        tx.execute_batch(
            "CREATE TABLE app_preferences (id INTEGER PRIMARY KEY CHECK (id = 1), view_mode TEXT NOT NULL, category_filter_id TEXT REFERENCES categories(id) ON DELETE SET NULL);
             INSERT INTO app_preferences (id, view_mode, category_filter_id) VALUES (1, 'week', NULL);",
        ).map_err(|error| StorageError::Migration(error.to_string()))?;
        tx.pragma_update(None, "user_version", 2_u32)
            .map_err(|error| StorageError::Migration(error.to_string()))?;
    }
    if current < 3 {
        tx.execute_batch(
            "CREATE TABLE recurrence_series (id TEXT PRIMARY KEY NOT NULL, category_id TEXT NOT NULL REFERENCES categories(id) ON DELETE RESTRICT, start_date TEXT NOT NULL, data TEXT NOT NULL);
             CREATE INDEX recurrence_series_category_date_idx ON recurrence_series(category_id, start_date);
             CREATE TABLE recurrence_exceptions (series_id TEXT NOT NULL REFERENCES recurrence_series(id) ON DELETE CASCADE, original_date TEXT NOT NULL, data TEXT NOT NULL, PRIMARY KEY(series_id, original_date));
             CREATE INDEX recurrence_exceptions_date_idx ON recurrence_exceptions(original_date);",
        )
        .map_err(|error| StorageError::Migration(error.to_string()))?;
        tx.pragma_update(None, "user_version", 3_u32)
            .map_err(|error| StorageError::Migration(error.to_string()))?;
    }
    if current < 4 {
        tx.execute_batch(
            "ALTER TABLE events ADD COLUMN reminder_minutes INTEGER;
             ALTER TABLE app_preferences ADD COLUMN notifications_enabled INTEGER NOT NULL DEFAULT 0 CHECK (notifications_enabled IN (0, 1));
             ALTER TABLE app_preferences ADD COLUMN reduce_motion INTEGER NOT NULL DEFAULT 0 CHECK (reduce_motion IN (0, 1));",
        )
            .map_err(|error| StorageError::Migration(error.to_string()))?;
        tx.pragma_update(None, "user_version", 4_u32)
            .map_err(|error| StorageError::Migration(error.to_string()))?;
    }
    if current < 5 {
        tx.execute_batch(
            "ALTER TABLE app_preferences ADD COLUMN appearance_mode TEXT NOT NULL DEFAULT 'system' CHECK (appearance_mode IN ('system', 'light', 'dark'));
             ALTER TABLE app_preferences ADD COLUMN light_theme TEXT NOT NULL DEFAULT 'Default Light';
             ALTER TABLE app_preferences ADD COLUMN dark_theme TEXT NOT NULL DEFAULT 'Default Dark';
             ALTER TABLE app_preferences ADD COLUMN font_family TEXT NOT NULL DEFAULT '.SystemUIFont';
             ALTER TABLE app_preferences ADD COLUMN font_size INTEGER NOT NULL DEFAULT 16 CHECK (font_size IN (14, 16, 18));",
        )
        .map_err(|error| StorageError::Migration(error.to_string()))?;
        tx.pragma_update(None, "user_version", 5_u32)
            .map_err(|error| StorageError::Migration(error.to_string()))?;
    }
    tx.commit()
        .map_err(|error| StorageError::Migration(error.to_string()))
}

fn validate_database(connection: &Connection) -> Result<(), StorageError> {
    let quick_check: String = connection
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(sqlite_error)?;
    if quick_check != "ok" {
        return Err(StorageError::Corrupt(quick_check));
    }
    let foreign_key_errors: i64 = connection
        .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .map_err(sqlite_error)?;
    if foreign_key_errors != 0 {
        return Err(StorageError::Corrupt(format!(
            "{foreign_key_errors} foreign-key violations"
        )));
    }
    Ok(())
}

fn insert_categories(connection: &Connection, categories: &[Category]) -> Result<(), StorageError> {
    for category in categories {
        insert_category(connection, category)?;
    }
    Ok(())
}

fn insert_category(connection: &Connection, category: &Category) -> Result<(), StorageError> {
    connection
        .execute(
            "INSERT INTO categories (id, name, color, is_visible) VALUES (?1, ?2, ?3, ?4)",
            params![
                category.id().to_string(),
                category.name(),
                color_name(category.color_token()),
                i64::from(category.is_visible())
            ],
        )
        .map(|_| ())
        .map_err(|error| match error {
            rusqlite::Error::SqliteFailure(_, _) => StorageError::Sqlite(error.to_string()),
            other => StorageError::Sqlite(other.to_string()),
        })
}

fn insert_events(connection: &Connection, events: &[Event]) -> Result<(), StorageError> {
    for event in events {
        insert_event(connection, event)?;
    }
    Ok(())
}

fn insert_event(connection: &Connection, event: &Event) -> Result<(), StorageError> {
    connection.execute("INSERT INTO events (id, category_id, title, date, start_time, end_time, notes, reminder_minutes, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)", params![event.id().to_string(), event.category_id().to_string(), event.title(), event.date().to_string(), event.start_time().to_string(), event.end_time().to_string(), event.notes(), event.reminder().map(|reminder| i64::from(reminder.minutes())), event.created_at().to_string(), event.updated_at().to_string()]).map(|_| ()).map_err(|error| StorageError::Sqlite(error.to_string()))
}

fn insert_series(connection: &Connection, series: &[RecurrenceSeries]) -> Result<(), StorageError> {
    for series in series {
        let data = serde_json::to_string(series)
            .map_err(|error| StorageError::InvalidEntity(error.to_string()))?;
        let template = series.template();
        connection
            .execute(
                "INSERT INTO recurrence_series (id, category_id, start_date, data) VALUES (?1, ?2, ?3, ?4)",
                params![
                    series.id().to_string(),
                    template.category_id.to_string(),
                    template.date.to_string(),
                    data,
                ],
            )
            .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    }
    Ok(())
}

fn insert_exceptions(
    connection: &Connection,
    exceptions: &[RecurrenceException],
) -> Result<(), StorageError> {
    for exception in exceptions {
        let data = serde_json::to_string(exception)
            .map_err(|error| StorageError::InvalidEntity(error.to_string()))?;
        connection
            .execute(
                "INSERT INTO recurrence_exceptions (series_id, original_date, data) VALUES (?1, ?2, ?3)",
                params![
                    exception.series_id().to_string(),
                    exception.original_date().to_string(),
                    data,
                ],
            )
            .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    }
    Ok(())
}

fn update_settings(connection: &Connection, settings: &Settings) -> Result<(), StorageError> {
    connection.execute("UPDATE settings SET week_start = ?1, clock_format = ?2, timezone = ?3, snap_minutes = ?4, day_start = ?5, day_end = ?6 WHERE id = 1", params![week_start_name(settings.week_starts_on()), clock_format_name(settings.clock_format()), settings.time_zone().as_str(), i64::from(settings.snap_interval().minutes()), settings.day_start().to_string(), settings.day_end().to_string()]).map(|_| ()).map_err(|error| StorageError::Sqlite(error.to_string()))
}

fn update_preferences(
    connection: &Connection,
    preferences: &AppPreferences,
) -> Result<(), StorageError> {
    let view_mode = match preferences.view_mode {
        CalendarViewModePreference::Day => "day",
        CalendarViewModePreference::Week => "week",
    };
    connection
        .execute(
            "UPDATE app_preferences SET view_mode = ?1, category_filter_id = ?2, notifications_enabled = ?3, reduce_motion = ?4, appearance_mode = ?5, light_theme = ?6, dark_theme = ?7, font_family = ?8, font_size = ?9 WHERE id = 1",
            params![
                view_mode,
                preferences.category_filter.map(|id| id.to_string()),
                i64::from(preferences.notifications_enabled),
                i64::from(preferences.reduce_motion),
                appearance_mode_name(preferences.appearance.mode),
                preferences.appearance.light_theme,
                preferences.appearance.dark_theme,
                preferences.appearance.font_family,
                i64::from(preferences.appearance.font_size),
            ],
        )
        .map(|_| ())
        .map_err(|error| StorageError::Sqlite(error.to_string()))
}

fn appearance_mode_name(mode: AppearanceMode) -> &'static str {
    match mode {
        AppearanceMode::System => "system",
        AppearanceMode::Light => "light",
        AppearanceMode::Dark => "dark",
    }
}

fn appearance_from_values(
    mode: &str,
    light_theme: &str,
    dark_theme: &str,
    font_family: &str,
    font_size: i64,
) -> Result<AppearancePreferences, StorageError> {
    let mode = match mode {
        "system" => AppearanceMode::System,
        "light" => AppearanceMode::Light,
        "dark" => AppearanceMode::Dark,
        value => {
            return Err(StorageError::InvalidEntity(format!(
                "unknown appearance mode '{value}'"
            )));
        }
    };
    let font_size = u16::try_from(font_size)
        .ok()
        .filter(|size| AppearancePreferences::FONT_SIZES.contains(size))
        .ok_or_else(|| StorageError::InvalidEntity("unsupported font size".to_owned()))?;
    Ok(AppearancePreferences {
        mode,
        light_theme: light_theme.to_owned(),
        dark_theme: dark_theme.to_owned(),
        font_family: font_family.to_owned(),
        font_size,
    })
}

fn settings_from_values(
    week_start: &str,
    clock_format: &str,
    timezone: &str,
    snap_minutes: i64,
    day_start: &str,
    day_end: &str,
) -> Result<Settings, StorageError> {
    let week_start = match week_start {
        "sunday" => WeekStart::Sunday,
        "monday" => WeekStart::Monday,
        value => {
            return Err(StorageError::InvalidEntity(format!(
                "unknown week start '{value}'"
            )));
        }
    };
    let clock_format = match clock_format {
        "12h" => ClockFormat::TwelveHour,
        "24h" => ClockFormat::TwentyFourHour,
        value => {
            return Err(StorageError::InvalidEntity(format!(
                "unknown clock format '{value}'"
            )));
        }
    };
    let snap_minutes = u16::try_from(snap_minutes).map_err(|_| {
        StorageError::InvalidEntity("snap interval is outside the supported range".to_owned())
    })?;
    let snap_minutes = SnapInterval::new(snap_minutes)
        .map_err(|error| StorageError::InvalidEntity(error.to_string()))?;
    let timezone = TimeZoneId::new(timezone)
        .map_err(|error| StorageError::InvalidEntity(error.to_string()))?;
    let day_start = day_start
        .parse::<Time>()
        .map_err(|error| StorageError::InvalidEntity(error.to_string()))?;
    let day_end = day_end
        .parse::<Time>()
        .map_err(|error| StorageError::InvalidEntity(error.to_string()))?;
    Settings::new(
        week_start,
        clock_format,
        timezone,
        snap_minutes,
        day_start,
        day_end,
    )
    .map_err(|error| StorageError::InvalidEntity(error.to_string()))
}

fn category_from_values(
    id: &str,
    name: &str,
    color: &str,
    visible: bool,
) -> Result<Category, StorageError> {
    let id = id
        .parse::<Uuid>()
        .map(CategoryId::from_uuid)
        .map_err(|error| StorageError::InvalidEntity(format!("invalid category ID: {error}")))?;
    let color = parse_color(color)?;
    Category::new(id, name, color, visible)
        .map_err(|error| StorageError::InvalidEntity(error.to_string()))
}

type EventRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<i64>,
    String,
    String,
);

fn event_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EventRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
    ))
}

fn event_from_values(values: EventRow) -> Result<Event, StorageError> {
    let (
        id,
        category_id,
        title,
        date,
        start_time,
        end_time,
        notes,
        reminder_minutes,
        created_at,
        updated_at,
    ) = values;
    let id = id
        .parse::<Uuid>()
        .map(EventId::from_uuid)
        .map_err(|error| StorageError::InvalidEntity(format!("invalid event ID: {error}")))?;
    let category_id = category_id
        .parse::<Uuid>()
        .map(CategoryId::from_uuid)
        .map_err(|error| {
            StorageError::InvalidEntity(format!("invalid event category ID: {error}"))
        })?;
    let date = date
        .parse::<Date>()
        .map_err(|error| StorageError::InvalidEntity(format!("invalid event date: {error}")))?;
    let start_time = start_time.parse::<Time>().map_err(|error| {
        StorageError::InvalidEntity(format!("invalid event start time: {error}"))
    })?;
    let end_time = end_time
        .parse::<Time>()
        .map_err(|error| StorageError::InvalidEntity(format!("invalid event end time: {error}")))?;
    let created_at = created_at.parse::<Timestamp>().map_err(|error| {
        StorageError::InvalidEntity(format!("invalid event creation timestamp: {error}"))
    })?;
    let updated_at = updated_at.parse::<Timestamp>().map_err(|error| {
        StorageError::InvalidEntity(format!("invalid event update timestamp: {error}"))
    })?;
    Event::from_persisted(
        id,
        EventDraft::new(title, date, start_time, end_time, category_id, notes).with_reminder(
            reminder_minutes
                .map(|minutes| {
                    u16::try_from(minutes)
                        .map_err(|_| {
                            StorageError::InvalidEntity("invalid reminder offset".to_owned())
                        })
                        .and_then(|minutes| {
                            ReminderOffset::new(minutes)
                                .map_err(|error| StorageError::InvalidEntity(error.to_string()))
                        })
                })
                .transpose()?,
        ),
        created_at,
        updated_at,
    )
    .map_err(|error| StorageError::InvalidEntity(error.to_string()))
}

fn parse_color(value: &str) -> Result<CategoryColor, StorageError> {
    match value {
        "lime" => Ok(CategoryColor::Lime),
        "yellow" => Ok(CategoryColor::Yellow),
        "coral" => Ok(CategoryColor::Coral),
        "violet" => Ok(CategoryColor::Violet),
        "cyan" => Ok(CategoryColor::Cyan),
        "blue" => Ok(CategoryColor::Blue),
        "orange" => Ok(CategoryColor::Orange),
        "rose" => Ok(CategoryColor::Rose),
        "magenta" => Ok(CategoryColor::Magenta),
        "indigo" => Ok(CategoryColor::Indigo),
        "teal" => Ok(CategoryColor::Teal),
        "slate" => Ok(CategoryColor::Slate),
        value => Err(StorageError::InvalidEntity(format!(
            "unknown category color '{value}'"
        ))),
    }
}

const fn color_name(color: CategoryColor) -> &'static str {
    match color {
        CategoryColor::Lime => "lime",
        CategoryColor::Yellow => "yellow",
        CategoryColor::Coral => "coral",
        CategoryColor::Violet => "violet",
        CategoryColor::Cyan => "cyan",
        CategoryColor::Blue => "blue",
        CategoryColor::Orange => "orange",
        CategoryColor::Rose => "rose",
        CategoryColor::Magenta => "magenta",
        CategoryColor::Indigo => "indigo",
        CategoryColor::Teal => "teal",
        CategoryColor::Slate => "slate",
    }
}

const fn week_start_name(week_start: WeekStart) -> &'static str {
    match week_start {
        WeekStart::Sunday => "sunday",
        WeekStart::Monday => "monday",
    }
}
const fn clock_format_name(format: ClockFormat) -> &'static str {
    match format {
        ClockFormat::TwelveHour => "12h",
        ClockFormat::TwentyFourHour => "24h",
    }
}

#[allow(clippy::needless_pass_by_value)]
fn sqlite_error(error: rusqlite::Error) -> StorageError {
    StorageError::Sqlite(error.to_string())
}

fn map_repository_error(error: StorageError) -> RepositoryError {
    let message = error.to_string();
    if message.contains("UNIQUE constraint failed: events.id") {
        RepositoryError::DuplicateEvent
    } else if message.contains("UNIQUE constraint failed: recurrence_series.id") {
        RepositoryError::DuplicateSeries
    } else if message.contains("UNIQUE constraint failed: categories.id") {
        RepositoryError::DuplicateCategory
    } else if message.contains("FOREIGN KEY constraint failed") {
        RepositoryError::CategoryNotFound
    } else {
        RepositoryError::from(error)
    }
}
