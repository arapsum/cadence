use gpui::{Context, Window};
use gpui_component::{WindowExt as _, button::Button, notification::Notification};

use crate::{
    domain::{OccurrenceId, RecurrenceException},
    store::TimetableRepository,
};

use super::{
    history::{CalendarChange, ChangeKind},
    state::{CadenceView, HistoryEffect},
};

impl CadenceView {
    /// Opens confirmation for the currently selected visible occurrences.
    pub(in crate::app) fn confirm_delete_selected(
        &self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.is_interactive() || !self.is_bulk_selecting() {
            return;
        }
        let selected = self.selected_occurrences();
        if selected.is_empty() {
            return;
        }
        let standalone_count = selected
            .iter()
            .filter(|id| id.standalone().is_some())
            .count();
        let recurring_count = selected.len() - standalone_count;
        let count = selected.len();
        let description = if recurring_count == 0 {
            format!("Delete {count} events? This can be undone during this session.")
        } else if standalone_count == 0 {
            format!(
                "Delete {count} recurring occurrences? Only the selected dates will be cancelled; the rest of each series will remain. This can be undone during this session.",
            )
        } else {
            format!(
                "Delete {count} events, including {recurring_count} recurring occurrences? Selected recurring dates will be cancelled; the rest of each series will remain. This can be undone during this session.",
            )
        };
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let owner = owner.clone();
            alert
                .title(format!("Delete {count} events?"))
                .description(description.clone())
                .button_props(
                    gpui_component::dialog::DialogButtonProps::default()
                        .ok_text(format!("Delete {count} events"))
                        .ok_variant(gpui_component::button::ButtonVariant::Danger)
                        .cancel_text("Keep events")
                        .show_cancel(true),
                )
                .on_ok({
                    let selected = selected.clone();
                    move |_, window, app| {
                        owner
                            .update(app, |view, cx| {
                                window.close_all_dialogs(cx);
                                view.delete_selected_events(&selected, window, cx);
                            })
                            .ok();
                        true
                    }
                })
        });
    }

    fn delete_selected_events(
        &mut self,
        selected: &[OccurrenceId],
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.is_interactive()
            || !self.is_bulk_selecting()
            || selected.is_empty()
            || selected.iter().any(|id| !self.is_bulk_selected(*id))
        {
            return;
        }
        let before = match self.repository.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.show_error(error.to_string(), window, cx);
                return;
            }
        };
        let rollback = self.rollback_view_state();
        let repository_rollback = self.repository.clone();
        let result = (|| -> Result<(), String> {
            for id in selected {
                if self
                    .repository
                    .occurrence(*id)
                    .map_err(|error| error.to_string())?
                    .is_none()
                {
                    return Err("One or more selected events are no longer available.".to_owned());
                }
            }
            for id in selected {
                if let Some(event_id) = id.standalone() {
                    self.repository
                        .delete_event(event_id)
                        .map_err(|error| error.to_string())?;
                } else if let Some((series_id, original_date)) = id.recurring() {
                    self.repository
                        .upsert_exception(RecurrenceException::cancelled(series_id, original_date))
                        .map_err(|error| error.to_string())?;
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.repository = repository_rollback;
            self.show_error(error, window, cx);
            return;
        }
        let after = match self.repository.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.repository = repository_rollback;
                self.show_error(error.to_string(), window, cx);
                return;
            }
        };
        let count = selected.len();
        self.event_selection = super::state::EventSelection::Single;
        self.state.clear_selection();
        self.refresh_snapshot();
        self.persist_snapshot(
            before.clone(),
            rollback,
            HistoryEffect::Record(CalendarChange::Snapshot {
                before: Box::new(before),
                after: Box::new(after),
                kind: ChangeKind::DeleteMany,
            }),
            cx,
        );
        let owner = cx.entity().downgrade();
        window.push_notification(
            Notification::new()
                .message(format!("{count} events deleted"))
                .action(move |_, _, _| {
                    let owner = owner.clone();
                    Button::new("undo-delete-many")
                        .label("Undo")
                        .on_click(move |_, window, app| {
                            owner.update(app, |view, cx| view.undo(window, cx)).ok();
                        })
                }),
            cx,
        );
        cx.notify();
    }
}
