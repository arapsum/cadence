use gpui::{AppContext as _, Context, Entity, ParentElement as _, Styled as _, Window, px};
use gpui_component::{
    WindowExt as _,
    button::{Button, ButtonVariants as _},
    dialog::{DialogAction, DialogButtonProps, DialogClose, DialogFooter},
    notification::Notification,
    window_paddings,
};
use jiff::{
    Timestamp,
    civil::{Date, Time},
};

use crate::{
    calendar::CategoryFilter,
    domain::{
        Category, CategoryId, Event, EventDraft, EventId, OccurrenceId, RecurrenceSeries,
        RecurrenceSeriesId,
    },
    editor::{EditorMode, FormDraft},
    store::TimetableRepository,
};

use super::super::{
    history::{CalendarChange, ChangeKind},
    state::CadenceView,
    style::dialog_margin_top,
};
use super::{form::EventEditor, recurrence::RecurrenceScope};

impl CadenceView {
    pub(in crate::app) fn new_event(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if !self.is_interactive() {
            return;
        }
        let (today, current_time) =
            crate::app::presentation::local_date_time(self.now, &self.settings);
        let date = self.state.selected_date();
        let (start_time, end_time) = crate::editor::default_times(
            date,
            today,
            current_time,
            self.settings.day_start(),
            self.settings.day_end(),
            self.settings.snap_interval().minutes(),
        );
        self.open_editor(
            EditorMode::Create,
            &FormDraft {
                title: String::new(),
                notes: String::new(),
                date,
                start_time,
                end_time,
                category_id: self.default_category(),
                recurrence: None,
                ends_on: None,
                reminder: None,
            },
            window,
            cx,
        );
    }

    pub(in crate::app) fn new_event_at(
        &mut self,
        date: Date,
        start_time: Time,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.is_interactive() {
            return;
        }
        let end_time = start_time
            .checked_add(jiff::SignedDuration::from_hours(1))
            .unwrap_or_else(|_| self.settings.day_end())
            .min(self.settings.day_end());
        self.open_editor(
            EditorMode::Create,
            &FormDraft {
                title: String::new(),
                notes: String::new(),
                date,
                start_time,
                end_time,
                category_id: self.default_category(),
                recurrence: None,
                ends_on: None,
                reminder: None,
            },
            window,
            cx,
        );
    }

    fn default_category(&self) -> Option<CategoryId> {
        if let CategoryFilter::Only(category_id) = self.state.category_filter() {
            return Some(category_id);
        }
        if let Some(category_id) = self.last_category {
            return Some(category_id);
        }
        self.repository
            .categories()
            .ok()
            .and_then(|categories| categories.first().map(Category::id))
    }

    pub(in crate::app::editor) fn open_editor(
        &mut self,
        mode: EditorMode,
        draft: &FormDraft,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let categories = match self.repository.categories() {
            Ok(categories) => categories,
            Err(error) => {
                self.show_error(error.to_string(), window, cx);
                return;
            }
        };
        let settings = self.settings.clone();
        let editor = cx.new(|cx| EventEditor::new(mode, draft, categories, &settings, window, cx));
        editor.update(cx, |editor, cx| editor.subscribe(window, cx));
        let owner = cx.entity().downgrade();
        let content_editor = editor.clone();
        let ok_editor = editor.clone();
        let cancel_editor = editor.clone();
        let title = match mode {
            EditorMode::Create => "Create Event",
            EditorMode::Edit(_) => "Edit Event",
        };

        window.open_dialog(cx, move |dialog, dialog_window, _| {
            let viewport = dialog_window.viewport_size();
            let padding = window_paddings(dialog_window);

            let available_height = viewport.height - padding.top - padding.bottom;
            let dialog_height = px(520.0);

            let margin_top = dialog_margin_top(available_height, dialog_height);

            let owner_ok = owner.clone();
            let owner_cancel = owner.clone();

            let content_editor = content_editor.clone();

            let ok_editor = ok_editor.clone();
            let cancel_editor = cancel_editor.clone();

            dialog
                .margin_top(margin_top)
                .w(px(480.0))
                .max_w(px(560.0))
                .title(title)
                .overlay_closable(false)
                .content(move |content, _, _| content.child(content_editor.clone()))
                .footer(
                    DialogFooter::new()
                        .pb_4()
                        .child(
                            DialogClose::new()
                                .child(Button::new("event-cancel").outline().label("Cancel")),
                        )
                        .child(
                            DialogAction::new()
                                .child(Button::new("event-save").primary().label("Save")),
                        ),
                )
                .on_ok(move |_, window, app| {
                    let ok_editor = ok_editor.clone();
                    owner_ok
                        .update(app, |view, cx| view.commit_editor(&ok_editor, window, cx))
                        .unwrap_or(false)
                })
                .on_cancel(move |_, window, app| {
                    let cancel_editor = cancel_editor.clone();
                    let dirty = cancel_editor.read(app).is_dirty(app);
                    if !dirty {
                        return true;
                    }
                    let owner = owner_cancel.clone();
                    window.open_alert_dialog(app, move |alert, alert_window, _| {
                        let viewport = alert_window.viewport_size();
                        let padding = window_paddings(alert_window);

                        let available_height = viewport.height - padding.top - padding.bottom;
                        let dialog_height = px(240.0);

                        let margin_top = dialog_margin_top(available_height, dialog_height);

                        alert
                            .title("Discard changes?")
                            .description(
                                "Your unsaved changes will be lost if you leave this form.",
                            )
                            .mt(margin_top)
                            .button_props(
                                DialogButtonProps::default()
                                    .ok_text("Discard")
                                    .cancel_text("Keep editing")
                                    .show_cancel(true),
                            )
                            .on_ok({
                                let owner = owner.clone();
                                move |_, window, app| {
                                    owner
                                        .update(app, |_, cx| {
                                            window.close_all_dialogs(cx);
                                        })
                                        .ok();
                                    true
                                }
                            })
                    });
                    false
                })
        });
    }

