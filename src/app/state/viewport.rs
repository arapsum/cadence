use crate::calendar::CalendarViewMode;

use super::super::{
    presentation::{day_index, local_date_time},
    style::PIXELS_PER_MINUTE,
};

use super::CadenceView;

#[derive(Debug, Clone, Copy)]
pub(in crate::app) struct RollbackViewState {
    pub(in crate::app::state) calendar_state: crate::calendar::CalendarState,
    pub(in crate::app::state) last_category: Option<crate::domain::CategoryId>,
    pub(in crate::app::state) scroll_offset: gpui::Point<gpui::Pixels>,
    pub(in crate::app::state) scroll_initialized: bool,
    pub(in crate::app::state) pending_scroll_minutes: Option<f32>,
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
        let last_day = snapshot
            .range
            .end()
            .yesterday()
            .unwrap_or_else(|_| snapshot.range.start());
        let start = snapshot.range.start().strftime("%b %-d");
        let end = last_day.strftime("%b %-d, %Y");
        format!("{start} – {end}")
    }

    pub(in crate::app) fn initial_scroll_offset(&mut self, column_width: f32) -> (f32, f32) {
        let pending_scroll_minutes = self.pending_scroll_minutes.take();
        let Some(snapshot) = &self.snapshot else {
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
        let horizontal = if self.state.view_mode() == CalendarViewMode::Day {
            0.0
        } else {
            day_index(snapshot.range, self.state.selected_date()).map_or(0.0, |day| {
                let day = f32::from(u16::try_from(day).expect("calendar day fits in u16"));
                ((day - 2.0) * column_width).max(0.0)
            })
        };
        (horizontal, target_minutes * PIXELS_PER_MINUTE)
    }

    pub(in crate::app::state) fn current_scroll_minutes(&self) -> f32 {
        (-self.scroll_handle.offset().y.as_f32() / PIXELS_PER_MINUTE).max(0.0)
    }

    pub(in crate::app) fn rollback_view_state(&self) -> RollbackViewState {
        RollbackViewState {
            calendar_state: self.state,
            last_category: self.last_category,
            scroll_offset: self.scroll_handle.offset(),
            scroll_initialized: self.scroll_initialized,
            pending_scroll_minutes: self.pending_scroll_minutes,
        }
    }

    pub(in crate::app::state) fn restore_view_state(&mut self, view_state: RollbackViewState) {
        self.state = view_state.calendar_state;
        self.last_category = view_state.last_category;
        self.scroll_handle.set_offset(view_state.scroll_offset);
        self.scroll_initialized = view_state.scroll_initialized;
        self.pending_scroll_minutes = view_state.pending_scroll_minutes;
        self.refresh_snapshot();
    }
}
