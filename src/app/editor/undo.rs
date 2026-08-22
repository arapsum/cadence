use gpui::Context;
use gpui::Window;
use gpui_component::{WindowExt as _, notification::Notification};
use jiff::Timestamp;

use crate::{
    domain::{EventId, OccurrenceId},
    store::{InMemoryRepository, TimetableRepository},
};

use super::super::{history::CalendarChange, state::CadenceView};

impl CadenceView {
    pub(in crate::app) fn undo(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if !self.is_interactive() || window.has_active_dialog(cx) {
            return;
        }
        let Some(change) = self.history.peek_undo().cloned() else {
            return;
        };
        self.apply_history_change(&change, false, window, cx);
    }

    pub(in crate::app) fn redo(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if !self.is_interactive() || window.has_active_dialog(cx) {
            return;
        }
        let Some(change) = self.history.peek_redo().cloned() else {
            return;
        };
        self.apply_history_change(&change, true, window, cx);
    }

    fn apply_history_change(
        &mut self,
        change: &CalendarChange,
        forward: bool,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let snapshot_preferences = match change {
            CalendarChange::Snapshot { before, after, .. } => {
                Some(if forward { after } else { before })
            }
            CalendarChange::Create { .. }
            | CalendarChange::Update { .. }
            | CalendarChange::Delete { .. } => None,
        };
        let rollback = self.rollback_view_state();
        let before = match self.repository.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.show_error(error.to_string(), window, cx);
                return;
            }
        };
        let result = match &change {
            CalendarChange::Create { event } if forward => {
                self.repository.create_event(event.clone())
            }
            CalendarChange::Create { event } => {
                self.repository.delete_event(event.id()).map(|_| ())
            }
            CalendarChange::Update {
                id, before, after, ..
            } => self.revise_event(*id, if forward { after } else { before }),
            CalendarChange::Delete { event } if forward => {
                self.repository.delete_event(event.id()).map(|_| ())
            }
            CalendarChange::Delete { event } => self.repository.create_event(event.clone()),
            CalendarChange::Snapshot { before, after, .. } => {
                let snapshot = if forward { after } else { before };
                InMemoryRepository::from_snapshot(snapshot).map(|repository| {
                    self.repository = repository;
                })
            }
        };
        if let Err(error) = result {
            self.show_error(error.to_string(), window, cx);
            return;
        }

        if let Some(snapshot) = snapshot_preferences {
            self.restore_snapshot_category_state(snapshot, window, cx);
        }

        match &change {
            CalendarChange::Create { event } => {
                if forward {
                    self.state
                        .select_event(OccurrenceId::Standalone(event.id()), event.date());
                    self.last_category = Some(event.category_id());
                } else {
                    self.state.clear_selection();
                }
            }
            CalendarChange::Update {
                id, before, after, ..
            } => {
                let draft = if forward { after } else { before };
                self.state
                    .select_event(OccurrenceId::Standalone(*id), draft.date);
                self.last_category = Some(draft.category_id);
            }
            CalendarChange::Delete { event } => {
                if forward {
                    self.state.clear_selection();
                } else {
                    self.state
                        .select_event(OccurrenceId::Standalone(event.id()), event.date());
                    self.last_category = Some(event.category_id());
                }
            }
            CalendarChange::Snapshot { .. } => {
                self.state.clear_selection();
            }
        }
        self.pending_scroll_minutes = None;
        self.reset_scroll_initialization();
        self.refresh_snapshot();
        let effect = if forward {
            super::super::state::HistoryEffect::Redo(change.clone())
        } else {
            super::super::state::HistoryEffect::Undo(change.clone())
        };
        self.persist_snapshot(before, rollback, effect, cx);
        window.push_notification(
            Notification::success(format!(
                "{} {}",
                if forward { "Redid" } else { "Undid" },
                change.kind().label()
            )),
            cx,
        );
        cx.notify();
    }

    fn revise_event(
        &mut self,
        event_id: EventId,
        draft: &crate::domain::EventDraft,
    ) -> Result<(), crate::domain::RepositoryError> {
        let mut event = self
            .repository
            .event(event_id)?
            .ok_or(crate::domain::RepositoryError::EventNotFound)?;
        event
            .revise(draft.clone(), Timestamp::now())
            .map_err(|error| crate::domain::RepositoryError::InvalidEntity(error.to_string()))?;
        self.repository.update_event(event)
    }

    fn restore_snapshot_category_state(
        &mut self,
        snapshot: &crate::store::PersistenceSnapshot,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.settings = snapshot.settings.clone();
        self.notifications_enabled = snapshot.preferences.notifications_enabled;
        self.reduce_motion = snapshot.preferences.reduce_motion;
        cx.set_reduce_motion(self.reduce_motion);
        let filter = snapshot
            .preferences
            .category_filter
            .filter(|id| {
                snapshot
                    .categories
                    .iter()
                    .any(|category| category.id() == *id && category.is_visible())
            })
            .map_or(
                crate::calendar::CategoryFilter::All,
                crate::calendar::CategoryFilter::Only,
            );
        self.state.set_category_filter(filter);
        self.last_category = self.last_category.filter(|id| {
            snapshot
                .categories
                .iter()
                .any(|category| category.id() == *id)
        });
        self.sync_category_filter(window, cx);
    }
}
