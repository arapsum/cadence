//! Conflict detection for standalone and recurring timetable items.

use jiff::civil::{Date, Time, Weekday};

use super::{
    DateRange, Event, EventOccurrence, OccurrenceId, RecurrenceException, RecurrenceExceptionKind,
    RecurrenceRule, RecurrenceSeries,
};

/// Details about an existing occurrence that blocks a proposed change.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScheduleConflict {
    blocking_occurrence: OccurrenceId,
    date: Date,
    start_time: Time,
    end_time: Time,
    title: String,
}

impl ScheduleConflict {
    /// Creates a conflict description from an existing occurrence.
    #[must_use]
    pub fn from_occurrence(occurrence: &EventOccurrence) -> Self {
        Self {
            blocking_occurrence: occurrence.id(),
            date: occurrence.date(),
            start_time: occurrence.start_time(),
            end_time: occurrence.end_time(),
            title: occurrence.title().to_owned(),
        }
    }

    /// Returns the occurrence that blocks the proposed change.
    #[must_use]
    pub const fn blocking_occurrence(&self) -> OccurrenceId {
        self.blocking_occurrence
    }

    /// Returns the date on which the conflict occurs.
    #[must_use]
    pub const fn date(&self) -> Date {
        self.date
    }

    /// Returns the blocking occurrence's start time.
    #[must_use]
    pub const fn start_time(&self) -> Time {
        self.start_time
    }

    /// Returns the blocking occurrence's end time.
    #[must_use]
    pub const fn end_time(&self) -> Time {
        self.end_time
    }

    /// Returns the blocking occurrence's title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }
}

/// A pair of rendered occurrences that overlap in a visible range.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct OccurrenceConflict {
    first: OccurrenceId,
    second: OccurrenceId,
}

impl OccurrenceConflict {
    /// Returns the first conflicting occurrence.
    #[must_use]
    pub const fn first(&self) -> OccurrenceId {
        self.first
    }

    /// Returns the second conflicting occurrence.
    #[must_use]
    pub const fn second(&self) -> OccurrenceId {
        self.second
    }
}

/// Finds an overlap for a proposed standalone event.
///
/// # Parameters
///
/// - `candidate`: Event that is about to be created or updated.
/// - `events`: Existing standalone events.
/// - `series`: Existing recurring series.
/// - `exceptions`: Existing recurring exceptions.
///
/// # Returns
///
/// The first blocking occurrence, or `None` when the candidate is conflict-free.
#[must_use]
pub fn find_event_conflict(
    candidate: &Event,
    events: &[Event],
    series: &[RecurrenceSeries],
    exceptions: &[RecurrenceException],
) -> Option<ScheduleConflict> {
    let candidate_occurrence = EventOccurrence::standalone(candidate);
    let existing = occurrences_on_date(
        candidate.date(),
        events,
        series,
        exceptions,
        Some(candidate.id()),
    );
    existing.iter().find_map(|occurrence| {
        overlapping_occurrences(&candidate_occurrence, occurrence)
            .map(|()| ScheduleConflict::from_occurrence(occurrence))
    })
}

