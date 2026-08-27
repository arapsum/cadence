use gpui::{
    Context, DragMoveEvent, Hsla, IntoElement, KeyDownEvent, MouseButton, Window, div, prelude::*,
    px,
};
use gpui_component::scroll::ScrollableElement as _;
use gpui_component::{ActiveTheme as _, ElementExt as _, StyledExt as _};
use jiff::civil::{Date, Time};

use crate::{calendar::CalendarViewMode, domain::format_time};

use super::{
    actions, grid,
    presentation::{dates_in_range, local_date_time},
    state::CadenceView,
    style::{
        DAY_HEADER_HEIGHT, MIN_COLUMN_WIDTH, PIXELS_PER_MINUTE, PLANE_HEIGHT, TIME_GUTTER_WIDTH,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SurfaceMode {
    Day,
    Week,
}

impl SurfaceMode {
    pub(super) const fn calendar_mode(self) -> CalendarViewMode {
        match self {
            Self::Day => CalendarViewMode::Day,
            Self::Week => CalendarViewMode::Week,
        }
    }

    pub(super) const fn key(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
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
    let calendar_mode = mode.calendar_mode();
    let available_width = (view.surface_width(calendar_mode) - TIME_GUTTER_WIDTH).max(24.0);
    let column_count = match mode {
        SurfaceMode::Day => 1,
        SurfaceMode::Week => view
            .surface_snapshot(calendar_mode)
            .map_or(super::state::WEEK_VISIBLE_DAYS, |snapshot| {
                super::presentation::dates_in_range(snapshot.range).len()
            }),
    };
    let visible_width = if mode == SurfaceMode::Week {
        available_width.max(
            MIN_COLUMN_WIDTH
                * f32::from(
                    u16::try_from(super::state::WEEK_VISIBLE_DAYS)
                        .expect("visible week columns fit in u16"),
                ),
        )
    } else {
        available_width
    };
    let visible_columns = f32::from(
        u16::try_from(if mode == SurfaceMode::Week {
            super::state::WEEK_VISIBLE_DAYS
        } else {
            1
        })
        .expect("surface columns fit in u16"),
    );
    let column_width = visible_width / visible_columns;
    let plane_width =
        column_width * f32::from(u16::try_from(column_count).expect("surface columns fit in u16"));

    if view.viewport(calendar_mode).initialization == super::state::ScrollInitialization::Pending {
        let initial = view.initial_scroll_offset(calendar_mode, column_width);
        // Prime the handle immediately so the first painted frame keeps the
        // selected week header in view. The deferred pass below still runs
        // after layout to apply the same offset once the scroll bounds exist.
        view.viewport_mut(calendar_mode)
            .handle
            .set_offset(gpui::point(gpui::px(-initial.0), gpui::px(-initial.1)));
        view.viewport_mut(calendar_mode).initialization =
            super::state::ScrollInitialization::Scheduled;
        let scroll_view = cx.entity().downgrade();
        window.defer(cx, move |_, cx| {
            scroll_view
                .update(cx, |view, _| {
                    let measured_column_width = if calendar_mode == CalendarViewMode::Week {
                        let available_width =
                            (view.week_surface_width - TIME_GUTTER_WIDTH).max(24.0);
                        available_width.max(
                            MIN_COLUMN_WIDTH
                                * f32::from(
                                    u16::try_from(super::state::WEEK_VISIBLE_DAYS)
                                        .expect("visible week columns fit in u16"),
                                ),
                        ) / f32::from(
                            u16::try_from(super::state::WEEK_VISIBLE_DAYS)
                                .expect("visible week columns fit in u16"),
                        )
                    } else {
                        (view.day_surface_width - TIME_GUTTER_WIDTH).max(24.0)
                    };
                    view.initialize_scroll(
                        calendar_mode,
                        view.initial_scroll_offset(calendar_mode, measured_column_width),
                    );
                })
                .ok();
        });
    }

    if mode == SurfaceMode::Week
        && view.viewport(calendar_mode).initialization
            == super::state::ScrollInitialization::Initialized
    {
        view.schedule_week_scroll_sync(window, column_width, cx);
    }

    let scroll_handle = view.viewport(calendar_mode).handle.clone();
    let scroll_offset = scroll_handle.offset();
    let update_view = cx.entity().downgrade();
    let drop_view = update_view.clone();
    let cancel_view = update_view.clone();
    let measure_view = update_view.clone();
    let body = div()
        .id(format!("{}-calendar-plane-scroll", mode.key()))
        .absolute()
        .top(px(DAY_HEADER_HEIGHT))
        .left(px(TIME_GUTTER_WIDTH))
        .right(px(0.0))
        .bottom(px(0.0))
        .track_scroll(&scroll_handle)
        .overflow_scroll()
        .on_drag_move(
            move |event: &DragMoveEvent<super::interaction::DragPayload>, _, app| {
                update_view
                    .update(app, |view, cx| {
                        view.update_manipulation(
                            event,
                            calendar_mode,
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
                if payload.surface != calendar_mode {
                    return;
                }
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
    let gutter = render_time_gutter(view, mode, scroll_offset, cx);
    let week_focus = view.week_viewport_focus.clone();
    let focus_week_surface = cx.listener(|view, _, window, cx| {
        view.week_viewport_focus.focus(window, cx);
    });
    let slide_week_backward =
        cx.listener(|view, _: &actions::SlideWeekBackward, _, cx| view.slide_week_window(-1, cx));
    let slide_week_forward =
        cx.listener(|view, _: &actions::SlideWeekForward, _, cx| view.slide_week_window(1, cx));
    let scroll_week_down =
        cx.listener(|view, _: &actions::ScrollWeekDown, _, cx| view.scroll_week_by_hours(1, cx));
    let scroll_week_up =
        cx.listener(|view, _: &actions::ScrollWeekUp, _, cx| view.scroll_week_by_hours(-1, cx));
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
        .border_color(cx.theme().border.opacity(0.72))
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(cx.theme().muted_foreground)
                .child("Time"),
        );

    div()
        .id(format!("{}-calendar-surface", mode.key()))
        .when(mode == SurfaceMode::Day, |this| {
            this.debug_selector(|| "day-calendar-surface".into())
        })
        .relative()
        .flex_1()
        .min_h_0()
        .bg(cx.theme().background)
        .overflow_hidden()
        .when(mode == SurfaceMode::Week, |this| {
            this.key_context(actions::WEEK_VIEWPORT_CONTEXT)
                .track_focus(&week_focus)
                .on_click(focus_week_surface)
                .on_action(slide_week_backward)
                .on_action(slide_week_forward)
                .on_action(scroll_week_down)
                .on_action(scroll_week_up)
        })
        .on_prepaint(move |bounds, _, app| {
            measure_view
                .update(app, |view, cx| {
                    view.set_surface_width(calendar_mode, bounds.size.width.as_f32(), cx);
                })
                .ok();
        })
        .child(body)
        .child(header)
        .child(gutter)
        .child(corner)
        .vertical_scrollbar(&scroll_handle)
        .when(mode.has_horizontal_scroll(), |this| {
            this.horizontal_scrollbar(&scroll_handle)
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
    let Some(snapshot) = view.surface_snapshot(mode.calendar_mode()) else {
        return div().into_any_element();
    };
    let (today, _) = local_date_time(view.now, &view.settings);
    let selected_date = view.state.selected_date();
    let dates = dates_in_range(snapshot.range);
    let owner = cx.entity().downgrade();
    let cells = dates.into_iter().map(move |date| {
        let day_name = if mode == SurfaceMode::Day {
            date.strftime("%A").to_string()
        } else {
            date.strftime("%a").to_string()
        };
        render_header_cell(
            HeaderCell {
                mode,
                date,
                is_today: date == today,
                is_selected: date == selected_date,
                day_name,
                day_number: date.strftime("%-d").to_string(),
                month: date.strftime("%B %Y").to_string(),
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
        .border_color(cx.theme().border.opacity(0.72))
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
    mode: SurfaceMode,
    date: Date,
    is_today: bool,
    is_selected: bool,
    day_name: String,
    day_number: String,
    month: String,
    column_width: f32,
    owner: gpui::WeakEntity<CadenceView>,
}

fn render_header_cell(cell: HeaderCell, cx: &Context<'_, CadenceView>) -> gpui::AnyElement {
    let HeaderCell {
        mode,
        date,
        is_today,
        is_selected,
        day_name,
        day_number,
        month,
        column_width,
        owner,
    } = cell;
    let key_owner = owner.clone();
    div()
        .id(format!("{}-calendar-day-header-{date}", mode.key()))
        .when(mode == SurfaceMode::Week, |this| {
            this.role(gpui::Role::Button)
                .aria_label(format!(
                    "Open day plan for {day_name} {day_number}, {month}"
                ))
                .aria_selected(is_selected)
                .tab_index(0)
                .cursor_pointer()
        })
        .w(px(column_width))
        .h(px(DAY_HEADER_HEIGHT))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .border_l_1()
        .border_color(cx.theme().border.opacity(0.52))
        .focus(|this| {
            this.bg(cx.theme().secondary)
                .border_color(cx.theme().primary)
        })
        .when(is_selected && mode == SurfaceMode::Week, |this| {
            this.bg(cx.theme().primary.opacity(if cx.theme().mode.is_dark() {
                0.18
            } else {
                0.08
            }))
            .border_b_2()
            .border_color(cx.theme().primary)
        })
        .when(mode == SurfaceMode::Week && is_selected, |this| {
            this.debug_selector(|| "calendar-day-header".into())
        })
        .child(
            div()
                .text_xs()
                .font_medium()
                .text_color(if is_selected {
                    cx.theme().foreground
                } else {
                    cx.theme().muted_foreground
                })
                .child(day_name),
        )
        .child(
            div()
                .when(mode == SurfaceMode::Day, gpui::Styled::text_3xl)
                .when(mode == SurfaceMode::Week, gpui::Styled::text_lg)
                .font_semibold()
                .text_color(cx.theme().foreground)
                .child(day_number),
        )
        .when(mode == SurfaceMode::Day, |this| {
            this.child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(month),
            )
        })
        .when(mode == SurfaceMode::Week, |this| {
            this.child(div().w(px(5.0)).h(px(5.0)).rounded_full().bg(if is_today {
                cx.theme().success
            } else {
                Hsla::transparent_black()
            }))
        })
        .on_click(move |_, window, app| {
            if mode == SurfaceMode::Week {
                owner
                    .update(app, |this, cx| this.open_day_plan(date, window, cx))
                    .ok();
            }
        })
        .on_key_down(move |event: &KeyDownEvent, window, app| {
            if mode == SurfaceMode::Week
                && matches!(event.keystroke.key.as_str(), "enter" | "return" | "space")
            {
                app.stop_propagation();
                key_owner
                    .update(app, |this, cx| this.open_day_plan(date, window, cx))
                    .ok();
            }
        })
        .into_any_element()
}

fn render_time_gutter(
    view: &CadenceView,
    mode: SurfaceMode,
    scroll_offset: gpui::Point<gpui::Pixels>,
    cx: &Context<'_, CadenceView>,
) -> gpui::AnyElement {
    let labels = (0_u8..=24).map(|hour| {
        let y = f32::from(hour) * 60.0 * PIXELS_PER_MINUTE;
        let time = Time::constant(i8::try_from(hour % 24).expect("hour fits in i8"), 0, 0, 0);
        div()
            .id(format!("{}-time-label-{hour}", mode.key()))
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
