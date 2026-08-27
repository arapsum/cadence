use std::collections::HashMap;

use gpui::{
    App, Context, Entity, IntoElement, Render, SharedString, Subscription, Window, div, prelude::*,
    px,
};
use gpui_component::{
    ActiveTheme as _, Disableable as _, IconName, Selectable as _, Sizable as _, StyledExt as _,
    WindowExt as _,
    button::{Button, ButtonVariants as _},
    dialog::{DialogAction, DialogClose, DialogFooter},
    input::{Input, InputEvent, InputState},
    notification::Notification,
    select::{Select, SelectItem, SelectState},
    switch::Switch,
    window_paddings,
};
use jiff::Timestamp;

use crate::{
    calendar::CategoryFilter,
    domain::{Category, CategoryColor, CategoryId, RecurrenceException, RecurrenceExceptionKind},
    store::{InMemoryRepository, PersistenceSnapshot, TimetableRepository},
};

use super::{
    history::{CalendarChange, ChangeKind},
    state::{CadenceView, HistoryEffect},
    style::{category_dot, category_palette, dialog_margin_top},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum CategoryEditorMode {
    Create,
    Edit(CategoryId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CategoryDraft {
    name: String,
    color: CategoryColor,
    visible: bool,
}

impl CategoryDraft {
    fn from_category(category: &Category) -> Self {
        Self {
            name: category.name().to_owned(),
            color: category.color_token(),
            visible: category.is_visible(),
        }
    }
}

pub(in crate::app) struct CategoryEditor {
    mode: CategoryEditorMode,
    initial: CategoryDraft,
    name: Entity<InputState>,
    color: CategoryColor,
    visible: bool,
    error: Option<String>,
    focus_name: bool,
    subscriptions: Vec<Subscription>,
}

impl CategoryEditor {
    fn new(
        mode: CategoryEditorMode,
        initial: CategoryDraft,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let name = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("e.g. Health")
                .default_value(initial.name.clone())
        });
        Self {
            mode,
            color: initial.color,
            visible: initial.visible,
            initial,
            name,
            error: None,
            focus_name: true,
            subscriptions: Vec::new(),
        }
    }

    fn subscribe(&mut self, cx: &mut Context<'_, Self>) {
        let name = self.name.clone();
        self.subscriptions
            .push(cx.subscribe(&name, |this, _, _: &InputEvent, cx| {
                this.error = None;
                cx.notify();
            }));
    }

    fn draft(&self, cx: &App) -> CategoryDraft {
        CategoryDraft {
            name: self.name.read(cx).value().to_string(),
            color: self.color,
            visible: self.visible,
        }
    }

    fn is_dirty(&self, cx: &App) -> bool {
        self.draft(cx) != self.initial
    }

    const fn mode(&self) -> CategoryEditorMode {
        self.mode
    }

    fn set_error(&mut self, error: impl Into<String>, cx: &mut Context<'_, Self>) {
        self.error = Some(error.into());
        cx.notify();
    }

    fn choose_color(&mut self, color: CategoryColor, cx: &mut Context<'_, Self>) {
        self.color = color;
        cx.notify();
    }

    fn set_visible(&mut self, visible: bool, cx: &mut Context<'_, Self>) {
        self.visible = visible;
        cx.notify();
    }
}

impl Render for CategoryEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        if self.focus_name {
            self.focus_name = false;
            let name = self.name.clone();
            window.defer(cx, move |window, cx| {
                name.update(cx, |name, cx| name.focus(window, cx));
            });
        }

        let owner = cx.entity().downgrade();
        let selected_color = self.color;
        let visible_owner = owner.clone();
        div()
            .id("category-editor-form")
            .v_flex()
            .gap_4()
            .child(
                div()
                    .v_flex()
                    .gap_1()
                    .child(div().text_sm().child("Name"))
                    .child(Input::new(&self.name).aria_label("Category name")),
            )
            .when_some(self.error.clone(), |this, error| {
                this.child(div().text_sm().text_color(cx.theme().danger).child(error))
            })
            .child(
                div()
                    .v_flex()
                    .gap_2()
                    .child(div().text_sm().child("Colour"))
                    .child(div().flex().flex_wrap().gap_2().children(
                        CategoryColor::ALL.into_iter().map(|color| {
                            let color_owner = owner.clone();
                            let label = color.label();
                            Button::new(format!("category-color-{}", label.to_lowercase()))
                                .outline()
                                .compact()
                                .selected(selected_color == color)
                                .toggled(selected_color == color)
                                .tooltip(label)
                                .child(
                                    div().size(px(16.0)).rounded_full().bg(category_palette(
                                        color,
                                        cx.theme(),
                                    )
                                    .indicator),
                                )
                                .on_click(move |_, _, app| {
                                    color_owner
                                        .update(app, |editor, cx| {
                                            editor.choose_color(color, cx);
                                        })
                                        .ok();
                                })
                                .into_any_element()
                        }),
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .child(
                        div()
                            .v_flex()
                            .gap_1()
                            .child(div().text_sm().child("Show on calendar"))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Visible categories can be filtered and scheduled."),
                            ),
                    )
                    .child(
                        Switch::new("category-editor-visible")
                            .checked(self.visible)
                            .tooltip("Show category on calendar")
                            .on_click(move |visible, _, app| {
                                visible_owner
                                    .update(app, |editor, cx| {
                                        editor.set_visible(*visible, cx);
                                    })
                                    .ok();
                            }),
                    ),
            )
    }
}

