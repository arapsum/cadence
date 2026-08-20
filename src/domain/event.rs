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
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

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
    pub fn new(
        id: EventId,
        draft: EventDraft,
        timestamp: Timestamp,
    ) -> Result<Self, ValidationError> {
        let (title, notes) = normalize_text(draft.title, draft.notes);
        validate(&title, draft.start_time, draft.end_time)?;

        Ok(Self {
            id,
            category_id: draft.category_id,
            title,
            date: draft.date,
            start_time: draft.start_time,
            end_time: draft.end_time,
            notes,
            created_at: timestamp,
            updated_at: timestamp,
        })
    }

    pub fn revise(
        &mut self,
        draft: EventDraft,
        timestamp: Timestamp,
    ) -> Result<(), ValidationError> {
        let (title, notes) = normalize_text(draft.title, draft.notes);
        validate(&title, draft.start_time, draft.end_time)?;

        self.category_id = draft.category_id;
        self.title = title;
        self.date = draft.date;
        self.start_time = draft.start_time;
        self.end_time = draft.end_time;
        self.notes = notes;
        self.updated_at = timestamp;
        Ok(())
    }

    pub fn id(&self) -> EventId {
        self.id
    }

    pub fn category_id(&self) -> CategoryId {
        self.category_id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn date(&self) -> Date {
        self.date
    }

    pub fn start_time(&self) -> Time {
        self.start_time
    }

    pub fn end_time(&self) -> Time {
        self.end_time
    }

    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }

    pub fn created_at(&self) -> Timestamp {
        self.created_at
    }

    pub fn updated_at(&self) -> Timestamp {
        self.updated_at
    }

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

fn normalize_text(title: String, notes: Option<String>) -> (String, Option<String>) {
    let title = title.trim().to_owned();
    let notes = notes
        .map(|notes| notes.trim().to_owned())
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
