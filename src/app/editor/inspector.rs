use gpui::{App, Context, IntoElement, Window, div, prelude::*, px};
use gpui_component::{
    ActiveTheme as _, StyledExt as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    dialog::{DialogAction, DialogButtonProps, DialogClose, DialogFooter},
    notification::Notification,
    window_paddings,
};
use jiff::civil::Date;

use crate::{
    domain::{Category, EventOccurrence, OccurrenceId, format_time},
    editor::{EditorMode, FormDraft},
    store::TimetableRepository,
};

use super::super::{history::CalendarChange, state::CadenceView, style::dialog_margin_top};
use super::recurrence::RecurrenceScope;

fn inspector_details(
    category_label: String,
    date_label: String,
    time_label: String,
    notes: Option<String>,
    cx: &App,
) -> impl IntoElement {
    div()
        .debug_selector(|| "event-inspector-details".into())
        .v_flex()
        .gap_3()
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(category_label),
        )
        .child(div().text_base().child(date_label))
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(time_label),
        )
        .when_some(notes, |this, notes| {
            this.child(
                div()
                    .mt_2()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(notes),
            )
        })
}

struct InspectorDialogData {
    owner: gpui::WeakEntity<CadenceView>,
    event_id: OccurrenceId,
    title: String,
    category_label: String,
    date_label: String,
    time_label: String,
    notes: Option<String>,
    duplicate: FormDraft,
}

fn open_inspector_dialog(
    data: InspectorDialogData,
    window: &mut Window,
    cx: &mut Context<'_, CadenceView>,
) {
    let InspectorDialogData {
        owner,
        event_id,
        title,
        category_label,
        date_label,
        time_label,
        notes,
        duplicate,
    } = data;
    window.open_dialog(cx, move |dialog, dialog_window, _| {
        let viewport = dialog_window.viewport_size();
        let padding = window_paddings(dialog_window);
        let available_height = viewport.height - padding.top - padding.bottom;
        let dialog_height = px(420.0);
        let margin_top = dialog_margin_top(available_height, dialog_height);

        let edit_owner = owner.clone();
        let duplicate_owner = owner.clone();
        let delete_owner = owner.clone();

        dialog
            .margin_top(margin_top)
            .w(px(420.0))
            .title(title.clone())
            .overlay_closable(false)
            .content({
                let category_label = category_label.clone();
                let date_label = date_label.clone();
                let time_label = time_label.clone();
                let notes = notes.clone();
                move |content, _, cx| {
                    content.child(inspector_details(
                        category_label.clone(),
                        date_label.clone(),
                        time_label.clone(),
                        notes.clone(),
                        cx,
                    ))
                }
            })
            .footer(
                DialogFooter::new()
                    .w_full()
                    .px_4()
                    .child(
                        Button::new("delete-event")
                            .ghost()
                            .danger()
                            .label("Delete")
                            .on_click(move |_, window, app| {
                                delete_owner
                                    .update(app, |view, cx| {
                                        view.confirm_delete(event_id, window, cx);
                                    })
                                    .ok();
                            }),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("duplicate-event")
                            .outline()
                            .label("Duplicate")
                            .on_click({
                                let duplicate = duplicate.clone();
                                move |_, window, app| {
                                    duplicate_owner
                                        .update(app, |view, cx| {
                                            window.close_dialog(cx);
                                            view.open_editor(
                                                EditorMode::Create,
                                                &duplicate,
                                                window,
                                                cx,
                                            );
                                        })
                                        .ok();
                                }
                            }),
                    )
                    .child(Button::new("edit-event").primary().label("Edit").on_click({
                        let duplicate = duplicate.clone();
                        move |_, window, app| {
                            edit_owner
                                .update(app, |view, cx| {
                                    window.close_dialog(cx);
                                    view.open_editor(
                                        EditorMode::Edit(event_id),
                                        &duplicate,
                                        window,
                                        cx,
                                    );
                                })
                                .ok();
                        }
                    })),
            )
    });
}

impl CadenceView {
    pub(in crate::app) fn inspect_event(
        &mut self,
        event_id: OccurrenceId,
        date: Date,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.is_interactive() {
            return;
        }
        self.state.select_event(event_id, date);
        let event = match self.repository.occurrence(event_id) {
            Ok(Some(event)) => event,
            Ok(None) => {
                self.show_error("That event is no longer available.", window, cx);
                return;
            }
            Err(error) => {
                self.show_error(error.to_string(), window, cx);
                return;
            }
        };
        let category = self.repository.category(event.category_id()).ok().flatten();
        self.open_inspector(&event, category.as_ref(), window, cx);
    }

