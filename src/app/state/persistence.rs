use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use gpui::{Context, Window};
use gpui_component::WindowExt as _;

use crate::{
    app::history::EventChange,
    calendar::{CalendarViewMode, CategoryFilter},
    store::{
        AppPreferences, CalendarViewModePreference, InMemoryRepository, StorageError,
        TimetableRepository,
    },
};

use super::{CadenceView, RollbackViewState};

#[derive(Debug, Clone)]
pub(in crate::app) enum HistoryEffect {
    None,
    Record(EventChange),
    Undo(EventChange),
    Redo(EventChange),
}

#[derive(Debug, Clone)]
pub(super) struct PendingWrite {
    rollback: crate::store::PersistenceSnapshot,
    view_state: RollbackViewState,
    effect: HistoryEffect,
}

/// Lifecycle state shown while the local database is opened or written.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(in crate::app) enum PersistenceState {
    /// The database is being opened or migrated.
    Opening,
    /// The database is available for normal use.
    Ready,
    /// A write is being committed.
    Writing,
    /// The database needs user-directed recovery.
    Recovery(StorageError),
}

impl CadenceView {
    pub(in crate::app) fn retry_storage(&mut self, window: &Window, cx: &Context<'_, Self>) {
        self.persistence_state = PersistenceState::Opening;
        self.error = None;
        self.snapshot = None;
        let storage = self.storage.clone();
        let weak_view = cx.entity().downgrade();
        cx.spawn_in(window, async move |_, cx| {
            let result = storage
                .load()
                .recv()
                .await
                .map_err(|_| StorageError::Io("storage worker stopped unexpectedly".to_owned()))
                .and_then(std::convert::identity);
            let _ = weak_view.update_in(cx, |view, window, cx| {
                view.apply_loaded(result, window, cx);
            });
        })
        .detach();
    }

    pub(in crate::app) fn archive_and_start_fresh(
        &self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !matches!(self.persistence_state, PersistenceState::Recovery(_)) {
            return;
        }
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let owner = owner.clone();
            alert
                .title("Archive database and start fresh?")
                .description("The unreadable database will be moved into a recovery folder. This cannot be undone from Cadence.")
                .button_props(
                    gpui_component::dialog::DialogButtonProps::default()
                        .ok_text("Archive and start fresh")
                        .ok_variant(gpui_component::button::ButtonVariant::Danger)
                        .cancel_text("Keep database")
                        .show_cancel(true),
                )
                .on_ok(move |_, window, app| {
                    owner
                        .update(app, |view, cx| view.start_archive(window, cx))
                        .ok();
                    true
                })
        });
    }

    fn start_archive(&mut self, window: &Window, cx: &Context<'_, Self>) {
        self.persistence_state = PersistenceState::Opening;
        self.error = None;
        self.snapshot = None;
        let storage = self.storage.clone();
        let weak_view = cx.entity().downgrade();
        cx.spawn_in(window, async move |_, cx| {
            let result = storage
                .archive_and_start_fresh()
                .recv()
                .await
                .map_err(|_| StorageError::Io("storage worker stopped unexpectedly".to_owned()))
                .and_then(std::convert::identity);
            let _ = weak_view.update_in(cx, |view, window, cx| {
                view.apply_loaded(result, window, cx);
            });
        })
        .detach();
    }

    pub(in crate::app) fn export_backup(&self, window: &Window, cx: &Context<'_, Self>) {
        if !self.is_interactive() {
            return;
        }
        let directory = self
            .storage_path
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let receiver = cx.prompt_for_new_path(&directory, Some("cadence-backup.json"));
        let storage = self.storage.clone();
        let weak_view = cx.entity().downgrade();
        cx.spawn_in(window, async move |_, cx| {
            let selected = receiver.await.ok().and_then(Result::ok).flatten();
            let Some(path) = selected else {
                return;
            };
            let result = storage
                .export_json()
                .recv()
                .await
                .map_err(|_| StorageError::Io("storage worker stopped unexpectedly".to_owned()))
                .and_then(std::convert::identity)
                .and_then(|json| write_backup_atomically(&path, &json));
            let _ = weak_view.update(cx, |view, cx| match result {
                Ok(()) => {
                    view.error = None;
                    cx.notify();
                }
                Err(error) => {
                    view.error = Some(format!("Could not export backup: {error}"));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(in crate::app) const fn preferences(&self) -> AppPreferences {
        AppPreferences {
            view_mode: match self.state.view_mode() {
                CalendarViewMode::Day => CalendarViewModePreference::Day,
                CalendarViewMode::Week => CalendarViewModePreference::Week,
            },
            category_filter: match self.state.category_filter() {
                CategoryFilter::All => None,
                CategoryFilter::Only(id) => Some(id),
            },
        }
    }

    pub(in crate::app) fn persist_snapshot(
        &mut self,
        before: crate::store::PersistenceSnapshot,
        view_state: RollbackViewState,
        effect: HistoryEffect,
        cx: &Context<'_, Self>,
    ) {
        if matches!(self.persistence_state, PersistenceState::Writing) {
            return;
        }
        let Ok(after) = self.repository.snapshot() else {
            return;
        };
        self.persistence_state = PersistenceState::Writing;
        self.pending_write = Some(PendingWrite {
            rollback: before,
            view_state,
            effect,
        });
        let storage = self.storage.clone();
        let weak_view = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let result = storage
                .replace(after)
                .recv()
                .await
                .map_err(|_| StorageError::Io("storage worker stopped unexpectedly".to_owned()))
                .and_then(std::convert::identity);
            let _ = weak_view.update(cx, |view, cx| {
                view.finish_persist(result, cx);
            });
        })
        .detach();
    }

    fn finish_persist(&mut self, result: Result<(), StorageError>, cx: &mut Context<'_, Self>) {
        match result {
            Ok(()) => {
                if let Some(pending) = self.pending_write.take() {
                    match pending.effect {
                        HistoryEffect::None => {}
                        HistoryEffect::Record(change) => self.history.record(change),
                        HistoryEffect::Undo(change) => {
                            let _ = self.history.finish_undo(&change);
                        }
                        HistoryEffect::Redo(change) => {
                            let _ = self.history.finish_redo(&change);
                        }
                    }
                }
                self.persistence_state = PersistenceState::Ready;
                self.error = None;
            }
            Err(error) => {
                if let Some(pending) = self.pending_write.take()
                    && let Ok(repository) = InMemoryRepository::from_snapshot(&pending.rollback)
                {
                    self.settings = pending.rollback.settings.clone();
                    self.repository = repository;
                    self.state.set_category_filter(
                        pending
                            .rollback
                            .preferences
                            .category_filter
                            .map_or(CategoryFilter::All, CategoryFilter::Only),
                    );
                    self.state
                        .set_view_mode(match pending.rollback.preferences.view_mode {
                            CalendarViewModePreference::Day => CalendarViewMode::Day,
                            CalendarViewModePreference::Week => CalendarViewMode::Week,
                        });
                    self.state = pending.view_state.calendar_state;
                    self.last_category = pending.view_state.last_category;
                    self.scroll_handle
                        .set_offset(pending.view_state.scroll_offset);
                    self.scroll_initialized = pending.view_state.scroll_initialized;
                    self.pending_scroll_minutes = pending.view_state.pending_scroll_minutes;
                    self.refresh_snapshot();
                }
                self.persistence_state = PersistenceState::Ready;
                self.error = Some(format!("Could not save changes: {error}"));
            }
        }
        cx.notify();
    }
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
