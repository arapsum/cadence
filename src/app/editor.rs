use gpui::{
    App, Context, Entity, IntoElement, Render, SharedString, StatefulInteractiveElement as _,
    Subscription, Window, div, prelude::*, px,
};
use gpui_component::{
    ActiveTheme as _, StyledExt as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    dialog::{DialogButtonProps, DialogFooter},
    input::{Input, InputEvent, InputState, Textarea, TextareaState},
    notification::Notification,
    select::{Select, SelectItem, SelectState},
};
use jiff::{
    Timestamp,
    civil::{Date, Time},
};

use crate::{
    calendar::CategoryFilter,
    domain::{Category, CategoryId, Event, EventId, Settings, format_time},
    editor::{EditorMode, FormDraft, FormErrors, chrono_date, jiff_date, time_options},
    store::TimetableRepository,
};

use super::{state::CadenceView, style::category_dot};

#[derive(Clone)]
pub(super) struct CategoryOption {
    pub(super) category: Category,
    id: CategoryId,
}

impl SelectItem for CategoryOption {
    type Value = CategoryId;

    fn title(&self) -> SharedString {
        self.category.name().into()
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }

    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(category_dot(Some(self.category.color_token())))
            .child(self.category.name().to_owned())
    }
}

#[derive(Clone)]
pub(super) struct TimeOption {
    pub(super) time: Time,
    pub(super) label: SharedString,
}

impl SelectItem for TimeOption {
    type Value = Time;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.time
    }
}

pub(super) struct EventEditor {
    pub(super) mode: EditorMode,
    pub(super) initial: FormDraft,
    pub(super) title: Entity<InputState>,
    pub(super) notes: Entity<TextareaState>,
    pub(super) date: Entity<DatePickerState>,
    pub(super) start_time: Entity<SelectState<Vec<TimeOption>>>,
    pub(super) end_time: Entity<SelectState<Vec<TimeOption>>>,
    pub(super) category: Entity<SelectState<Vec<CategoryOption>>>,
    pub(super) errors: FormErrors,
    focus_title: bool,
    subscriptions: Vec<Subscription>,
}

