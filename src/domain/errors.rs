use std::fmt;

use jiff::civil::{Date, Time};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ValidationError {
    EmptyTitle,
    EmptyCategoryName,
    EndNotAfterStart { start: Time, end: Time },
    InvalidTimeZone { identifier: String },
    InvalidDayRange { start: Time, end: Time },
    InvalidSnapInterval { minutes: u16 },
    InvalidRecurrence(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTitle => f.write_str("Add a title."),
            Self::EmptyCategoryName => f.write_str("Add a category name."),
            Self::EndNotAfterStart { .. } => f.write_str("End time must be later than start time."),
            Self::InvalidTimeZone { identifier } => {
                write!(f, "'{identifier}' is not a valid IANA time zone.")
            }
            Self::InvalidDayRange { .. } => {
                f.write_str("The display day must end after it starts.")
            }
            Self::InvalidSnapInterval { minutes } => {
                write!(
                    f,
                    "Snap interval must be between 1 and 60 minutes; got {minutes}."
                )
            }
            Self::InvalidRecurrence(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CalendarError {
    InvalidDateRange { start: Date, end: Date },
    DateArithmetic,
    InvalidPixelsPerMinute,
    InvalidSnapInterval { minutes: u16 },
    SnapOverflow,
}

impl fmt::Display for CalendarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDateRange { start, end } => {
                write!(f, "date range must be non-empty ({start}..{end})")
            }
            Self::DateArithmetic => f.write_str("date arithmetic exceeded the supported range."),
            Self::InvalidPixelsPerMinute => {
                f.write_str("pixels per minute must be finite and greater than zero.")
            }
            Self::InvalidSnapInterval { minutes } => {
                write!(
                    f,
                    "snap interval must be between 1 and 60 minutes; got {minutes}."
                )
            }
            Self::SnapOverflow => f.write_str("snapping exceeded the supported date range."),
        }
    }
}

impl std::error::Error for CalendarError {}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RepositoryError {
    DuplicateEvent,
    EventNotFound,
    DuplicateSeries,
    SeriesNotFound,
    DuplicateCategory,
    CategoryNotFound,
    CategoryInUse,
    InvalidEntity(String),
    Storage(String),
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateEvent => f.write_str("an event with this ID already exists."),
            Self::EventNotFound => f.write_str("the event could not be found."),
            Self::DuplicateSeries => f.write_str("a recurring series with this ID already exists."),
            Self::SeriesNotFound => f.write_str("the recurring series could not be found."),
            Self::DuplicateCategory => f.write_str("a category with this ID already exists."),
            Self::CategoryNotFound => f.write_str("the category could not be found."),
            Self::CategoryInUse => f.write_str("the category is still used by an event."),
            Self::InvalidEntity(message) | Self::Storage(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for RepositoryError {}

impl From<CalendarError> for RepositoryError {
    fn from(error: CalendarError) -> Self {
        Self::InvalidEntity(error.to_string())
    }
}

impl From<jiff::Error> for CalendarError {
    fn from(_: jiff::Error) -> Self {
        Self::DateArithmetic
    }
}
