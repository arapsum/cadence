//! UI-independent event editor behavior.

use chrono::Datelike as _;
use jiff::civil::{Date, Time};

use crate::domain::{
    CategoryId, EventDraft, EventOccurrence, OccurrenceId, RecurrenceRule, ReminderOffset,
    ValidationError,
};

/// The operation currently being performed by the event editor.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EditorMode {
    Create,
    Edit(OccurrenceId),
}

/// Editable values held by the event form before they are committed.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FormDraft {
    pub title: String,
    pub notes: String,
    pub date: Date,
    pub start_time: Time,
    pub end_time: Time,
    pub category_id: Option<CategoryId>,
    pub recurrence: Option<RecurrenceRule>,
    pub ends_on: Option<Date>,
    pub reminder: Option<ReminderOffset>,
}

impl FormDraft {
    /// Copies an existing event into editable form state.
    ///
    /// # Parameters
    ///
    /// - `event`: Event whose editable values should seed the form.
    ///
    /// # Returns
    ///
    /// A form draft containing the event's title, notes, date, times, and category.
    #[must_use]
    pub fn from_occurrence(event: &EventOccurrence) -> Self {
        let draft = event.draft();
        Self {
            title: draft.title,
            notes: draft.notes.unwrap_or_default(),
            date: draft.date,
            start_time: draft.start_time,
            end_time: draft.end_time,
            category_id: Some(draft.category_id),
            recurrence: None,
            ends_on: None,
            reminder: draft.reminder,
        }
    }

    /// Validates the form state and converts it into a domain draft.
    ///
    /// # Returns
    ///
    /// A domain event draft when all required fields are valid.
    ///
    /// # Errors
    ///
    /// Returns field-level errors when:
    ///
    /// - The title is empty after trimming.
    /// - No category is selected.
    /// - The end time is not later than the start time.
    /// - A recurrence end date precedes the event date.
    pub fn to_domain(&self) -> Result<EventDraft, Box<FormErrors>> {
        let category_id = self.category_id.ok_or_else(|| {
            Box::new(FormErrors {
                category: Some("Choose a category.".to_owned()),
                ..FormErrors::default()
            })
        })?;
        if self.ends_on.is_some_and(|ends_on| ends_on < self.date) {
            return Err(Box::new(FormErrors {
                ends_on: Some("End date must be on or after the event date.".to_owned()),
                ..FormErrors::default()
            }));
        }

        let draft = EventDraft::new(
            self.title.clone(),
            self.date,
            self.start_time,
            self.end_time,
            category_id,
            Some(self.notes.clone()),
        )
        .with_reminder(self.reminder);

        validate_draft(&draft)?;
        Ok(draft)
    }
}

/// Field-level validation messages displayed beside the editor controls.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct FormErrors {
    pub title: Option<String>,
    pub date: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub category: Option<String>,
    pub recurrence: Option<String>,
    pub ends_on: Option<String>,
    pub conflict: Option<String>,
}

impl FormErrors {
    /// Reports whether the form has no field-level validation errors.
    ///
    /// # Returns
    ///
    /// `true` when every form field is valid.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.date.is_none()
            && self.start_time.is_none()
            && self.end_time.is_none()
            && self.category.is_none()
            && self.recurrence.is_none()
            && self.ends_on.is_none()
            && self.conflict.is_none()
    }
}

fn validate_draft(draft: &EventDraft) -> Result<(), Box<FormErrors>> {
    let mut errors = FormErrors::default();
    if draft.title.trim().is_empty() {
        errors.title = Some(ValidationError::EmptyTitle.to_string());
    }
    if draft.end_time <= draft.start_time {
        let message = ValidationError::EndNotAfterStart {
            start: draft.start_time,
            end: draft.end_time,
        }
        .to_string();
        errors.end_time = Some(message);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Box::new(errors))
    }
}

