use cadence_core::domain::{
    CalendarError, ClockFormat, DateRange, Event, EventDraft, EventId, Settings, SnapInterval,
    SnapMode, TimeZoneId, ValidationError, WeekStart, format_time, minutes_since_midnight,
    next_week, snap_datetime, start_of_week, time_to_offset,
};
use jiff::{
    Timestamp,
    civil::{Date, DateTime, Time},
};
use uuid::Uuid;

const fn date(year: i16, month: i8, day: i8) -> Date {
    Date::constant(year, month, day)
}

const fn time(hour: i8, minute: i8) -> Time {
    Time::constant(hour, minute, 0, 0)
}

#[test]
fn week_boundaries_support_sunday_and_monday_starts() {
    let wednesday = date(2024, 1, 3);
    assert_eq!(
        start_of_week(wednesday, WeekStart::Sunday).unwrap(),
        date(2023, 12, 31)
    );
    assert_eq!(
        start_of_week(wednesday, WeekStart::Monday).unwrap(),
        date(2024, 1, 1)
    );

    let next = next_week(wednesday, WeekStart::Monday).unwrap();
    assert_eq!(next, date(2024, 1, 8));
    assert!(DateRange::new(date(2024, 1, 1), date(2024, 1, 1)).is_err());
}

#[test]
fn leap_days_and_year_boundaries_are_preserved() {
    let range = DateRange::week(date(2024, 2, 29), WeekStart::Sunday).unwrap();
    assert_eq!(range.start(), date(2024, 2, 25));
    assert_eq!(range.end(), date(2024, 3, 3));

    let new_year = DateRange::week(date(2024, 1, 1), WeekStart::Sunday).unwrap();
    assert_eq!(new_year.start(), date(2023, 12, 31));
}

#[test]
fn time_formatting_and_offsets_are_deterministic() {
    assert_eq!(format_time(time(0, 0), ClockFormat::TwelveHour), "12:00 AM");
    assert_eq!(
        format_time(time(12, 0), ClockFormat::TwelveHour),
        "12:00 PM"
    );
    assert_eq!(
        format_time(time(18, 5), ClockFormat::TwentyFourHour),
        "18:05"
    );
    assert_eq!(minutes_since_midnight(time(6, 30)), 390);
    assert!((time_to_offset(time(6, 30), 2.0).unwrap() - 780.0).abs() < f32::EPSILON);
    assert!(matches!(
        time_to_offset(time(6, 30), 0.0),
        Err(CalendarError::InvalidPixelsPerMinute)
    ));
}

#[test]
fn snapping_supports_all_modes_and_midnight_carry() {
    let before = DateTime::constant(2024, 3, 15, 10, 7, 30, 0);
    let tie = DateTime::constant(2024, 3, 15, 10, 7, 30, 0);
    let late = DateTime::constant(2024, 3, 15, 23, 59, 0, 0);

    assert_eq!(
        snap_datetime(before, 15, SnapMode::Down).unwrap(),
        DateTime::constant(2024, 3, 15, 10, 0, 0, 0)
    );
    assert_eq!(
        snap_datetime(tie, 15, SnapMode::Nearest).unwrap(),
        DateTime::constant(2024, 3, 15, 10, 15, 0, 0)
    );
    assert_eq!(
        snap_datetime(late, 15, SnapMode::Up).unwrap(),
        DateTime::constant(2024, 3, 16, 0, 0, 0, 0)
    );
    assert!(matches!(
        snap_datetime(before, 0, SnapMode::Nearest),
        Err(CalendarError::InvalidSnapInterval { .. })
    ));
}

#[test]
fn events_and_settings_reject_invalid_values_with_user_facing_errors() {
    let category_id = Uuid::from_u128(1).into();
    let timestamp = Timestamp::from_second(0).unwrap();
    let invalid = Event::new(
        EventId::from_uuid(Uuid::from_u128(2)),
        EventDraft::new(
            "   ",
            date(2024, 3, 15),
            time(10, 0),
            time(9, 0),
            category_id,
            Some("  ".to_owned()),
        ),
        timestamp,
    )
    .unwrap_err();
    assert_eq!(invalid, ValidationError::EmptyTitle);

    assert!(TimeZoneId::new("Not/AZone").is_err());
    assert!(SnapInterval::new(0).is_err());
    assert!(
        Settings::new(
            WeekStart::Sunday,
            ClockFormat::TwelveHour,
            TimeZoneId::new("Etc/UTC").unwrap(),
            SnapInterval::default(),
            time(22, 0),
            time(6, 0),
        )
        .is_err()
    );
}
