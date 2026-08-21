use std::{fmt, str::FromStr};

use jiff::{
    Timestamp,
    civil::{Date, Time, Weekday},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{CategoryId, Event, EventDraft, EventId, ValidationError};

/// Stable identifier for a recurring-event series.
#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RecurrenceSeriesId(Uuid);

impl RecurrenceSeriesId {
    /// Creates a new time-ordered series identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Wraps an existing UUID as a series identifier.
    #[must_use]
    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Returns the underlying UUID.
    #[must_use]
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for RecurrenceSeriesId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for RecurrenceSeriesId {
    fn from(value: Uuid) -> Self {
        Self::from_uuid(value)
    }
}

impl FromStr for RecurrenceSeriesId {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self::from_uuid)
    }
}

impl fmt::Display for RecurrenceSeriesId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A set of weekdays used by a weekly recurrence rule.
#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct WeekdaySet(u8);

impl WeekdaySet {
    /// Creates a set containing one weekday.
    #[must_use]
    pub fn one(day: Weekday) -> Self {
        Self(1 << day.to_monday_zero_offset())
    }

    /// Creates a set from a weekday bit mask.
    ///
    /// # Errors
    ///
    /// Returns an error when no weekday is selected or an unused bit is set.
    pub fn from_bits(bits: u8) -> Result<Self, ValidationError> {
        if bits == 0 || bits > 0b0111_1111 {
            return Err(ValidationError::InvalidRecurrence(
                "select at least one weekday".to_owned(),
            ));
        }
        Ok(Self(bits))
    }

    /// Returns the encoded weekday bit mask.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Reports whether `day` belongs to this set.
    #[must_use]
    pub fn contains(self, day: Weekday) -> bool {
        self.0 & (1 << day.to_monday_zero_offset()) != 0
    }

    /// Returns the selected weekdays in Monday-first order.
    #[must_use]
    pub fn days(self) -> Vec<Weekday> {
        (0..7)
            .filter_map(|offset| {
                let day = Weekday::from_monday_zero_offset(offset).ok()?;
                self.contains(day).then_some(day)
            })
            .collect()
    }

    /// Toggles one weekday and returns the resulting set.
    ///
    /// # Errors
    ///
    /// Returns an error when removing `day` would leave no weekdays selected.
    pub fn toggled(self, day: Weekday) -> Result<Self, ValidationError> {
        let bit = 1_u8 << day.to_monday_zero_offset();
        let bits = self.0 ^ bit;
        Self::from_bits(bits)
    }
}

/// Supported recurring-event frequencies.
#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
pub enum RecurrenceRule {
    /// Every civil date from the series start.
    Daily,
    /// Monday through Friday.
    Weekdays,
    /// Selected weekdays in each calendar week.
    Weekly(WeekdaySet),
}

