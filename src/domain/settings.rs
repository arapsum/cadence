use jiff::{civil::Time, tz::TimeZone};
use serde::{Deserialize, Serialize};

use super::ValidationError;

#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum WeekStart {
    Sunday,
    Monday,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ClockFormat {
    TwelveHour,
    TwentyFourHour,
}

#[derive(Debug, Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct TimeZoneId(String);

impl TimeZoneId {
    pub fn new(identifier: impl Into<String>) -> Result<Self, ValidationError> {
        let identifier = identifier.into();
        TimeZone::get(&identifier).map_err(|_| ValidationError::InvalidTimeZone {
            identifier: identifier.clone(),
        })?;
        Ok(Self(identifier))
    }

    pub fn system() -> Self {
        let system_zone = TimeZone::system();
        let identifier = system_zone.iana_name().unwrap_or("Etc/UTC");
        // The fallback is a known-good IANA identifier.
        Self::new(identifier).expect("Etc/UTC must be available")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TimeZoneId {
    fn default() -> Self {
        Self::system()
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct SnapInterval(u16);

impl SnapInterval {
    pub fn new(minutes: u16) -> Result<Self, ValidationError> {
        if !(1..=60).contains(&minutes) {
            return Err(ValidationError::InvalidSnapInterval { minutes });
        }
        Ok(Self(minutes))
    }

    pub const fn minutes(self) -> u16 {
        self.0
    }
}

impl Default for SnapInterval {
    fn default() -> Self {
        Self(15)
    }
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct Settings {
    week_starts_on: WeekStart,
    clock_format: ClockFormat,
    time_zone: TimeZoneId,
    snap_minutes: SnapInterval,
    day_start: Time,
    day_end: Time,
}

impl Settings {
    pub fn new(
        week_starts_on: WeekStart,
        clock_format: ClockFormat,
        time_zone: TimeZoneId,
        snap_minutes: SnapInterval,
        day_start: Time,
        day_end: Time,
    ) -> Result<Self, ValidationError> {
        if day_end <= day_start {
            return Err(ValidationError::InvalidDayRange {
                start: day_start,
                end: day_end,
            });
        }

        Ok(Self {
            week_starts_on,
            clock_format,
            time_zone,
            snap_minutes,
            day_start,
            day_end,
        })
    }

    pub fn week_starts_on(&self) -> WeekStart {
        self.week_starts_on
    }

    pub fn clock_format(&self) -> ClockFormat {
        self.clock_format
    }

    pub fn time_zone(&self) -> &TimeZoneId {
        &self.time_zone
    }

    pub fn snap_interval(&self) -> SnapInterval {
        self.snap_minutes
    }

    pub fn day_start(&self) -> Time {
        self.day_start
    }

    pub fn day_end(&self) -> Time {
        self.day_end
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new(
            WeekStart::Sunday,
            ClockFormat::TwelveHour,
            TimeZoneId::system(),
            SnapInterval::default(),
            Time::constant(6, 0, 0, 0),
            Time::constant(22, 0, 0, 0),
        )
        .expect("default settings must be valid")
    }
}
