use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

use gpui::{
    AppContext as _, Context, DragMoveEvent, Entity, Pixels, Point, ScrollHandle, Subscription,
    Window,
};
use gpui_component::{
    IndexPath, WindowExt as _,
    select::{SelectEvent, SelectState},
};
use jiff::{Timestamp, civil::Date};

use crate::{
    calendar::{CalendarState, CalendarViewMode, CategoryFilter},
    domain::Settings,
    store::{
        AppPreferences, CalendarViewModePreference, InMemoryRepository, StorageClient,
        StorageError, TimetableRepository, database_path, default_categories,
    },
};

use super::{
    history::{ChangeKind, EventChange, EventHistory},
    interaction::{DragPayload, Manipulation, ManipulationKind, ManipulationUpdate},
    presentation::{
        CalendarSnapshot, day_index, event_matches_filter, layout_events, local_date_time,
    },
    style::PIXELS_PER_MINUTE,
    toolbar::FilterOption,
};

pub(super) struct CadenceView {
    pub(super) repository: InMemoryRepository,
    pub(super) storage: StorageClient,
    pub(super) storage_path: PathBuf,
    pub(super) persistence_state: PersistenceState,
    pending_write: Option<PendingWrite>,
    pub(super) manipulation: Option<Manipulation>,
    manipulation_rollback: Option<RollbackViewState>,
    pub(super) history: EventHistory,
    pub(super) settings: Settings,
    pub(super) state: CalendarState,
    pub(super) category_filter: Entity<SelectState<Vec<FilterOption>>>,
    pub(super) scroll_handle: ScrollHandle,
    pub(super) snapshot: Option<CalendarSnapshot>,
    pub(super) now: Timestamp,
    pub(super) scroll_initialized: bool,
    pub(super) pending_scroll_minutes: Option<f32>,
    pub(super) error: Option<String>,
    pub(super) last_category: Option<crate::domain::CategoryId>,
    pub(super) subscriptions: Vec<Subscription>,
}

#[derive(Debug, Clone)]
pub(super) enum HistoryEffect {
    None,
    Record(EventChange),
    Undo(EventChange),
    Redo(EventChange),
}