impl EventEditor {
    pub(super) fn new(
        mode: EditorMode,
        draft: FormDraft,
        categories: Vec<Category>,
        settings: &Settings,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let title = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("e.g. Morning routine")
                .default_value(draft.title.clone())
        });
        let notes = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("Optional notes")
                .default_value(draft.notes.clone())
        });
        let date = cx.new(|cx| {
            let mut state = DatePickerState::new(window, cx).date_format("%B %-d, %Y");
            state.set_date(chrono_date(draft.date), window, cx);
            state
        });

        let raw_times = time_options(
            settings.snap_interval().minutes(),
            &[draft.start_time, draft.end_time],
        );
        let time_items = |times: &[Time]| {
            times
                .iter()
                .copied()
                .map(|time| TimeOption {
                    time,
                    label: format_time(time, settings.clock_format()).into(),
                })
                .collect::<Vec<_>>()
        };
        let start_options = time_items(&raw_times);
        let end_options = start_options.clone();
        let start_index = start_options
            .iter()
            .position(|option| option.time == draft.start_time)
            .map(gpui_component::IndexPath::new);
        let end_index = end_options
            .iter()
            .position(|option| option.time == draft.end_time)
            .map(gpui_component::IndexPath::new);

        let category_options = categories
            .into_iter()
            .map(|category| CategoryOption {
                id: category.id(),
                category,
            })
            .collect::<Vec<_>>();
        let category_index = draft.category_id.and_then(|id| {
            category_options
                .iter()
                .position(|option| option.category.id() == id)
        });

        let start_time = cx.new(|cx| SelectState::new(start_options, start_index, window, cx));
        let end_time = cx.new(|cx| SelectState::new(end_options, end_index, window, cx));
        let category = cx.new(|cx| {
            SelectState::new(
                category_options,
                category_index.map(gpui_component::IndexPath::new),
                window,
                cx,
            )
        });

        Self {
            mode,
            initial: draft,
            title,
            notes,
            date,
            start_time,
            end_time,
            category,
            errors: FormErrors::default(),
            focus_title: true,
            subscriptions: Vec::new(),
        }
    }

    pub(super) fn subscribe(&mut self, cx: &mut Context<'_, Self>) {
        let title = self.title.clone();
        self.subscriptions
            .push(cx.subscribe(&title, |this, _, _: &InputEvent, cx| {
                this.errors.title = None;
                cx.notify();
            }));
        let notes = self.notes.clone();
        self.subscriptions
            .push(cx.subscribe(&notes, |_, _, _: &InputEvent, cx| {
                cx.notify();
            }));
        let date = self.date.clone();
        self.subscriptions
            .push(cx.subscribe(&date, |this, _, _: &DatePickerEvent, cx| {
                this.errors.date = None;
                cx.notify();
            }));
        let start_time = self.start_time.clone();
        self.subscriptions.push(cx.subscribe(
            &start_time,
            |this, _, _: &gpui_component::select::SelectEvent<Vec<TimeOption>>, cx| {
                this.errors.start_time = None;
                this.errors.end_time = None;
                cx.notify();
            },
        ));
        let end_time = self.end_time.clone();
        self.subscriptions.push(cx.subscribe(
            &end_time,
            |this, _, _: &gpui_component::select::SelectEvent<Vec<TimeOption>>, cx| {
                this.errors.end_time = None;
                cx.notify();
            },
        ));
        let category = self.category.clone();
        self.subscriptions.push(cx.subscribe(
            &category,
            |this, _, _: &gpui_component::select::SelectEvent<Vec<CategoryOption>>, cx| {
                this.errors.category = None;
                cx.notify();
            },
        ));
    }

    pub(super) fn form(&self, cx: &App) -> FormDraft {
        FormDraft {
            title: self.title.read(cx).value().to_string(),
            notes: self.notes.read(cx).value().to_string(),
            date: self
                .date
                .read(cx)
                .date()
                .start()
                .map_or(self.initial.date, jiff_date),
            start_time: self
                .start_time
                .read(cx)
                .selected_value()
                .copied()
                .unwrap_or(self.initial.start_time),
            end_time: self
                .end_time
                .read(cx)
                .selected_value()
                .copied()
                .unwrap_or(self.initial.end_time),
            category_id: self.category.read(cx).selected_value().copied(),
        }
    }

    pub(super) fn is_dirty(&self, cx: &App) -> bool {
        self.form(cx) != self.initial
    }
}

impl Render for EventEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        if self.focus_title {
            self.focus_title = false;
            let title = self.title.clone();
            window.defer(cx, move |window, cx| {
                title.update(cx, |title, cx| title.focus(window, cx));
            });
        }

        let errors = self.errors.clone();
        let date = DatePicker::new(&self.date).placeholder("Choose a date");
        let start_time = Select::new(&self.start_time)
            .w(px(148.0))
            .placeholder("Start time");
        let end_time = Select::new(&self.end_time)
            .w(px(148.0))
            .placeholder("End time");
        let category = Select::new(&self.category)
            .w_full()
            .placeholder("Choose a category");

        div()
            .id("event-editor-form")
            .debug_selector(|| "event-editor-form".into())
            .v_flex()
            .gap_4()
            .max_h(px(560.0))
            .overflow_y_scroll()
            .child(field("Title", Input::new(&self.title), errors.title))
            .child(field("Notes", Textarea::new(&self.notes).h(px(84.0)), None))
            .child(field("Date", date, errors.date))
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(field("Start time", start_time, errors.start_time))
                    .child(field("End time", end_time, errors.end_time)),
            )
            .child(field("Category", category, errors.category))
    }
}