    fn open_recurrence_scope_prompt(
        editor: &Entity<EventEditor>,
        _delete: bool,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let owner = cx.entity().downgrade();
        let editor_this = editor.clone();
        let editor_following = editor.clone();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let owner_this = owner.clone();
            let owner_following = owner.clone();
            alert
                .title("Apply recurring change to…")
                .description("Choose whether this change affects one occurrence or this and all following occurrences.")
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("This event")
                        .cancel_text("This and following")
                        .show_cancel(true),
                )
                .on_ok({
                    let editor = editor_this.clone();
                    move |_, window, app| {
                    owner_this
                        .update(app, |view, cx| {
                            window.close_all_dialogs(cx);
                            view.commit_editor_scope(
                                &editor,
                                Some(RecurrenceScope::This),
                                window,
                                cx,
                            );
                        })
                        .ok();
                    true
                    }
                })
                .on_cancel({
                    let editor = editor_following.clone();
                    move |_, window, app| {
                    owner_following
                        .update(app, |view, cx| {
                            window.close_all_dialogs(cx);
                            view.commit_editor_scope(
                                &editor,
                                Some(RecurrenceScope::Following),
                                window,
                                cx,
                            );
                        })
                        .ok();
                    true
                    }
                })
        });
    }

    pub(in crate::app::editor) fn commit_editor(
        &mut self,
        editor: &Entity<EventEditor>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        self.commit_editor_scope(editor, None, window, cx)
    }

    pub(in crate::app::editor) fn commit_editor_scope(
        &mut self,
        editor: &Entity<EventEditor>,
        scope: Option<RecurrenceScope>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let form = editor.read_with(cx, EventEditor::form);
        let draft = match form.to_domain() {
            Ok(draft) => draft,
            Err(errors) => {
                editor.update(cx, |editor, _| editor.set_errors(*errors));
                return false;
            }
        };
        let mode = editor.read_with(cx, |editor, _| editor.mode());
        if scope.is_none() && matches!(mode, EditorMode::Edit(OccurrenceId::Recurring { .. })) {
            Self::open_recurrence_scope_prompt(editor, false, window, cx);
            return false;
        }
        let rollback = self.rollback_view_state();
        let before = match self.repository.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.show_error(error.to_string(), window, cx);
                return false;
            }
        };
        let timestamp = Timestamp::now();
        let result = match mode {
            EditorMode::Create => self.create_editor_change(&form, &draft, &before, timestamp),
            EditorMode::Edit(occurrence_id) => {
                self.edit_editor_change(occurrence_id, &form, &draft, scope, &before, timestamp)
            }
        };
        let (id, change) = match result {
            Ok(result) => result,
            Err(error) => {
                self.show_error(error, window, cx);
                return false;
            }
        };

        self.last_category = Some(draft.category_id);
        self.state.select_event(id, draft.date);
        self.pending_scroll_minutes = None;
        self.reset_scroll_initialization();
        self.refresh_snapshot();
        self.persist_snapshot(
            before,
            rollback,
            super::super::state::HistoryEffect::Record(change),
            cx,
        );
        window.push_notification(
            Notification::success(match mode {
                EditorMode::Create => "Event created",
                EditorMode::Edit(_) => "Event updated",
            }),
            cx,
        );
        cx.notify();
        true
    }

    fn create_editor_change(
        &mut self,
        form: &FormDraft,
        draft: &EventDraft,
        before: &crate::store::PersistenceSnapshot,
        timestamp: Timestamp,
    ) -> Result<(OccurrenceId, CalendarChange), String> {
        if let Some(rule) = form.recurrence {
            let series = RecurrenceSeries::new(
                RecurrenceSeriesId::new(),
                draft.clone(),
                rule,
                form.ends_on,
                timestamp,
            )
            .map_err(|error| error.to_string())?;
            let id = series.id();
            self.repository
                .create_series(series)
                .map_err(|error| error.to_string())?;
            let after = self
                .repository
                .snapshot()
                .map_err(|error| error.to_string())?;
            Ok((
                OccurrenceId::Recurring {
                    series_id: id,
                    original_date: draft.date,
                },
                CalendarChange::Snapshot {
                    before: before.clone(),
                    after,
                    kind: ChangeKind::Create,
                },
            ))
        } else {
            let id = EventId::new();
            Event::new(id, draft.clone(), timestamp)
                .map_err(|error| error.to_string())
                .and_then(|event| {
                    let change = CalendarChange::Create {
                        event: event.clone(),
                    };
                    self.repository
                        .create_event(event)
                        .map_err(|error| error.to_string())
                        .map(|()| (OccurrenceId::Standalone(id), change))
                })
        }
    }

    fn edit_editor_change(
        &mut self,
        occurrence_id: OccurrenceId,
        form: &FormDraft,
        draft: &EventDraft,
        scope: Option<RecurrenceScope>,
        before: &crate::store::PersistenceSnapshot,
        timestamp: Timestamp,
    ) -> Result<(OccurrenceId, CalendarChange), String> {
        match occurrence_id {
            OccurrenceId::Standalone(id) if form.recurrence.is_some() => {
                self.convert_standalone_to_series(id, form, draft, before, timestamp)
            }
            OccurrenceId::Standalone(id) => self.update_standalone_event(id, draft, timestamp),
            OccurrenceId::Recurring {
                series_id,
                original_date,
            } => {
                let active_scope = scope.ok_or_else(|| "Choose a recurrence scope.".to_owned())?;
                self.update_recurring_event(
                    series_id,
                    original_date,
                    form,
                    active_scope,
                    before,
                    timestamp,
                )
            }
        }
    }

    fn convert_standalone_to_series(
        &mut self,
        id: EventId,
        form: &FormDraft,
        draft: &EventDraft,
        before: &crate::store::PersistenceSnapshot,
        timestamp: Timestamp,
    ) -> Result<(OccurrenceId, CalendarChange), String> {
        let event = self
            .repository
            .delete_event(id)
            .map_err(|error| error.to_string())?;
        let rule = form.recurrence.expect("checked above");
        let series = RecurrenceSeries::new(
            RecurrenceSeriesId::new(),
            draft.clone(),
            rule,
            form.ends_on,
            timestamp,
        )
        .map_err(|error| error.to_string())?;
        let series_id = series.id();
        self.repository
            .create_series(series)
            .map_err(|error| error.to_string())?;
        let after = self
            .repository
            .snapshot()
            .map_err(|error| error.to_string())?;
        Ok((
            OccurrenceId::Recurring {
                series_id,
                original_date: event.date(),
            },
            CalendarChange::Snapshot {
                before: before.clone(),
                after,
                kind: ChangeKind::Edit,
            },
        ))
    }

    fn update_standalone_event(
        &mut self,
        id: EventId,
        draft: &EventDraft,
        timestamp: Timestamp,
    ) -> Result<(OccurrenceId, CalendarChange), String> {
        let mut event = self
            .repository
            .event(id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "That event is no longer available.".to_owned())?;
        let event_before = event.draft();
        event
            .revise(draft.clone(), timestamp)
            .map_err(|error| error.to_string())?;
        let after = event.draft();
        self.repository
            .update_event(event)
            .map_err(|error| error.to_string())?;
        Ok((
            OccurrenceId::Standalone(id),
            CalendarChange::Update {
                id,
                before: event_before,
                after,
                kind: ChangeKind::Edit,
            },
        ))
    }

    fn update_recurring_event(
        &mut self,
        series_id: RecurrenceSeriesId,
        original_date: Date,
        form: &FormDraft,
        scope: RecurrenceScope,
        before: &crate::store::PersistenceSnapshot,
        timestamp: Timestamp,
    ) -> Result<(OccurrenceId, CalendarChange), String> {
        let id = self.apply_recurring_edit(series_id, original_date, form, scope, timestamp)?;
        let after = self
            .repository
            .snapshot()
            .map_err(|error| error.to_string())?;
        Ok((
            id,
            CalendarChange::Snapshot {
                before: before.clone(),
                after,
                kind: ChangeKind::Edit,
            },
        ))
    }
}