#[derive(Debug, Clone)]
struct PendingWrite {
    rollback: crate::store::PersistenceSnapshot,
    view_state: RollbackViewState,
    effect: HistoryEffect,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct RollbackViewState {
    calendar_state: CalendarState,
    last_category: Option<crate::domain::CategoryId>,
    scroll_offset: Point<gpui::Pixels>,
    scroll_initialized: bool,
    pending_scroll_minutes: Option<f32>,
}

/// Lifecycle state shown while the local database is opened or written.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(super) enum PersistenceState {
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
    #[allow(clippy::too_many_lines)]
    pub(super) fn new(window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let settings = Settings::default();
        let now = Timestamp::now();
        let (today, _) = local_date_time(now, &settings);
        let mut repository = InMemoryRepository::new(settings.clone());
        for category in default_categories() {
            let _ = repository.create_category(category);
        }

        let storage_path = database_path().unwrap_or_else(|_| PathBuf::from("cadence.sqlite3"));
        let storage = StorageClient::spawn(storage_path.clone());
        #[cfg(not(test))]
        let load_storage = storage.clone();

        let categories = repository.categories().unwrap_or_default();
        let filter_options = std::iter::once(FilterOption::all())
            .chain(categories.iter().map(|category| FilterOption {
                filter: CategoryFilter::Only(category.id()),
                label: category.name().into(),
                color: Some(category.color_token()),
            }))
            .collect::<Vec<_>>();
        let category_filter = cx.new(|cx| {
            SelectState::new(
                filter_options,
                Some(IndexPath::default().row(0)),
                window,
                cx,
            )
        });

        let state = CalendarState::new(today, settings.week_starts_on(), CalendarViewMode::Week);
        let mut this = Self {
            repository,
            storage,
            storage_path,
            persistence_state: PersistenceState::Opening,
            pending_write: None,
            manipulation: None,
            manipulation_rollback: None,
            history: EventHistory::new(),
            settings,
            state,
            category_filter,
            scroll_handle: ScrollHandle::new(),
            snapshot: None,
            now,
            scroll_initialized: false,
            pending_scroll_minutes: None,
            error: None,
            last_category: None,
            subscriptions: Vec::new(),
        };

        let category_filter_entity = this.category_filter.clone();
        this.subscriptions.push(cx.subscribe(
            &category_filter_entity,
            |this, _, event: &SelectEvent<Vec<FilterOption>>, cx| {
                if let SelectEvent::Confirm(Some(filter)) = event {
                    if !this.is_interactive() {
                        return;
                    }
                    let rollback = this.rollback_view_state();
                    let before = this.repository.snapshot().ok();
                    this.state.set_category_filter(*filter);
                    this.state.clear_selection();
                    this.scroll_initialized = false;
                    this.refresh_snapshot();
                    let _ = this.repository.replace_preferences(this.preferences());
                    if let Some(before) = before {
                        this.persist_snapshot(before, rollback, HistoryEffect::None, cx);
                    }
                    cx.notify();
                }
            },
        ));

        #[cfg(not(test))]
        cx.spawn_in(window, async move |weak_view, cx| {
            let result = load_storage
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

        #[cfg(test)]
        {
            this.repository = InMemoryRepository::new(this.settings.clone());
            let _ = crate::store::seed_sample_week(&mut this.repository, today, now);
            this.persistence_state = PersistenceState::Ready;
            this.refresh_snapshot();
        }

        cx.spawn(async move |weak_view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(30))
                    .await;
                if weak_view
                    .update(cx, |view, cx| {
                        view.now = Timestamp::now();
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

        this
    }

    fn apply_loaded(
        &mut self,
        result: Result<crate::store::StorageSnapshot, StorageError>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let snapshot = match result {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.persistence_state = PersistenceState::Recovery(error.clone());
                self.error = Some(error.user_message());
                cx.notify();
                return;
            }
        };
        match InMemoryRepository::from_snapshot(&snapshot) {
            Ok(repository) => {
                self.settings = snapshot.settings.clone();
                self.repository = repository;
                let (today, _) = local_date_time(self.now, &self.settings);
                self.state = CalendarState::new(
                    today,
                    self.settings.week_starts_on(),
                    match snapshot.preferences.view_mode {
                        CalendarViewModePreference::Day => CalendarViewMode::Day,
                        CalendarViewModePreference::Week => CalendarViewMode::Week,
                    },
                );
                let filter = snapshot
                    .preferences
                    .category_filter
                    .filter(|id| {
                        snapshot
                            .categories
                            .iter()
                            .any(|category| category.id() == *id)
                    })
                    .map_or(CategoryFilter::All, CategoryFilter::Only);
                self.state.set_category_filter(filter);
                let filter_options = std::iter::once(FilterOption::all())
                    .chain(snapshot.categories.iter().map(|category| FilterOption {
                        filter: CategoryFilter::Only(category.id()),
                        label: category.name().into(),
                        color: Some(category.color_token()),
                    }))
                    .collect::<Vec<_>>();
                self.category_filter.update(cx, |select, cx| {
                    select.set_items(filter_options, window, cx);
                    select.set_selected_value(&filter, window, cx);
                });
                self.persistence_state = PersistenceState::Ready;
                self.error = None;
                self.scroll_initialized = false;
                self.pending_scroll_minutes = None;
                self.refresh_snapshot();
            }
            Err(error) => {
                self.persistence_state =
                    PersistenceState::Recovery(StorageError::InvalidEntity(error.to_string()));
                self.error = Some(error.to_string());
            }
        }
        cx.notify();
    }

    pub(super) fn refresh_snapshot(&mut self) {
        if !matches!(
            self.persistence_state,
            PersistenceState::Ready | PersistenceState::Writing
        ) {
            self.snapshot = None;
            return;
        }
        let range = match self.state.visible_range() {
            Ok(range) => range,
            Err(error) => {
                self.error = Some(error.to_string());
                self.snapshot = None;
                return;
            }
        };

        let categories = match self.repository.categories() {
            Ok(categories) => categories,
            Err(error) => {
                self.error = Some(error.to_string());
                self.snapshot = None;
                return;
            }
        };
        let events = match self.repository.events(range) {
            Ok(events) => events
                .into_iter()
                .filter(|event| event_matches_filter(event, self.state.category_filter()))
                .collect::<Vec<_>>(),
            Err(error) => {
                self.error = Some(error.to_string());
                self.snapshot = None;
                return;
            }
        };
        let positions = match layout_events(&events, range) {
            Ok(positions) => positions,
            Err(error) => {
                self.error = Some(format!("Could not lay out calendar: {error:?}"));
                self.snapshot = None;
                return;
            }
        };

        self.snapshot = Some(CalendarSnapshot {
            range,
            events,
            positions,
            categories,
        });
        self.error = None;
    }

    pub(super) fn go_to_today(&mut self, cx: &mut Context<'_, Self>) {
        if !self.is_interactive() {
            return;
        }
        self.now = Timestamp::now();
        let (today, _) = local_date_time(self.now, &self.settings);
        self.state.go_to_today(today);
        self.pending_scroll_minutes = None;
        self.scroll_initialized = false;
        self.refresh_snapshot();
        cx.notify();
    }

    pub(super) fn shift_period(&mut self, next: bool, cx: &mut Context<'_, Self>) {
        if !self.is_interactive() {
            return;
        }
        let result = if next {
            self.state.next_period()
        } else {
            self.state.previous_period()
        };
        if let Err(error) = result {
            self.error = Some(error.to_string());
        } else {
            self.pending_scroll_minutes = None;
            self.scroll_initialized = false;
            self.refresh_snapshot();
        }
        cx.notify();
    }

    pub(super) fn clear_selection(&mut self, cx: &mut Context<'_, Self>) {
        self.state.clear_selection();
        cx.notify();
    }

    pub(super) fn begin_manipulation(
        &mut self,
        payload: &DragPayload,
        cursor_offset: Point<Pixels>,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.is_interactive() {
            return;
        }
        self.manipulation_rollback = Some(self.rollback_view_state());
        self.state
            .select_event(payload.event.id(), payload.event.date());
        self.manipulation = Some(Manipulation::new(payload, cursor_offset));
        let owner = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                let keep_running = owner
                    .update(cx, |view, cx| {
                        let Some(manipulation) = &mut view.manipulation else {
                            return false;
                        };
                        let delta = manipulation.edge_velocity();
                        if delta.x == gpui::px(0.0) && delta.y == gpui::px(0.0) {
                            return true;
                        }
                        let next_offset = manipulation.scroll_by(delta);
                        view.scroll_handle.set_offset(next_offset);
                        let pointer = manipulation.pointer;
                        let viewport = manipulation.viewport;
                        let plane_width = manipulation.plane_width;
                        let column_width = manipulation.column_width;
                        let column_count = manipulation.column_count;
                        let range = view.snapshot.as_ref().map(|snapshot| snapshot.range);
                        if let Some(range) = range {
                            manipulation.update(ManipulationUpdate {
                                pointer,
                                viewport,
                                scroll_offset: next_offset,
                                plane_width,
                                column_width,
                                column_count,
                                range,
                                snap_minutes: view.settings.snap_interval().minutes(),
                            });
                        }
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !keep_running {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }

    pub(super) fn update_manipulation(
        &mut self,
        event: &DragMoveEvent<DragPayload>,
        column_width: f32,
        plane_width: f32,
        column_count: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(snapshot) = self.snapshot.as_ref() else {
            return;
        };
        let range = snapshot.range;
        let scroll_offset = self.scroll_handle.offset();
        let snap_minutes = self.settings.snap_interval().minutes();
        if let Some(manipulation) = &mut self.manipulation {
            manipulation.update(ManipulationUpdate {
                pointer: event.event.position,
                viewport: event.bounds,
                scroll_offset,
                plane_width,
                column_width,
                column_count,
                range,
                snap_minutes,
            });
            cx.notify();
        }
    }

    pub(super) fn cancel_manipulation(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.manipulation.take().is_some() {
            if let Some(rollback) = self.manipulation_rollback.take() {
                self.restore_view_state(rollback);
            }
            cx.stop_active_drag(window);
            cx.notify();
        }
    }

    pub(super) fn finish_manipulation(
        &mut self,
        payload: &DragPayload,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(manipulation) = self.manipulation.take() else {
            return;
        };
        let rollback = self
            .manipulation_rollback
            .take()
            .unwrap_or_else(|| self.rollback_view_state());
        cx.stop_active_drag(window);
        if manipulation.event.id() != payload.event.id() || !manipulation.changed() {
            self.restore_view_state(rollback);
            cx.notify();
            return;
        }
        let before = match self.repository.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.restore_view_state(rollback);
                self.error = Some(error.to_string());
                cx.notify();
                return;
            }
        };
        let Some(mut event) = self
            .repository
            .event(manipulation.event.id())
            .ok()
            .flatten()
        else {
            self.restore_view_state(rollback);
            self.error = Some("That event is no longer available.".to_owned());
            self.refresh_snapshot();
            cx.notify();
            return;
        };
        let before_draft = event.draft();
        let after_draft = manipulation.proposed;
        if let Err(error) = event.revise(after_draft.clone(), Timestamp::now()) {
            self.restore_view_state(rollback);
            self.error = Some(error.to_string());
            cx.notify();
            return;
        }
        if let Err(error) = self.repository.update_event(event) {
            self.restore_view_state(rollback);
            self.error = Some(error.to_string());
            cx.notify();
            return;
        }
        self.state
            .select_event(manipulation.event.id(), after_draft.date);
        self.last_category = Some(after_draft.category_id);
        self.pending_scroll_minutes = None;
        self.scroll_initialized = false;
        self.refresh_snapshot();
        let kind = match manipulation.kind {
            ManipulationKind::Move => ChangeKind::Move,
            ManipulationKind::Resize(_) => ChangeKind::Resize,
        };
        self.persist_snapshot(
            before,
            rollback,
            HistoryEffect::Record(EventChange::Update {
                id: manipulation.event.id(),
                before: before_draft,
                after: after_draft,
                kind,
            }),
            cx,
        );
        cx.notify();
    }

    pub(super) fn select_date(&mut self, date: Date, cx: &mut Context<'_, Self>) {
        if !self.is_interactive() {
            return;
        }
        self.state.select_date(date);
        self.pending_scroll_minutes = Some(self.current_scroll_minutes());
        self.scroll_initialized = false;
        self.refresh_snapshot();
        cx.notify();
    }

    pub(super) fn select_event(
        &mut self,
        event_id: crate::domain::EventId,
        date: Date,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.is_interactive() {
            return;
        }
        self.state.select_event(event_id, date);
        cx.notify();
    }

    pub(super) fn set_view_mode(
        &mut self,
        view_mode: CalendarViewMode,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.is_interactive() {
            return;
        }
        if self.state.view_mode() == view_mode {
            return;
        }
        let rollback = self.rollback_view_state();
        let before = self.repository.snapshot().ok();
        self.pending_scroll_minutes = Some(self.current_scroll_minutes());
        self.state.set_view_mode(view_mode);
        self.scroll_initialized = false;
        self.refresh_snapshot();
        let _ = self.repository.replace_preferences(self.preferences());
        if let Some(before) = before {
            self.persist_snapshot(before, rollback, HistoryEffect::None, cx);
        }
        cx.notify();
    }

    pub(super) const fn is_interactive(&self) -> bool {
        matches!(self.persistence_state, PersistenceState::Ready) && self.manipulation.is_none()
    }

    pub(super) fn retry_storage(&mut self, window: &Window, cx: &Context<'_, Self>) {
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

    #[allow(clippy::unused_self, clippy::needless_pass_by_ref_mut)]
    pub(super) fn archive_and_start_fresh(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
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

    pub(super) fn export_backup(&self, window: &Window, cx: &Context<'_, Self>) {
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

    pub(super) const fn preferences(&self) -> AppPreferences {
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

    pub(super) fn persist_snapshot(
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

    pub(super) fn range_label(&self) -> String {
        let Some(snapshot) = &self.snapshot else {
            return "No calendar loaded".to_owned();
        };
        if self.state.view_mode() == CalendarViewMode::Day {
            return self
                .state
                .selected_date()
                .strftime("%A, %b %-d, %Y")
                .to_string();
        }
        let last_day = snapshot
            .range
            .end()
            .yesterday()
            .unwrap_or_else(|_| snapshot.range.start());
        let start = snapshot.range.start().strftime("%b %-d");
        let end = last_day.strftime("%b %-d, %Y");
        format!("{start} – {end}")
    }

    pub(super) fn initial_scroll_offset(&mut self, column_width: f32) -> (f32, f32) {
        let pending_scroll_minutes = self.pending_scroll_minutes.take();
        let Some(snapshot) = &self.snapshot else {
            return (0.0, 0.0);
        };
        let target_minutes = pending_scroll_minutes.unwrap_or_else(|| {
            let (today, current_time) = local_date_time(self.now, &self.settings);
            if snapshot.range.contains(today) {
                f32::from(current_time.hour())
                    .mul_add(60.0, f32::from(current_time.minute()) - 90.0)
                    .max(0.0)
            } else {
                snapshot
                    .events
                    .iter()
                    .map(|event| {
                        f32::from(event.start_time().hour())
                            .mul_add(60.0, f32::from(event.start_time().minute()))
                    })
                    .min_by(f32::total_cmp)
                    .map_or(5.0 * 60.0, |minutes| (minutes - 60.0).max(0.0))
            }
        });
        let horizontal = if self.state.view_mode() == CalendarViewMode::Day {
            0.0
        } else {
            day_index(snapshot.range, self.state.selected_date()).map_or(0.0, |day| {
                let day = f32::from(u16::try_from(day).expect("calendar day fits in u16"));
                ((day - 2.0) * column_width).max(0.0)
            })
        };
        (horizontal, target_minutes * PIXELS_PER_MINUTE)
    }

    fn current_scroll_minutes(&self) -> f32 {
        (-self.scroll_handle.offset().y.as_f32() / PIXELS_PER_MINUTE).max(0.0)
    }

    pub(super) fn rollback_view_state(&self) -> RollbackViewState {
        RollbackViewState {
            calendar_state: self.state,
            last_category: self.last_category,
            scroll_offset: self.scroll_handle.offset(),
            scroll_initialized: self.scroll_initialized,
            pending_scroll_minutes: self.pending_scroll_minutes,
        }
    }

    fn restore_view_state(&mut self, view_state: RollbackViewState) {
        self.state = view_state.calendar_state;
        self.last_category = view_state.last_category;
        self.scroll_handle.set_offset(view_state.scroll_offset);
        self.scroll_initialized = view_state.scroll_initialized;
        self.pending_scroll_minutes = view_state.pending_scroll_minutes;
        self.refresh_snapshot();
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
