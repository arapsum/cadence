use std::{path::PathBuf, time::Duration};

use gpui::{AppContext as _, Context, ScrollHandle, Window};
use gpui_component::{
    IndexPath,
    select::{SelectEvent, SelectState},
};
use jiff::Timestamp;

use crate::{
    calendar::{CalendarState, CalendarViewMode, CategoryFilter},
    domain::Settings,
    store::{
        CalendarViewModePreference, InMemoryRepository, StorageClient, StorageError,
        TimetableRepository, database_path, default_categories,
    },
};

use super::super::{presentation::local_date_time, toolbar::FilterOption};

use super::{CadenceView, EventHistory, HistoryEffect, PersistenceState};

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

        this.subscribe_category_filter(cx);

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
}
