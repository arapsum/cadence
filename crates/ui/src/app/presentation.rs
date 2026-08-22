use jiff::{
    Timestamp,
    civil::{Date, Time},
    tz::TimeZone,
};
use std::collections::HashSet;

use crate::{
    calendar::{CalendarViewMode, CategoryFilter, LayoutMetrics, PositionedEvent},
    domain::{Category, DateRange, EventOccurrence, Settings},
};

#[derive(Clone)]
pub(super) struct SurfaceSnapshot {
    pub(super) range: DateRange,
    pub(super) events: Vec<EventOccurrence>,
    pub(super) positions: Vec<PositionedEvent>,
}

#[derive(Clone)]
pub(super) struct WorkspaceSnapshot {
    pub(super) day: SurfaceSnapshot,
    pub(super) week: SurfaceSnapshot,
    pub(super) categories: Vec<Category>,
    pub(super) summary_events: Vec<EventOccurrence>,
    pub(super) conflict_ids: HashSet<crate::domain::OccurrenceId>,
}

impl WorkspaceSnapshot {
    pub(super) const fn surface(&self, mode: CalendarViewMode) -> &SurfaceSnapshot {
        match mode {
            CalendarViewMode::Day => &self.day,
            CalendarViewMode::Week => &self.week,
        }
    }
}

pub(super) fn local_date_time(timestamp: Timestamp, settings: &Settings) -> (Date, Time) {
    let timezone = TimeZone::get(settings.time_zone().as_str()).unwrap_or(TimeZone::UTC);
    let zoned = timestamp.to_zoned(timezone);
    (zoned.date(), zoned.time())
}

pub(super) fn day_index(range: DateRange, date: Date) -> Option<usize> {
    let mut current = range.start();
    let mut offset = 0_usize;
    while current < range.end() {
        if current == date {
            return Some(offset);
        }
        current = current.tomorrow().ok()?;
        offset += 1;
    }
    None
}

pub(super) fn dates_in_range(range: DateRange) -> Vec<Date> {
    let mut dates = Vec::new();
    let mut current = range.start();
    while current < range.end() {
        dates.push(current);
        let Some(next) = current.tomorrow().ok() else {
            break;
        };
        current = next;
    }
    dates
}

pub(super) fn layout_events(
    events: &[EventOccurrence],
    range: DateRange,
) -> Result<Vec<PositionedEvent>, crate::calendar::LayoutError> {
    crate::calendar::layout_events(events, range, LayoutMetrics::default())
}

pub(super) fn event_matches_filter(event: &EventOccurrence, filter: CategoryFilter) -> bool {
    match filter {
        CategoryFilter::All => true,
        CategoryFilter::Only(category_id) => event.category_id() == category_id,
    }
}