#[derive(Clone)]
struct CategoryOption {
    category: Category,
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

    fn render(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(category_dot(Some(self.category.color_token()), cx.theme()))
            .child(self.category.name().to_owned())
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct CategoryUsage {
    events: usize,
    series: usize,
    exceptions: usize,
}

impl CategoryUsage {
    const fn references(self) -> usize {
        self.events + self.series + self.exceptions
    }

    fn describes_reassignment(self) -> String {
        let mut parts = Vec::new();
        if self.events > 0 {
            parts.push(format!(
                "{} standalone event{}",
                self.events,
                plural(self.events)
            ));
        }
        if self.series > 0 {
            parts.push(format!("{} recurring series", self.series));
        }
        if self.exceptions > 0 {
            parts.push(format!(
                "{} modified occurrence{}",
                self.exceptions,
                plural(self.exceptions)
            ));
        }
        format!(
            "{} will move to the replacement category.",
            join_parts(&parts)
        )
    }
}

const fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

fn join_parts(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let mut joined = parts[..parts.len() - 1].join(", ");
            joined.push_str(", and ");
            joined.push_str(parts.last().expect("non-empty parts"));
            joined
        }
    }
}

struct CategoryDeleteDialog {
    usage: CategoryUsage,
    requires_replacement: bool,
    replacement: Entity<SelectState<Vec<CategoryOption>>>,
    error: Option<String>,
}

impl CategoryDeleteDialog {
    fn new(
        source: CategoryId,
        categories: &[Category],
        usage: CategoryUsage,
        requires_replacement: bool,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let options = categories
            .iter()
            .filter(|category| category.id() != source)
            .cloned()
            .map(|category| CategoryOption {
                id: category.id(),
                category,
            })
            .collect::<Vec<_>>();
        let replacement = cx.new(|cx| SelectState::new(options, None, window, cx));
        Self {
            usage,
            requires_replacement,
            replacement,
            error: None,
        }
    }

    fn replacement(&self, cx: &App) -> Option<CategoryId> {
        self.replacement.read(cx).selected_value().copied()
    }

    fn can_confirm(&self, cx: &App) -> bool {
        !self.requires_replacement || self.replacement(cx).is_some()
    }