/// Finds an overlap for a proposed recurring series.
///
/// # Parameters
///
/// - `candidate`: Series that is about to be created or updated.
/// - `candidate_exceptions`: Exceptions currently belonging to `candidate`.
/// - `events`: Existing standalone events.
/// - `series`: Existing recurring series.
/// - `exceptions`: Exceptions belonging to `series`.
///
/// # Returns
///
/// The first blocking occurrence, or `None` when every candidate occurrence is conflict-free.
#[must_use]
pub fn find_series_conflict(
    candidate: &RecurrenceSeries,
    candidate_exceptions: &[RecurrenceException],
    events: &[Event],
    series: &[RecurrenceSeries],
    exceptions: &[RecurrenceException],
) -> Option<ScheduleConflict> {
    for event in events {
        let candidate_occurrences = occurrences_on_date(
            event.date(),
            &[],
            std::slice::from_ref(candidate),
            candidate_exceptions,
            None,
        );
        let existing = EventOccurrence::standalone(event);
        if let Some(conflict) = candidate_occurrences.iter().find_map(|occurrence| {
            overlapping_occurrences(occurrence, &existing)
                .map(|()| ScheduleConflict::from_occurrence(&existing))
        }) {
            return Some(conflict);
        }
    }

    for existing_series in series {
        if existing_series.id() == candidate.id() {
            continue;
        }
        let existing_exceptions = exceptions
            .iter()
            .filter(|exception| exception.series_id() == existing_series.id())
            .cloned()
            .collect::<Vec<_>>();
        if let Some(conflict) = find_series_pair_conflict(
            candidate,
            candidate_exceptions,
            existing_series,
            &existing_exceptions,
        ) {
            return Some(conflict);
        }
    }

    find_series_self_conflict(candidate, candidate_exceptions)
}

/// Finds an overlap for a proposed recurring exception.
///
/// # Parameters
///
/// - `candidate`: Exception that is about to be inserted or replaced.
/// - `series`: Owning recurring series.
/// - `events`: Existing standalone events.
/// - `series_list`: Existing recurring series.
/// - `exceptions`: Existing exceptions.
///
/// # Returns
///
/// The first blocking occurrence, or `None` when the replacement is conflict-free.
#[must_use]
pub fn find_exception_conflict(
    candidate: &RecurrenceException,
    series: &RecurrenceSeries,
    events: &[Event],
    series_list: &[RecurrenceSeries],
    exceptions: &[RecurrenceException],
) -> Option<ScheduleConflict> {
    let RecurrenceExceptionKind::Modified(draft) = candidate.kind() else {
        return None;
    };
    let candidate_occurrence = EventOccurrence::recurring(
        candidate.series_id(),
        candidate.original_date(),
        draft.clone(),
    );

    for event in events {
        let existing = EventOccurrence::standalone(event);
        if overlapping_occurrences(&candidate_occurrence, &existing).is_some() {
            return Some(ScheduleConflict::from_occurrence(&existing));
        }
    }

    for existing_series in series_list {
        if existing_series.id() == candidate.series_id() {
            continue;
        }
        let existing_exceptions = exceptions
            .iter()
            .filter(|exception| exception.series_id() == existing_series.id())
            .cloned()
            .collect::<Vec<_>>();
        let occurrences = occurrences_on_date(
            candidate_occurrence.date(),
            &[],
            std::slice::from_ref(existing_series),
            &existing_exceptions,
            None,
        );
        if let Some(existing) = occurrences
            .iter()
            .find(|existing| overlapping_occurrences(&candidate_occurrence, existing).is_some())
        {
            return Some(ScheduleConflict::from_occurrence(existing));
        }
    }

    let own_exceptions = exceptions
        .iter()
        .filter(|exception| {
            exception.series_id() == candidate.series_id()
                && exception.original_date() != candidate.original_date()
        })
        .cloned()
        .collect::<Vec<_>>();
    let own_occurrences = occurrences_on_date(
        candidate_occurrence.date(),
        &[],
        std::slice::from_ref(series),
        &own_exceptions,
        None,
    );
    own_occurrences.iter().find_map(|existing| {
        overlapping_occurrences(&candidate_occurrence, existing)
            .map(|()| ScheduleConflict::from_occurrence(existing))
    })
}

/// Finds all overlap pairs in already-rendered occurrences.
///
/// # Parameters
///
/// - `occurrences`: Occurrences from a visible day or week range.
///
/// # Returns
///
/// Every pair whose displayed intervals overlap, in input order.
#[must_use]
pub fn find_occurrence_conflicts(occurrences: &[EventOccurrence]) -> Vec<OccurrenceConflict> {
    let mut conflicts = Vec::new();
    for (index, first) in occurrences.iter().enumerate() {
        for second in occurrences.iter().skip(index + 1) {
            if overlapping_occurrences(first, second).is_some() {
                conflicts.push(OccurrenceConflict {
                    first: first.id(),
                    second: second.id(),
                });
            }
        }
    }
    conflicts
}

