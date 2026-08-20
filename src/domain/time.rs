use jiff::{
    RoundMode, Unit,
    civil::{Date, DateTime, DateTimeRound, Time, Weekday},
};

use super::{CalendarError, ClockFormat, WeekStart};

/// A half-open civil date range used by calendar queries.
///
/// # Fields
///
/// - `start`: First included date.
/// - `end`: First excluded date.
#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DateRange {
    start: Date,
    end: Date,
}

impl DateRange {
    /// Creates a non-empty half-open date range.
    ///
    /// # Parameters
    ///
    /// - `start`: First included date.
    /// - `end`: First excluded date.
    ///
    /// # Returns
    ///
    /// A date range containing dates from `start` through the day before `end`.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - `end` is not later than `start`.
    pub fn new(start: Date, end: Date) -> Result<Self, CalendarError> {
        if end <= start {
            return Err(CalendarError::InvalidDateRange { start, end });
        }
        Ok(Self { start, end })
    }

    /// Creates the one-day range containing `date`.
    ///
    /// # Parameters
    ///
    /// - `date`: Date to include.
    ///
    /// # Returns
    ///
    /// A range from `date` through the following day.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - Date arithmetic cannot produce the following day.
    pub fn day(date: Date) -> Result<Self, CalendarError> {
        Self::new(date, date.tomorrow()?)
    }

    /// Creates the seven-day range containing `date`.
    ///
    /// # Parameters
    ///
    /// - `date`: Date used to locate the calendar week.
    /// - `week_start`: First weekday of the calendar week.
    ///
    /// # Returns
    ///
    /// A range from the week's first day through the following seven days.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - Date arithmetic cannot produce the requested week.
    pub fn week(date: Date, week_start: WeekStart) -> Result<Self, CalendarError> {
        let start = start_of_week(date, week_start)?;
        let mut end = start;
        for _ in 0..7 {
            end = end.tomorrow()?;
        }
        Self::new(start, end)
    }

    /// Returns the first included date.
    #[must_use]
    pub const fn start(self) -> Date {
        self.start
    }

    /// Returns the first excluded date.
    #[must_use]
    pub const fn end(self) -> Date {
        self.end
    }

    /// Reports whether a date belongs to this range.
    ///
    /// # Parameters
    ///
    /// - `date`: Date to test.
    ///
    /// # Returns
    ///
    /// `true` when `date` is on or after `start` and before `end`.
    #[must_use]
    pub fn contains(self, date: Date) -> bool {
        self.start <= date && date < self.end
    }
}

/// Returns the first day of the calendar week containing `date`.
///
/// # Parameters
///
/// - `date`: Date used to locate the calendar week.
/// - `week_start`: First weekday of the calendar week.
///
/// # Returns
///
/// The selected week's first date.
///
/// # Errors
///
/// Returns an error when:
///
/// - Date arithmetic cannot reach the start of the week.
pub fn start_of_week(date: Date, week_start: WeekStart) -> Result<Date, CalendarError> {
    let first_weekday = match week_start {
        WeekStart::Sunday => Weekday::Sunday,
        WeekStart::Monday => Weekday::Monday,
    };
    let days_since_start = date.weekday().since(first_weekday);
    let mut result = date;
    for _ in 0..days_since_start {
        result = result.yesterday()?;
    }
    Ok(result)
}

/// Returns the civil date immediately before `date`.
///
/// # Parameters
///
/// - `date`: Date to move backward from.
///
/// # Returns
///
/// The previous civil date.
///
/// # Errors
///
/// Returns an error when:
///
/// - Date arithmetic cannot produce the previous date.
pub fn previous_day(date: Date) -> Result<Date, CalendarError> {
    Ok(date.yesterday()?)
}

/// Returns the civil date immediately after `date`.
///
/// # Parameters
///
/// - `date`: Date to move forward from.
///
/// # Returns
///
/// The next civil date.
///
/// # Errors
///
/// Returns an error when:
///
/// - Date arithmetic cannot produce the next date.
pub fn next_day(date: Date) -> Result<Date, CalendarError> {
    Ok(date.tomorrow()?)
}

/// Returns the start date of the previous calendar week.
///
/// # Parameters
///
/// - `date`: Date used to locate the current calendar week.
/// - `week_start`: First weekday of the calendar week.
///
/// # Returns
///
/// The first date of the preceding calendar week.
///
/// # Errors
///
/// Returns an error when:
///
/// - Date arithmetic cannot produce the previous week.
pub fn previous_week(date: Date, week_start: WeekStart) -> Result<Date, CalendarError> {
    let mut result = start_of_week(date, week_start)?;
    for _ in 0..7 {
        result = result.yesterday()?;
    }
    Ok(result)
}

