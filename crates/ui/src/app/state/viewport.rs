use gpui::{Context, ScrollHandle};

use crate::calendar::CalendarViewMode;

use super::super::{
    presentation::{SurfaceSnapshot, day_index, local_date_time},
    style::PIXELS_PER_MINUTE,
};

use super::{CadenceView, EventSelection};

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
    pub(in crate::app::state) pending_scroll_minutes: Option<f32>,
    pub(in crate::app::state) event_selection: EventSelection,
}

impl CadenceView {
    pub(in crate::app) fn range_label(&self) -> String {
        let Some(snapshot) = &self.snapshot else {
            return "No calendar loaded".to_owned();
        };
        if self.state.view_mode() == CalendarViewMode::Day {
            return self
                .state
                .selected_date()
                .strftime("%A, %b %-d, %Y")
                .to_string();
        }
        let week = &snapshot.week;
        let last_day = week
            .range
            .end()
            .yesterday()
            .unwrap_or_else(|_| week.range.start());
        let start = week.range.start().strftime("%b %-d");
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
        *current = measured;
        cx.notify();
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
            day_index(snapshot.range, self.state.selected_date()).map_or(0.0, |day| {
                let day = f32::from(u16::try_from(day).expect("calendar day fits in u16"));
                ((day - 2.0) * column_width).max(0.0)
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
        self.pending_scroll_minutes = view_state.pending_scroll_minutes;
        self.event_selection = view_state.event_selection;
        self.refresh_snapshot();
    }
}