fn find_series_self_conflict(
    candidate: &RecurrenceSeries,
    exceptions: &[RecurrenceException],
) -> Option<ScheduleConflict> {
    for exception in exceptions {
        let RecurrenceExceptionKind::Modified(draft) = exception.kind() else {
            continue;
        };
        let candidate_occurrence =
            EventOccurrence::recurring(candidate.id(), exception.original_date(), draft.clone());
        let own_occurrences = occurrences_on_date(
            candidate_occurrence.date(),
            &[],
            std::slice::from_ref(candidate),
            exceptions,
            None,
        );
        if let Some(existing) = own_occurrences.iter().find(|existing| {
            existing.id() != candidate_occurrence.id()
                && overlapping_occurrences(&candidate_occurrence, existing).is_some()
        }) {
            return Some(ScheduleConflict::from_occurrence(existing));
        }
    }
    None
}

fn find_series_pair_conflict(
    candidate: &RecurrenceSeries,
    candidate_exceptions: &[RecurrenceException],
    existing: &RecurrenceSeries,
    existing_exceptions: &[RecurrenceException],
) -> Option<ScheduleConflict> {
    let candidate_mask = recurrence_mask(candidate.rule());
    let existing_mask = recurrence_mask(existing.rule());
    let shared_mask = candidate_mask & existing_mask;
    if intervals_overlap(
        candidate.template().start_time,
        candidate.template().end_time,
        existing.template().start_time,
        existing.template().end_time,
    ) && let Some(mut date) = first_shared_date(candidate, existing, shared_mask)
    {
        loop {
            let candidate_occurrence =
                occurrence_for_original_date(candidate, candidate_exceptions, date);
            let existing_occurrence =
                occurrence_for_original_date(existing, existing_exceptions, date);
            if let (Some(candidate_occurrence), Some(existing_occurrence)) =
                (candidate_occurrence, existing_occurrence)
                && overlapping_occurrences(&candidate_occurrence, &existing_occurrence).is_some()
            {
                return Some(ScheduleConflict::from_occurrence(&existing_occurrence));
            }
            let Some(next) = next_shared_date(date, candidate, existing, shared_mask) else {
                break;
            };
            date = next;
        }
    }

    // A modified exception can move an occurrence outside its base pattern.
    for exception in candidate_exceptions {
        let RecurrenceExceptionKind::Modified(draft) = exception.kind() else {
            continue;
        };
        let candidate_occurrence =
            EventOccurrence::recurring(candidate.id(), exception.original_date(), draft.clone());
        let existing_occurrences = occurrences_on_date(
            candidate_occurrence.date(),
            &[],
            std::slice::from_ref(existing),
            existing_exceptions,
            None,
        );
        if let Some(existing_occurrence) = existing_occurrences
            .iter()
            .find(|occurrence| overlapping_occurrences(&candidate_occurrence, occurrence).is_some())
        {
            return Some(ScheduleConflict::from_occurrence(existing_occurrence));
        }
    }
    for exception in existing_exceptions {
        let RecurrenceExceptionKind::Modified(draft) = exception.kind() else {
            continue;
        };
        let existing_occurrence =
            EventOccurrence::recurring(existing.id(), exception.original_date(), draft.clone());
        let candidate_occurrences = occurrences_on_date(
            existing_occurrence.date(),
            &[],
            std::slice::from_ref(candidate),
            candidate_exceptions,
            None,
        );
        if overlapping_occurrences_with_any(&candidate_occurrences, &existing_occurrence) {
            return Some(ScheduleConflict::from_occurrence(&existing_occurrence));
        }
    }
    None
}