    fn require_replacement(&mut self, cx: &mut Context<'_, Self>) {
        self.error = Some("Choose a replacement category before deleting this one.".to_owned());
        cx.notify();
    }
}

impl Render for CategoryDeleteDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(if self.usage.references() == 0 {
                        "This category has no scheduled items."
                    } else {
                        "Choose where its scheduled items should go."
                    }),
            )
            .when(self.usage.references() > 0, |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(self.usage.describes_reassignment()),
                )
            })
            .when(self.requires_replacement, |this| {
                this.child(
                    div()
                        .v_flex()
                        .gap_1()
                        .child(div().text_sm().child("Replacement category"))
                        .child(
                            Select::new(&self.replacement)
                                .placeholder("Choose a replacement category"),
                        ),
                )
            })
            .when_some(self.error.clone(), |this, error| {
                this.child(div().text_sm().text_color(cx.theme().danger).child(error))
            })
    }
}

pub(in crate::app) struct CategoryManager {
    owner: gpui::WeakEntity<CadenceView>,
    _subscriptions: Vec<Subscription>,
}

impl CategoryManager {
    pub(in crate::app) fn new(owner: &Entity<CadenceView>, cx: &mut Context<'_, Self>) -> Self {
        let subscription = cx.observe(owner, |_this, _, cx| {
            cx.notify();
        });
        Self {
            owner: owner.downgrade(),
            _subscriptions: vec![subscription],
        }
    }

    fn render_header(&self, interactive: bool, cx: &Context<'_, Self>) -> impl IntoElement {
        let owner = self.owner.clone();
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Create, edit, and organise your calendar categories."),
            )
            .child(
                Button::new("settings-new-category")
                    .outline()
                    .small()
                    .icon(IconName::Plus)
                    .label("Add category")
                    .disabled(!interactive)
                    .on_click(move |_, window, app| {
                        owner
                            .update(app, |view, cx| view.new_category(window, cx))
                            .ok();
                    }),
            )
    }

    fn render_category_row(
        &self,
        category: &Category,
        only_category: bool,
        interactive: bool,
        cx: &Context<'_, Self>,
    ) -> gpui::AnyElement {
        let category_id = category.id();
        let name = category.name().to_owned();
        let color = category.color_token();
        let visible = category.is_visible();
        let owner = self.owner.clone();
        let edit_owner = owner.clone();
        let delete_owner = owner.clone();
        let visibility_owner = owner;
        div()
            .id(format!("settings-category-{category_id}"))
            .flex()
            .items_center()
            .gap_3()
            .rounded_md()
            .border_1()
            .border_color(cx.theme().border)
            .p_2()
            .child(category_dot(Some(color), cx.theme()))
            .child(div().flex_1().min_w_0().text_sm().child(name.clone()))
            .child(
                Switch::new(format!("settings-category-visible-{category_id}"))
                    .checked(visible)
                    .tooltip(format!("Show {name} events"))
                    .disabled(!interactive)
                    .on_click(move |visible, window, app| {
                        visibility_owner
                            .update(app, |view, cx| {
                                view.set_category_visibility(category_id, *visible, window, cx);
                            })
                            .ok();
                    }),
            )
            .child(
                Button::new(format!("settings-edit-category-{category_id}"))
                    .ghost()
                    .small()
                    .icon(IconName::Ellipsis)
                    .tooltip(format!("Edit {name}"))
                    .disabled(!interactive)
                    .on_click(move |_, window, app| {
                        edit_owner
                            .update(app, |view, cx| {
                                view.edit_category(category_id, window, cx);
                            })
                            .ok();
                    }),
            )
            .child(
                Button::new(format!("settings-delete-category-{category_id}"))
                    .ghost()
                    .small()
                    .icon(IconName::Close)
                    .tooltip(format!("Delete {name}"))
                    .disabled(!interactive || only_category)
                    .on_click(move |_, window, app| {
                        delete_owner
                            .update(app, |view, cx| {
                                view.confirm_delete_category(category_id, window, cx);
                            })
                            .ok();
                    }),
            )
            .into_any_element()
    }
}