/// The persisted template and schedule for a recurring event.
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecurrenceSeries {
    id: RecurrenceSeriesId,
    template: EventDraft,
    rule: RecurrenceRule,
    ends_on: Option<Date>,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl RecurrenceSeries {
    /// Creates and validates a recurring series.
    ///
    /// # Errors
    ///
    /// Returns an error when the event draft or recurrence bounds are invalid.
    pub fn new(
        id: RecurrenceSeriesId,
        template: EventDraft,
        rule: RecurrenceRule,
        ends_on: Option<Date>,
        timestamp: Timestamp,
    ) -> Result<Self, ValidationError> {
        let normalized = Event::new(EventId::new(), template, timestamp)?.draft();
        validate_rule(normalized.date, rule, ends_on)?;
        Ok(Self {
            id,
            template: normalized,
            rule,
            ends_on,
            created_at: timestamp,
            updated_at: timestamp,
        })
    }

    /// Reconstructs a series loaded from durable storage.
    ///
    /// # Errors
    ///
    /// Returns an error when the persisted draft or recurrence bounds are invalid.
    pub fn from_persisted(
        id: RecurrenceSeriesId,
        template: EventDraft,
        rule: RecurrenceRule,
        ends_on: Option<Date>,
        created_at: Timestamp,
        updated_at: Timestamp,
    ) -> Result<Self, ValidationError> {
        let mut series = Self::new(id, template, rule, ends_on, created_at)?;
        series.updated_at = updated_at;
        Ok(series)
    }

    /// Revises the template and recurrence settings.
    ///
    /// # Errors
    ///
    /// Returns an error when the revised draft or recurrence bounds are invalid.
    pub fn revise(
        &mut self,
        template: EventDraft,
        rule: RecurrenceRule,
        ends_on: Option<Date>,
        timestamp: Timestamp,
    ) -> Result<(), ValidationError> {
        let normalized = Event::new(EventId::new(), template, timestamp)?.draft();
        validate_rule(normalized.date, rule, ends_on)?;
        self.template = normalized;
        self.rule = rule;
        self.ends_on = ends_on;
        self.updated_at = timestamp;
        Ok(())
    }

    /// Returns the series identifier.
    #[must_use]
    pub const fn id(&self) -> RecurrenceSeriesId {
        self.id
    }

    /// Returns the event template, whose date is the series start date.
    #[must_use]
    pub fn template(&self) -> EventDraft {
        self.template.clone()
    }

    /// Returns the recurrence rule.
    #[must_use]
    pub const fn rule(&self) -> RecurrenceRule {
        self.rule
    }

    /// Returns the optional inclusive end date.
    #[must_use]
    pub const fn ends_on(&self) -> Option<Date> {
        self.ends_on
    }

    /// Returns the creation timestamp.
    #[must_use]
    pub const fn created_at(&self) -> Timestamp {
        self.created_at
    }

    /// Returns the latest revision timestamp.
    #[must_use]
    pub const fn updated_at(&self) -> Timestamp {
        self.updated_at
    }

    /// Returns all base occurrence dates intersecting `range`.
    #[must_use]
    pub fn occurrence_dates(&self, range: crate::domain::DateRange) -> Vec<Date> {
        let start = self.template.date.max(range.start());
        let end = self.ends_on.map_or_else(
            || range.end(),
            |ends_on| {
                ends_on
                    .tomorrow()
                    .unwrap_or_else(|_| range.end())
                    .min(range.end())
            },
        );
        if start >= end {
            return Vec::new();
        }

        let mut dates = Vec::new();
        let mut date = start;
        while date < end {
            let matches = match self.rule {
                RecurrenceRule::Daily => true,
                RecurrenceRule::Weekdays => {
                    matches!(
                        date.weekday(),
                        Weekday::Monday
                            | Weekday::Tuesday
                            | Weekday::Wednesday
                            | Weekday::Thursday
                            | Weekday::Friday
                    )
                }
                RecurrenceRule::Weekly(days) => days.contains(date.weekday()),
            };
            if matches {
                dates.push(date);
            }
            date = date
                .checked_add(jiff::SignedDuration::from_hours(24))
                .unwrap_or(end);
        }
        dates
    }

    /// Returns whether `date` is a generated base occurrence.
    #[must_use]
    pub fn contains_date(&self, date: Date) -> bool {
        let Ok(range) = crate::domain::DateRange::new(date, date.tomorrow().unwrap_or(date)) else {
            return false;
        };
        self.occurrence_dates(range)
            .into_iter()
            .any(|candidate| candidate == date)
    }
}

/// A one-occurrence override for a recurring series.
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub enum RecurrenceExceptionKind {
    /// Do not render the original occurrence.
    Cancelled,
    /// Render this replacement draft for the original occurrence.
    Modified(EventDraft),
}

/// A persisted recurring occurrence exception.
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecurrenceException {
    series_id: RecurrenceSeriesId,
    original_date: Date,
    kind: RecurrenceExceptionKind,
}

impl RecurrenceException {
    /// Creates a cancellation exception.
    #[must_use]
    pub const fn cancelled(series_id: RecurrenceSeriesId, original_date: Date) -> Self {
        Self {
            series_id,
            original_date,
            kind: RecurrenceExceptionKind::Cancelled,
        }
    }

    /// Creates a replacement exception and validates its draft.
    ///
    /// # Errors
    ///
    /// Returns an error when the replacement draft is invalid.
    pub fn modified(
        series_id: RecurrenceSeriesId,
        original_date: Date,
        draft: EventDraft,
        timestamp: Timestamp,
    ) -> Result<Self, ValidationError> {
        let draft = Event::new(EventId::new(), draft, timestamp)?.draft();
        Ok(Self {
            series_id,
            original_date,
            kind: RecurrenceExceptionKind::Modified(draft),
        })
    }

    /// Returns the owning series identifier.
    #[must_use]
    pub const fn series_id(&self) -> RecurrenceSeriesId {
        self.series_id
    }

    /// Returns the original generated date used as the exception key.
    #[must_use]
    pub const fn original_date(&self) -> Date {
        self.original_date
    }

    /// Returns the exception kind.
    #[must_use]
    pub const fn kind(&self) -> &RecurrenceExceptionKind {
        &self.kind
    }
}

/// Stable identity for a rendered calendar occurrence.
#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum OccurrenceId {
    /// A stored standalone event.
    Standalone(EventId),
    /// A generated occurrence keyed by its original series date.
    Recurring {
        /// Owning series identifier.
        series_id: RecurrenceSeriesId,
        /// Original generated date.
        original_date: Date,
    },
}