fn occurrence_for_original_date(
    series: &RecurrenceSeries,
    exceptions: &[RecurrenceException],
    date: Date,
) -> Option<EventOccurrence> {
    if !series.contains_date(date) {
        return None;
    }
    if let Some(exception) = exceptions
        .iter()
        .find(|exception| exception.original_date() == date)
    {
        return match exception.kind() {
            RecurrenceExceptionKind::Cancelled => None,
            RecurrenceExceptionKind::Modified(draft) => {
                Some(EventOccurrence::recurring(series.id(), date, draft.clone()))
            }
        };
    }
    let mut draft = series.template();
    draft.date = date;
    Some(EventOccurrence::recurring(series.id(), date, draft))
}

fn occurrences_on_date(
    date: Date,
    events: &[Event],
    series: &[RecurrenceSeries],
    exceptions: &[RecurrenceException],
    ignored_event: Option<super::EventId>,
) -> Vec<EventOccurrence> {
    let mut occurrences = events
        .iter()
        .filter(|event| Some(event.id()) != ignored_event && event.date() == date)
        .map(EventOccurrence::standalone)
        .collect::<Vec<_>>();
    let Ok(range) = DateRange::day(date) else {
        return occurrences;
    };
    for series in series {
        let series_exceptions = exceptions
            .iter()
            .filter(|exception| exception.series_id() == series.id())
            .cloned()
            .collect::<Vec<_>>();
        occurrences.extend(super::expand_series(series, &series_exceptions, range));
    }
    occurrences
}

fn overlapping_occurrences(candidate: &EventOccurrence, existing: &EventOccurrence) -> Option<()> {
    if candidate.id() == existing.id() || candidate.date() != existing.date() {
        return None;
    }
    intervals_overlap(
        candidate.start_time(),
        candidate.end_time(),
        existing.start_time(),
        existing.end_time(),
    )
    .then_some(())
}

fn overlapping_occurrences_with_any(
    candidates: &[EventOccurrence],
    existing: &EventOccurrence,
) -> bool {
    candidates
        .iter()
        .any(|candidate| overlapping_occurrences(candidate, existing).is_some())
}

fn intervals_overlap(left_start: Time, left_end: Time, right_start: Time, right_end: Time) -> bool {
    left_start < right_end && right_start < left_end
}

const fn recurrence_mask(rule: RecurrenceRule) -> u8 {
    match rule {
        RecurrenceRule::Daily => 0b0111_1111,
        RecurrenceRule::Weekdays => 0b0001_1111,
        RecurrenceRule::Weekly(days) => days.bits(),
    }
}

fn weekday_bit(day: Weekday) -> u8 {
    1 << day.to_monday_zero_offset()
}

fn first_shared_date(left: &RecurrenceSeries, right: &RecurrenceSeries, mask: u8) -> Option<Date> {
    let start = left.template().date.max(right.template().date);
    let end = match (left.ends_on(), right.ends_on()) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(end), None) | (None, Some(end)) => Some(end),
        (None, None) => None,
    };
    next_matching_date(start, end, mask)
}

fn next_shared_date(
    current: Date,
    left: &RecurrenceSeries,
    right: &RecurrenceSeries,
    mask: u8,
) -> Option<Date> {
    let next = current.tomorrow().ok()?;
    let end = match (left.ends_on(), right.ends_on()) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(end), None) | (None, Some(end)) => Some(end),
        (None, None) => None,
    };
    next_matching_date(next, end, mask)
}

fn next_matching_date(mut date: Date, end: Option<Date>, mask: u8) -> Option<Date> {
    for _ in 0..7 {
        if end.is_some_and(|end| date > end) {
            return None;
        }
        if mask & weekday_bit(date.weekday()) != 0 {
            return Some(date);
        }
        date = date.tomorrow().ok()?;
    }
    None
}