impl Render for CategoryManager {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let (categories, interactive) = self
            .owner
            .read_with(cx, |view, _| {
                (
                    view.repository.categories().unwrap_or_default(),
                    view.is_interactive(),
                )
            })
            .unwrap_or_default();
        let only_category = categories.len() == 1;
        let rows = categories
            .iter()
            .map(|category| self.render_category_row(category, only_category, interactive, cx))
            .collect::<Vec<_>>();
        div()
            .id("category-manager")
            .v_flex()
            .gap_2()
            .w_full()
            .child(self.render_header(interactive, cx))
            .children(rows)
    }
}

impl CadenceView {
    /// Opens the create-category dialog.
    pub(in crate::app) fn new_category(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if !self.is_interactive() {
            return;
        }
        Self::open_category_editor(
            CategoryEditorMode::Create,
            CategoryDraft {
                name: String::new(),
                color: self.next_category_color(),
                visible: true,
            },
            window,
            cx,
        );
    }

    /// Opens the editor for an existing category.
    ///
    /// # Parameters
    ///
    /// - `id`: Stable identity of the category to edit.
    pub(in crate::app) fn edit_category(
        &mut self,
        id: CategoryId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.is_interactive() {
            return;
        }
        let category = self.repository.category(id).ok().flatten();
        let Some(category) = category else {
            self.show_error("That category is no longer available.", window, cx);
            return;
        };
        Self::open_category_editor(
            CategoryEditorMode::Edit(id),
            CategoryDraft::from_category(&category),
            window,
            cx,
        );
    }

    fn next_category_color(&self) -> CategoryColor {
        let mut uses = HashMap::<CategoryColor, usize>::new();
        for category in self.repository.categories().unwrap_or_default() {
            *uses.entry(category.color_token()).or_default() += 1;
        }
        CategoryColor::ALL
            .into_iter()
            .min_by_key(|color| uses.get(color).copied().unwrap_or_default())
            .expect("category color palette is non-empty")
    }

    fn open_category_editor(
        mode: CategoryEditorMode,
        initial: CategoryDraft,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let editor = cx.new(|cx| CategoryEditor::new(mode, initial, window, cx));
        editor.update(cx, CategoryEditor::subscribe);
        let owner = cx.entity().downgrade();
        let content_editor = editor.clone();
        let save_editor = editor.clone();
        let cancel_editor = editor;
        let title = match mode {
            CategoryEditorMode::Create => "Create category",
            CategoryEditorMode::Edit(_) => "Edit category",
        };

        window.open_dialog(cx, move |dialog, dialog_window, _| {
            let viewport = dialog_window.viewport_size();
            let padding = window_paddings(dialog_window);
            let available_height = viewport.height - padding.top - padding.bottom;
            let dialog_height = px(390.0);
            let margin_top = dialog_margin_top(available_height, dialog_height);
            let save_owner = owner.clone();
            let cancel_owner = owner.clone();
            let content_editor = content_editor.clone();
            let save_editor = save_editor.clone();
            let cancel_editor = cancel_editor.clone();

            dialog
                .margin_top(margin_top)
                .w(px(440.0))
                .title(title)
                .overlay_closable(false)
                .content(move |content, _, _| content.child(content_editor.clone()))
                .footer(
                    DialogFooter::new()
                        .pb_4()
                        .child(
                            DialogClose::new()
                                .child(Button::new("category-cancel").outline().label("Cancel")),
                        )
                        .child(
                            DialogAction::new().child(
                                Button::new("category-save").primary().label("Save"),
                            ),
                        ),
                )
                .on_ok(move |_, window, app| {
                    let save_editor = save_editor.clone();
                    save_owner
                        .update(app, |view, cx| {
                            view.commit_category_editor(&save_editor, window, cx)
                        })
                        .unwrap_or(false)
                })
                .on_cancel(move |_, window, app| {
                    if !cancel_editor.read(app).is_dirty(app) {
                        return true;
                    }
                    let owner = cancel_owner.clone();
                    window.open_alert_dialog(app, move |alert, alert_window, _| {
                        let viewport = alert_window.viewport_size();
                        let padding = window_paddings(alert_window);
                        let available_height = viewport.height - padding.top - padding.bottom;
                        alert
                            .title("Discard changes?")
                            .description(
                                "Your unsaved category changes will be lost if you leave this form.",
                            )
                            .mt(dialog_margin_top(available_height, px(240.0)))
                            .button_props(
                                gpui_component::dialog::DialogButtonProps::default()
                                    .ok_text("Discard")
                                    .cancel_text("Keep editing")
                                    .show_cancel(true),
                            )
                            .on_ok({
                                let owner = owner.clone();
                                move |_, window, app| {
                                    owner
                                        .update(app, |_, cx| window.close_all_dialogs(cx))
                                        .ok();
                                    true
                                }
                            })
                    });
                    false
                })
        });
    }

