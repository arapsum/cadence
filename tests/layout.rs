use cadence::{
    calendar::{CalendarState, CalendarViewMode, CategoryFilter, LayoutMetrics, layout_events},
    domain::{
        CategoryId, DateRange, Event, EventDraft, EventId, EventOccurrence, OccurrenceId, WeekStart,
    },
};
use jiff::{
    Timestamp,
    civil::{Date, Time},
};
use uuid::Uuid;

const fn date(year: i16, month: i8, day: i8) -> Date {
    Date::constant(year, month, day)
}

const fn time(hour: i8, minute: i8) -> Time {
    Time::constant(hour, minute, 0, 0)
}

fn event(id: u128, day: Date, start: (i8, i8), end: (i8, i8)) -> EventOccurrence {
    let event = Event::new(
        EventId::from_uuid(Uuid::from_u128(id)),
        EventDraft::new(
            format!("Event {id}"),
            day,
            time(start.0, start.1),
            time(end.0, end.1),
            CategoryId::from_uuid(Uuid::from_u128(900)),
            None,
        ),
        Timestamp::from_second(0).unwrap(),
    )
    .unwrap();
    EventOccurrence::standalone(&event)
}

#[test]
fn adjacent_events_share_a_lane_without_overlap() {
    let monday = date(2024, 3, 4);
    let range = DateRange::week(monday, WeekStart::Monday).unwrap();
    let placements = layout_events(
        &[
            event(1, monday, (9, 0), (10, 0)),
            event(2, monday, (10, 0), (11, 0)),
        ],
        range,
        LayoutMetrics::default(),
    )
    .unwrap();

    assert_eq!(placements.len(), 2);
    assert_eq!(placements[0].lane(), 0);
    assert_eq!(placements[1].lane(), 0);
    assert_eq!(placements[0].lane_count(), 1);
}

#[test]
fn overlapping_events_get_lanes_and_expand_when_space_is_free() {
    let monday = date(2024, 3, 4);
    let range = DateRange::week(monday, WeekStart::Monday).unwrap();
    let placements = layout_events(
        &[
            event(1, monday, (9, 0), (11, 0)),
            event(2, monday, (10, 0), (10, 30)),
            event(3, monday, (11, 0), (12, 0)),
        ],
        range,
        LayoutMetrics::default(),
    )
    .unwrap();

    let first = placements
        .iter()
        .find(|item| {
            item.occurrence_id() == OccurrenceId::Standalone(EventId::from_uuid(Uuid::from_u128(1)))
        })
        .unwrap();
    let second = placements
        .iter()
        .find(|item| {
            item.occurrence_id() == OccurrenceId::Standalone(EventId::from_uuid(Uuid::from_u128(2)))
        })
        .unwrap();
    let third = placements
        .iter()
        .find(|item| {
            item.occurrence_id() == OccurrenceId::Standalone(EventId::from_uuid(Uuid::from_u128(3)))
        })
        .unwrap();
    assert_eq!(first.lane_count(), 2);
    assert_eq!(second.lane(), 1);
    assert_eq!(second.lane_span(), 1);
    assert_eq!(third.lane(), 0);
    assert_eq!(third.lane_count(), 1);
    assert_eq!(third.lane_span(), 1);
}

#[test]
fn short_events_keep_minimum_occupancy_and_visual_height() {
    let monday = date(2024, 3, 4);
    let range = DateRange::week(monday, WeekStart::Monday).unwrap();
    let metrics = LayoutMetrics::new(1.5, 22.0, 15.0).unwrap();
    let placements = layout_events(
        &[
            event(1, monday, (9, 0), (9, 1)),
            event(2, monday, (9, 10), (9, 11)),
        ],
        range,
        metrics,
    )
    .unwrap();

    assert!((placements[0].height() - 22.0).abs() < f32::EPSILON);
    assert!((placements[1].height() - 22.0).abs() < f32::EPSILON);
    assert_eq!(placements[0].lane_count(), 2);
}

#[test]
fn day_layout_ignores_events_outside_the_selected_date() {
    let monday = date(2024, 3, 4);
    let range = DateRange::day(monday).unwrap();
    let placements = layout_events(
        &[
            event(1, monday, (9, 0), (10, 0)),
            event(2, date(2024, 3, 5), (9, 0), (10, 0)),
        ],
        range,
        LayoutMetrics::default(),
    )
    .unwrap();

    assert_eq!(placements.len(), 1);
    assert_eq!(placements[0].day_offset(), 0);
}

#[test]
fn day_and_week_layouts_keep_event_geometry_identical() {
    let monday = date(2024, 3, 4);
    let event = event(1, monday, (9, 15), (10, 45));
    let day = layout_events(
        std::slice::from_ref(&event),
        DateRange::day(monday).unwrap(),
        LayoutMetrics::default(),
    )
    .unwrap();
    let week = layout_events(
        std::slice::from_ref(&event),
        DateRange::week(monday, WeekStart::Monday).unwrap(),
        LayoutMetrics::default(),
    )
    .unwrap();

    assert!((day[0].top() - week[0].top()).abs() < f32::EPSILON);
    assert!((day[0].height() - week[0].height()).abs() < f32::EPSILON);
}

#[test]
fn calendar_state_navigates_and_resets_selection_on_week_change() {
    let monday = date(2024, 3, 4);
    let mut state = CalendarState::new(monday, WeekStart::Monday, CalendarViewMode::Week);
    let category = CategoryId::from_uuid(Uuid::from_u128(5));
    state.set_category_filter(CategoryFilter::Only(category));
    state.select_event(
        OccurrenceId::Standalone(EventId::from_uuid(Uuid::from_u128(8))),
        monday,
    );

    state.next_period().unwrap();
    assert_eq!(state.selected_date(), date(2024, 3, 11));
    assert_eq!(state.selected_event(), None);
    assert_eq!(state.category_filter(), CategoryFilter::Only(category));

    state.previous_period().unwrap();
    assert_eq!(state.selected_date(), monday);
}

#[test]
fn calendar_state_day_mode_navigates_by_day_and_derives_one_day_ranges() {
    let monday = date(2024, 3, 4);
    let mut state = CalendarState::new(monday, WeekStart::Monday, CalendarViewMode::Day);
    assert_eq!(
        state.visible_range().unwrap(),
        DateRange::day(monday).unwrap()
    );

    let event_id = EventId::from_uuid(Uuid::from_u128(9));
    let tuesday = date(2024, 3, 5);
    state.select_event(OccurrenceId::Standalone(event_id), tuesday);
    assert_eq!(state.selected_date(), tuesday);
    assert_eq!(
        state.visible_range().unwrap(),
        DateRange::day(tuesday).unwrap()
    );

    state.next_period().unwrap();
    assert_eq!(state.selected_date(), date(2024, 3, 6));
    assert_eq!(state.selected_event(), None);

    state.set_view_mode(CalendarViewMode::Week);
    assert_eq!(
        state.visible_range().unwrap(),
        DateRange::week(date(2024, 3, 6), WeekStart::Monday).unwrap()
    );
}
