use std::cmp::Ordering;

use jiff::civil::{Date, Time};

use crate::domain::{DateRange, EventOccurrence, OccurrenceId};

const MINUTES_PER_DAY: f32 = 24.0 * 60.0;

/// Rendering constants for a calendar plane.
///
/// # Fields
///
/// - `pixels_per_minute`: Vertical scale of the calendar plane.
/// - `minimum_event_height`: Minimum visual height assigned to an event.
/// - `minimum_occupancy_minutes`: Minimum collision occupancy for an event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutMetrics {
    pixels_per_minute: f32,
    minimum_event_height: f32,
    minimum_occupancy_minutes: f32,
}

impl LayoutMetrics {
    /// Creates validated rendering metrics for a calendar plane.
    ///
    /// # Parameters
    ///
    /// - `pixels_per_minute`: Vertical scale of the calendar plane.
    /// - `minimum_event_height`: Minimum visual height assigned to an event.
    /// - `minimum_occupancy_minutes`: Minimum collision occupancy for an event.
    ///
    /// # Returns
    ///
    /// Rendering metrics containing the supplied positive finite values.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - `pixels_per_minute` is not finite and greater than zero.
    /// - `minimum_event_height` is not finite and greater than zero.
    /// - `minimum_occupancy_minutes` is not finite and greater than zero.
    pub fn new(
        pixels_per_minute: f32,
        minimum_event_height: f32,
        minimum_occupancy_minutes: f32,
    ) -> Result<Self, LayoutError> {
        if !pixels_per_minute.is_finite() || pixels_per_minute <= 0.0 {
            return Err(LayoutError::InvalidMetric("pixels per minute"));
        }
        if !minimum_event_height.is_finite() || minimum_event_height <= 0.0 {
            return Err(LayoutError::InvalidMetric("minimum event height"));
        }
        if !minimum_occupancy_minutes.is_finite() || minimum_occupancy_minutes <= 0.0 {
            return Err(LayoutError::InvalidMetric("minimum occupancy minutes"));
        }

        Ok(Self {
            pixels_per_minute,
            minimum_event_height,
            minimum_occupancy_minutes,
        })
    }

    /// Returns the vertical scale of the calendar plane.
    #[must_use]
    pub const fn pixels_per_minute(self) -> f32 {
        self.pixels_per_minute
    }

    /// Returns the minimum visual event height.
    #[must_use]
    pub const fn minimum_event_height(self) -> f32 {
        self.minimum_event_height
    }

    /// Returns the minimum collision occupancy in minutes.
    #[must_use]
    pub const fn minimum_occupancy_minutes(self) -> f32 {
        self.minimum_occupancy_minutes
    }
}

impl Default for LayoutMetrics {
    fn default() -> Self {
        Self {
            pixels_per_minute: 1.5,
            minimum_event_height: 22.0,
            minimum_occupancy_minutes: 15.0,
        }
    }
}

/// Describes why calendar layout could not be produced.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LayoutError {
    InvalidMetric(&'static str),
    DateArithmetic,
    RangeTooWide,
    TooManyLanes,
}

/// The screen-space placement of one event in a day column.
///
/// # Fields
///
/// - `occurrence_id`: Identifier of the positioned occurrence.
/// - `day_offset`: Zero-based day offset from the layout range start.
/// - `top`: Vertical offset in pixels.
/// - `height`: Visual event height in pixels.
/// - `lane`: Zero-based overlap lane.
/// - `lane_span`: Number of lanes occupied by the event.
/// - `lane_count`: Total lanes in the event's overlap cluster.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionedEvent {
    occurrence_id: OccurrenceId,
    day_offset: u8,
    top: f32,
    height: f32,
    lane: u16,
    lane_span: u16,
    lane_count: u16,
}

impl PositionedEvent {
    /// Returns the positioned event identifier.
    #[must_use]
    pub const fn occurrence_id(self) -> OccurrenceId {
        self.occurrence_id
    }

    /// Returns the zero-based day offset.
    #[must_use]
    pub const fn day_offset(self) -> u8 {
        self.day_offset
    }

    /// Returns the vertical offset in pixels.
    #[must_use]
    pub const fn top(self) -> f32 {
        self.top
    }

    /// Returns the visual event height in pixels.
    #[must_use]
    pub const fn height(self) -> f32 {
        self.height
    }

    /// Returns the event's zero-based overlap lane.
    #[must_use]
    pub const fn lane(self) -> u16 {
        self.lane
    }

    /// Returns the number of lanes occupied by the event.
    #[must_use]
    pub const fn lane_span(self) -> u16 {
        self.lane_span
    }

    /// Returns the total number of lanes in the overlap cluster.
    #[must_use]
    pub const fn lane_count(self) -> u16 {
        self.lane_count
    }
}

#[derive(Debug, Clone, Copy)]
struct WorkingEvent {
    occurrence_id: OccurrenceId,
    start: f32,
    actual_end: f32,
    occupied_end: f32,
}

#[derive(Debug, Clone)]
struct Placement {
    event: WorkingEvent,
    lane: usize,
}