impl OccurrenceId {
    /// Returns the standalone event ID when this is a standalone occurrence.
    #[must_use]
    pub const fn standalone(self) -> Option<EventId> {
        match self {
            Self::Standalone(id) => Some(id),
            Self::Recurring { .. } => None,
        }
    }

    /// Returns the owning series and original date for a recurring occurrence.
    #[must_use]
    pub const fn recurring(self) -> Option<(RecurrenceSeriesId, Date)> {
        match self {
            Self::Standalone(_) => None,
            Self::Recurring {
                series_id,
                original_date,
            } => Some((series_id, original_date)),
        }
    }
}

/// A standalone or generated event prepared for calendar rendering.
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventOccurrence {
    id: OccurrenceId,
    draft: EventDraft,
}

impl EventOccurrence {
    /// Creates a standalone occurrence from a persisted event.
    #[must_use]
    pub fn standalone(event: &Event) -> Self {
        Self {
            id: OccurrenceId::Standalone(event.id()),
            draft: event.draft(),
        }
    }

    /// Creates a generated recurring occurrence.
    #[must_use]
    pub const fn recurring(
        series_id: RecurrenceSeriesId,
        original_date: Date,
        draft: EventDraft,
    ) -> Self {
        Self {
            id: OccurrenceId::Recurring {
                series_id,
                original_date,
            },
            draft,
        }
    }

    /// Returns the occurrence identity.
    #[must_use]
    pub const fn id(&self) -> OccurrenceId {
        self.id
    }

    /// Returns the category identifier.
    #[must_use]
    pub const fn category_id(&self) -> CategoryId {
        self.draft.category_id
    }

    /// Returns the title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.draft.title
    }

    /// Returns the displayed date.
    #[must_use]
    pub const fn date(&self) -> Date {
        self.draft.date
    }

    /// Returns the start time.
    #[must_use]
    pub const fn start_time(&self) -> Time {
        self.draft.start_time
    }

    /// Returns the end time.
    #[must_use]
    pub const fn end_time(&self) -> Time {
        self.draft.end_time
    }

    /// Returns optional notes.
    #[must_use]
    pub fn notes(&self) -> Option<&str> {
        self.draft.notes.as_deref()
    }

    /// Returns an editable copy of this occurrence.
    #[must_use]
    pub fn draft(&self) -> EventDraft {
        self.draft.clone()
    }
}

/// Expands one series and its exceptions inside a visible date range.
///
/// # Parameters
///
/// - `series`: Series whose base occurrences should be expanded.
/// - `exceptions`: Exceptions belonging to `series`.
/// - `range`: Visible date range to populate.
///
/// # Returns
///
/// Renderable occurrences sorted by displayed date and time.
#[must_use]
pub fn expand_series(
    series: &RecurrenceSeries,
    exceptions: &[RecurrenceException],
    range: crate::domain::DateRange,
) -> Vec<EventOccurrence> {
    let mut occurrences = Vec::new();
    let mut represented = std::collections::HashSet::new();
    for original_date in series.occurrence_dates(range) {
        represented.insert(original_date);
        let exception = exceptions
            .iter()
            .find(|exception| exception.original_date == original_date);
        match exception.map(|exception| &exception.kind) {
            Some(RecurrenceExceptionKind::Cancelled) => {}
            Some(RecurrenceExceptionKind::Modified(draft)) => occurrences.push(
                EventOccurrence::recurring(series.id, original_date, draft.clone()),
            ),
            None => occurrences.push(EventOccurrence::recurring(
                series.id,
                original_date,
                series.template_for(original_date),
            )),
        }
    }

    // A moved replacement can be visible even when its original base date is
    // outside the query range. Keep the stable original-date identity.
    for exception in exceptions {
        let RecurrenceExceptionKind::Modified(draft) = &exception.kind else {
            continue;
        };
        if represented.contains(&exception.original_date) || !range.contains(draft.date) {
            continue;
        }
        occurrences.push(EventOccurrence::recurring(
            series.id,
            exception.original_date,
            draft.clone(),
        ));
    }
    occurrences.sort_by_key(|occurrence| {
        (
            occurrence.date(),
            occurrence.start_time(),
            occurrence.end_time(),
            occurrence.id(),
        )
    });
    occurrences
}

impl RecurrenceSeries {
    fn template_for(&self, date: Date) -> EventDraft {
        let mut draft = self.template.clone();
        draft.date = date;
        draft
    }
}

