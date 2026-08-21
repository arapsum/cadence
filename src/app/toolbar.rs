use gpui::{App, Context, IntoElement, SharedString, Window, div, prelude::*, px};
use gpui_component::{
    ActiveTheme as _, Disableable as _, IconName, StyledExt as _, Theme, ThemeMode,
    button::{Button, ButtonVariants as _},
    select::{Select, SelectItem},
    tab::{Tab, TabBar},
};

use crate::calendar::{CalendarViewMode, CategoryFilter};
use crate::domain::CategoryColor;

use super::{state::CadenceView, style::category_dot};

#[derive(Clone)]
pub(super) struct FilterOption {
    pub(super) filter: CategoryFilter,
    pub(super) label: SharedString,
    pub(super) color: Option<CategoryColor>,
}

impl FilterOption {
    pub(super) fn all() -> Self {
        Self {
            filter: CategoryFilter::All,
            label: "All Category".into(),
            color: None,
        }
    }
}

impl SelectItem for FilterOption {
    type Value = CategoryFilter;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.filter
    }

    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(category_dot(self.color))
            .child(self.label.clone())
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn render(
    view: &CadenceView,
    window: &Window,
    cx: &Context<'_, CadenceView>,
) -> gpui::AnyElement {
    let compact = window.viewport_size().width.as_f32() < 760.0;
    let is_dark = cx.theme().mode.is_dark();
    let theme_icon = if is_dark {
        IconName::Sun
    } else {
        IconName::Moon
    };
    let theme_label = if is_dark {
        "Use light theme"
    } else {
        "Use dark theme"
    };
    let title = div()
        .text_2xl()
        .font_semibold()
        .child("Timetable")
        .when(
            matches!(
                view.persistence_state,
                super::state::PersistenceState::Writing
            ),
            |this| {
                this.child(
                    div()
                        .ml_2()
                        .text_xs()
                        .font_normal()
                        .text_color(cx.theme().muted_foreground)
                        .child("Saving…"),
                )
            },
        )
        .into_any_element();
    let interactive = view.is_interactive();
    let undo_button = Button::new("undo")
        .ghost()
        .icon(IconName::Undo2)
        .disabled(!interactive || !view.history.can_undo())
        .tooltip("Undo (Ctrl/Cmd+Z)")
        .on_click(cx.listener(|this, _, window, cx| this.undo(window, cx)))
        .into_any_element();
    let redo_button = Button::new("redo")
        .ghost()
        .icon(IconName::Redo2)
        .disabled(!interactive || !view.history.can_redo())
        .tooltip("Redo (Ctrl/Cmd+Shift+Z)")
        .on_click(cx.listener(|this, _, window, cx| this.redo(window, cx)))
        .into_any_element();
    let filter = Select::new(&view.category_filter)
        .w(px(if compact { 190.0 } else { 210.0 }))
        .appearance(false)
        .disabled(!interactive)
        .placeholder("Filter categories")
        .into_any_element();
    let mode_control = TabBar::new("calendar-view-mode")
        .segmented()
        .selected_index(match view.state.view_mode() {
            CalendarViewMode::Day => 0,
            CalendarViewMode::Week => 1,
        })
        .on_click(cx.listener(|this, index: &usize, _, cx| {
            let mode = if *index == 0 {
                CalendarViewMode::Day
            } else {
                CalendarViewMode::Week
            };
            this.set_view_mode(mode, cx);
        }))
        .child(Tab::new().label("Day"))
        .child(Tab::new().label("Week"))
        .into_any_element();
    let navigation = render_navigation(view, compact, cx);
    let today_button = Button::new("today")
        .outline()
        .disabled(!interactive)
        .label("Today")
        .on_click(cx.listener(|this, _, _, cx| this.go_to_today(cx)))
        .into_any_element();
    let new_event_button = Button::new("new-event")
        .debug_selector(|| "new-event".into())
        .primary()
        .disabled(!interactive)
        .label("New event")
        .tooltip("New event (Ctrl/Cmd+N)")
        .on_click(cx.listener(|this, _, window, cx| {
            cx.stop_propagation();
            this.new_event(window, cx);
        }))
        .into_any_element();
    let export_button = Button::new("export-backup")
        .outline()
        .disabled(!interactive)
        .label("Export")
        .tooltip("Export a JSON backup")
        .on_click(cx.listener(|this, _, window, cx| this.export_backup(window, cx)))
        .into_any_element();
    let theme_button = Button::new("toggle-theme")
        .ghost()
        .icon(theme_icon)
        .tooltip(theme_label)
        .on_click(|_, window, cx| {
            let mode = if cx.theme().mode.is_dark() {
                ThemeMode::Light
            } else {
                ThemeMode::Dark
            };
            Theme::change(mode, Some(window), cx);
        })
        .into_any_element();

    let elements = ToolbarElements {
        title,
        undo_button,
        redo_button,
        theme_button,
        mode_control,
        filter,
        today_button,
        new_event_button,
        export_button,
        navigation,
    };
    if compact {
        render_compact(elements)
    } else {
        render_wide(elements)
    }
}

