use std::{collections::HashSet, path::PathBuf, time::Duration};

use gpui::{AppContext as _, Context, SystemNotification, SystemNotificationAction, Window};
use gpui_component::{
    IndexPath,
    select::{SelectEvent, SelectState},
};
use jiff::{SignedDuration, Timestamp, tz::TimeZone};

use crate::{
    calendar::{CalendarState, CalendarViewMode, CategoryFilter},
    domain::DateRange,
    domain::Settings,
    store::{
        AppearancePreferences, CalendarViewModePreference, InMemoryRepository, StorageClient,
        StorageError, TimetableRepository, database_path, default_categories,
    },
};

use super::super::{presentation::local_date_time, toolbar::FilterOption};

use super::{
    CadenceView, CalendarHistory, HistoryEffect, PersistenceState, viewport::SurfaceViewportState,
};

impl CadenceView {
    pub(in crate::app) fn new(window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
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
        let storage_for_load = storage.clone();

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
            storage_task: None,
            pending_write_task: None,
            export_task: None,
            clock_task: None,
            manipulation: None,
            manipulation_rollback: None,
            history: CalendarHistory::new(),
            event_selection: super::EventSelection::default(),
            settings,
            state,
            category_filter,
            day_viewport: SurfaceViewportState::new(),
            week_viewport: SurfaceViewportState::new(),
            day_surface_width: 400.0,
            week_surface_width: 720.0,
            snapshot: None,
            now,
            pending_scroll_minutes: None,
            error: None,
            last_category: None,
            notifications_enabled: false,
            reduce_motion: false,
            appearance: AppearancePreferences::default(),
            delivered_reminders: HashSet::new(),
            subscriptions: Vec::new(),
        };

        this.subscribe_category_filter(cx);
        super::super::appearance::apply(&this.appearance, Some(window), cx);
        this.subscriptions
            .push(cx.observe_window_appearance(window, |view, window, cx| {
                if matches!(view.appearance.mode, crate::store::AppearanceMode::System) {
                    super::super::appearance::apply(&view.appearance, Some(window), cx);
                    cx.notify();
                }
            }));

        #[cfg(not(test))]
        start_storage_load(&mut this, storage_for_load, window, cx);

        #[cfg(test)]
        {
            this.repository = InMemoryRepository::new(this.settings.clone());
            let _ = crate::store::seed_sample_week(&mut this.repository, today, now);
            this.persistence_state = PersistenceState::Ready;
            this.refresh_snapshot();
        }

        start_clock(&mut this, cx);

        this
    }

    fn tick_clock(&mut self, cx: &mut Context<'_, Self>) {
        self.now = Timestamp::now();
        self.deliver_due_reminders(cx);
        cx.notify();
    }

    fn deliver_due_reminders(&mut self, cx: &Context<'_, Self>) {
        if !self.notifications_enabled || !self.is_interactive() {
            return;
        }
        let (today, _) = local_date_time(self.now, &self.settings);
        let end = today
            .tomorrow()
            .and_then(jiff::civil::Date::tomorrow)
            .unwrap_or(today);
        let Ok(range) = DateRange::new(today, end) else {
            return;
        };
        let Ok(events) = self.repository.occurrences(range) else {
            return;
        };
        let timezone = TimeZone::get(self.settings.time_zone().as_str()).unwrap_or(TimeZone::UTC);
        for event in events {
            let Some(reminder) = event.draft().reminder else {
                continue;
            };
            let Ok(start) = event
                .date()
                .to_datetime(event.start_time())
                .to_zoned(timezone.clone())
            else {
                continue;
            };
            let Ok(due) = start
                .timestamp()
                .checked_sub(SignedDuration::from_mins(i64::from(reminder.minutes())))
            else {
                continue;
            };
            let tag = format!("cadence-reminder-{:?}-{}", event.id(), event.date());
            if due <= self.now && self.delivered_reminders.insert(tag.clone()) {
                cx.show_system_notification(SystemNotification {
                    tag: tag.into(),
                    title: event.title().into(),
                    body: format!("{} starts at {}.", event.category_id(), event.start_time())
                        .into(),
                    actions: vec![SystemNotificationAction {
                        id: "open".into(),
                        label: "Open Cadence".into(),
                    }],
                });
            }
        }
    }

    fn subscribe_category_filter(&mut self, cx: &mut Context<'_, Self>) {
        let category_filter_entity = self.category_filter.clone();
        self.subscriptions.push(cx.subscribe(
            &category_filter_entity,
            |this, _, event: &SelectEvent<Vec<FilterOption>>, cx| {
                if let SelectEvent::Confirm(Some(filter)) = event {
                    if !this.is_interactive() {
                        return;
                    }
                    let rollback = this.rollback_view_state();
                    let before = this.repository.snapshot().ok();
                    this.event_selection = super::EventSelection::Single;
                    this.state.set_category_filter(*filter);
                    this.state.clear_selection();
                    this.reset_scroll_initialization();
                    this.refresh_snapshot();
                    let _ = this.repository.replace_preferences(this.preferences());
                    if let Some(before) = before {
                        this.persist_snapshot(before, rollback, HistoryEffect::None, cx);
                    }
                    cx.notify();
                }
            },
        ));
    }

    pub(in crate::app::state) fn apply_loaded(
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
                self.notifications_enabled = snapshot.preferences.notifications_enabled;
                self.reduce_motion = snapshot.preferences.reduce_motion;
                self.appearance =
                    super::super::appearance::normalize(&snapshot.preferences.appearance, cx);
                cx.set_reduce_motion(self.reduce_motion);
                super::super::appearance::apply(&self.appearance, Some(window), cx);
                self.repository = repository;
                self.event_selection = super::EventSelection::Single;
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
                self.reset_scroll_initialization();
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
}

#[cfg(not(test))]
fn start_storage_load(
    view: &mut CadenceView,
    storage: StorageClient,
    window: &Window,
    cx: &Context<'_, CadenceView>,
) {
    view.storage_task = Some(cx.spawn_in(window, async move |weak_view, cx| {
        let result = storage
            .load()
            .recv()
            .await
            .map_err(|_| StorageError::Io("storage worker stopped unexpectedly".to_owned()))
            .and_then(std::convert::identity);
        let _ = weak_view.update_in(cx, |view, window, cx| {
            view.apply_loaded(result, window, cx);
        });
    }));
}

fn start_clock(view: &mut CadenceView, cx: &Context<'_, CadenceView>) {
    view.clock_task = Some(cx.spawn(async move |weak_view, cx| {
        loop {
            cx.background_executor()
                .timer(Duration::from_secs(30))
                .await;
            if weak_view
                .update(cx, |view, cx| {
                    view.tick_clock(cx);
                    cx.notify();
                })
                .is_err()
            {
                break;
            }
        }
    }));
}
