use gpui::{Bounds, Pixels, Point, px};
use jiff::civil::Date;

use crate::{
    calendar::{ResizeEdge, propose_move, propose_resize},
    domain::{DateRange, EventDraft, EventOccurrence, OccurrenceId},
};

use super::style::PIXELS_PER_MINUTE;

const AUTO_SCROLL_EDGE: f32 = 32.0;
const AUTO_SCROLL_MAX_STEP: f32 = 16.0;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) enum ManipulationKind {
    Move,
    Resize(ResizeEdge),
}

#[derive(Debug, Clone)]
pub(super) struct DragPayload {
    pub(super) occurrence_id: OccurrenceId,
    pub(super) kind: ManipulationKind,
    pub(super) original_day: u8,
    pub(super) range_start: Date,
}

#[derive(Debug, Clone)]
pub(super) struct Manipulation {
    pub(super) event: EventOccurrence,
    pub(super) kind: ManipulationKind,
    pub(super) proposed: EventDraft,
    pub(super) original_day: u8,
    pub(super) range_start: Date,
    pub(super) pointer: Point<Pixels>,
    pub(super) viewport: Bounds<Pixels>,
    pub(super) plane_width: f32,
    pub(super) column_width: f32,
    pub(super) column_count: usize,
    pub(super) scroll_offset: Point<Pixels>,
    grab_offset_minutes: i32,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ManipulationUpdate {
    pub(super) pointer: Point<Pixels>,
    pub(super) viewport: Bounds<Pixels>,
    pub(super) scroll_offset: Point<Pixels>,
    pub(super) plane_width: f32,
    pub(super) column_width: f32,
    pub(super) column_count: usize,
    pub(super) range: DateRange,
    pub(super) snap_minutes: u16,
}

impl Manipulation {
    #[allow(clippy::cast_possible_truncation)]
    pub(super) fn new(
        payload: &DragPayload,
        event: &EventOccurrence,
        cursor_offset: Point<Pixels>,
    ) -> Self {
        let grab_offset_minutes = match payload.kind {
            ManipulationKind::Move => (cursor_offset.y.as_f32() / PIXELS_PER_MINUTE).round() as i32,
            ManipulationKind::Resize(_) => 0,
        };
        Self {
            event: event.clone(),
            kind: payload.kind,
            proposed: event.draft(),
            original_day: payload.original_day,
            range_start: payload.range_start,
            pointer: Point::default(),
            viewport: Bounds::default(),
            plane_width: 0.0,
            column_width: 0.0,
            column_count: 0,
            scroll_offset: Point::default(),
            grab_offset_minutes,
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(super) fn update(&mut self, update: ManipulationUpdate) {
        self.pointer = update.pointer;
        self.viewport = update.viewport;
        self.scroll_offset = update.scroll_offset;
        self.plane_width = update.plane_width;
        self.column_width = update.column_width;
        self.column_count = update.column_count;
        self.range_start = update.range.start();

        let local_x = update.pointer.x.as_f32()
            - update.viewport.origin.x.as_f32()
            - update.scroll_offset.x.as_f32();
        let local_y = update.pointer.y.as_f32()
            - update.viewport.origin.y.as_f32()
            - update.scroll_offset.y.as_f32();
        let day = if update.column_width > 0.0 {
            let maximum_day =
                i32::try_from(update.column_count.saturating_sub(1)).unwrap_or(i32::MAX);
            (local_x / update.column_width).floor().clamp(
                0.0,
                f32::from(u16::try_from(maximum_day).unwrap_or(u16::MAX)),
            ) as i32
        } else {
            0
        };
        let pointer_minute = (local_y / PIXELS_PER_MINUTE).round() as i32;
        self.proposed = match self.kind {
            ManipulationKind::Move => {
                let original_start = i32::from(crate::domain::minutes_since_midnight(
                    self.event.start_time(),
                ));
                let minute_delta = pointer_minute - self.grab_offset_minutes - original_start;
                propose_move(
                    &self.event.draft(),
                    day - i32::from(self.original_day),
                    minute_delta,
                    update.snap_minutes,
                )
                .unwrap_or_else(|| self.event.draft())
            }
            ManipulationKind::Resize(edge) => propose_resize(
                &self.event.draft(),
                edge,
                pointer_minute,
                update.snap_minutes,
            ),
        };
    }

    pub(super) fn target_day(&self) -> usize {
        let mut date = self.range_start;
        for day in 0..self.column_count {
            if date == self.proposed.date {
                return day;
            }
            date = date.tomorrow().unwrap_or(date);
        }
        self.original_day as usize
    }

    pub(super) fn changed(&self) -> bool {
        self.proposed != self.event.draft()
    }

    pub(super) fn edge_velocity(&self) -> Point<Pixels> {
        let x = edge_velocity(
            self.pointer.x.as_f32(),
            self.viewport.origin.x.as_f32(),
            self.viewport.right().as_f32(),
        );
        let y = edge_velocity(
            self.pointer.y.as_f32(),
            self.viewport.origin.y.as_f32(),
            self.viewport.bottom().as_f32(),
        );
        let x = if self.column_count > 1 { x } else { 0.0 };
        px_point(x, y)
    }

    pub(super) fn scroll_by(&self, delta: Point<Pixels>) -> Point<Pixels> {
        let max_x = (self.viewport.size.width.as_f32() - self.plane_width).min(0.0);
        let max_y = (self.viewport.size.height.as_f32() - super::style::PLANE_HEIGHT).min(0.0);
        px_point(
            (self.scroll_offset.x.as_f32() + delta.x.as_f32()).clamp(max_x, 0.0),
            (self.scroll_offset.y.as_f32() + delta.y.as_f32()).clamp(max_y, 0.0),
        )
    }

    pub(super) const fn occurrence_id(&self) -> OccurrenceId {
        self.event.id()
    }
}

fn edge_velocity(position: f32, start: f32, end: f32) -> f32 {
    if position < start + AUTO_SCROLL_EDGE {
        -((start + AUTO_SCROLL_EDGE - position) / AUTO_SCROLL_EDGE).clamp(0.0, 1.0)
            * AUTO_SCROLL_MAX_STEP
    } else if position > end - AUTO_SCROLL_EDGE {
        ((position - (end - AUTO_SCROLL_EDGE)) / AUTO_SCROLL_EDGE).clamp(0.0, 1.0)
            * AUTO_SCROLL_MAX_STEP
    } else {
        0.0
    }
}

const fn px_point(x: f32, y: f32) -> Point<Pixels> {
    Point::new(px(x), px(y))
}