/// Returns the start date of the next calendar week.
///
/// # Parameters
///
/// - `date`: Date used to locate the current calendar week.
/// - `week_start`: First weekday of the calendar week.
///
/// # Returns
///
/// The first date of the following calendar week.
///
/// # Errors
///
/// Returns an error when:
///
/// - Date arithmetic cannot produce the next week.
pub fn next_week(date: Date, week_start: WeekStart) -> Result<Date, CalendarError> {
    let mut result = start_of_week(date, week_start)?;
    for _ in 0..7 {
        result = result.tomorrow()?;
    }
    Ok(result)
}

/// Formats a civil time using the selected clock representation.
///
/// # Parameters
///
/// - `time`: Civil time to format.
/// - `format`: Clock representation to use.
///
/// # Returns
///
/// A user-facing time label.
#[must_use]
pub fn format_time(time: Time, format: ClockFormat) -> String {
    match format {
        ClockFormat::TwentyFourHour => format!("{:02}:{:02}", time.hour(), time.minute()),
        ClockFormat::TwelveHour => {
            let hour = time.hour();
            let display_hour = match hour % 12 {
                0 => 12,
                value => value,
            };
            let meridiem = if hour < 12 { "AM" } else { "PM" };
            format!("{:02}:{:02} {meridiem}", display_hour, time.minute())
        }
    }
}

/// Converts a civil time into whole minutes since midnight.
///
/// # Parameters
///
/// - `time`: Civil time to convert.
///
/// # Returns
///
/// The number of whole minutes since midnight.
///
/// # Panics
///
/// Panics when a `jiff` time component cannot fit in the supported unsigned
/// range.
#[must_use]
pub fn minutes_since_midnight(time: Time) -> u16 {
    let hour = u16::try_from(time.hour()).expect("time hour is non-negative");
    let minute = u16::try_from(time.minute()).expect("time minute is non-negative");
    hour * 60 + minute
}

/// Converts a civil time into a vertical pixel offset.
///
/// # Parameters
///
/// - `time`: Civil time to convert.
/// - `pixels_per_minute`: Scale used by the calendar plane.
///
/// # Returns
///
/// The vertical offset represented by `time`.
///
/// # Errors
///
/// Returns an error when:
///
/// - `pixels_per_minute` is not finite and greater than zero.
pub fn time_to_offset(time: Time, pixels_per_minute: f32) -> Result<f32, CalendarError> {
    if !pixels_per_minute.is_finite() || pixels_per_minute <= 0.0 {
        return Err(CalendarError::InvalidPixelsPerMinute);
    }

    #[allow(clippy::cast_precision_loss)]
    let subsecond = time.subsec_nanosecond() as f32 / 60_000_000_000.0;
    let minutes =
        f32::from(minutes_since_midnight(time)) + f32::from(time.second()) / 60.0 + subsecond;
    Ok(minutes * pixels_per_minute)
}

/// Rounding direction used when snapping a date-time.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum SnapMode {
    Down,
    Nearest,
    Up,
}

/// Snaps a civil date-time to a minute interval.
///
/// # Parameters
///
/// - `datetime`: Date-time to round.
/// - `interval_minutes`: Snap interval in minutes.
/// - `mode`: Rounding direction to apply.
///
/// # Returns
///
/// The date-time rounded to the requested interval.
///
/// # Errors
///
/// Returns an error when:
///
/// - `interval_minutes` is outside the inclusive range `1..=60`.
/// - Rounding exceeds the supported date range.
pub fn snap_datetime(
    datetime: DateTime,
    interval_minutes: u16,
    mode: SnapMode,
) -> Result<DateTime, CalendarError> {
    if !(1..=60).contains(&interval_minutes) {
        return Err(CalendarError::InvalidSnapInterval {
            minutes: interval_minutes,
        });
    }

    let round_mode = match mode {
        SnapMode::Down => RoundMode::Trunc,
        SnapMode::Nearest => RoundMode::HalfExpand,
        SnapMode::Up => RoundMode::Ceil,
    };
    datetime
        .round(
            DateTimeRound::new()
                .smallest(Unit::Minute)
                .increment(i64::from(interval_minutes))
                .mode(round_mode),
        )
        .map_err(|_| CalendarError::SnapOverflow)
}
