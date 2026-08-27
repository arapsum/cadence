use gpui::{Context, ScrollHandle, Window, point, px};
use jiff::{SignedDuration, civil::Date};

use crate::calendar::CalendarViewMode;

use super::super::{
    presentation::{SurfaceSnapshot, day_index, local_date_time},
    style::{MIN_COLUMN_WIDTH, PIXELS_PER_MINUTE, TIME_GUTTER_WIDTH},
};

use super::{CadenceView, EventSelection};

/// Number of dates that the week surface keeps visible as its logical window.
pub(in crate::app) const WEEK_VISIBLE_DAYS: usize = 7;
/// Number of dates kept on either side of the logical window for seamless scrolling.
pub(in crate::app) const WEEK_BUFFER_DAYS: usize = 7;
/// Rebase before the logical viewport reaches the edge of its buffer.
pub(in crate::app) const WEEK_REBASE_GUARD_DAYS: usize = 3;

pub(in crate::app) fn shift_date(date: Date, days: i32) -> Option<Date> {
    date.checked_add(SignedDuration::from_hours(i64::from(days) * 24))
        .ok()
}

pub(in crate::app) fn rolling_week_range(
    buffer_start: Date,
) -> Result<crate::domain::DateRange, crate::domain::CalendarError> {
    let end = shift_date(
        buffer_start,
        i32::try_from(WEEK_VISIBLE_DAYS + WEEK_BUFFER_DAYS * 2)
            .expect("rolling week buffer fits in i32"),
    )
    .ok_or(crate::domain::CalendarError::DateArithmetic)?;
    crate::domain::DateRange::new(buffer_start, end)
}

pub(in crate::app) fn logical_week_start(
    buffer_start: Date,
    scroll_left: f32,
    column_width: f32,
) -> Option<Date> {
    if !column_width.is_finite() || column_width <= 0.0 || !scroll_left.is_finite() {
        return None;
    }
    let index = (scroll_left.max(0.0) / column_width).floor();
    let index = floor_to_i32(index)?;
    shift_date(buffer_start, index)
}

pub(in crate::app) fn week_rebase_delta(
    buffer_start: Date,
    scroll_left: f32,
    column_width: f32,
    column_count: usize,
    can_rebase: bool,
) -> Option<i32> {
    if !can_rebase
        || column_count <= WEEK_VISIBLE_DAYS
        || !column_width.is_finite()
        || column_width <= 0.0
    {
        return None;
    }
    let first_visible = floor_to_usize(scroll_left.max(0.0) / column_width)?;
    let trailing_edge = first_visible
        .saturating_add(WEEK_VISIBLE_DAYS)
        .saturating_add(WEEK_REBASE_GUARD_DAYS);
    if first_visible < WEEK_REBASE_GUARD_DAYS {
        let days = week_visible_days_i32();
        shift_date(buffer_start, -days).map(|_| -days)
    } else if trailing_edge > column_count {
        let days = week_visible_days_i32();
        shift_date(buffer_start, days).map(|_| days)
    } else {
        None
    }
}

/// State of the one-time scroll position applied after surface layout.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(in crate::app) enum ScrollInitialization {
    /// The surface needs its initial position.
    Pending,
    /// A deferred layout callback will apply the initial position.
    Scheduled,
    /// The initial position has been applied.
    Initialized,
}

pub(in crate::app) struct SurfaceViewportState {
    pub(in crate::app) handle: ScrollHandle,
    pub(in crate::app) initialization: ScrollInitialization,
}

