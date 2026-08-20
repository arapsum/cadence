use jiff::{
    Timestamp,
    civil::{Date, Time},
    tz::TimeZone,
};

use crate::{
    calendar::{CategoryFilter, LayoutMetrics, PositionedEvent, layout_week},
    domain::{Category, DateRange, Event, Settings},
};

#[derive(Clone)]
pub(crate) struct WeekSnapshot {
    pub(crate) range: DateRange,
    pub(crate) events: Vec<Event>,
    pub(crate) positions: Vec<PositionedEvent>,
    pub(crate) categories: Vec<Category>,
}

pub(crate) fn local_date_time(timestamp: Timestamp, settings: &Settings) -> (Date, Time) {
    let timezone = TimeZone::get(settings.time_zone().as_str()).unwrap_or(TimeZone::UTC);
    let zoned = timestamp.to_zoned(timezone);
    (zoned.date(), zoned.time())
}

pub(crate) fn day_index(range: DateRange, date: Date) -> Option<usize> {
    let mut current = range.start();
    for offset in 0..7 {
        if current == date {
            return Some(offset);
        }
        current = current.tomorrow().ok()?;
    }
    None
}

pub(crate) fn layout_events(
    events: &[Event],
    range: DateRange,
) -> Result<Vec<PositionedEvent>, crate::calendar::LayoutError> {
    layout_week(events, range, LayoutMetrics::default())
}

pub(crate) fn event_matches_filter(event: &Event, filter: CategoryFilter) -> bool {
    match filter {
        CategoryFilter::All => true,
        CategoryFilter::Only(category_id) => event.category_id() == category_id,
    }
}