    fn commit_category_editor(
        &mut self,
        editor: &Entity<CategoryEditor>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if !self.is_interactive() {
            return false;
        }
        let draft = editor.read_with(cx, CategoryEditor::draft);
        let mode = editor.read_with(cx, |editor, _| editor.mode());
        let rollback = self.rollback_view_state();
        let before = match self.repository.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.show_error(error.to_string(), window, cx);
                return false;
            }
        };
        let result = match mode {
            CategoryEditorMode::Create => {
                Category::new(CategoryId::new(), draft.name, draft.color, draft.visible)
                    .map_err(|error| error.to_string())
                    .and_then(|category| {
                        self.repository
                            .create_category(category)
                            .map_err(|error| error.to_string())
                    })
            }
            CategoryEditorMode::Edit(id) => self
                .revise_category(id, draft)
                .map_err(|error| error.to_string()),
        };
        if let Err(error) = result {
            editor.update(cx, |editor, cx| editor.set_error(error, cx));
            return false;
        }
        let kind = match mode {
            CategoryEditorMode::Create => ChangeKind::CreateCategory,
            CategoryEditorMode::Edit(_) => ChangeKind::EditCategory,
        };
        self.commit_category_snapshot(before, rollback, kind, window, cx);
        window.push_notification(
            Notification::success(match mode {
                CategoryEditorMode::Create => "Category created",
                CategoryEditorMode::Edit(_) => "Category updated",
            }),
            cx,
        );
        true
    }

    fn revise_category(
        &mut self,
        id: CategoryId,
        draft: CategoryDraft,
    ) -> Result<(), crate::domain::RepositoryError> {
        let categories = self.repository.categories()?;
        let Some(mut category) = categories.into_iter().find(|category| category.id() == id) else {
            return Err(crate::domain::RepositoryError::CategoryNotFound);
        };
        if !draft.visible
            && category.is_visible()
            && self
                .repository
                .categories()?
                .iter()
                .filter(|category| category.is_visible())
                .count()
                == 1
        {
            return Err(crate::domain::RepositoryError::InvalidEntity(
                "Keep at least one category visible.".to_owned(),
            ));
        }
        category
            .revise(draft.name, draft.color, draft.visible)
            .map_err(|error| crate::domain::RepositoryError::InvalidEntity(error.to_string()))?;
        self.repository.update_category(category)
    }

    /// Opens the destructive confirmation flow for a category.
    ///
    /// # Parameters
    ///
    /// - `id`: Stable identity of the category to remove.
    pub(in crate::app) fn confirm_delete_category(
        &mut self,
        id: CategoryId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.is_interactive() {
            return;
        }
        let categories = match self.repository.categories() {
            Ok(categories) => categories,
            Err(error) => {
                self.show_error(error.to_string(), window, cx);
                return;
            }
        };
        let Some(category) = categories.iter().find(|category| category.id() == id) else {
            self.show_error("That category is no longer available.", window, cx);
            return;
        };
        if categories.len() == 1 {
            self.show_error("Keep at least one category.", window, cx);
            return;
        }
        let usage = self.category_usage(id);
        let sole_visible = category.is_visible()
            && categories
                .iter()
                .filter(|category| category.is_visible())
                .count()
                == 1;
        let requires_replacement = usage.references() > 0 || sole_visible;
        let source_name = category.name().to_owned();
        let deletion = cx.new(|cx| {
            CategoryDeleteDialog::new(id, &categories, usage, requires_replacement, window, cx)
        });
        let owner = cx.entity().downgrade();
        let content_deletion = deletion.clone();
        let confirm_deletion = deletion;

        window.open_dialog(cx, move |dialog, dialog_window, _| {
            let viewport = dialog_window.viewport_size();
            let padding = window_paddings(dialog_window);
            let available_height = viewport.height - padding.top - padding.bottom;
            let content_deletion = content_deletion.clone();
            let confirm_deletion = confirm_deletion.clone();
            let confirm_owner = owner.clone();
            dialog
                .margin_top(dialog_margin_top(available_height, px(310.0)))
                .w(px(440.0))
                .title(format!("Delete {source_name}?"))
                .overlay_closable(false)
                .content(move |content, _, _| content.child(content_deletion.clone()))
                .footer(
                    DialogFooter::new()
                        .pb_4()
                        .child(
                            DialogClose::new().child(
                                Button::new("cancel-delete-category")
                                    .outline()
                                    .label("Cancel"),
                            ),
                        )
                        .child(
                            DialogAction::new().child(
                                Button::new("confirm-delete-category")
                                    .danger()
                                    .label("Delete category"),
                            ),
                        ),
                )
                .on_ok(move |_, window, app| {
                    if !confirm_deletion.read_with(app, CategoryDeleteDialog::can_confirm) {
                        confirm_deletion.update(app, CategoryDeleteDialog::require_replacement);
                        return false;
                    }
                    let replacement =
                        confirm_deletion.read_with(app, CategoryDeleteDialog::replacement);
                    confirm_owner
                        .update(app, |view, cx| {
                            view.delete_category(id, replacement, window, cx)
                        })
                        .unwrap_or(false)
                })
        });
    }

    fn category_usage(&self, id: CategoryId) -> CategoryUsage {
        let Ok(snapshot) = self.repository.snapshot() else {
            return CategoryUsage::default();
        };
        CategoryUsage {
            events: snapshot
                .events
                .iter()
                .filter(|event| event.category_id() == id)
                .count(),
            series: snapshot
                .recurrence_series
                .iter()
                .filter(|series| series.template().category_id == id)
                .count(),
            exceptions: snapshot
                .recurrence_exceptions
                .iter()
                .filter(|exception| exception_uses_category(exception, id))
                .count(),
        }
    }

    fn delete_category(
        &mut self,
        id: CategoryId,
        replacement: Option<CategoryId>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let rollback = self.rollback_view_state();
        let before = match self.repository.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.show_error(error.to_string(), window, cx);
                return false;
            }
        };
        let after = match remove_category_from_snapshot(&before, id, replacement, Timestamp::now())
        {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.show_error(error, window, cx);
                return false;
            }
        };
        match InMemoryRepository::from_snapshot(&after) {
            Ok(repository) => self.repository = repository,
            Err(error) => {
                self.show_error(error.to_string(), window, cx);
                return false;
            }
        }
        self.commit_category_snapshot(before, rollback, ChangeKind::DeleteCategory, window, cx);
        window.push_notification(Notification::success("Category deleted"), cx);
        true
    }

    fn commit_category_snapshot(
        &mut self,
        before: PersistenceSnapshot,
        rollback: super::state::RollbackViewState,
        kind: ChangeKind,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.reconcile_category_state(window, cx);
        let Ok(after) = self.repository.snapshot() else {
            return;
        };
        let change = CalendarChange::Snapshot {
            before: Box::new(before.clone()),
            after: Box::new(after),
            kind,
        };
        self.state.clear_selection();
        self.pending_scroll_minutes = None;
        self.reset_scroll_initialization();
        self.refresh_snapshot();
        self.persist_snapshot(before, rollback, HistoryEffect::Record(change), cx);
        cx.notify();
    }

    pub(in crate::app) fn reconcile_category_state(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let categories = self.repository.categories().unwrap_or_default();
        let valid_filter = match self.state.category_filter() {
            CategoryFilter::All => true,
            CategoryFilter::Only(id) => categories
                .iter()
                .any(|category| category.id() == id && category.is_visible()),
        };
        if !valid_filter {
            self.state.set_category_filter(CategoryFilter::All);
        }
        if self
            .last_category
            .is_some_and(|id| !categories.iter().any(|category| category.id() == id))
        {
            self.last_category = None;
        }
        let _ = self.repository.replace_preferences(self.preferences());
        self.sync_category_filter(window, cx);
    }
}