/// Lay out all events in a date range using end-exclusive overlap lanes.
///
/// Events are kept in their true time positions, while a minimum occupancy is
/// used for collision detection so very short adjacent events remain clickable
/// and visually distinct. Events outside the requested range are ignored.
///
/// # Parameters
///
/// - `events`: Events to position.
/// - `range`: Date range represented by the calendar plane.
/// - `metrics`: Rendering metrics used to convert time into pixels.
///
/// # Returns
///
/// Screen-space placements sorted by day, vertical position, and event ID.
///
/// # Errors
///
/// Returns an error when:
///
/// - The rendering metrics are invalid.
/// - Date arithmetic cannot map an event into the requested range.
/// - The range contains more days than the layout can address.
/// - An overlap cluster contains more lanes than the layout can address.
pub fn layout_events(
    events: &[EventOccurrence],
    range: DateRange,
    metrics: LayoutMetrics,
) -> Result<Vec<PositionedEvent>, LayoutError> {
    // Revalidate values in case a future constructor or deserializer bypasses it.
    LayoutMetrics::new(
        metrics.pixels_per_minute,
        metrics.minimum_event_height,
        metrics.minimum_occupancy_minutes,
    )?;

    let day_count = day_count(range)?;
    let mut by_day = vec![Vec::<WorkingEvent>::new(); day_count];
    for event in events {
        if !range.contains(event.date()) {
            continue;
        }
        let day_offset = day_offset(range.start(), event.date(), day_count)?;
        let start = time_to_minutes(event.start_time());
        let actual_end = time_to_minutes(event.end_time()).max(start);
        let occupied_end = (start + metrics.minimum_occupancy_minutes)
            .max(actual_end)
            .min(MINUTES_PER_DAY);
        by_day[day_offset].push(WorkingEvent {
            occurrence_id: event.id(),
            start,
            actual_end,
            occupied_end,
        });
    }

    let mut result = Vec::with_capacity(events.len());
    for (day_offset, mut day_events) in by_day.into_iter().enumerate() {
        day_events.sort_by(|left, right| {
            left.start
                .partial_cmp(&right.start)
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    left.occupied_end
                        .partial_cmp(&right.occupied_end)
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| left.occurrence_id.cmp(&right.occurrence_id))
        });

        let mut cursor = 0;
        while cursor < day_events.len() {
            let cluster_start = cursor;
            let mut cluster_end = day_events[cursor].occupied_end;
            cursor += 1;
            while cursor < day_events.len() && day_events[cursor].start < cluster_end {
                cluster_end = cluster_end.max(day_events[cursor].occupied_end);
                cursor += 1;
            }

            let cluster = &day_events[cluster_start..cursor];
            let mut lanes: Vec<Vec<WorkingEvent>> = Vec::new();
            let mut placements = Vec::with_capacity(cluster.len());

            for event in cluster.iter().copied() {
                let lane = lanes
                    .iter()
                    .position(|lane_events| {
                        lane_events
                            .last()
                            .is_some_and(|previous| previous.occupied_end <= event.start)
                    })
                    .unwrap_or_else(|| {
                        lanes.push(Vec::new());
                        lanes.len() - 1
                    });
                lanes[lane].push(event);
                placements.push(Placement { event, lane });
            }

            let lane_count = u16::try_from(lanes.len()).map_err(|_| LayoutError::TooManyLanes)?;
            for placement in placements {
                let mut lane_span = 1u16;
                for lane_events in lanes.iter().skip(placement.lane + 1) {
                    let blocked = lane_events
                        .iter()
                        .any(|other| overlaps(placement.event, *other));
                    if blocked {
                        break;
                    }
                    lane_span += 1;
                }

                let top = placement.event.start * metrics.pixels_per_minute;
                let height = ((placement.event.actual_end - placement.event.start)
                    * metrics.pixels_per_minute)
                    .max(metrics.minimum_event_height)
                    .min(MINUTES_PER_DAY.mul_add(metrics.pixels_per_minute, -top));
                result.push(PositionedEvent {
                    occurrence_id: placement.event.occurrence_id,
                    day_offset: u8::try_from(day_offset).map_err(|_| LayoutError::RangeTooWide)?,
                    top,
                    height,
                    lane: u16::try_from(placement.lane).map_err(|_| LayoutError::TooManyLanes)?,
                    lane_span,
                    lane_count,
                });
            }
        }
    }

    result.sort_by(|left, right| {
        left.day_offset
            .cmp(&right.day_offset)
            .then_with(|| left.top.partial_cmp(&right.top).unwrap_or(Ordering::Equal))
            .then_with(|| left.occurrence_id.cmp(&right.occurrence_id))
    });
    Ok(result)
}

fn day_count(range: DateRange) -> Result<usize, LayoutError> {
    let mut current = range.start();
    let mut count = 0_usize;
    while current < range.end() {
        count += 1;
        if count > usize::from(u8::MAX) + 1 {
            return Err(LayoutError::RangeTooWide);
        }
        current = current
            .tomorrow()
            .map_err(|_| LayoutError::DateArithmetic)?;
    }
    Ok(count)
}

fn day_offset(start: Date, date: Date, day_count: usize) -> Result<usize, LayoutError> {
    let mut current = start;
    for offset in 0..day_count {
        if current == date {
            return Ok(offset);
        }
        current = current
            .tomorrow()
            .map_err(|_| LayoutError::DateArithmetic)?;
    }
    Err(LayoutError::DateArithmetic)
}

fn time_to_minutes(time: Time) -> f32 {
    let hours = f32::from(time.hour());
    let minutes = f32::from(time.minute());
    let seconds = f32::from(time.second());
    #[allow(clippy::cast_precision_loss)]
    let subsecond = time.subsec_nanosecond() as f32 / 60_000_000_000.0;
    hours.mul_add(60.0, minutes + seconds / 60.0 + subsecond)
}

fn overlaps(left: WorkingEvent, right: WorkingEvent) -> bool {
    left.start < right.occupied_end && right.start < left.occupied_end
}
