use std::collections::HashMap;

use gpui::{Context, IntoElement, div, prelude::*, px};
use gpui_component::ActiveTheme as _;

use crate::{calendar::CategoryFilter, domain::time_to_offset};

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
    children.extend(render_empty_slots(view, column_width, column_count, cx));
    if let Some(current_line) = render_current_line(view, column_width, cx) {
        children.push(current_line);
    }
    children.extend(render_event_cards(
        view,
        column_width,
        plane_width,
        mode,
        cx,
    ));

    div()
        .id("calendar-plane")
        .relative()
        .w(px(plane_width))
        .h(px(PLANE_HEIGHT))
        .bg(cx.theme().background)
        .on_click({
            let view = cx.entity().downgrade();
            move |_, _, app| {
                view.update(app, CadenceView::clear_selection).ok();
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
    cx: &Context<'_, CadenceView>,
) -> Option<gpui::AnyElement> {
    let (today, current_time) = local_date_time(view.now, &view.settings);
    let day = view
        .snapshot
        .as_ref()
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

fn render_empty_slots(
    view: &CadenceView,
    column_width: f32,
    column_count: usize,
    cx: &Context<'_, CadenceView>,
) -> Vec<gpui::AnyElement> {
    let empty_border = cx.theme().border.opacity(0.65);
    let empty_foreground = cx.theme().muted_foreground.opacity(0.7);
    view.snapshot
        .as_ref()
        .map(|snapshot| {
            (0..column_count)
                .flat_map(|day| {
                    (6_u8..22).filter_map(move |hour| {
                        let day_number = day;
                        let hour = i32::from(hour);
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
                        let top = (f32::from(u16::try_from(hour).expect("hour fits in u16"))
                            * 60.0)
                            .mul_add(PIXELS_PER_MINUTE, 4.0);
                        let day = f32::from(
                            u16::try_from(day).expect("surface column count fits in u16"),
                        );
                        let left = day.mul_add(column_width, 4.0);
                        Some(
                            div()
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
                                .text_color(empty_foreground)
                                .child("+")
                                .into_any_element(),
                        )
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn render_event_cards(
    view: &CadenceView,
    column_width: f32,
    plane_width: f32,
    mode: SurfaceMode,
    cx: &Context<'_, CadenceView>,
) -> Vec<gpui::AnyElement> {
    let Some(snapshot) = &view.snapshot else {
        return Vec::new();
    };
    let categories = snapshot
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
                .find(|event| event.id() == position.event_id())?;
            let category = categories.get(&event.category_id())?;
            Some(event_card::render(
                view,
                event,
                category,
                *position,
                column_width,
                mode,
                cx,
            ))
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