impl SurfaceViewportState {
    pub(in crate::app) fn new() -> Self {
        Self {
            handle: ScrollHandle::new(),
            initialization: ScrollInitialization::Pending,
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::app) struct RollbackViewState {
    pub(in crate::app::state) calendar_state: crate::calendar::CalendarState,
    pub(in crate::app::state) last_category: Option<crate::domain::CategoryId>,
    pub(in crate::app::state) day_scroll_offset: gpui::Point<gpui::Pixels>,
    pub(in crate::app::state) week_scroll_offset: gpui::Point<gpui::Pixels>,
    pub(in crate::app::state) day_scroll_initialization: ScrollInitialization,
    pub(in crate::app::state) week_scroll_initialization: ScrollInitialization,
    pub(in crate::app::state) week_visible_start: Date,
    pub(in crate::app::state) week_buffer_start: Date,
    pub(in crate::app::state) pending_scroll_minutes: Option<f32>,
    pub(in crate::app::state) event_selection: EventSelection,
}

impl CadenceView {
    /// Resets the rolling week buffer around a logical seven-day viewport start.
    pub(in crate::app) fn set_week_window_start(&mut self, visible_start: Date) {
        self.week_visible_start = visible_start;
        self.week_buffer_start = shift_date(
            visible_start,
            -(i32::try_from(WEEK_BUFFER_DAYS).expect("week buffer fits in i32")),
        )
        .unwrap_or(visible_start);
        self.week_scroll_sync_scheduled = false;
    }

    /// Returns the seven-day range represented by the logical week viewport.
    pub(in crate::app) fn visible_week_range(&self) -> Option<crate::domain::DateRange> {
        let end = shift_date(
            self.week_visible_start,
            i32::try_from(WEEK_VISIBLE_DAYS).expect("week viewport fits in i32"),
        )?;
        crate::domain::DateRange::new(self.week_visible_start, end).ok()
    }

    pub(in crate::app) fn week_query_range(
        &self,
    ) -> Result<crate::domain::DateRange, crate::domain::CalendarError> {
        rolling_week_range(self.week_buffer_start)
    }

    /// Schedules a post-layout reconciliation of the logical week date window.
    pub(in crate::app) fn schedule_week_scroll_sync(
        &mut self,
        window: &Window,
        column_width: f32,
        cx: &mut Context<'_, Self>,
    ) {
        if self.week_scroll_sync_scheduled || !column_width.is_finite() || column_width <= 0.0 {
            return;
        }
        self.week_scroll_sync_scheduled = true;
        let owner = cx.entity().downgrade();
        window.defer(cx, move |_, cx| {
            owner
                .update(cx, |view, cx| {
                    view.week_scroll_sync_scheduled = false;
                    view.sync_week_scroll(column_width, cx);
                })
                .ok();
        });
    }

    fn sync_week_scroll(&mut self, column_width: f32, cx: &mut Context<'_, Self>) {
        let scroll_left = (-self.week_viewport.handle.offset().x.as_f32()).max(0.0);
        let column_count = self
            .snapshot
            .as_ref()
            .map_or(WEEK_VISIBLE_DAYS + WEEK_BUFFER_DAYS * 2, |snapshot| {
                crate::app::presentation::dates_in_range(snapshot.week.range).len()
            });
        let rebase = week_rebase_delta(
            self.week_buffer_start,
            scroll_left,
            column_width,
            column_count,
            self.manipulation.is_none(),
        );
        let logical_start = logical_week_start(self.week_buffer_start, scroll_left, column_width);
        if let Some(delta) = rebase
            && self.rebase_week_buffer(delta, column_width)
        {
            cx.notify();
            return;
        }
        if let Some(logical_start) = logical_start
            && logical_start != self.week_visible_start
        {
            self.week_visible_start = logical_start;
            cx.notify();
        }
    }

    fn rebase_week_buffer(&mut self, delta: i32, column_width: f32) -> bool {
        let scroll_left = (-self.week_viewport.handle.offset().x.as_f32()).max(0.0);
        let Some(logical_start) =
            logical_week_start(self.week_buffer_start, scroll_left, column_width)
        else {
            return false;
        };
        let Some(new_buffer_start) = shift_date(self.week_buffer_start, delta) else {
            return false;
        };
        let old_offset = self.week_viewport.handle.offset();
        self.week_buffer_start = new_buffer_start;
        self.week_visible_start = logical_start;
        self.refresh_snapshot();
        self.week_viewport.handle.set_offset(point(
            old_offset.x
                + px(column_width * f32::from(i16::try_from(delta).expect("rebase fits in i16"))),
            old_offset.y,
        ));
        true
    }

    pub(in crate::app) fn range_label(&self) -> String {
        if self.snapshot.is_none() {
            return "No calendar loaded".to_owned();
        }
        if self.state.view_mode() == CalendarViewMode::Day {
            return self
                .state
                .selected_date()
                .strftime("%A, %b %-d, %Y")
                .to_string();
        }
        let Some(week) = self.visible_week_range() else {
            return "No calendar loaded".to_owned();
        };
        let last_day = week.end().yesterday().unwrap_or_else(|_| week.start());
        let start = week.start().strftime("%b %-d");
        let end = last_day.strftime("%b %-d, %Y");
        format!("{start} – {end}")
    }

    pub(in crate::app) fn surface_snapshot(
        &self,
        mode: CalendarViewMode,
    ) -> Option<&SurfaceSnapshot> {
        self.snapshot
            .as_ref()
            .map(|snapshot| snapshot.surface(mode))
    }

    pub(in crate::app) const fn viewport(&self, mode: CalendarViewMode) -> &SurfaceViewportState {
        match mode {
            CalendarViewMode::Day => &self.day_viewport,
            CalendarViewMode::Week => &self.week_viewport,
        }
    }

    pub(in crate::app) const fn viewport_mut(
        &mut self,
        mode: CalendarViewMode,
    ) -> &mut SurfaceViewportState {
        match mode {
            CalendarViewMode::Day => &mut self.day_viewport,
            CalendarViewMode::Week => &mut self.week_viewport,
        }
    }

    pub(in crate::app) const fn surface_width(&self, mode: CalendarViewMode) -> f32 {
        match mode {
            CalendarViewMode::Day => self.day_surface_width,
            CalendarViewMode::Week => self.week_surface_width,
        }
    }

    pub(in crate::app) fn set_surface_width(
        &mut self,
        mode: CalendarViewMode,
        width: f32,
        cx: &mut Context<'_, Self>,
    ) {
        let measured = width.max(1.0);
        let current = match mode {
            CalendarViewMode::Day => &mut self.day_surface_width,
            CalendarViewMode::Week => &mut self.week_surface_width,
        };
        if (*current - measured).abs() <= 1.0 {
            return;
        }
        let previous = *current;
        *current = measured;
        if mode == CalendarViewMode::Week {
            self.rescale_week_scroll(previous, measured);
        }
        cx.notify();
    }

    fn rescale_week_scroll(&self, previous_width: f32, measured_width: f32) {
        let previous_column_width = week_column_width(previous_width);
        let measured_column_width = week_column_width(measured_width);
        if !previous_column_width.is_finite()
            || previous_column_width <= 0.0
            || !measured_column_width.is_finite()
            || measured_column_width <= 0.0
        {
            return;
        }
        let old_scroll_left = (-self.week_viewport.handle.offset().x.as_f32()).max(0.0);
        let scroll_ratio = old_scroll_left / previous_column_width;
        if !scroll_ratio.is_finite() {
            return;
        }
        let new_scroll_left = scroll_ratio * measured_column_width;
        let offset = self.week_viewport.handle.offset();
        self.week_viewport
            .handle
            .set_offset(point(px(-new_scroll_left), offset.y));
    }

    /// Calculates the initial scroll offset for a calendar surface.
    ///
    /// # Parameters
    ///
    /// - `mode`: Calendar surface receiving the initial offset.
    /// - `column_width`: Width of one visible day column in pixels.
    ///
    /// # Returns
    ///
    /// The horizontal and vertical scroll offsets in pixels.
    ///
    /// # Panics
    ///
    /// Panics when:
    ///
    /// - The visible calendar contains more days than fit in a `u16`.
    pub(in crate::app) fn initial_scroll_offset(
        &self,
        mode: CalendarViewMode,
        column_width: f32,
    ) -> (f32, f32) {
        let pending_scroll_minutes = self.pending_scroll_minutes;
        let Some(snapshot) = self.surface_snapshot(mode) else {
            return (0.0, 0.0);
        };
        let target_minutes = pending_scroll_minutes.unwrap_or_else(|| {
            let (today, current_time) = local_date_time(self.now, &self.settings);
            if snapshot.range.contains(today) {
                f32::from(current_time.hour())
                    .mul_add(60.0, f32::from(current_time.minute()) - 90.0)
                    .max(0.0)
            } else {
                snapshot
                    .events
                    .iter()
                    .map(|event| {
                        f32::from(event.start_time().hour())
                            .mul_add(60.0, f32::from(event.start_time().minute()))
                    })
                    .min_by(f32::total_cmp)
                    .map_or(5.0 * 60.0, |minutes| (minutes - 60.0).max(0.0))
            }
        });
        let horizontal = if mode == CalendarViewMode::Day {
            0.0
        } else {
            day_index(snapshot.range, self.week_visible_start).map_or(0.0, |day| {
                let day = f32::from(u16::try_from(day).expect("calendar day fits in u16"));
                day * column_width
            })
        };
        (horizontal, target_minutes * PIXELS_PER_MINUTE)
    }

    /// Applies a measured initial scroll offset after a surface is laid out.
    ///
    /// # Parameters
    ///
    /// - `mode`: Calendar surface receiving the offset.
    /// - `offset`: Horizontal and vertical scroll offsets in pixels.
    pub(in crate::app) fn initialize_scroll(&mut self, mode: CalendarViewMode, offset: (f32, f32)) {
        let viewport = self.viewport_mut(mode);
        viewport
            .handle
            .set_offset(gpui::point(gpui::px(-offset.0), gpui::px(-offset.1)));
        viewport.initialization = ScrollInitialization::Initialized;
        if self.day_viewport.initialization == ScrollInitialization::Initialized
            && self.week_viewport.initialization == ScrollInitialization::Initialized
        {
            self.pending_scroll_minutes = None;
        }
    }

    pub(in crate::app::state) fn current_scroll_minutes(&self) -> f32 {
        (-self
            .viewport(self.state.view_mode())
            .handle
            .offset()
            .y
            .as_f32()
            / PIXELS_PER_MINUTE)
            .max(0.0)
    }

    pub(in crate::app) const fn reset_scroll_initialization(&mut self) {
        self.day_viewport.initialization = ScrollInitialization::Pending;
        self.week_viewport.initialization = ScrollInitialization::Pending;
    }

    pub(in crate::app) fn rollback_view_state(&self) -> RollbackViewState {
        RollbackViewState {
            calendar_state: self.state,
            last_category: self.last_category,
            day_scroll_offset: self.day_viewport.handle.offset(),
            week_scroll_offset: self.week_viewport.handle.offset(),
            day_scroll_initialization: self.day_viewport.initialization,
            week_scroll_initialization: self.week_viewport.initialization,
            week_visible_start: self.week_visible_start,
            week_buffer_start: self.week_buffer_start,
            pending_scroll_minutes: self.pending_scroll_minutes,
            event_selection: self.event_selection.clone(),
        }
    }

    pub(in crate::app::state) fn restore_view_state(&mut self, view_state: RollbackViewState) {
        self.state = view_state.calendar_state;
        self.last_category = view_state.last_category;
        self.day_viewport
            .handle
            .set_offset(view_state.day_scroll_offset);
        self.week_viewport
            .handle
            .set_offset(view_state.week_scroll_offset);
        self.day_viewport.initialization = view_state.day_scroll_initialization;
        self.week_viewport.initialization = view_state.week_scroll_initialization;
        self.week_visible_start = view_state.week_visible_start;
        self.week_buffer_start = view_state.week_buffer_start;
        self.week_scroll_sync_scheduled = false;
        self.pending_scroll_minutes = view_state.pending_scroll_minutes;
        self.event_selection = view_state.event_selection;
        self.refresh_snapshot();
    }
}

fn week_column_width(surface_width: f32) -> f32 {
    let available_width = (surface_width - TIME_GUTTER_WIDTH).max(24.0);
    let visible_width = available_width.max(
        MIN_COLUMN_WIDTH
            * f32::from(u16::try_from(WEEK_VISIBLE_DAYS).expect("visible week columns fit")),
    );
    visible_width / f32::from(u16::try_from(WEEK_VISIBLE_DAYS).expect("visible week columns fit"))
}

fn week_visible_days_i32() -> i32 {
    i32::try_from(WEEK_VISIBLE_DAYS).expect("visible week days fit in i32")
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
fn floor_to_i32(value: f32) -> Option<i32> {
    if !value.is_finite() || !(0.0..=2_147_483_647.0).contains(&value) {
        None
    } else {
        Some(value.floor() as i32)
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
fn floor_to_usize(value: f32) -> Option<usize> {
    if !value.is_finite() || value < 0.0 {
        None
    } else {
        Some(value.floor() as usize)
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::Date;

    use super::{
        WEEK_BUFFER_DAYS, WEEK_REBASE_GUARD_DAYS, WEEK_VISIBLE_DAYS, logical_week_start,
        rolling_week_range, shift_date, week_rebase_delta,
    };
    use crate::app::presentation::dates_in_range;

    #[test]
    fn rolling_range_keeps_a_week_of_buffer_on_each_side() {
        let visible_start = Date::constant(2026, 8, 23);
        let buffer_start = shift_date(
            visible_start,
            -(i32::try_from(WEEK_BUFFER_DAYS).expect("buffer fits in i32")),
        )
        .expect("date arithmetic succeeds");
        let range = rolling_week_range(buffer_start).expect("rolling range is valid");
        let dates = dates_in_range(range);

        assert_eq!(dates.len(), WEEK_VISIBLE_DAYS + WEEK_BUFFER_DAYS * 2);
        assert_eq!(dates[WEEK_BUFFER_DAYS], visible_start);
        assert_eq!(dates.last(), Some(&Date::constant(2026, 9, 5)));
    }

    #[test]
    fn logical_start_tracks_the_column_under_the_scroll_position() {
        let buffer_start = Date::constant(2026, 8, 16);

        assert_eq!(
            logical_week_start(buffer_start, 0.0, 120.0),
            Some(buffer_start)
        );
        assert_eq!(
            logical_week_start(buffer_start, 120.0 * 7.9, 120.0),
            Some(Date::constant(2026, 8, 23))
        );
        assert_eq!(logical_week_start(buffer_start, f32::NAN, 120.0), None);
    }

    #[test]
    fn rebasing_preserves_a_full_seven_day_viewport() {
        let buffer_start = Date::constant(2026, 8, 16);
        let column_count = WEEK_VISIBLE_DAYS + WEEK_BUFFER_DAYS * 2;

        assert_eq!(
            week_rebase_delta(
                buffer_start,
                120.0 * f32::from(u16::try_from(WEEK_REBASE_GUARD_DAYS - 1).expect("fits")),
                120.0,
                column_count,
                true,
            ),
            Some(-(i32::try_from(WEEK_VISIBLE_DAYS).expect("fits"))),
        );
        assert_eq!(
            week_rebase_delta(buffer_start, 120.0 * 12.0, 120.0, column_count, true,),
            Some(i32::try_from(WEEK_VISIBLE_DAYS).expect("fits")),
        );
        assert_eq!(
            week_rebase_delta(buffer_start, 0.0, 120.0, column_count, false),
            None
        );
    }
}
