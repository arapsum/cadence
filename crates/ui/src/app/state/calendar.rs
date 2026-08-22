use gpui::Context;
use jiff::{Timestamp, civil::Date};

use crate::{calendar::CalendarViewMode, domain::DateRange, store::TimetableRepository};

use super::super::presentation::{
    SurfaceSnapshot, WorkspaceSnapshot, event_matches_filter, layout_events, local_date_time,
};
use crate::domain::find_occurrence_conflicts;

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
        let day_range = match DateRange::day(self.state.selected_date()) {
            Ok(range) => range,
            Err(error) => {
                self.error = Some(error.to_string());
                self.snapshot = None;
                return;
            }
        };
        let week_range =
            match DateRange::week(self.state.selected_date(), self.settings.week_starts_on()) {
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
        let visible_events = match self.repository.occurrences(week_range) {
            Ok(events) => events
                .into_iter()
                .filter(|event| {
                    categories
                        .iter()
                        .find(|category| category.id() == event.category_id())
                        .is_some_and(crate::domain::Category::is_visible)
                })
                .collect::<Vec<_>>(),
            Err(error) => {
                self.error = Some(error.to_string());
                self.snapshot = None;
                return;
            }
        };
        let summary_events = visible_events
            .iter()
            .filter(|event| event.date() == self.state.selected_date())
            .cloned()
            .collect::<Vec<_>>();
        let conflict_ids = find_occurrence_conflicts(&visible_events)
            .into_iter()
            .flat_map(|conflict| [conflict.first(), conflict.second()])
            .collect();
        let week_events = visible_events
            .into_iter()
            .filter(|event| event_matches_filter(event, self.state.category_filter()))
            .collect::<Vec<_>>();
        let day_events = week_events
            .iter()
            .filter(|event| event.date() == self.state.selected_date())
            .cloned()
            .collect::<Vec<_>>();
        let week_positions = match layout_events(&week_events, week_range) {
            Ok(positions) => positions,
            Err(error) => {
                self.error = Some(format!("Could not lay out calendar: {error:?}"));
                self.snapshot = None;
                return;
            }
        };
        let day_positions = match layout_events(&day_events, day_range) {
            Ok(positions) => positions,
            Err(error) => {
                self.error = Some(format!("Could not lay out calendar: {error:?}"));
                self.snapshot = None;
                return;
            }
        };

        self.snapshot = Some(WorkspaceSnapshot {
            day: SurfaceSnapshot {
                range: day_range,
                events: day_events,
                positions: day_positions,
            },
            week: SurfaceSnapshot {
                range: week_range,
                events: week_events,
                positions: week_positions,
            },
            categories,
            summary_events,
            conflict_ids,
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
        self.reset_scroll_initialization();
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
            self.reset_scroll_initialization();
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
        self.reset_scroll_initialization();
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

    pub(in crate::app) fn activate_surface(
        &mut self,
        view_mode: CalendarViewMode,
        cx: &mut Context<'_, Self>,
    ) {
        if self.state.view_mode() == view_mode {
            return;
        }
        self.state.set_view_mode(view_mode);
        if let Err(error) = self.repository.replace_preferences(self.preferences()) {
            self.error = Some(error.to_string());
        }
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
