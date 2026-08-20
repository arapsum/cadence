use std::time::Duration;

use gpui::{AppContext as _, Context, Entity, ScrollHandle, Subscription, Window};
use gpui_component::{
    IndexPath,
    select::{SelectEvent, SelectState},
};
use jiff::{Timestamp, civil::Date};

use crate::{
    calendar::{CalendarState, CalendarViewMode, CategoryFilter},
    domain::{EventId, Settings},
    store::{InMemoryRepository, TimetableRepository, seed_sample_week},
};

use super::{
    presentation::{
        CalendarSnapshot, day_index, event_matches_filter, layout_events, local_date_time,
    },
    style::PIXELS_PER_MINUTE,
    toolbar::FilterOption,
};

pub(super) struct CadenceView {
    pub(super) repository: InMemoryRepository,
    pub(super) settings: Settings,
    pub(super) state: CalendarState,
    pub(super) category_filter: Entity<SelectState<Vec<FilterOption>>>,
    pub(super) scroll_handle: ScrollHandle,
    pub(super) snapshot: Option<CalendarSnapshot>,
    pub(super) now: Timestamp,
    pub(super) scroll_initialized: bool,
    pub(super) pending_scroll_minutes: Option<f32>,
    pub(super) error: Option<String>,
    pub(super) subscriptions: Vec<Subscription>,
}

impl CadenceView {
    pub(super) fn new(window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let settings = Settings::default();
        let now = Timestamp::now();
        let (today, _) = local_date_time(now, &settings);
        let mut repository = InMemoryRepository::new(settings.clone());
        let error = if let Err(seed_error) = seed_sample_week(&mut repository, today, now) {
            Some(format!("Could not load sample week: {seed_error}"))
        } else {
            None
        };

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
            settings,
            state,
            category_filter,
            scroll_handle: ScrollHandle::new(),
            snapshot: None,
            now,
            scroll_initialized: false,
            pending_scroll_minutes: None,
            error,
            subscriptions: Vec::new(),
        };
        this.refresh_snapshot();

        let category_filter_entity = this.category_filter.clone();
        this.subscriptions.push(cx.subscribe(
            &category_filter_entity,
            |this, _, event: &SelectEvent<Vec<FilterOption>>, cx| {
                if let SelectEvent::Confirm(Some(filter)) = event {
                    this.state.set_category_filter(*filter);
                    this.state.clear_selection();
                    this.scroll_initialized = false;
                    this.refresh_snapshot();
                    cx.notify();
                }
            },
        ));

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

    pub(super) fn refresh_snapshot(&mut self) {
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
        self.now = Timestamp::now();
        let (today, _) = local_date_time(self.now, &self.settings);
        self.state.go_to_today(today);
        self.pending_scroll_minutes = None;
        self.scroll_initialized = false;
        self.refresh_snapshot();
        cx.notify();
    }

    pub(super) fn shift_period(&mut self, next: bool, cx: &mut Context<'_, Self>) {
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

    pub(super) fn select_date(&mut self, date: Date, cx: &mut Context<'_, Self>) {
        self.state.select_date(date);
        self.pending_scroll_minutes = Some(self.current_scroll_minutes());
        self.scroll_initialized = false;
        self.refresh_snapshot();
        cx.notify();
    }

    pub(super) fn select_event(
        &mut self,
        event_id: EventId,
        date: Date,
        cx: &mut Context<'_, Self>,
    ) {
        self.state.select_event(event_id, date);
        cx.notify();
    }

    pub(super) fn set_view_mode(
        &mut self,
        view_mode: CalendarViewMode,
        cx: &mut Context<'_, Self>,
    ) {
        if self.state.view_mode() == view_mode {
            return;
        }
        self.pending_scroll_minutes = Some(self.current_scroll_minutes());
        self.state.set_view_mode(view_mode);
        self.scroll_initialized = false;
        self.refresh_snapshot();
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
}
