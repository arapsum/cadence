use std::{fmt, str::FromStr};

use jiff::{
    Timestamp,
    civil::{Date, Time},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{CategoryId, ValidationError};

/// Stable identifier for an event.
#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EventId(Uuid);

impl EventId {
    /// Creates a new time-ordered event identifier.
    ///
    /// # Returns
    ///
    /// A new event identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Wraps an existing `Uuid` as an event identifier.
    ///
    /// # Parameters
    ///
    /// - `id`: UUID value to wrap.
    ///
    /// # Returns
    ///
    /// An event identifier backed by `id`.
    #[must_use]
    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Returns the underlying `Uuid` value.
    ///
    /// # Returns
    ///
    /// The UUID stored by this identifier.
    #[must_use]
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for EventId {
    fn from(value: Uuid) -> Self {
        Self::from_uuid(value)
    }
}

impl FromStr for EventId {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self::from_uuid)
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Editable values used to create or revise an event.
///
/// # Fields
///
/// - `title`: User-facing event title.
/// - `date`: Civil date on which the event occurs.
/// - `start_time`: Inclusive event start time.
/// - `end_time`: Exclusive event end time.
/// - `category_id`: Category assigned to the event.
/// - `notes`: Optional supporting notes.
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventDraft {
    pub title: String,
    pub date: Date,
    pub start_time: Time,
    pub end_time: Time,
    pub category_id: CategoryId,
    pub notes: Option<String>,
}

impl EventDraft {
    /// Creates an event draft from its editable values.
    ///
    /// # Parameters
    ///
    /// - `title`: User-facing event title.
    /// - `date`: Civil date on which the event occurs.
    /// - `start_time`: Inclusive event start time.
    /// - `end_time`: Exclusive event end time.
    /// - `category_id`: Category assigned to the event.
    /// - `notes`: Optional supporting notes.
    ///
    /// # Returns
    ///
    /// An event draft containing the supplied values.
    pub fn new(
        title: impl Into<String>,
        date: Date,
        start_time: Time,
        end_time: Time,
        category_id: CategoryId,
        notes: Option<String>,
    ) -> Self {
        Self {
            title: title.into(),
            date,
            start_time,
            end_time,
            category_id,
            notes,
        }
    }
}

/// A validated timetable event with immutable creation metadata.
///
/// # Fields
///
/// - `id`: Stable identifier for the event.
/// - `category_id`: Category assigned to the event.
/// - `title`: Trimmed user-facing event title.
/// - `date`: Civil date on which the event occurs.
/// - `start_time`: Inclusive event start time.
/// - `end_time`: Exclusive event end time.
/// - `notes`: Optional trimmed supporting notes.
/// - `created_at`: Timestamp at which the event was created.
/// - `updated_at`: Timestamp at which the event was last revised.
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct Event {
    id: EventId,
    category_id: CategoryId,
    title: String,
    date: Date,
    start_time: Time,
    end_time: Time,
    notes: Option<String>,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl Event {
    /// Creates a validated event from an editable draft.
    ///
    /// # Parameters
    ///
    /// - `id`: Stable identifier for the event.
    /// - `draft`: Editable event values to validate and store.
    /// - `timestamp`: Creation timestamp for the event.
    ///
    /// # Returns
    ///
    /// A validated event with normalized text values.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - The title is empty after trimming.
    /// - The end time is not later than the start time.
    pub fn new(
        id: EventId,
        draft: EventDraft,
        timestamp: Timestamp,
    ) -> Result<Self, ValidationError> {
        let EventDraft {
            title: draft_title,
            date,
            start_time,
            end_time,
            category_id,
            notes: draft_notes,
        } = draft;
        let (title, notes) = normalize_text(&draft_title, draft_notes.as_deref());
        validate(&title, start_time, end_time)?;

        Ok(Self {
            id,
            category_id,
            title,
            date,
            start_time,
            end_time,
            notes,
            created_at: timestamp,
            updated_at: timestamp,
        })
    }

    /// Reconstructs an event loaded from durable storage.
    ///
    /// # Parameters
    ///
    /// - `id`: Stable event identifier.
    /// - `draft`: Persisted editable event values.
    /// - `created_at`: Original creation timestamp.
    /// - `updated_at`: Timestamp of the latest persisted revision.
    ///
    /// # Returns
    ///
    /// A validated event retaining both persisted timestamps.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - The persisted title is empty.
    /// - The persisted end time is not later than the start time.
    pub fn from_persisted(
        id: EventId,
        draft: EventDraft,
        created_at: Timestamp,
        updated_at: Timestamp,
    ) -> Result<Self, ValidationError> {
        let mut event = Self::new(id, draft, created_at)?;
        event.updated_at = updated_at;
        Ok(event)
    }

    /// Applies revised editable values to an existing event.
    ///
    /// # Parameters
    ///
    /// - `draft`: Editable event values to validate and store.
    /// - `timestamp`: Revision timestamp for the event.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the revised values are validated and stored.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - The title is empty after trimming.
    /// - The end time is not later than the start time.
    pub fn revise(
        &mut self,
        draft: EventDraft,
        timestamp: Timestamp,
    ) -> Result<(), ValidationError> {
        let EventDraft {
            title: draft_title,
            date,
            start_time,
            end_time,
            category_id,
            notes: draft_notes,
        } = draft;
        let (title, notes) = normalize_text(&draft_title, draft_notes.as_deref());
        validate(&title, start_time, end_time)?;

        self.category_id = category_id;
        self.title = title;
        self.date = date;
        self.start_time = start_time;
        self.end_time = end_time;
        self.notes = notes;
        self.updated_at = timestamp;
        Ok(())
    }

    /// Returns the event identifier.
    #[must_use]
    pub const fn id(&self) -> EventId {
        self.id
    }

    /// Returns the category identifier.
    #[must_use]
    pub const fn category_id(&self) -> CategoryId {
        self.category_id
    }

    /// Returns the trimmed event title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the event date.
    #[must_use]
    pub const fn date(&self) -> Date {
        self.date
    }

    /// Returns the inclusive event start time.
    #[must_use]
    pub const fn start_time(&self) -> Time {
        self.start_time
    }

    /// Returns the exclusive event end time.
    #[must_use]
    pub const fn end_time(&self) -> Time {
        self.end_time
    }

    /// Returns the optional trimmed event notes.
    #[must_use]
    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }

    /// Returns the event creation timestamp.
    #[must_use]
    pub const fn created_at(&self) -> Timestamp {
        self.created_at
    }

    /// Returns the timestamp of the latest revision.
    #[must_use]
    pub const fn updated_at(&self) -> Timestamp {
        self.updated_at
    }

    /// Returns the event values as an editable draft.
    ///
    /// # Returns
    ///
    /// A draft containing the event's current editable values.
    #[must_use]
    pub fn draft(&self) -> EventDraft {
        EventDraft {
            title: self.title.clone(),
            date: self.date,
            start_time: self.start_time,
            end_time: self.end_time,
            category_id: self.category_id,
            notes: self.notes.clone(),
        }
    }
}

fn normalize_text(title: &str, notes: Option<&str>) -> (String, Option<String>) {
    let title = title.trim().to_owned();
    let notes = notes
        .map(str::trim)
        .map(str::to_owned)
        .filter(|notes| !notes.is_empty());
    (title, notes)
}

fn validate(title: &str, start_time: Time, end_time: Time) -> Result<(), ValidationError> {
    if title.is_empty() {
        return Err(ValidationError::EmptyTitle);
    }
    if end_time <= start_time {
        return Err(ValidationError::EndNotAfterStart {
            start: start_time,
            end: end_time,
        });
    }
    Ok(())
}