/// Calculates the initial one-hour range for a new event.
///
/// # Parameters
///
/// - `date`: Date selected for the new event.
/// - `today`: Current local date.
/// - `current_time`: Current local wall-clock time.
/// - `day_start`: Earliest configured display time.
/// - `day_end`: Latest configured display time.
/// - `snap_minutes`: Snap interval in minutes.
///
/// # Returns
///
/// A start and end time snapped and clamped to the configured display day.
#[must_use]
pub fn default_times(
    date: Date,
    today: Date,
    current_time: Time,
    day_start: Time,
    day_end: Time,
    snap_minutes: u16,
) -> (Time, Time) {
    let mut start = if date == today {
        snap_up(current_time, snap_minutes)
    } else {
        day_start
    };
    let latest_start = day_end
        .checked_sub(jiff::SignedDuration::from_hours(1))
        .unwrap_or(day_start);
    if start > latest_start {
        start = latest_start;
    }
    let end = start
        .checked_add(jiff::SignedDuration::from_hours(1))
        .unwrap_or(day_end);
    (start, end.min(day_end))
}

/// Rounds a wall-clock time up to the next snap boundary.
///
/// # Parameters
///
/// - `time`: Wall-clock time to round.
/// - `snap_minutes`: Snap interval in minutes; values below one use one minute.
///
/// # Returns
///
/// The rounded time, limited to the final representable minute of the day.
///
/// # Panics
///
/// Panics when:
///
/// - The computed hour or minute cannot fit in Jiff's supported `i8` fields.
#[must_use]
pub fn snap_up(time: Time, snap_minutes: u16) -> Time {
    let total = i64::from(time.hour()) * 60 + i64::from(time.minute());
    let snap = i64::from(snap_minutes.max(1));
    let rounded = ((total + snap - 1) / snap) * snap;
    let rounded = rounded.min(23 * 60 + 59);
    Time::constant(
        i8::try_from(rounded / 60).expect("snapped hour fits in i8"),
        i8::try_from(rounded % 60).expect("snapped minute fits in i8"),
        0,
        0,
    )
}

/// Builds select options for snapped times and existing off-grid values.
///
/// # Parameters
///
/// - `snap_minutes`: Snap interval in minutes; values below one use one minute.
/// - `preferred_start`: Time at which the options should begin.
/// - `extra`: Existing event times that must remain selectable.
///
/// # Returns
///
/// Deduplicated times covering the day and every value in `extra`, ordered from
/// `preferred_start` through the end of the day and then wrapped through the
/// earlier times.
///
/// # Panics
///
/// Panics when:
///
/// - The snap interval cannot fit the platform's `usize` step size.
/// - A generated time component cannot fit in Jiff's supported `i8` fields.
#[must_use]
pub fn time_options(snap_minutes: u16, preferred_start: Time, extra: &[Time]) -> Vec<Time> {
    let snap = i64::from(snap_minutes.max(1));
    let mut options = (0_i64..24 * 60)
        .step_by(usize::try_from(snap).expect("snap interval fits usize"))
        .map(|minutes| {
            Time::constant(
                i8::try_from(minutes / 60).expect("time option hour fits in i8"),
                i8::try_from(minutes % 60).expect("time option minute fits in i8"),
                0,
                0,
            )
        })
        .collect::<Vec<_>>();
    options.extend(extra.iter().copied());
    options.sort_unstable();
    options.dedup();
    if let Some(pivot) = options.iter().position(|time| *time >= preferred_start) {
        options.rotate_left(pivot);
    }
    options
}

/// Converts a Jiff civil date to the Chrono date used by GPUI Component.
///
/// # Parameters
///
/// - `date`: Jiff civil date to convert.
///
/// # Returns
///
/// The equivalent Chrono `NaiveDate`.
///
/// # Panics
///
/// Panics when:
///
/// - A valid Jiff date cannot be represented by Chrono.
#[must_use]
pub fn chrono_date(date: Date) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(
        i32::from(date.year()),
        u32::try_from(date.month()).expect("valid Jiff month fits u32"),
        u32::try_from(date.day()).expect("valid Jiff day fits u32"),
    )
    .expect("valid Jiff date converts to a valid Chrono date")
}

