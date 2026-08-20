use jiff::{
    RoundMode, Unit,
    civil::{Date, DateTime, DateTimeRound, Time, Weekday},
};

use super::{CalendarError, ClockFormat, WeekStart};

#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DateRange {
    start: Date,
    end: Date,
}

impl DateRange {
    pub fn new(start: Date, end: Date) -> Result<Self, CalendarError> {
        if end <= start {
            return Err(CalendarError::InvalidDateRange { start, end });
        }
        Ok(Self { start, end })
    }

    pub fn day(date: Date) -> Result<Self, CalendarError> {
        Self::new(date, date.tomorrow()?)
    }

    pub fn week(date: Date, week_start: WeekStart) -> Result<Self, CalendarError> {
        let start = start_of_week(date, week_start)?;
        let mut end = start;
        for _ in 0..7 {
            end = end.tomorrow()?;
        }
        Self::new(start, end)
    }

    pub fn start(self) -> Date {
        self.start
    }

    pub fn end(self) -> Date {
        self.end
    }

    pub fn contains(self, date: Date) -> bool {
        self.start <= date && date < self.end
    }
}

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

pub fn previous_day(date: Date) -> Result<Date, CalendarError> {
    Ok(date.yesterday()?)
}

pub fn next_day(date: Date) -> Result<Date, CalendarError> {
    Ok(date.tomorrow()?)
}

pub fn previous_week(date: Date, week_start: WeekStart) -> Result<Date, CalendarError> {
    let mut result = start_of_week(date, week_start)?;
    for _ in 0..7 {
        result = result.yesterday()?;
    }
    Ok(result)
}

pub fn next_week(date: Date, week_start: WeekStart) -> Result<Date, CalendarError> {
    let mut result = start_of_week(date, week_start)?;
    for _ in 0..7 {
        result = result.tomorrow()?;
    }
    Ok(result)
}

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

pub fn minutes_since_midnight(time: Time) -> u16 {
    (time.hour() as u16 * 60) + time.minute() as u16
}

pub fn time_to_offset(time: Time, pixels_per_minute: f32) -> Result<f32, CalendarError> {
    if !pixels_per_minute.is_finite() || pixels_per_minute <= 0.0 {
        return Err(CalendarError::InvalidPixelsPerMinute);
    }

    let minutes = minutes_since_midnight(time) as f32
        + time.second() as f32 / 60.0
        + time.subsec_nanosecond() as f32 / 60_000_000_000.0;
    Ok(minutes * pixels_per_minute)
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum SnapMode {
    Down,
    Nearest,
    Up,
}

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
                .increment(interval_minutes as i64)
                .mode(round_mode),
        )
        .map_err(|_| CalendarError::SnapOverflow)
}
