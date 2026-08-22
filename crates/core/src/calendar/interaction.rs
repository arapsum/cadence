//! Time and date proposal math used by direct calendar manipulation.

use jiff::{SignedDuration, civil::Time};

use crate::domain::{EventDraft, minutes_since_midnight};

const MINUTES_PER_DAY: i32 = 24 * 60;
const LAST_MINUTE: i32 = MINUTES_PER_DAY - 1;

/// Edge of an event used for a resize operation.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum ResizeEdge {
    /// The event start is being changed.
    Start,
    /// The event end is being changed.
    End,
}

/// Proposes a moved event at a snapped date and time.
///
/// # Parameters
///
/// - `draft`: Original event values.
/// - `day_delta`: Number of calendar days to move.
/// - `minute_delta`: Pointer movement in minutes relative to the original start.
/// - `snap_minutes`: Configured snapping interval.
///
/// # Returns
///
/// A moved draft clamped to a valid single-day interval, or `None` when the
/// requested date cannot be represented.
#[must_use]
pub fn propose_move(
    draft: &EventDraft,
    day_delta: i32,
    minute_delta: i32,
    snap_minutes: u16,
) -> Option<EventDraft> {
    let duration = minutes_since_midnight(draft.end_time)
        .saturating_sub(minutes_since_midnight(draft.start_time));
    let duration = i32::from(duration);
    let start = snap_nearest(
        i32::from(minutes_since_midnight(draft.start_time)) + minute_delta,
        snap_minutes,
    );
    let latest_start = LAST_MINUTE.saturating_sub(duration).max(0);
    let start = start.clamp(0, latest_start);
    let date = draft
        .date
        .checked_add(SignedDuration::from_hours(i64::from(day_delta) * 24))
        .ok()?;
    Some(EventDraft {
        date,
        start_time: time_from_minutes(start),
        end_time: time_from_minutes((start + duration).min(LAST_MINUTE)),
        ..draft.clone()
    })
}

/// Proposes a resized event at a snapped pointer time.
///
/// # Parameters
///
/// - `draft`: Original event values.
/// - `edge`: Start or end boundary being changed.
/// - `pointer_minute`: Pointer position in minutes since midnight.
/// - `snap_minutes`: Configured snapping interval.
///
/// # Returns
///
/// A resized draft with at least one snap interval of duration.
#[must_use]
pub fn propose_resize(
    draft: &EventDraft,
    edge: ResizeEdge,
    pointer_minute: i32,
    snap_minutes: u16,
) -> EventDraft {
    let interval = i32::from(snap_minutes.clamp(1, 60));
    let original_start = i32::from(minutes_since_midnight(draft.start_time));
    let original_end = i32::from(minutes_since_midnight(draft.end_time));
    let minimum_end = original_start.saturating_add(interval).min(LAST_MINUTE);
    let minimum_start = original_end.saturating_sub(interval).max(0);
    let snapped = snap_nearest(pointer_minute, snap_minutes);

    let (start, end) = match edge {
        ResizeEdge::Start => (snapped.clamp(0, minimum_start), original_end),
        ResizeEdge::End => (original_start, snapped.clamp(minimum_end, LAST_MINUTE)),
    };
    EventDraft {
        start_time: time_from_minutes(start),
        end_time: time_from_minutes(end),
        ..draft.clone()
    }
}

fn snap_nearest(minutes: i32, snap_minutes: u16) -> i32 {
    let interval = i64::from(snap_minutes.clamp(1, 60));
    let minutes = i64::from(minutes);
    let snapped = (minutes + interval / 2).div_euclid(interval) * interval;
    i32::try_from(snapped.clamp(0, i64::from(LAST_MINUTE))).expect("snapped minutes fit in i32")
}

fn time_from_minutes(minutes: i32) -> Time {
    let minutes = minutes.clamp(0, LAST_MINUTE);
    Time::constant(
        i8::try_from(minutes / 60).expect("clamped hour fits in i8"),
        i8::try_from(minutes % 60).expect("clamped minute fits in i8"),
        0,
        0,
    )
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Date, Time};

    use super::{ResizeEdge, propose_move, propose_resize};
    use crate::domain::{CategoryId, EventDraft};
    use uuid::Uuid;

    fn draft(date: Date, start: i8, end: i8) -> EventDraft {
        EventDraft::new(
            "Focus",
            date,
            Time::constant(start, 0, 0, 0),
            Time::constant(end, 0, 0, 0),
            CategoryId::from_uuid(Uuid::from_u128(1)),
            None,
        )
    }

    #[test]
    fn move_snaps_preserves_duration_and_changes_day() {
        let original = draft(Date::constant(2026, 8, 21), 9, 10);
        let moved = propose_move(&original, 2, 17, 15).expect("date can move");
        assert_eq!(moved.date, Date::constant(2026, 8, 23));
        assert_eq!(moved.start_time, Time::constant(9, 15, 0, 0));
        assert_eq!(moved.end_time, Time::constant(10, 15, 0, 0));
    }

    #[test]
    fn move_clamps_to_the_end_of_the_day() {
        let original = draft(Date::constant(2026, 8, 21), 22, 23);
        let moved = propose_move(&original, 0, 240, 15).expect("date can move");
        assert_eq!(moved.start_time, Time::constant(22, 59, 0, 0));
        assert_eq!(moved.end_time, Time::constant(23, 59, 0, 0));
    }

    #[test]
    fn resize_enforces_a_snap_interval() {
        let original = draft(Date::constant(2026, 8, 21), 9, 10);
        let start = propose_resize(&original, ResizeEdge::Start, 9 * 60 + 52, 15);
        let end = propose_resize(&original, ResizeEdge::End, 9 * 60 + 2, 15);
        assert_eq!(start.start_time, Time::constant(9, 45, 0, 0));
        assert_eq!(start.end_time, Time::constant(10, 0, 0, 0));
        assert_eq!(end.start_time, Time::constant(9, 0, 0, 0));
        assert_eq!(end.end_time, Time::constant(9, 15, 0, 0));
    }
}
