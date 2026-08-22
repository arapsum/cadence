use std::collections::HashMap;

use gpui::{App, Context, IntoElement, KeyDownEvent, div, prelude::*, px};
use gpui_component::{ActiveTheme as _, StyledExt as _};

use crate::{
    calendar::CategoryFilter,
    domain::{format_time, time_to_offset},
};
use jiff::civil::Time;

use super::{
    event_card,
    presentation::{day_index, local_date_time},
    state::CadenceView,
    style::{PIXELS_PER_MINUTE, PLANE_HEIGHT},
    surface::SurfaceMode,
};

pub(super) fn render_plane(
    view: &CadenceView,
    plane_width: f32,
    column_width: f32,
    column_count: usize,
    mode: SurfaceMode,
    cx: &Context<'_, CadenceView>,
) -> impl IntoElement {
    let mut children = render_grid_lines(plane_width, column_width, column_count, cx);
    children.extend(render_empty_slots(
        view,
        column_width,
        column_count,
        mode,
        cx,
    ));
    if let Some(current_line) = render_current_line(view, column_width, mode, cx) {
        children.push(current_line);
    }
    children.extend(render_event_cards(
        view,
        column_width,
        plane_width,
        mode,
        cx,
    ));
    if let Some(preview) = render_manipulation_preview(view, column_width, mode, cx) {
        children.push(preview);
    }

    div()
        .id(format!("{}-calendar-plane", mode.key()))
        .relative()
        .w(px(plane_width))
        .h(px(PLANE_HEIGHT))
        .bg(cx.theme().background)
        .on_click({
            let view = cx.entity().downgrade();
            move |_, _, app| {
                view.update(app, |view, cx| {
                    view.activate_surface(mode.calendar_mode(), cx);
                    view.clear_selection(cx);
                })
                .ok();
            }
        })
        .children(children)
}

fn render_grid_lines(
    plane_width: f32,
    column_width: f32,
    column_count: usize,
    cx: &Context<'_, CadenceView>,
) -> Vec<gpui::AnyElement> {
    let horizontal_lines = (0_u16..=48).map(|line| {
        let y = f32::from(line) * 30.0 * PIXELS_PER_MINUTE;
        div()
            .absolute()
            .top(px(y))
            .left(px(0.0))
            .w(px(plane_width))
            .h(px(if line % 2 == 0 { 1.0 } else { 0.5 }))
            .bg(cx
                .theme()
                .border
                .opacity(if line % 2 == 0 { 0.7 } else { 0.35 }))
            .into_any_element()
    });
    let vertical_lines = (0..=column_count).map(|line| {
        let is_edge = line == 0 || line == column_count;
        let line = f32::from(u16::try_from(line).expect("surface column count fits in u16"));
        div()
            .absolute()
            .top(px(0.0))
            .left(px(line * column_width))
            .w(px(if is_edge { 1.0 } else { 0.5 }))
            .h(px(PLANE_HEIGHT))
            .bg(cx.theme().border.opacity(0.65))
            .into_any_element()
    });
    horizontal_lines.chain(vertical_lines).collect()
}

fn render_current_line(
    view: &CadenceView,
    column_width: f32,
    mode: SurfaceMode,
    cx: &Context<'_, CadenceView>,
) -> Option<gpui::AnyElement> {
    let (today, current_time) = local_date_time(view.now, &view.settings);
    let day = view
        .surface_snapshot(mode.calendar_mode())
        .and_then(|snapshot| day_index(snapshot.range, today))?;
    let y = time_to_offset(current_time, PIXELS_PER_MINUTE).ok()?;
    let day = f32::from(u16::try_from(day).expect("week day fits in u16"));
    Some(
        div()
            .absolute()
            .top(px(y))
            .left(px(day * column_width))
            .w(px(column_width))
            .h(px(1.5))
            .bg(cx.theme().success)
            .child(
                div()
                    .absolute()
                    .left(px(-3.0))
                    .top(px(-2.0))
                    .w(px(6.0))
                    .h(px(6.0))
                    .rounded_full()
                    .bg(cx.theme().success),
            )
            .into_any_element(),
    )
}