/// Converts a Chrono date from GPUI Component back to a Jiff civil date.
///
/// # Parameters
///
/// - `date`: Chrono `NaiveDate` to convert.
///
/// # Returns
///
/// The equivalent Jiff civil date.
///
/// # Panics
///
/// Panics when:
///
/// - A Chrono date component cannot fit Jiff's supported fields.
#[must_use]
pub fn jiff_date(date: chrono::NaiveDate) -> Date {
    Date::constant(
        i16::try_from(date.year()).expect("chrono year fits in i16"),
        i8::try_from(date.month()).expect("chrono month fits in i8"),
        i8::try_from(date.day()).expect("chrono day fits in i8"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::CategoryId;
    use uuid::Uuid;

    fn category() -> CategoryId {
        CategoryId::from_uuid(Uuid::from_u128(1))
    }

    #[test]
    fn empty_title_and_invalid_duration_are_field_errors() {
        let form = FormDraft {
            title: "  ".to_owned(),
            notes: String::new(),
            date: Date::constant(2026, 2, 28),
            start_time: Time::constant(10, 0, 0, 0),
            end_time: Time::constant(9, 0, 0, 0),
            category_id: Some(category()),
            recurrence: None,
            ends_on: None,
            reminder: None,
        };

        let errors = form.to_domain().expect_err("invalid form must be rejected");
        assert_eq!(errors.title.as_deref(), Some("Add a title."));
        assert_eq!(
            errors.end_time.as_deref(),
            Some("End time must be later than start time.")
        );
    }

    #[test]
    fn recurrence_end_date_cannot_precede_the_series_start() {
        let start = Date::constant(2026, 8, 20);
        let form = FormDraft {
            title: "Routine".to_owned(),
            notes: String::new(),
            date: start,
            start_time: Time::constant(8, 0, 0, 0),
            end_time: Time::constant(9, 0, 0, 0),
            category_id: Some(category()),
            recurrence: Some(RecurrenceRule::Daily),
            ends_on: Some(Date::constant(2026, 8, 19)),
            reminder: None,
        };

        let errors = form
            .to_domain()
            .expect_err("invalid recurrence bounds must be rejected");
        assert_eq!(
            errors.ends_on.as_deref(),
            Some("End date must be on or after the event date.")
        );
    }

    #[test]
    fn default_times_snap_today_and_clamp_late_evening() {
        let date = Date::constant(2026, 8, 20);
        let (start, end) = default_times(
            date,
            date,
            Time::constant(21, 58, 0, 0),
            Time::constant(6, 0, 0, 0),
            Time::constant(22, 0, 0, 0),
            15,
        );
        assert_eq!(start, Time::constant(21, 0, 0, 0));
        assert_eq!(end, Time::constant(22, 0, 0, 0));
    }

    #[test]
    fn time_options_keep_off_grid_values_for_existing_events() {
        let options = time_options(
            15,
            Time::constant(6, 0, 0, 0),
            &[Time::constant(8, 7, 0, 0), Time::constant(9, 7, 0, 0)],
        );
        assert!(options.contains(&Time::constant(8, 7, 0, 0)));
        assert!(options.contains(&Time::constant(9, 7, 0, 0)));
        assert_eq!(options.first(), Some(&Time::constant(6, 0, 0, 0)));
        assert_eq!(options.last(), Some(&Time::constant(5, 45, 0, 0)));
    }

    #[test]
    fn time_options_wrap_midnight_after_the_preferred_day() {
        let options = time_options(60, Time::constant(7, 0, 0, 0), &[]);

        assert_eq!(options.first(), Some(&Time::constant(7, 0, 0, 0)));
        assert_eq!(options[16], Time::constant(23, 0, 0, 0));
        assert_eq!(options[17], Time::constant(0, 0, 0, 0));
    }

    #[test]
    fn chrono_round_trip_preserves_leap_day() {
        let date = Date::constant(2028, 2, 29);
        assert_eq!(jiff_date(chrono_date(date)), date);
    }
}
