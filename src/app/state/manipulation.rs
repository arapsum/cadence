use std::time::Duration;

use gpui::{Context, DragMoveEvent, Pixels, Point, Window};
use gpui_component::WindowExt as _;
use jiff::Timestamp;

use crate::app::{
    history::{CalendarChange, ChangeKind},
    interaction::{DragPayload, Manipulation, ManipulationKind, ManipulationUpdate},
};
use crate::store::TimetableRepository;

use super::{CadenceView, HistoryEffect, RollbackViewState};

impl CadenceView {
    pub(in crate::app) fn begin_manipulation(
        &mut self,
        payload: &DragPayload,
        cursor_offset: Point<Pixels>,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.is_interactive() {
            return;
        }
        let Some(event) = self
            .repository
            .occurrence(payload.occurrence_id)
            .ok()
            .flatten()
        else {
            return;
        };
        self.manipulation_rollback = Some(self.rollback_view_state());
        self.state.select_event(event.id(), event.date());
        self.manipulation = Some(Manipulation::new(payload, &event, cursor_offset));
        let owner = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                let keep_running = owner
                    .update(cx, |view, cx| {
                        let Some(manipulation) = &view.manipulation else {
                            return false;
                        };
                        let delta = manipulation.edge_velocity();
                        if delta.x == gpui::px(0.0) && delta.y == gpui::px(0.0) {
                            return true;
                        }
                        let next_offset = manipulation.scroll_by(delta);
                        let pointer = manipulation.pointer;
                        let viewport = manipulation.viewport;
                        let plane_width = manipulation.plane_width;
                        let column_width = manipulation.column_width;
                        let column_count = manipulation.column_count;
                        let surface = manipulation.surface();
                        view.viewport(surface).handle.set_offset(next_offset);
                        let range = view
                            .surface_snapshot(surface)
                            .map(|snapshot| snapshot.range);
                        if let Some(range) = range
                            && let Some(manipulation) = &mut view.manipulation
                        {
                            manipulation.update(ManipulationUpdate {
                                pointer,
                                viewport,
                                scroll_offset: next_offset,
                                plane_width,
                                column_width,
                                column_count,
                                range,
                                snap_minutes: view.settings.snap_interval().minutes(),
                            });
                        }
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !keep_running {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }

    pub(in crate::app) fn update_manipulation(
        &mut self,
        event: &DragMoveEvent<DragPayload>,
        surface: crate::calendar::CalendarViewMode,
        column_width: f32,
        plane_width: f32,
        column_count: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(active_surface) = self.manipulation.as_ref().map(Manipulation::surface) else {
            return;
        };
        if active_surface != surface {
            return;
        }
        let Some(snapshot) = self.surface_snapshot(surface) else {
            return;
        };
        let range = snapshot.range;
        let scroll_offset = self.viewport(surface).handle.offset();
        let snap_minutes = self.settings.snap_interval().minutes();
        if let Some(manipulation) = &mut self.manipulation {
            manipulation.update(ManipulationUpdate {
                pointer: event.event.position,
                viewport: event.bounds,
                scroll_offset,
                plane_width,
                column_width,
                column_count,
                range,
                snap_minutes,
            });
            cx.notify();
        }
    }

    pub(in crate::app) fn cancel_manipulation(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.manipulation.take().is_some() {
            if let Some(rollback) = self.manipulation_rollback.take() {
                self.restore_view_state(rollback);
            }
            cx.stop_active_drag(window);
            cx.notify();
        }
    }

    pub(in crate::app) fn finish_manipulation(
        &mut self,
        payload: &DragPayload,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(manipulation) = self.manipulation.take() else {
            return;
        };
        let rollback = self
            .manipulation_rollback
            .take()
            .unwrap_or_else(|| self.rollback_view_state());
        cx.stop_active_drag(window);
        if manipulation.event.id() != payload.occurrence_id || !manipulation.changed() {
            self.restore_view_state(rollback);
            cx.notify();
            return;
        }
        let before = match self.repository.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.restore_view_state(rollback);
                self.error = Some(error.to_string());
                cx.notify();
                return;
            }
        };
        if manipulation.event.id().recurring().is_some() {
            Self::open_manipulation_scope_prompt(manipulation, before, rollback, window, cx);
            return;
        }
        let Some(event_id) = manipulation.event.id().standalone() else {
            self.restore_view_state(rollback);
            self.error = Some("Recurring events are edited from the event form.".to_owned());
            cx.notify();
            return;
        };
        let Some(mut event) = self.repository.event(event_id).ok().flatten() else {
            self.restore_view_state(rollback);
            self.error = Some("That event is no longer available.".to_owned());
            self.refresh_snapshot();
            cx.notify();
            return;
        };
        let before_draft = event.draft();
        let after_draft = manipulation.proposed;
        if let Err(error) = event.revise(after_draft.clone(), Timestamp::now()) {
            self.restore_view_state(rollback);
            self.error = Some(error.to_string());
            cx.notify();
            return;
        }
        if let Err(error) = self.repository.update_event(event) {
            self.restore_view_state(rollback);
            self.error = Some(error.to_string());
            cx.notify();
            return;
        }
        self.state.select_event(
            crate::domain::OccurrenceId::Standalone(event_id),
            after_draft.date,
        );
        self.last_category = Some(after_draft.category_id);
        self.pending_scroll_minutes = None;
        self.reset_scroll_initialization();
        self.refresh_snapshot();
        let kind = match manipulation.kind {
            ManipulationKind::Move => ChangeKind::Move,
            ManipulationKind::Resize(_) => ChangeKind::Resize,
        };
        self.persist_snapshot(
            before,
            rollback,
            HistoryEffect::Record(CalendarChange::Update {
                id: event_id,
                before: before_draft,
                after: after_draft,
                kind,
            }),
            cx,
        );
        cx.notify();
    }

    fn open_manipulation_scope_prompt(
        manipulation: Manipulation,
        before: crate::store::PersistenceSnapshot,
        rollback: RollbackViewState,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let owner_this = owner.clone();
            let owner_following = owner.clone();
            let manipulation_this = manipulation.clone();
            let manipulation_following = manipulation.clone();
            let before_this = before.clone();
            let before_following = before.clone();
            alert
                .title("Apply recurring change to…")
                .description("Choose whether this move or resize affects one occurrence or this and all following occurrences.")
                .button_props(
                    gpui_component::dialog::DialogButtonProps::default()
                        .ok_text("This event")
                        .cancel_text("This and following")
                        .show_cancel(true),
                )
                .on_ok(move |_, window, app| {
                    owner_this
                        .update(app, |view, cx| {
                            window.close_all_dialogs(cx);
                            view.apply_manipulation_scope(
                                &manipulation_this,
                                before_this.clone(),
                                rollback,
                                super::super::editor::RecurrenceScope::This,
                                window,
                                cx,
                            );
                        })
                        .ok();
                    true
                })
                .on_cancel(move |_, window, app| {
                    owner_following
                        .update(app, |view, cx| {
                            window.close_all_dialogs(cx);
                            view.apply_manipulation_scope(
                                &manipulation_following,
                                before_following.clone(),
                                rollback,
                                super::super::editor::RecurrenceScope::Following,
                                window,
                                cx,
                            );
                        })
                        .ok();
                    true
                })
        });
    }

    fn apply_manipulation_scope(
        &mut self,
        manipulation: &Manipulation,
        before: crate::store::PersistenceSnapshot,
        rollback: RollbackViewState,
        scope: super::super::editor::RecurrenceScope,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some((series_id, original_date)) = manipulation.event.id().recurring() else {
            self.restore_view_state(rollback);
            return;
        };
        let Some(series) = self.repository.series(series_id).ok().flatten() else {
            self.restore_view_state(rollback);
            self.show_error("That recurring series is no longer available.", window, cx);
            return;
        };
        let proposed = manipulation.proposed.clone();
        let form = crate::editor::FormDraft {
            title: proposed.title.clone(),
            notes: proposed.notes.clone().unwrap_or_default(),
            date: proposed.date,
            start_time: proposed.start_time,
            end_time: proposed.end_time,
            category_id: Some(proposed.category_id),
            recurrence: Some(series.rule()),
            ends_on: series.ends_on(),
            reminder: proposed.reminder,
        };
        let result = self
            .apply_recurring_edit(series_id, original_date, &form, scope, Timestamp::now())
            .map_err(crate::domain::RepositoryError::InvalidEntity);
        let kind = match manipulation.kind {
            ManipulationKind::Move => ChangeKind::Move,
            ManipulationKind::Resize(_) => ChangeKind::Resize,
        };
        let selected_id = match result {
            Ok(id) => id,
            Err(error) => {
                self.restore_view_state(rollback);
                self.show_error(error.to_string(), window, cx);
                return;
            }
        };
        let after = match self.repository.snapshot() {
            Ok(after) => after,
            Err(error) => {
                self.restore_view_state(rollback);
                self.show_error(error.to_string(), window, cx);
                return;
            }
        };
        self.state.select_event(selected_id, proposed.date);
        self.last_category = Some(proposed.category_id);
        self.refresh_snapshot();
        self.persist_snapshot(
            before.clone(),
            rollback,
            HistoryEffect::Record(CalendarChange::Snapshot {
                before,
                after,
                kind,
            }),
            cx,
        );
        cx.notify();
    }
}