fn field<E: IntoElement>(label: &'static str, input: E, error: Option<String>) -> impl IntoElement {
    div()
        .v_flex()
        .gap_1()
        .flex_1()
        .child(div().text_sm().child(label))
        .child(input)
        .when_some(error, |this, error| {
            this.child(
                div()
                    .text_xs()
                    .text_color(gpui::rgb(0x00D9_304F))
                    .child(error),
            )
        })
}

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

impl CadenceView {
    pub(super) fn new_event(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let (today, current_time) = super::presentation::local_date_time(self.now, &self.settings);
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
            FormDraft {
                title: String::new(),
                notes: String::new(),
                date,
                start_time,
                end_time,
                category_id: self.default_category(),
            },
            window,
            cx,
        );
    }

    pub(super) fn new_event_at(
        &mut self,
        date: Date,
        start_time: Time,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let end_time = start_time
            .checked_add(jiff::SignedDuration::from_hours(1))
            .unwrap_or_else(|_| self.settings.day_end())
            .min(self.settings.day_end());
        self.open_editor(
            EditorMode::Create,
            FormDraft {
                title: String::new(),
                notes: String::new(),
                date,
                start_time,
                end_time,
                category_id: self.default_category(),
            },
            window,
            cx,
        );
    }

    pub(super) fn inspect_event(
        &mut self,
        event_id: EventId,
        date: Date,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.state.select_event(event_id, date);
        let event = match self.repository.event(event_id) {
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

    fn open_editor(
        &mut self,
        mode: EditorMode,
        draft: FormDraft,
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
        editor.update(cx, EventEditor::subscribe);
        let owner = cx.entity().downgrade();
        let content_editor = editor.clone();
        let ok_editor = editor.clone();
        let cancel_editor = editor.clone();
        let title = match mode {
            EditorMode::Create => "Create Event",
            EditorMode::Edit(_) => "Edit Event",
        };

        window.open_dialog(cx, move |dialog, _, _| {
            let owner_ok = owner.clone();
            let owner_cancel = owner.clone();
            let content_editor = content_editor.clone();
            let ok_editor = ok_editor.clone();
            let cancel_editor = cancel_editor.clone();
            dialog
                .w(px(480.0))
                .max_w(px(560.0))
                .title(title)
                .overlay_closable(false)
                .content(move |content, _, _| content.child(content_editor.clone()))
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("Save")
                        .cancel_text("Cancel")
                        .show_cancel(true)
                        .on_ok(move |_, window, app| {
                            let ok_editor = ok_editor.clone();
                            owner_ok
                                .update(app, |view, cx| view.commit_editor(&ok_editor, window, cx))
                                .unwrap_or(false)
                        }),
                )
                .on_cancel(move |_, window, app| {
                    let cancel_editor = cancel_editor.clone();
                    let dirty = cancel_editor.read(app).is_dirty(app);
                    if !dirty {
                        return true;
                    }
                    let owner = owner_cancel.clone();
                    window.open_alert_dialog(app, move |alert, _, _| {
                        alert
                            .title("Discard changes?")
                            .description(
                                "Your unsaved changes will be lost if you leave this form.",
                            )
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

    fn commit_editor(
        &mut self,
        editor: &Entity<EventEditor>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let result = editor.read_with(cx, |editor, app| {
            let form = editor.form(app);
            form.to_domain()
        });
        let draft = match result {
            Ok(draft) => draft,
            Err(errors) => {
                editor.update(cx, |editor, _| editor.errors = errors);
                return false;
            }
        };
        let mode = editor.read_with(cx, |editor, _| editor.mode);
        let timestamp = Timestamp::now();
        let result = match mode {
            EditorMode::Create => {
                let id = EventId::new();
                Event::new(id, draft.clone(), timestamp)
                    .map_err(|error| error.to_string())
                    .and_then(|event| {
                        self.repository
                            .create_event(event)
                            .map_err(|error| error.to_string())
                            .map(|()| id)
                    })
            }
            EditorMode::Edit(id) => self
                .repository
                .event(id)
                .map_err(|error| error.to_string())
                .and_then(|event| {
                    event.ok_or_else(|| "That event is no longer available.".to_owned())
                })
                .and_then(|mut event| {
                    event
                        .revise(draft.clone(), timestamp)
                        .map_err(|error| error.to_string())?;
                    self.repository
                        .update_event(event)
                        .map_err(|error| error.to_string())
                        .map(|()| id)
                }),
        };
        let id = match result {
            Ok(id) => id,
            Err(error) => {
                self.show_error(error, window, cx);
                return false;
            }
        };

        self.last_category = Some(draft.category_id);
        self.state.select_event(id, draft.date);
        self.pending_scroll_minutes = None;
        self.scroll_initialized = false;
        self.refresh_snapshot();
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

    fn open_inspector(
        &self,
        event: &Event,
        category: Option<&Category>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let owner = cx.entity().downgrade();
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
        let duplicate = FormDraft::from_event(event);
        window.open_dialog(cx, move |dialog, _, _| {
            let edit_owner = owner.clone();
            let duplicate_owner = owner.clone();
            let delete_owner = owner.clone();
            dialog
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
                                                    duplicate.clone(),
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
                                            duplicate.clone(),
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

    fn confirm_delete(
        &mut self,
        event_id: EventId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(event) = self.repository.event(event_id).ok().flatten() else {
            self.show_error("That event is no longer available.", window, cx);
            return;
        };
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

    fn delete_event(&mut self, event_id: EventId, window: &mut Window, cx: &mut Context<'_, Self>) {
        match self.repository.delete_event(event_id) {
            Ok(event) => {
                self.last_deleted = Some(event);
                self.state.clear_selection();
                self.refresh_snapshot();
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
                                            view.undo_delete(window, cx);
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

    pub(super) fn undo_delete(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(event) = self.last_deleted.clone() else {
            return;
        };
        match self.repository.create_event(event.clone()) {
            Ok(()) => {
                self.last_deleted = None;
                self.state.select_event(event.id(), event.date());
                self.refresh_snapshot();
                window.push_notification(Notification::success("Event restored"), cx);
                cx.notify();
            }
            Err(error) => self.show_error(error.to_string(), window, cx),
        }
    }

    fn show_error(
        &mut self,
        message: impl Into<String>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let message = message.into();
        self.error = Some(message.clone());
        window.push_notification(Notification::error(message), cx);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{AppContext as _, Entity, Modifiers, TestAppContext};
    use gpui_component::{Root, WindowExt as _};

    use super::CadenceView;

    #[gpui::test]
    fn event_entry_points_render_their_dialogs(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let calendar = Rc::new(RefCell::new(None::<Entity<CadenceView>>));
        let captured_calendar = Rc::clone(&calendar);
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| CadenceView::new(window, cx));
            captured_calendar.replace(Some(view.clone()));
            Root::new(view, window, cx)
        });
        let calendar = calendar
            .borrow()
            .clone()
            .expect("calendar view was captured while building the root");

        cx.update(|window, app| window.draw(app).clear(app));
        let new_event = cx
            .debug_bounds("new-event")
            .expect("new event button was rendered");
        cx.simulate_click(new_event.center(), Modifiers::none());

        assert!(cx.update(|window, app| window.has_active_dialog(app)));
        assert!(cx.update(|window, app| Root::render_dialog_layer(window, app).is_some()));
        cx.update(|window, app| window.draw(app).clear(app));
        assert!(cx.debug_bounds("event-editor-form").is_some());

        cx.update(|window, app| window.close_all_dialogs(app));
        let (event_id, event_date) = calendar.read_with(cx, |view, _| {
            let event = view
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.events.first())
                .expect("the seeded calendar contains an event");
            (event.id(), event.date())
        });
        calendar.update_in(cx, |view, window, app| {
            view.inspect_event(event_id, event_date, window, app);
        });
        cx.update(|window, app| window.draw(app).clear(app));
        assert!(cx.debug_bounds("event-inspector-details").is_some());
    }
}
