use gpui::{
    Context, DragMoveEvent, Hsla, IntoElement, KeyDownEvent, MouseButton, Window, div, prelude::*,
    px,
};
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::{ActiveTheme as _, StyledExt as _};
use jiff::civil::{Date, Time};

use crate::domain::format_time;

use super::{
    grid,
    presentation::{dates_in_range, local_date_time},
    state::CadenceView,
    style::{
        DAY_HEADER_HEIGHT, MIN_COLUMN_WIDTH, PIXELS_PER_MINUTE, PLANE_HEIGHT, TIME_GUTTER_WIDTH,
    },
};

#[derive(Clone, Copy)]
pub(super) enum SurfaceMode {
    Day,
    Week,
}

impl SurfaceMode {
    const fn column_count(self) -> usize {
        match self {
            Self::Day => 1,
            Self::Week => 7,
        }
    }

    const fn has_horizontal_scroll(self) -> bool {
        matches!(self, Self::Week)
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn render(
    view: &mut CadenceView,
    window: &Window,
    mode: SurfaceMode,
    cx: &mut Context<'_, CadenceView>,
) -> impl IntoElement {
    let viewport_width = window.viewport_size().width.as_f32();
    let available_width = (viewport_width - 48.0 - TIME_GUTTER_WIDTH).max(0.0);
    let column_count = mode.column_count();
    let plane_width = if matches!(mode, SurfaceMode::Week) {
        available_width.max(MIN_COLUMN_WIDTH * 7.0)
    } else {
        available_width.max(24.0)
    };
    let column_width =
        plane_width / f32::from(u16::try_from(column_count).expect("surface columns fit in u16"));

    if matches!(
        view.scroll_initialization,
        super::state::ScrollInitialization::Pending
    ) {
        let initial = view.initial_scroll_offset(column_width);
        view.scroll_initialization = super::state::ScrollInitialization::Scheduled;
        let scroll_view = cx.entity().downgrade();
        window.defer(cx, move |_, cx| {
            scroll_view
                .update(cx, |view, _| view.initialize_scroll(initial))
                .ok();
        });
    }
    let scroll_offset = view.scroll_handle.offset();
    let view_id = cx.entity_id();
    let update_view = cx.entity().downgrade();
    let drop_view = update_view.clone();
    let cancel_view = update_view.clone();
    let body = div()
        .id("calendar-plane-scroll")
        .absolute()
        .top(px(DAY_HEADER_HEIGHT))
        .left(px(TIME_GUTTER_WIDTH))
        .right(px(0.0))
        .bottom(px(0.0))
        .track_scroll(&view.scroll_handle)
        .overflow_scroll()
        .on_scroll_wheel(move |_, _, cx| cx.notify(view_id))
        .on_drag_move(
            move |event: &DragMoveEvent<super::interaction::DragPayload>, _, app| {
                update_view
                    .update(app, |view, cx| {
                        view.update_manipulation(
                            event,
                            column_width,
                            plane_width,
                            column_count,
                            cx,
                        );
                    })
                    .ok();
            },
        )
        .on_drop(
            move |payload: &super::interaction::DragPayload, window, app| {
                drop_view
                    .update(app, |view, cx| {
                        view.finish_manipulation(payload, window, cx);
                    })
                    .ok();
            },
        )
        .on_mouse_up_out(MouseButton::Left, move |_, window, app| {
            cancel_view
                .update(app, |view, cx| view.cancel_manipulation(window, cx))
                .ok();
        })
        .child(grid::render_plane(
            view,
            plane_width,
            column_width,
            column_count,
            mode,
            cx,
        ));
    let header = render_header(view, mode, plane_width, column_width, scroll_offset, cx);
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
        .id("calendar-surface")
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
        .when(mode.has_horizontal_scroll(), |this| {
            this.horizontal_scrollbar(&view.scroll_handle)
        })
}

fn render_header(
    view: &CadenceView,
    mode: SurfaceMode,
    plane_width: f32,
    column_width: f32,
    scroll_offset: gpui::Point<gpui::Pixels>,
    cx: &Context<'_, CadenceView>,
) -> gpui::AnyElement {
    let Some(snapshot) = &view.snapshot else {
        return div().into_any_element();
    };
    let (today, _) = local_date_time(view.now, &view.settings);
    let selected_date = view.state.selected_date();
    let dates = dates_in_range(snapshot.range);
    let owner = cx.entity().downgrade();
    let cells = dates.into_iter().map(move |date| {
        let day_name = if matches!(mode, SurfaceMode::Day) {
            date.strftime("%A").to_string()
        } else {
            date.strftime("%a").to_string()
        };
        render_header_cell(
            HeaderCell {
                date,
                is_today: date == today,
                is_selected: date == selected_date,
                day_name,
                day_number: date.strftime("%-d").to_string(),
                column_width,
                owner: owner.clone(),
            },
            cx,
        )
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

struct HeaderCell {
    date: Date,
    is_today: bool,
    is_selected: bool,
    day_name: String,
    day_number: String,
    column_width: f32,
    owner: gpui::WeakEntity<CadenceView>,
}

fn render_header_cell(cell: HeaderCell, cx: &Context<'_, CadenceView>) -> gpui::AnyElement {
    let HeaderCell {
        date,
        is_today,
        is_selected,
        day_name,
        day_number,
        column_width,
        owner,
    } = cell;
    let key_owner = owner.clone();
    div()
        .id(format!("calendar-day-header-{date}"))
        .role(gpui::Role::Button)
        .aria_label(format!("{day_name} {day_number}"))
        .aria_selected(is_selected)
        .tab_index(0)
        .cursor_pointer()
        .w(px(column_width))
        .h(px(DAY_HEADER_HEIGHT))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .border_l_1()
        .border_color(if is_selected {
            cx.theme().primary
        } else {
            cx.theme().border.opacity(0.7)
        })
        .when(is_today, |this| {
            this.bg(cx.theme().primary.opacity(if cx.theme().mode.is_dark() {
                0.08
            } else {
                0.04
            }))
        })
        .when(is_selected, |this| {
            this.border_b_2().border_color(cx.theme().primary)
        })
        .on_click(move |_, _, app| {
            owner
                .update(app, |this, cx| this.select_date(date, cx))
                .ok();
        })
        .on_key_down(move |event: &KeyDownEvent, _, app| {
            if matches!(event.keystroke.key.as_str(), "enter" | "return" | "space") {
                app.stop_propagation();
                key_owner
                    .update(app, |this, cx| this.select_date(date, cx))
                    .ok();
            }
        })
        .child(
            div()
                .text_xl()
                .font_semibold()
                .text_color(if is_selected {
                    cx.theme().primary
                } else {
                    cx.theme().foreground
                })
                .child(day_number),
        )
        .child(
            div()
                .text_xs()
                .text_color(if is_today || is_selected {
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
