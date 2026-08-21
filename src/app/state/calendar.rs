use gpui::Context;
use jiff::{Timestamp, civil::Date};

use crate::{calendar::CalendarViewMode, store::TimetableRepository};

use super::super::presentation::{
    CalendarSnapshot, event_matches_filter, layout_events, local_date_time,
};

use super::{CadenceView, HistoryEffect, PersistenceState};

impl CadenceView {
    pub(in crate::app) fn refresh_snapshot(&mut self) {
        if !matches!(
            self.persistence_state,
            PersistenceState::Ready | PersistenceState::Writing
        ) {
            self.snapshot = None;
            return;
        }
        let range = match self.state.visible_range() {
            Ok(range) => range,
            Err(error) => {
                self.error = Some(error.to_string());
                self.snapshot = None;
                return;
            }
        };

        let categories = match self.repository.categories() {
            Ok(categories) => categories,
            Err(error) => {
                self.error = Some(error.to_string());
                self.snapshot = None;
                return;
            }
        };
        let events = match self.repository.occurrences(range) {
            Ok(events) => events
                .into_iter()
                .filter(|event| event_matches_filter(event, self.state.category_filter()))
                .collect::<Vec<_>>(),
            Err(error) => {
                self.error = Some(error.to_string());
                self.snapshot = None;
                return;
            }
        };
        let positions = match layout_events(&events, range) {
            Ok(positions) => positions,
            Err(error) => {
                self.error = Some(format!("Could not lay out calendar: {error:?}"));
                self.snapshot = None;
                return;
            }
        };

        self.snapshot = Some(CalendarSnapshot {
            range,
            events,
            positions,
            categories,
        });
        self.error = None;
    }

    pub(in crate::app) fn go_to_today(&mut self, cx: &mut Context<'_, Self>) {
        if !self.is_interactive() {
            return;
        }
        self.now = Timestamp::now();
        let (today, _) = local_date_time(self.now, &self.settings);
        self.state.go_to_today(today);
        self.pending_scroll_minutes = None;
        self.scroll_initialized = false;
        self.refresh_snapshot();
        cx.notify();
    }

    pub(in crate::app) fn shift_period(&mut self, next: bool, cx: &mut Context<'_, Self>) {
        if !self.is_interactive() {
            return;
        }
        let result = if next {
            self.state.next_period()
        } else {
            self.state.previous_period()
        };
        if let Err(error) = result {
            self.error = Some(error.to_string());
        } else {
            self.pending_scroll_minutes = None;
            self.scroll_initialized = false;
            self.refresh_snapshot();
        }
        cx.notify();
    }

    pub(in crate::app) fn clear_selection(&mut self, cx: &mut Context<'_, Self>) {
        self.state.clear_selection();
        cx.notify();
    }

    pub(in crate::app) fn select_date(&mut self, date: Date, cx: &mut Context<'_, Self>) {
        if !self.is_interactive() {
            return;
        }
        self.state.select_date(date);
        self.pending_scroll_minutes = Some(self.current_scroll_minutes());
        self.scroll_initialized = false;
        self.refresh_snapshot();
        cx.notify();
    }

    pub(in crate::app) fn select_event(
        &mut self,
        event_id: crate::domain::OccurrenceId,
        date: Date,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.is_interactive() {
            return;
        }
        self.state.select_event(event_id, date);
        cx.notify();
    }

    pub(in crate::app) fn set_view_mode(
        &mut self,
        view_mode: CalendarViewMode,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.is_interactive() {
            return;
        }
        if self.state.view_mode() == view_mode {
            return;
        }
        let rollback = self.rollback_view_state();
        let before = self.repository.snapshot().ok();
        self.pending_scroll_minutes = Some(self.current_scroll_minutes());
        self.state.set_view_mode(view_mode);
        self.scroll_initialized = false;
        self.refresh_snapshot();
        let _ = self.repository.replace_preferences(self.preferences());
        if let Some(before) = before {
            self.persist_snapshot(before, rollback, HistoryEffect::None, cx);
        }
        cx.notify();
    }

    pub(in crate::app) const fn is_interactive(&self) -> bool {
        matches!(self.persistence_state, PersistenceState::Ready) && self.manipulation.is_none()
    }
}