    pub(in crate::app::editor) fn open_inspector(
        &self,
        event: &EventOccurrence,
        category: Option<&Category>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.is_interactive() {
            return;
        }
        let event_id = event.id();
        let category_label = category.map_or_else(
            || "Uncategorized".to_owned(),
            |category| category.name().to_owned(),
        );
        let title = event.title().to_owned();
        let date_label = event.date().strftime("%A, %B %-d, %Y").to_string();
        let time_label = format!(
            "{} – {}",
            format_time(event.start_time(), self.settings.clock_format()),
            format_time(event.end_time(), self.settings.clock_format())
        );
        let notes = event.notes().map(str::to_owned);
        let mut duplicate = FormDraft::from_occurrence(event);
        if let Some((series_id, _)) = event_id.recurring()
            && let Ok(Some(series)) = self.repository.series(series_id)
        {
            duplicate.recurrence = Some(series.rule());
            duplicate.ends_on = series.ends_on();
        }
        open_inspector_dialog(
            InspectorDialogData {
                owner: cx.entity().downgrade(),
                event_id,
                title,
                category_label,
                date_label,
                time_label,
                notes,
                duplicate,
            },
            window,
            cx,
        );
    }

    pub(in crate::app::editor) fn confirm_delete(
        &mut self,
        event_id: OccurrenceId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(event) = self.repository.occurrence(event_id).ok().flatten() else {
            self.show_error("That event is no longer available.", window, cx);
            return;
        };
        if let Some((series_id, original_date)) = event_id.recurring() {
            let owner = cx.entity().downgrade();
            window.open_alert_dialog(cx, move |alert, _, _| {
                let owner_this = owner.clone();
                let owner_following = owner.clone();
                alert
                    .title("Delete recurring event…")
                    .description("Choose whether to delete one occurrence or this and all following occurrences.")
                    .footer(
                        DialogFooter::new()
                            .child(
                                DialogClose::new()
                                    .child(Button::new("keep-recurring-event").outline().label("Cancel")),
                            )
                            .child(
                                Button::new("delete-following-events")
                                    .outline()
                                    .danger()
                                    .label("This and following")
                                    .on_click(move |_, window, app| {
                                        owner_following
                                            .update(app, |view, cx| {
                                                window.close_all_dialogs(cx);
                                                view.delete_recurring(
                                                    series_id,
                                                    original_date,
                                                    RecurrenceScope::Following,
                                                    window,
                                                    cx,
                                                );
                                            })
                                            .ok();
                                    }),
                            )
                            .child(
                                DialogAction::new().child(
                                    Button::new("delete-this-event")
                                        .primary()
                                        .danger()
                                        .label("This event"),
                                ),
                            ),
                    )
                    .on_ok(move |_, window, app| {
                        owner_this
                            .update(app, |view, cx| {
                                window.close_all_dialogs(cx);
                                view.delete_recurring(
                                    series_id,
                                    original_date,
                                    RecurrenceScope::This,
                                    window,
                                    cx,
                                );
                            })
                            .ok();
                        true
                    })
            });
            return;
        }
        let owner = cx.entity().downgrade();
        let message = format!(
            "Delete ‘{}’ on {}? This can be undone during this session.",
            event.title(),
            event.date().strftime("%B %-d, %Y"),
        );
        window.open_alert_dialog(cx, move |alert, _, _| {
            let owner = owner.clone();
            alert
                .title("Delete event?")
                .description(message.clone())
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("Delete")
                        .ok_variant(gpui_component::button::ButtonVariant::Danger)
                        .cancel_text("Keep event")
                        .show_cancel(true),
                )
                .on_ok(move |_, window, app| {
                    owner
                        .update(app, |view, cx| {
                            view.delete_event(event_id, window, cx);
                            window.close_all_dialogs(cx);
                        })
                        .ok();
                    true
                })
        });
    }

    pub(in crate::app::editor) fn delete_event(
        &mut self,
        event_id: OccurrenceId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let rollback = self.rollback_view_state();
        let before = match self.repository.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.show_error(error.to_string(), window, cx);
                return;
            }
        };
        let Some(standalone_id) = event_id.standalone() else {
            self.show_error(
                "Recurring events are deleted from the event form scope dialog.",
                window,
                cx,
            );
            return;
        };
        match self.repository.delete_event(standalone_id) {
            Ok(event) => {
                self.state.clear_selection();
                self.refresh_snapshot();
                self.persist_snapshot(
                    before,
                    rollback,
                    super::super::state::HistoryEffect::Record(CalendarChange::Delete { event }),
                    cx,
                );
                let owner = cx.entity().downgrade();
                window.push_notification(
                    Notification::new()
                        .message("Event deleted")
                        .action(move |_, _, _| {
                            let owner = owner.clone();
                            Button::new("undo-delete").label("Undo").on_click(
                                move |_, window, app| {
                                    owner
                                        .update(app, |view, cx| {
                                            view.undo(window, cx);
                                        })
                                        .ok();
                                },
                            )
                        }),
                    cx,
                );
                cx.notify();
            }
            Err(error) => self.show_error(error.to_string(), window, cx),
        }
    }
}
