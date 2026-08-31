use std::{collections::HashMap, path::PathBuf, time::Duration};

use gpui::{AppContext as _, Context, SystemNotification, SystemNotificationAction, Window};
use gpui_component::{
    IndexPath,
    select::{SelectEvent, SelectState},
};
use jiff::{SignedDuration, Timestamp, tz::TimeZone};

use crate::{
    calendar::{CalendarState, CalendarViewMode, CategoryFilter},
    domain::Settings,
    domain::{DateRange, format_time},
    store::{
        AppearancePreferences, InMemoryRepository, StorageClient, StorageError,
        TimetableRepository, database_path, default_categories,
    },
};

use super::super::{presentation::local_date_time, toolbar::FilterOption};

use super::{
    CadenceView, CalendarHistory, HistoryEffect, PersistenceState, ReminderTarget,
    viewport::SurfaceViewportState,
};

const REMINDER_CATCH_UP_MINUTES: i64 = 5;

impl CadenceView {
    pub(in crate::app) fn new(window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let settings = Settings::default();
        let now = Timestamp::now();
        let (today, _) = local_date_time(now, &settings);
        let week_visible_start = today;
        let week_buffer_start = super::viewport::shift_date(
            week_visible_start,
            -i32::try_from(super::viewport::WEEK_BUFFER_DAYS).expect("week buffer fits in i32"),
        )
        .unwrap_or(week_visible_start);
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
            day_plan_open: false,
            day_plan_focus: cx.focus_handle(),
            week_viewport_focus: cx.focus_handle(),
            day_plan_previous_focus: None,
            category_filter,
            day_viewport: SurfaceViewportState::new(),
            week_viewport: SurfaceViewportState::new(),
            day_surface_width: 400.0,
            week_surface_width: 720.0,
            week_visible_start,
            week_buffer_start,
            week_scroll_sync_scheduled: false,
            snapshot: None,
            now,
            pending_scroll_minutes: None,
            error: None,
            last_category: None,
            notifications_enabled: false,
            reduce_motion: false,
            appearance: AppearancePreferences::default(),
            delivered_reminders: HashMap::new(),
            reminder_check_at: now,
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
        if !self.notifications_enabled
            || !matches!(
                self.persistence_state,
                PersistenceState::Ready | PersistenceState::Writing
            )
        {
            return;
        }
        let now = self.now;
        let catch_up_start = now
            .checked_sub(SignedDuration::from_mins(REMINDER_CATCH_UP_MINUTES))
            .unwrap_or(now);
        let window_start = self.reminder_check_at.max(catch_up_start);
        self.reminder_check_at = now;
        let (today, _) = local_date_time(now, &self.settings);
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
            if due <= window_start || due > now || self.delivered_reminders.contains_key(&tag) {
                continue;
            }
            let category = self
                .repository
                .category(event.category_id())
                .ok()
                .flatten()
                .map_or_else(
                    || "Uncategorized".to_owned(),
                    |category| category.name().to_owned(),
                );
            cx.show_system_notification(SystemNotification {
                tag: tag.clone().into(),
                title: event.title().into(),
                body: Self::reminder_body(
                    &category,
                    event.date(),
                    event.start_time(),
                    reminder.minutes(),
                    today,
                    self.settings.clock_format(),
                )
                .into(),
                actions: vec![SystemNotificationAction {
                    id: "open-event".into(),
                    label: "View event".into(),
                }],
            });
            self.delivered_reminders.insert(
                tag,
                ReminderTarget {
                    occurrence_id: event.id(),
                    date: event.date(),
                },
            );
        }
    }

    pub(in crate::app) fn handle_notification_response(
        &mut self,
        tag: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(target) = self.delivered_reminders.remove(tag) else {
            return;
        };
        self.open_notification_target(target, window, cx);
    }

    fn open_notification_target(
        &mut self,
        target: ReminderTarget,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !matches!(
            self.persistence_state,
            PersistenceState::Ready | PersistenceState::Writing
        ) {
            return;
        }

        let should_persist = matches!(self.persistence_state, PersistenceState::Ready);
        let before = should_persist
            .then(|| self.repository.snapshot().ok())
            .flatten();
        let rollback = should_persist.then(|| self.rollback_view_state());

        self.event_selection = super::EventSelection::Single;
        if !self.day_plan_open {
            self.day_plan_previous_focus = window.focused(cx);
        }
        self.pending_scroll_minutes = Some(self.current_scroll_minutes());
        self.state.select_date(target.date);
        self.state
            .set_view_mode(crate::calendar::CalendarViewMode::Day);
        self.day_plan_open = true;
        self.reset_scroll_initialization();
        self.refresh_snapshot();
        self.day_plan_focus.focus(window, cx);

        if should_persist {
            self.inspect_event(target.occurrence_id, target.date, window, cx);
        }

        if let (Some(before), Some(rollback)) = (before, rollback) {
            let _ = self.repository.replace_preferences(self.preferences());
            self.persist_snapshot(before, rollback, HistoryEffect::None, cx);
        }
        cx.notify();
    }

    fn reminder_body(
        category: &str,
        date: jiff::civil::Date,
        start_time: jiff::civil::Time,
        reminder_minutes: u16,
        today: jiff::civil::Date,
        clock_format: crate::domain::ClockFormat,
    ) -> String {
        let day_label = if date == today {
            "Today".to_owned()
        } else if date == today.tomorrow().unwrap_or(today) {
            "Tomorrow".to_owned()
        } else {
            date.strftime("%a, %b %-d").to_string()
        };
        let notice = match reminder_minutes {
            0 => "Starting now".to_owned(),
            1 => "1 minute reminder".to_owned(),
            minutes => format!("{minutes} minute reminder"),
        };
        format!(
            "{category} · {day_label} at {} · {notice}",
            format_time(start_time, clock_format)
        )
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
                    CalendarViewMode::Week,
                );
                self.set_week_window_start(today);
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
                self.reminder_check_at = self.now;
                self.delivered_reminders.clear();
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

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{AppContext as _, Entity, TestAppContext};
    use gpui_component::Root;
    use jiff::civil::{Date, Time};

    use super::CadenceView;
    use crate::{
        app::presentation::local_date_time,
        app::state::viewport::{WEEK_BUFFER_DAYS, WEEK_VISIBLE_DAYS, shift_date},
        calendar::{CalendarViewMode, CategoryFilter},
        domain::ClockFormat,
        store::TimetableRepository,
    };

    #[test]
    fn reminder_body_uses_human_readable_context() {
        let today = Date::constant(2026, 8, 27);
        let start = Time::constant(7, 45, 0, 0);

        assert_eq!(
            CadenceView::reminder_body("Routine", today, start, 15, today, ClockFormat::TwelveHour,),
            "Routine · Today at 07:45 AM · 15 minute reminder"
        );
    }

    #[test]
    fn reminder_body_labels_tomorrow_and_zero_minute_reminders() {
        let today = Date::constant(2026, 8, 27);
        let tomorrow = today.tomorrow().expect("valid date");
        let start = Time::constant(9, 0, 0, 0);

        assert_eq!(
            CadenceView::reminder_body(
                "Focus",
                tomorrow,
                start,
                0,
                today,
                ClockFormat::TwentyFourHour,
            ),
            "Focus · Tomorrow at 09:00 · Starting now"
        );
    }

    #[gpui::test]
    fn apply_loaded_restores_filter_and_anchors_the_week_on_today(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let calendar = Rc::new(RefCell::new(None::<Entity<CadenceView>>));
        let captured_calendar = Rc::clone(&calendar);
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| CadenceView::new(window, cx));
            captured_calendar.borrow_mut().replace(view.clone());
            Root::new(view, window, cx)
        });
        let calendar = calendar.borrow().clone().expect("calendar view");

        let (persisted, expected_filter) = calendar.read_with(cx, |view, _| {
            let mut snapshot = view.repository.snapshot().unwrap();
            let category_id = snapshot.categories.first().expect("seed category").id();
            snapshot.preferences.view_mode = crate::store::CalendarViewModePreference::Day;
            snapshot.preferences.category_filter = Some(category_id);
            (snapshot, category_id)
        });

        calendar.update_in(cx, |view, window, app| {
            view.apply_loaded(Ok(persisted), window, app);
        });

        calendar.read_with(cx, |view, _| {
            let (today, _) = local_date_time(view.now, &view.settings);
            assert_eq!(view.state.view_mode(), CalendarViewMode::Week);
            assert_eq!(view.state.selected_date(), today);
            assert_eq!(
                view.state.category_filter(),
                CategoryFilter::Only(expected_filter)
            );
            assert_eq!(view.visible_week_range().unwrap().start(), today);
            assert_eq!(
                view.visible_week_range().unwrap().end(),
                shift_date(
                    today,
                    i32::try_from(WEEK_VISIBLE_DAYS).expect("visible days fit in i32"),
                )
                .expect("visible range end"),
            );
            assert_eq!(
                view.week_buffer_start,
                shift_date(
                    today,
                    -i32::try_from(WEEK_BUFFER_DAYS).expect("buffer days fit in i32"),
                )
                .expect("buffer start"),
            );
            let expected_horizontal =
                f32::from(u16::try_from(WEEK_BUFFER_DAYS).expect("buffer days fit in u16")) * 120.0;
            assert!(
                (view.initial_scroll_offset(CalendarViewMode::Week, 120.0).0 - expected_horizontal)
                    .abs()
                    < f32::EPSILON
            );
        });
    }

    #[gpui::test]
    fn period_navigation_moves_the_today_first_window_by_seven_days(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let calendar = Rc::new(RefCell::new(None::<Entity<CadenceView>>));
        let captured_calendar = Rc::clone(&calendar);
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| CadenceView::new(window, cx));
            captured_calendar.borrow_mut().replace(view.clone());
            Root::new(view, window, cx)
        });
        let calendar = calendar.borrow().clone().expect("calendar view");

        let window_start = Date::constant(2026, 8, 27);
        let selected_date = Date::constant(2026, 8, 30);
        calendar.update_in(cx, |view, _, app| {
            view.set_week_window_start(window_start);
            view.state.select_date(selected_date);
            view.state.set_view_mode(CalendarViewMode::Day);
            view.refresh_snapshot();
            view.shift_period(true, app);
        });

        calendar.read_with(cx, |view, _| {
            assert_eq!(view.week_visible_start, Date::constant(2026, 9, 3));
            assert_eq!(view.state.selected_date(), Date::constant(2026, 9, 6));
            assert_eq!(view.state.view_mode(), CalendarViewMode::Day);
        });

        calendar.update_in(cx, |view, _, app| view.shift_period(false, app));
        calendar.read_with(cx, |view, _| {
            assert_eq!(view.week_visible_start, window_start);
            assert_eq!(view.state.selected_date(), selected_date);
        });
    }

    #[gpui::test]
    fn go_to_today_reanchors_the_week_on_today(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let calendar = Rc::new(RefCell::new(None::<Entity<CadenceView>>));
        let captured_calendar = Rc::clone(&calendar);
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| CadenceView::new(window, cx));
            captured_calendar.borrow_mut().replace(view.clone());
            Root::new(view, window, cx)
        });
        let calendar = calendar.borrow().clone().expect("calendar view");

        calendar.update_in(cx, |view, _, app| {
            view.set_week_window_start(Date::constant(2026, 8, 27));
            view.state.select_date(Date::constant(2026, 8, 30));
            view.refresh_snapshot();
            view.go_to_today(app);
        });

        calendar.read_with(cx, |view, _| {
            let (today, _) = local_date_time(view.now, &view.settings);
            assert_eq!(view.week_visible_start, today);
            assert_eq!(view.state.selected_date(), today);
        });
    }
}
