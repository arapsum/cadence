use gpui::{Context, Hsla, IntoElement, Window, div, point, prelude::*, px};
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::{ActiveTheme as _, StyledExt as _};

use crate::domain::format_time;
use jiff::civil::Time;

use super::{
    grid,
    presentation::local_date_time,
    state::CadenceView,
    style::{
        DAY_HEADER_HEIGHT, MIN_COLUMN_WIDTH, PIXELS_PER_MINUTE, PLANE_HEIGHT, TIME_GUTTER_WIDTH,
    },
};

pub(super) fn render(
    view: &mut CadenceView,
    window: &Window,
    cx: &Context<'_, CadenceView>,
) -> impl IntoElement {
    let viewport_width = window.viewport_size().width.as_f32();
    let available_width = (viewport_width - 48.0 - TIME_GUTTER_WIDTH).max(0.0);
    let plane_width = available_width.max(MIN_COLUMN_WIDTH * 7.0);
    let column_width = plane_width / 7.0;
    if !view.scroll_initialized {
        let initial = view.initial_scroll_offset(column_width);
        view.scroll_handle
            .set_offset(point(px(-initial.0), px(-initial.1)));
        view.scroll_initialized = true;
    }
    let scroll_offset = view.scroll_handle.offset();

    let view_id = cx.entity_id();
    let body = div()
        .id("week-plane-scroll")
        .absolute()
        .top(px(DAY_HEADER_HEIGHT))
        .left(px(TIME_GUTTER_WIDTH))
        .right(px(0.0))
        .bottom(px(0.0))
        .track_scroll(&view.scroll_handle)
        .overflow_scroll()
        .on_scroll_wheel(move |_, _, cx| cx.notify(view_id))
        .child(grid::render_plane(view, plane_width, column_width, cx));

    let header = render_header(view, plane_width, column_width, scroll_offset, cx);
    let gutter = render_time_gutter(view, scroll_offset, cx);
    let corner = div()
        .absolute()
        .top(px(0.0))
        .left(px(0.0))
        .w(px(TIME_GUTTER_WIDTH))
        .h(px(DAY_HEADER_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .bg(cx.theme().background)
        .border_b_1()
        .border_color(cx.theme().border)
        .child(div().font_medium().child("Time"));

    div()
        .id("week-surface")
        .relative()
        .flex_1()
        .min_h_0()
        .mx_4()
        .mb_4()
        .rounded_lg()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .overflow_hidden()
        .child(body)
        .child(header)
        .child(gutter)
        .child(corner)
        .vertical_scrollbar(&view.scroll_handle)
        .horizontal_scrollbar(&view.scroll_handle)
}

fn render_header(
    view: &CadenceView,
    plane_width: f32,
    column_width: f32,
    scroll_offset: gpui::Point<gpui::Pixels>,
    cx: &Context<'_, CadenceView>,
) -> gpui::AnyElement {
    let Some(snapshot) = &view.snapshot else {
        return div().into_any_element();
    };
    let (today, _) = local_date_time(view.now, &view.settings);
    let days = (0_u8..7).filter_map(|offset| {
        let span = jiff::Span::new().try_days(i64::from(offset)).ok()?;
        snapshot.range.start().checked_add(span).ok()
    });
    let cells = days.map(|date| {
        let is_today = date == today;
        let day_name = date.strftime("%a").to_string();
        let day_number = date.strftime("%-d").to_string();
        div()
            .w(px(column_width))
            .h(px(DAY_HEADER_HEIGHT))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_1()
            .border_l_1()
            .border_color(cx.theme().border.opacity(0.7))
            .when(is_today, |this| {
                this.bg(cx.theme().primary.opacity(if cx.theme().mode.is_dark() {
                    0.08
                } else {
                    0.04
                }))
            })
            .child(div().text_xl().font_semibold().child(day_number))
            .child(
                div()
                    .text_xs()
                    .text_color(if is_today {
                        cx.theme().primary
                    } else {
                        cx.theme().muted_foreground
                    })
                    .child(day_name),
            )
            .child(div().w(px(5.0)).h(px(5.0)).rounded_full().bg(if is_today {
                cx.theme().success
            } else {
                Hsla::transparent_black()
            }))
            .into_any_element()
    });
    div()
        .absolute()
        .top(px(0.0))
        .left(px(TIME_GUTTER_WIDTH))
        .right(px(0.0))
        .h(px(DAY_HEADER_HEIGHT))
        .overflow_hidden()
        .bg(cx.theme().background)
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            div()
                .absolute()
                .top(px(0.0))
                .left(scroll_offset.x)
                .w(px(plane_width))
                .h(px(DAY_HEADER_HEIGHT))
                .flex()
                .children(cells),
        )
        .into_any_element()
}

fn render_time_gutter(
    view: &CadenceView,
    scroll_offset: gpui::Point<gpui::Pixels>,
    cx: &Context<'_, CadenceView>,
) -> gpui::AnyElement {
    let labels = (0_u8..=24).map(|hour| {
        let y = f32::from(hour) * 60.0 * PIXELS_PER_MINUTE;
        let time = Time::constant(i8::try_from(hour % 24).expect("hour fits in i8"), 0, 0, 0);
        div()
            .absolute()
            .top(px((y - 8.0).max(0.0)))
            .right(px(10.0))
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(format_time(time, view.settings.clock_format()))
            .into_any_element()
    });
    div()
        .absolute()
        .top(px(DAY_HEADER_HEIGHT))
        .left(px(0.0))
        .w(px(TIME_GUTTER_WIDTH))
        .bottom(px(0.0))
        .overflow_hidden()
        .bg(cx.theme().background)
        .child(
            div()
                .absolute()
                .top(scroll_offset.y)
                .left(px(0.0))
                .w(px(TIME_GUTTER_WIDTH))
                .h(px(PLANE_HEIGHT))
                .children(labels),
        )
        .into_any_element()
}