fn validate_rule(
    start_date: Date,
    rule: RecurrenceRule,
    ends_on: Option<Date>,
) -> Result<(), ValidationError> {
    if ends_on.is_some_and(|ends_on| ends_on < start_date) {
        return Err(ValidationError::InvalidRecurrence(
            "end date must be on or after the start date".to_owned(),
        ));
    }
    if matches!(rule, RecurrenceRule::Weekly(WeekdaySet(0))) {
        return Err(ValidationError::InvalidRecurrence(
            "select at least one weekday".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn category() -> CategoryId {
        CategoryId::from_uuid(Uuid::from_u128(1))
    }

    fn draft(date: Date) -> EventDraft {
        EventDraft::new(
            "Routine",
            date,
            Time::constant(8, 0, 0, 0),
            Time::constant(9, 0, 0, 0),
            category(),
            None,
        )
    }

    fn range(start: Date, end: Date) -> crate::domain::DateRange {
        crate::domain::DateRange::new(start, end).expect("test range is valid")
    }

    #[test]
    fn daily_series_expands_only_the_visible_range() {
        let start = Date::constant(2026, 2, 27);
        let series = RecurrenceSeries::new(
            RecurrenceSeriesId::from_uuid(Uuid::from_u128(2)),
            draft(start),
            RecurrenceRule::Daily,
            None,
            Timestamp::from_second(0).expect("valid timestamp"),
        )
        .expect("series is valid");
        let dates = series.occurrence_dates(range(
            Date::constant(2026, 2, 28),
            Date::constant(2026, 3, 3),
        ));
        assert_eq!(
            dates,
            vec![
                Date::constant(2026, 2, 28),
                Date::constant(2026, 3, 1),
                Date::constant(2026, 3, 2),
            ]
        );
    }

    #[test]
    fn daily_series_keeps_wall_clock_time_across_dst_boundary() {
        let start = Date::constant(2026, 3, 7);
        let series = RecurrenceSeries::new(
            RecurrenceSeriesId::from_uuid(Uuid::from_u128(6)),
            draft(start),
            RecurrenceRule::Daily,
            None,
            Timestamp::from_second(0).expect("valid timestamp"),
        )
        .expect("series is valid");
        let occurrences = expand_series(&series, &[], range(start, Date::constant(2026, 3, 11)));

        assert_eq!(occurrences.len(), 4);
        assert!(occurrences.iter().all(|occurrence| {
            occurrence.start_time() == Time::constant(8, 0, 0, 0)
                && occurrence.end_time() == Time::constant(9, 0, 0, 0)
        }));
    }

    #[test]
    fn weekday_and_weekly_rules_select_the_expected_days() {
        let start = Date::constant(2026, 8, 17); // Monday
        let timestamp = Timestamp::from_second(0).expect("valid timestamp");
        let weekdays = RecurrenceSeries::new(
            RecurrenceSeriesId::from_uuid(Uuid::from_u128(3)),
            draft(start),
            RecurrenceRule::Weekdays,
            None,
            timestamp,
        )
        .expect("series is valid");
        assert_eq!(
            weekdays
                .occurrence_dates(range(start, Date::constant(2026, 8, 24),))
                .len(),
            5
        );

        let selected = WeekdaySet::one(Weekday::Tuesday);
        let weekly = RecurrenceSeries::new(
            RecurrenceSeriesId::from_uuid(Uuid::from_u128(4)),
            draft(start),
            RecurrenceRule::Weekly(selected),
            None,
            timestamp,
        )
        .expect("series is valid");
        assert_eq!(
            weekly.occurrence_dates(range(start, Date::constant(2026, 9, 1))),
            vec![Date::constant(2026, 8, 18), Date::constant(2026, 8, 25),]
        );
    }

    #[test]
    fn exceptions_cancel_and_replace_one_occurrence() {
        let start = Date::constant(2026, 2, 27);
        let timestamp = Timestamp::from_second(0).expect("valid timestamp");
        let series = RecurrenceSeries::new(
            RecurrenceSeriesId::from_uuid(Uuid::from_u128(5)),
            draft(start),
            RecurrenceRule::Daily,
            None,
            timestamp,
        )
        .expect("series is valid");
        let cancelled = RecurrenceException::cancelled(series.id(), start);
        let moved = RecurrenceException::modified(
            series.id(),
            Date::constant(2026, 3, 1),
            EventDraft::new(
                "Moved routine",
                Date::constant(2026, 3, 4),
                Time::constant(10, 0, 0, 0),
                Time::constant(11, 0, 0, 0),
                category(),
                None,
            ),
            timestamp,
        )
        .expect("replacement is valid");
        let occurrences = expand_series(
            &series,
            &[cancelled, moved],
            range(start, Date::constant(2026, 3, 5)),
        );
        assert!(
            !occurrences
                .iter()
                .any(|occurrence| occurrence.date() == start)
        );
        assert!(occurrences.iter().any(|occurrence| {
            occurrence.title() == "Moved routine" && occurrence.date() == Date::constant(2026, 3, 4)
        }));
    }
}