fn exception_uses_category(exception: &RecurrenceException, category_id: CategoryId) -> bool {
    matches!(
        exception.kind(),
        RecurrenceExceptionKind::Modified(draft) if draft.category_id == category_id
    )
}

fn remove_category_from_snapshot(
    before: &PersistenceSnapshot,
    source: CategoryId,
    replacement: Option<CategoryId>,
    timestamp: Timestamp,
) -> Result<PersistenceSnapshot, String> {
    let mut after = before.clone();
    let source_category = after
        .categories
        .iter()
        .find(|category| category.id() == source)
        .cloned()
        .ok_or_else(|| "That category is no longer available.".to_owned())?;
    if after.categories.len() == 1 {
        return Err("Keep at least one category.".to_owned());
    }
    let usage = CategoryUsage {
        events: after
            .events
            .iter()
            .filter(|event| event.category_id() == source)
            .count(),
        series: after
            .recurrence_series
            .iter()
            .filter(|series| series.template().category_id == source)
            .count(),
        exceptions: after
            .recurrence_exceptions
            .iter()
            .filter(|exception| exception_uses_category(exception, source))
            .count(),
    };
    let sole_visible = source_category.is_visible()
        && after
            .categories
            .iter()
            .filter(|category| category.is_visible())
            .count()
            == 1;
    let requires_replacement = usage.references() > 0 || sole_visible;
    if requires_replacement && replacement.is_none() {
        return Err("Choose a replacement category first.".to_owned());
    }
    let replacement = replacement.filter(|id| *id != source);
    if requires_replacement && replacement.is_none() {
        return Err("Choose a different replacement category.".to_owned());
    }
    if let Some(replacement) = replacement
        && !after
            .categories
            .iter()
            .any(|category| category.id() == replacement)
    {
        return Err("The replacement category is no longer available.".to_owned());
    }

    if let Some(replacement) = replacement {
        for event in &mut after.events {
            if event.category_id() == source {
                let mut draft = event.draft();
                draft.category_id = replacement;
                event
                    .revise(draft, timestamp)
                    .map_err(|error| error.to_string())?;
            }
        }
        for series in &mut after.recurrence_series {
            if series.template().category_id == source {
                let mut draft = series.template();
                draft.category_id = replacement;
                series
                    .revise(draft, series.rule(), series.ends_on(), timestamp)
                    .map_err(|error| error.to_string())?;
            }
        }
        for exception in &mut after.recurrence_exceptions {
            if let RecurrenceExceptionKind::Modified(draft) = exception.kind()
                && draft.category_id == source
            {
                let mut draft = draft.clone();
                draft.category_id = replacement;
                *exception = RecurrenceException::modified(
                    exception.series_id(),
                    exception.original_date(),
                    draft,
                    timestamp,
                )
                .map_err(|error| error.to_string())?;
            }
        }
        if sole_visible
            && let Some(category) = after
                .categories
                .iter_mut()
                .find(|category| category.id() == replacement)
        {
            category.set_visible(true);
        }
    }
    after.categories.retain(|category| category.id() != source);
    if after.preferences.category_filter == Some(source) {
        after.preferences.category_filter = None;
    }
    Ok(after)
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Date, Time};
    use uuid::Uuid;

    use super::*;
    use crate::{
        domain::{
            Event, EventDraft, EventId, RecurrenceRule, RecurrenceSeries, RecurrenceSeriesId,
        },
        store::{AppPreferences, CalendarViewModePreference},
    };

    fn test_category(id: u128, name: &str, color: CategoryColor, visible: bool) -> Category {
        Category::new(
            CategoryId::from_uuid(Uuid::from_u128(id)),
            name,
            color,
            visible,
        )
        .expect("valid category")
    }

    fn test_draft(
        title: &str,
        date: Date,
        start_hour: i8,
        end_hour: i8,
        category_id: CategoryId,
    ) -> EventDraft {
        EventDraft::new(
            title,
            date,
            Time::constant(start_hour, 0, 0, 0),
            Time::constant(end_hour, 0, 0, 0),
            category_id,
            None,
        )
    }

    #[test]
    fn replacement_deletion_rewrites_every_category_reference() {
        let source = test_category(1, "Source", CategoryColor::Coral, true);
        let target = test_category(2, "Target", CategoryColor::Blue, false);
        let date = Date::constant(2026, 8, 22);
        let timestamp = Timestamp::from_second(0).expect("valid timestamp");
        let event = Event::new(
            EventId::from_uuid(Uuid::from_u128(3)),
            test_draft("Standalone", date, 9, 10, source.id()),
            timestamp,
        )
        .expect("valid event");
        let series = RecurrenceSeries::new(
            RecurrenceSeriesId::from_uuid(Uuid::from_u128(4)),
            test_draft("Recurring", date, 11, 12, source.id()),
            RecurrenceRule::Daily,
            None,
            timestamp,
        )
        .expect("valid series");
        let exception = RecurrenceException::modified(
            series.id(),
            date,
            test_draft("Exception", date, 13, 14, source.id()),
            timestamp,
        )
        .expect("valid exception");
        let before = PersistenceSnapshot {
            settings: crate::domain::Settings::default(),
            preferences: AppPreferences {
                view_mode: CalendarViewModePreference::Week,
                category_filter: Some(source.id()),
                notifications_enabled: false,
                reduce_motion: false,
                appearance: crate::store::AppearancePreferences::default(),
            },
            categories: vec![source.clone(), target.clone()],
            events: vec![event],
            recurrence_series: vec![series],
            recurrence_exceptions: vec![exception],
        };

        let after =
            remove_category_from_snapshot(&before, source.id(), Some(target.id()), timestamp)
                .expect("replacement deletion succeeds");

        assert_eq!(
            after.categories,
            vec![test_category(2, "Target", CategoryColor::Blue, true)]
        );
        assert_eq!(after.preferences.category_filter, None);
        assert!(
            after
                .events
                .iter()
                .all(|event| event.category_id() == target.id())
        );
        assert!(
            after
                .recurrence_series
                .iter()
                .all(|series| series.template().category_id == target.id())
        );
        assert!(after.recurrence_exceptions.iter().all(|exception| {
            matches!(
                exception.kind(),
                RecurrenceExceptionKind::Modified(draft) if draft.category_id == target.id()
            )
        }));
        assert!(InMemoryRepository::from_snapshot(&after).is_ok());
    }
}