#[allow(clippy::too_many_lines)]
fn render_empty_slots(
    view: &CadenceView,
    column_width: f32,
    column_count: usize,
    mode: SurfaceMode,
    cx: &Context<'_, CadenceView>,
) -> Vec<gpui::AnyElement> {
    let empty_border = cx.theme().border.opacity(0.22);
    let empty_foreground = cx.theme().muted_foreground.opacity(0.7);
    view.surface_snapshot(mode.calendar_mode())
        .map(|snapshot| {
            (0..column_count)
                .flat_map(|day| {
                    let day_date = snapshot
                        .range
                        .start()
                        .checked_add(jiff::SignedDuration::from_hours(
                            24 * i64::try_from(day).expect("slot day fits in i64"),
                        ))
                        .ok();
                    (6_u8..22).filter_map(move |hour| {
                        let day_date = day_date?;
                        let day_number = day;
                        let hour = i32::from(hour);
                        let clock_format = view.settings.clock_format();
                        let has_event = snapshot.events.iter().any(|event| {
                            let start_minutes = i32::from(event.start_time().hour()) * 60
                                + i32::from(event.start_time().minute());
                            let end_minutes = i32::from(event.end_time().hour()) * 60
                                + i32::from(event.end_time().minute());
                            day_index(snapshot.range, event.date()) == Some(day_number)
                                && start_minutes < hour * 60 + 60
                                && end_minutes > hour * 60
                        });
                        if has_event {
                            return None;
                        }
                        let view = cx.entity().downgrade();
                        let key_view = view.clone();
                        let slot_time = Time::constant(
                            i8::try_from(hour).expect("slot hour fits in i8"),
                            0,
                            0,
                            0,
                        );
                        let slot_label = format!(
                            "Add event on {day_date} at {}",
                            format_time(slot_time, clock_format),
                        );
                        let top = (f32::from(u16::try_from(hour).expect("hour fits in u16"))
                            * 60.0)
                            .mul_add(PIXELS_PER_MINUTE, 4.0);
                        let day = f32::from(
                            u16::try_from(day).expect("surface column count fits in u16"),
                        );
                        let left = day.mul_add(column_width, 4.0);
                        Some(
                            div()
                                .id(format!("{}-empty-slot-{day_date}-{hour}", mode.key()))
                                .role(gpui::Role::Button)
                                .aria_label(slot_label)
                                .absolute()
                                .top(px(top))
                                .left(px(left))
                                .w(px((column_width - 8.0).max(24.0)))
                                .h(px(60.0_f32.mul_add(PIXELS_PER_MINUTE, -8.0)))
                                .flex()
                                .items_center()
                                .justify_center()
                                .border_1()
                                .border_dashed()
                                .border_color(empty_border)
                                .bg(cx.theme().background.opacity(0.01))
                                .text_color(empty_foreground)
                                .cursor_pointer()
                                .tab_index(0)
                                .focus(|this| {
                                    this.bg(cx.theme().secondary)
                                        .border_color(cx.theme().primary)
                                })
                                .hover(|this| {
                                    this.bg(cx.theme().secondary.opacity(0.7))
                                        .border_color(cx.theme().border.opacity(0.72))
                                })
                                .on_key_down(move |event: &KeyDownEvent, window, app| {
                                    if move_focus(event, window, app) {
                                        return;
                                    }
                                    if matches!(event.keystroke.key.as_str(), "enter" | "return") {
                                        app.stop_propagation();
                                        key_view
                                            .update(app, |view, cx| {
                                                view.activate_surface(mode.calendar_mode(), cx);
                                                view.new_event_at(day_date, slot_time, window, cx);
                                            })
                                            .ok();
                                    }
                                })
                                .on_click(move |_, window, app| {
                                    app.stop_propagation();
                                    view.update(app, |view, cx| {
                                        view.activate_surface(mode.calendar_mode(), cx);
                                        view.new_event_at(day_date, slot_time, window, cx);
                                    })
                                    .ok();
                                })
                                .child("+")
                                .into_any_element(),
                        )
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn move_focus(event: &KeyDownEvent, window: &mut gpui::Window, app: &mut App) -> bool {
    match event.keystroke.key.as_str() {
        "left" | "up" => {
            app.stop_propagation();
            window.focus_prev(app);
            true
        }
        "right" | "down" => {
            app.stop_propagation();
            window.focus_next(app);
            true
        }
        _ => false,
    }
}

fn render_event_cards(
    view: &CadenceView,
    column_width: f32,
    plane_width: f32,
    mode: SurfaceMode,
    cx: &Context<'_, CadenceView>,
) -> Vec<gpui::AnyElement> {
    let Some(snapshot) = view.surface_snapshot(mode.calendar_mode()) else {
        return Vec::new();
    };
    let Some(workspace) = &view.snapshot else {
        return Vec::new();
    };
    let categories = workspace
        .categories
        .iter()
        .map(|category| (category.id(), category))
        .collect::<HashMap<_, _>>();
    let mut cards = snapshot
        .positions
        .iter()
        .filter_map(|position| {
            let event = snapshot
                .events
                .iter()
                .find(|event| event.id() == position.occurrence_id())?;
            let category = categories.get(&event.category_id())?;
            let conflicted = workspace.conflict_ids.contains(&event.id());
            Some(event_card::render(&event_card::EventCardProps {
                view,
                event,
                category,
                conflicted,
                position: *position,
                column_width,
                mode,
                cx,
            }))
        })
        .collect::<Vec<_>>();

    if snapshot.events.is_empty() {
        let message = match view.state.category_filter() {
            CategoryFilter::All => match mode {
                SurfaceMode::Day => "Nothing scheduled this day",
                SurfaceMode::Week => "Nothing scheduled this week",
            },
            CategoryFilter::Only(_) => match mode {
                SurfaceMode::Day => "No events in this category today",
                SurfaceMode::Week => "No events in this category this week",
            },
        };
        cards.push(
            div()
                .absolute()
                .top(px(120.0))
                .left(px(0.0))
                .w(px(plane_width))
                .flex()
                .justify_center()
                .text_color(cx.theme().muted_foreground)
                .child(message)
                .into_any_element(),
        );
    }
    cards
}

fn render_manipulation_preview(
    view: &CadenceView,
    column_width: f32,
    mode: SurfaceMode,
    cx: &Context<'_, CadenceView>,
) -> Option<gpui::AnyElement> {
    let manipulation = view.manipulation.as_ref()?;
    if manipulation.surface() != mode.calendar_mode() {
        return None;
    }
    let snapshot = view.surface_snapshot(mode.calendar_mode())?;
    let position = snapshot
        .positions
        .iter()
        .find(|position| position.occurrence_id() == manipulation.occurrence_id())?;
    let category = view
        .snapshot
        .as_ref()?
        .categories
        .iter()
        .find(|category| category.id() == manipulation.event.category_id())?;
    let dark = cx.theme().mode.is_dark();
    let (background, foreground, border) =
        super::style::category_palette(category.color_token(), dark);
    let conflict = manipulation.conflict.is_some();
    let lane_count = f32::from(position.lane_count().max(1));
    let width = column_width * f32::from(position.lane_span()) / lane_count - 8.0;
    let lane = f32::from(position.lane());
    let day = f32::from(u16::try_from(manipulation.target_day()).ok()?);
    let left = day.mul_add(column_width, column_width * lane / lane_count + 4.0);
    let top = time_to_offset(manipulation.proposed.start_time, PIXELS_PER_MINUTE).ok()? + 4.0;
    let bottom = time_to_offset(manipulation.proposed.end_time, PIXELS_PER_MINUTE).ok()?;
    let height = (bottom - top + 4.0).max(18.0);
    let event_time = format!(
        "{} – {}",
        format_time(
            manipulation.proposed.start_time,
            view.settings.clock_format(),
        ),
        format_time(manipulation.proposed.end_time, view.settings.clock_format())
    );
    Some(
        div()
            .id(format!("{}-calendar-manipulation-preview", mode.key()))
            .absolute()
            .top(px(top))
            .left(px(left))
            .w(px(width.max(38.0)))
            .h(px(height))
            .v_flex()
            .gap_1()
            .rounded_md()
            .border_2()
            .border_dashed()
            .border_color(if conflict { cx.theme().warning } else { border })
            .bg(if conflict {
                cx.theme().warning.opacity(0.2)
            } else {
                background.opacity(0.28)
            })
            .text_color(foreground)
            .px_2()
            .py_1()
            .overflow_hidden()
            .child(div().text_xs().font_medium().child(if conflict {
                format!("Conflict: {}", manipulation.event.title())
            } else {
                manipulation.event.title().to_owned()
            }))
            .child(div().text_xs().opacity(0.8).child(event_time))
            .into_any_element(),
    )
}
