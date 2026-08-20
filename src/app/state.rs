use std::time::Duration;

use gpui::{AppContext as _, Context, Entity, ScrollHandle, Subscription, Window};
use gpui_component::{
    IndexPath,
    select::{SelectEvent, SelectState},
};
use jiff::Timestamp;

use crate::{
    calendar::{CalendarState, CategoryFilter},
    domain::{DateRange, EventId, Settings},
    store::{InMemoryRepository, TimetableRepository, seed_sample_week},
};

use super::{
    presentation::{WeekSnapshot, day_index, event_matches_filter, layout_events, local_date_time},
    style::PIXELS_PER_MINUTE,
    toolbar::FilterOption,
};

pub(super) struct CadenceView {
    pub(super) repository: InMemoryRepository,
    pub(super) settings: Settings,
    pub(super) state: CalendarState,
    pub(super) category_filter: Entity<SelectState<Vec<FilterOption>>>,
    pub(super) scroll_handle: ScrollHandle,
    pub(super) snapshot: Option<WeekSnapshot>,
    pub(super) now: Timestamp,
    pub(super) scroll_initialized: bool,
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

        let state = CalendarState::new(today, settings.week_starts_on());
        let mut this = Self {
            repository,
            settings,
            state,
            category_filter,
            scroll_handle: ScrollHandle::new(),
            snapshot: None,
            now,
            scroll_initialized: false,
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
        let range =
            match DateRange::week(self.state.selected_date(), self.settings.week_starts_on()) {
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
                self.error = Some(format!("Could not lay out week: {error:?}"));
                self.snapshot = None;
                return;
            }
        };

        self.snapshot = Some(WeekSnapshot {
            range,
            events,
            positions,
            categories,
        });
        self.error = None;
    }

    pub(super) fn go_to_today(&mut self, cx: &mut Context<'_, Self>) {
        let (today, _) = local_date_time(Timestamp::now(), &self.settings);
        self.state.go_to_today(today);
        self.scroll_initialized = false;
        self.refresh_snapshot();
        cx.notify();
    }

    pub(super) fn shift_week(&mut self, next: bool, cx: &mut Context<'_, Self>) {
        let result = if next {
            self.state.next_week()
        } else {
            self.state.previous_week()
        };
        if let Err(error) = result {
            self.error = Some(error.to_string());
        } else {
            self.scroll_initialized = false;
            self.refresh_snapshot();
        }
        cx.notify();
    }

    pub(super) fn clear_selection(&mut self, cx: &mut Context<'_, Self>) {
        self.state.clear_selection();
        cx.notify();
    }

    pub(super) fn select_event(&mut self, event_id: EventId, cx: &mut Context<'_, Self>) {
        self.state.select_event(event_id);
        cx.notify();
    }

    pub(super) fn week_range_label(&self) -> String {
        let Some(snapshot) = &self.snapshot else {
            return "No week loaded".to_owned();
        };
        let last_day = snapshot
            .range
            .end()
            .yesterday()
            .unwrap_or_else(|_| snapshot.range.start());
        let start = snapshot.range.start().strftime("%b %-d");
        let end = last_day.strftime("%b %-d, %Y");
        format!("{start} – {end}")
    }

    pub(super) fn initial_scroll_offset(&self, column_width: f32) -> (f32, f32) {
        let Some(snapshot) = &self.snapshot else {
            return (0.0, 0.0);
        };
        let (today, current_time) = local_date_time(self.now, &self.settings);
        let target_minutes = if snapshot.range.contains(today) {
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
        };
        let horizontal = day_index(snapshot.range, self.state.selected_date()).map_or(0.0, |day| {
            let day = f32::from(u16::try_from(day).expect("week day fits in u16"));
            ((day - 2.0) * column_width).max(0.0)
        });
        (horizontal, target_minutes * PIXELS_PER_MINUTE)
    }
}
