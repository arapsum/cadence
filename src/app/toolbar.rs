use gpui::{App, Context, IntoElement, SharedString, Window, div, prelude::*, px};
use gpui_component::{
    ActiveTheme as _, IconName, StyledExt as _, Theme, ThemeMode,
    button::{Button, ButtonVariants as _},
    select::{Select, SelectItem},
};

use crate::calendar::CategoryFilter;
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
        .into_any_element();
    let filter = Select::new(&view.category_filter)
        .w(px(if compact { 190.0 } else { 210.0 }))
        .appearance(false)
        .placeholder("Filter categories")
        .into_any_element();
    let navigation = render_navigation(view, compact, cx);
    let today_button = Button::new("today")
        .outline()
        .label("Today")
        .on_click(cx.listener(|this, _, _, cx| this.go_to_today(cx)));
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
        });

    if compact {
        div()
            .v_flex()
            .gap_3()
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(title)
                    .child(theme_button),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(filter)
                    .child(today_button)
                    .child(navigation),
            )
            .into_any_element()
    } else {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .p_4()
            .child(title)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(filter)
                    .child(today_button)
                    .child(navigation)
                    .child(theme_button),
            )
            .into_any_element()
    }
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
            Button::new("previous-week")
                .ghost()
                .icon(IconName::ChevronLeft)
                .on_click(cx.listener(|this, _, _, cx| this.shift_week(false, cx))),
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
                .child(view.week_range_label()),
        )
        .child(
            Button::new("next-week")
                .ghost()
                .icon(IconName::ChevronRight)
                .on_click(cx.listener(|this, _, _, cx| this.shift_week(true, cx))),
        )
        .into_any_element()
}