struct ToolbarElements {
    title: gpui::AnyElement,
    undo_button: gpui::AnyElement,
    redo_button: gpui::AnyElement,
    theme_button: gpui::AnyElement,
    mode_control: gpui::AnyElement,
    filter: gpui::AnyElement,
    today_button: gpui::AnyElement,
    new_event_button: gpui::AnyElement,
    export_button: gpui::AnyElement,
    navigation: gpui::AnyElement,
}

fn render_compact(elements: ToolbarElements) -> gpui::AnyElement {
    let ToolbarElements {
        title,
        undo_button,
        redo_button,
        theme_button,
        mode_control,
        filter,
        today_button,
        new_event_button,
        export_button,
        navigation,
    } = elements;
    let history_controls = div()
        .flex()
        .items_center()
        .gap_1()
        .child(undo_button)
        .child(redo_button);
    div()
        .v_flex()
        .gap_3()
        .p_4()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(title)
                        .child(history_controls),
                )
                .child(theme_button),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(mode_control)
                .child(filter),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(today_button)
                .child(new_event_button)
                .child(export_button)
                .child(navigation),
        )
        .into_any_element()
}

fn render_wide(elements: ToolbarElements) -> gpui::AnyElement {
    let ToolbarElements {
        title,
        undo_button,
        redo_button,
        theme_button,
        mode_control,
        filter,
        today_button,
        new_event_button,
        export_button,
        navigation,
    } = elements;
    let history_controls = div()
        .flex()
        .items_center()
        .gap_1()
        .child(undo_button)
        .child(redo_button);
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .p_4()
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(title)
                .child(history_controls),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(mode_control)
                .child(filter)
                .child(today_button)
                .child(new_event_button)
                .child(export_button)
                .child(navigation)
                .child(theme_button),
        )
        .into_any_element()
}

fn render_navigation(
    view: &CadenceView,
    compact: bool,
    cx: &Context<'_, CadenceView>,
) -> gpui::AnyElement {
    div()
        .flex()
        .items_center()
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .child(
            Button::new("previous-period")
                .ghost()
                .icon(IconName::ChevronLeft)
                .tooltip("Previous period (Alt+Left)")
                .on_click(cx.listener(|this, _, _, cx| this.shift_period(false, cx))),
        )
        .child(
            div()
                .min_w(px(if compact { 166.0 } else { 180.0 }))
                .px_3()
                .text_center()
                .text_sm()
                .font_medium()
                .border_l_1()
                .border_r_1()
                .border_color(cx.theme().border)
                .child(view.range_label()),
        )
        .child(
            Button::new("next-period")
                .ghost()
                .icon(IconName::ChevronRight)
                .tooltip("Next period (Alt+Right)")
                .on_click(cx.listener(|this, _, _, cx| this.shift_period(true, cx))),
        )
        .into_any_element()
}
