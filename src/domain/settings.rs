use jiff::{civil::Time, tz::TimeZone};
use serde::{Deserialize, Serialize};

use super::ValidationError;

/// First weekday displayed by a calendar week.
#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum WeekStart {
    Sunday,
    Monday,
}

/// Clock representation used by calendar labels.
#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ClockFormat {
    TwelveHour,
    TwentyFourHour,
}

/// A validated IANA time zone identifier.
#[derive(Debug, Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct TimeZoneId(String);

impl TimeZoneId {
    /// Creates a validated IANA time zone identifier.
    ///
    /// # Parameters
    ///
    /// - `identifier`: IANA time zone name to validate.
    ///
    /// # Returns
    ///
    /// A time zone identifier accepted by `jiff`.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - `identifier` is not a valid IANA time zone.
    pub fn new(identifier: impl Into<String>) -> Result<Self, ValidationError> {
        let identifier = identifier.into();
        TimeZone::get(&identifier).map_err(|_| ValidationError::InvalidTimeZone {
            identifier: identifier.clone(),
        })?;
        Ok(Self(identifier))
    }

    /// Returns the system time zone.
    ///
    /// # Returns
    ///
    /// The system IANA time zone, falling back to `Etc/UTC` when no name is available.
    ///
/// # Panics
///
/// Panics when the known-good `Etc/UTC` fallback cannot be validated.
#[must_use]
    pub fn system() -> Self {
        let system_zone = TimeZone::system();
        let identifier = system_zone.iana_name().unwrap_or("Etc/UTC");
        // The fallback is a known-good IANA identifier.
        Self::new(identifier).expect("Etc/UTC must be available")
    }

    /// Returns the IANA time zone name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TimeZoneId {
    fn default() -> Self {
        Self::system()
    }
}

/// A validated event snapping interval in minutes.
#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct SnapInterval(u16);

impl SnapInterval {
    /// Creates a snapping interval.
    ///
    /// # Parameters
    ///
    /// - `minutes`: Number of minutes between snap points.
    ///
    /// # Returns
    ///
    /// A snapping interval between one and sixty minutes.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - `minutes` is outside the inclusive range `1..=60`.
    pub fn new(minutes: u16) -> Result<Self, ValidationError> {
        if !(1..=60).contains(&minutes) {
            return Err(ValidationError::InvalidSnapInterval { minutes });
        }
        Ok(Self(minutes))
    }

    /// Returns the interval length in minutes.
    #[must_use]
    pub const fn minutes(self) -> u16 {
        self.0
    }
}

impl Default for SnapInterval {
    fn default() -> Self {
        Self(15)
    }
}

/// User preferences that shape calendar presentation and navigation.
///
/// # Fields
///
/// - `week_starts_on`: First weekday displayed by a calendar week.
/// - `clock_format`: Clock representation used by calendar labels.
/// - `time_zone`: Time zone used to interpret timestamps.
/// - `snap_minutes`: Event snapping interval.
/// - `day_start`: First time shown by a calendar surface.
/// - `day_end`: Last time shown by a calendar surface.
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
    /// Creates validated calendar settings.
    ///
    /// # Parameters
    ///
    /// - `week_starts_on`: First weekday displayed by a calendar week.
    /// - `clock_format`: Clock representation used by calendar labels.
    /// - `time_zone`: Time zone used to interpret timestamps.
    /// - `snap_minutes`: Event snapping interval.
    /// - `day_start`: First time shown by a calendar surface.
    /// - `day_end`: Last time shown by a calendar surface.
    ///
    /// # Returns
    ///
    /// Validated settings containing the supplied preferences.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - `day_end` is not later than `day_start`.
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

    /// Returns the configured first weekday.
    #[must_use]
    pub const fn week_starts_on(&self) -> WeekStart {
        self.week_starts_on
    }

    /// Returns the configured clock format.
    #[must_use]
    pub const fn clock_format(&self) -> ClockFormat {
        self.clock_format
    }

    /// Returns the configured time zone.
    #[must_use]
    pub const fn time_zone(&self) -> &TimeZoneId {
        &self.time_zone
    }

    /// Returns the configured snapping interval.
    #[must_use]
    pub const fn snap_interval(&self) -> SnapInterval {
        self.snap_minutes
    }

    /// Returns the first displayed time of day.
    #[must_use]
    pub const fn day_start(&self) -> Time {
        self.day_start
    }

    /// Returns the last displayed time of day.
    #[must_use]
    pub const fn day_end(&self) -> Time {
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
