use gpui::{Context, IntoElement, Window, div, prelude::*, px};
use gpui_component::{ActiveTheme as _, StyledExt as _};

use super::{sidebar, state::CadenceView, week};

pub(super) fn render(
    view: &mut CadenceView,
    window: &Window,
    cx: &mut Context<'_, CadenceView>,
) -> impl IntoElement {
    let sidebar_collapsed = window.viewport_size().width.as_f32() < 1_320.0;

    div()
        .flex()
        .flex_1()
        .min_h_0()
        .bg(cx.theme().muted.opacity(0.22))
        .child(sidebar::render(view, sidebar_collapsed, cx))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .p_3()
                .child(render_week_panel(view, window, cx)),
        )
}

fn render_week_panel(
    view: &mut CadenceView,
    window: &Window,
    cx: &mut Context<'_, CadenceView>,
) -> gpui::AnyElement {
    let range = view
        .surface_snapshot(crate::calendar::CalendarViewMode::Week)
        .map_or_else(String::new, |surface| {
            format!(
                "{} – {}",
                surface.range.start().strftime("%b %-d"),
                surface
                    .range
                    .end()
                    .yesterday()
                    .unwrap_or_else(|_| surface.range.start())
                    .strftime("%b %-d, %Y")
            )
        });

    div()
        .id("week-workspace-panel")
        .debug_selector(|| "week-workspace-panel".into())
        .v_flex()
        .size_full()
        .min_w_0()
        .overflow_hidden()
        .rounded_lg()
        .border_1()
        .border_color(cx.theme().primary.opacity(0.72))
        .bg(cx.theme().background)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .h(px(48.0))
                .flex_shrink_0()
                .px_4()
                .border_b_1()
                .border_color(cx.theme().border.opacity(0.72))
                .child(div().text_sm().font_semibold().child("Week overview"))
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(range),
                ),
        )
        .child(week::render(view, window, cx))
        .into_any_element()
}
