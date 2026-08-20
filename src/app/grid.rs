use std::collections::HashMap;

use gpui::{Context, IntoElement, div, prelude::*, px};
use gpui_component::ActiveTheme as _;

use crate::{calendar::CategoryFilter, domain::time_to_offset};

use super::{
    event_card,
    presentation::{day_index, local_date_time},
    state::CadenceView,
    style::{PIXELS_PER_MINUTE, PLANE_HEIGHT},
};

pub(crate) fn render_plane(
    view: &CadenceView,
    plane_width: f32,
    column_width: f32,
    cx: &mut Context<CadenceView>,
) -> impl IntoElement {
    let horizontal_lines = (0..=48)
        .map(|line| {
            let y = line as f32 * 30.0 * PIXELS_PER_MINUTE;
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
        })
        .collect::<Vec<_>>();
    let vertical_lines = (0..=7)
        .map(|line| {
            div()
                .absolute()
                .top(px(0.0))
                .left(px(line as f32 * column_width))
                .w(px(if line == 0 || line == 7 { 1.0 } else { 0.5 }))
                .h(px(PLANE_HEIGHT))
                .bg(cx.theme().border.opacity(0.65))
                .into_any_element()
        })
        .collect::<Vec<_>>();
    let (today, current_time) = local_date_time(view.now, &view.settings);
    let today_offset = view
        .snapshot
        .as_ref()
        .and_then(|snapshot| day_index(snapshot.range, today));
    let current_line = today_offset.and_then(|day| {
        let y = time_to_offset(current_time, PIXELS_PER_MINUTE).ok()?;
        Some(
            div()
                .absolute()
                .top(px(y))
                .left(px(day as f32 * column_width))
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
    });

    let empty_border = cx.theme().border.opacity(0.65);
    let empty_foreground = cx.theme().muted_foreground.opacity(0.7);
    let empty_slots = view
        .snapshot
        .as_ref()
        .map(|snapshot| {
            (0..7)
                .flat_map(|day| {
                    (6..22).filter_map(move |hour| {
                        let has_event = snapshot.events.iter().any(|event| {
                            let start_minutes = event.start_time().hour() as i32 * 60
                                + event.start_time().minute() as i32;
                            let end_minutes = event.end_time().hour() as i32 * 60
                                + event.end_time().minute() as i32;
                            day_index(snapshot.range, event.date()) == Some(day)
                                && start_minutes < hour * 60 + 60
                                && end_minutes > hour * 60
                        });
                        if has_event {
                            return None;
                        }
                        Some(
                            div()
                                .absolute()
                                .top(px(hour as f32 * 60.0 * PIXELS_PER_MINUTE + 4.0))
                                .left(px(day as f32 * column_width + 4.0))
                                .w(px((column_width - 8.0).max(24.0)))
                                .h(px(60.0 * PIXELS_PER_MINUTE - 8.0))
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
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut children = horizontal_lines;
    children.extend(vertical_lines);
    children.extend(empty_slots);
    if let Some(current_line) = current_line {
        children.push(current_line);
    }

    if let Some(snapshot) = &view.snapshot {
        let categories = snapshot
            .categories
            .iter()
            .map(|category| (category.id(), category))
            .collect::<HashMap<_, _>>();
        let positions = snapshot
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
                    cx,
                ))
            })
            .collect::<Vec<_>>();
        children.extend(positions);

        if snapshot.events.is_empty() {
            let message = match view.state.category_filter() {
                CategoryFilter::All => "Nothing scheduled this week",
                CategoryFilter::Only(_) => "No events in this category this week",
            };
            children.push(
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
    }

    div()
        .id("week-plane")
        .relative()
        .w(px(plane_width))
        .h(px(PLANE_HEIGHT))
        .bg(cx.theme().background)
        .on_click({
            let view = cx.entity().downgrade();
            move |_, _, app| {
                view.update(app, |this, cx| this.clear_selection(cx)).ok();
            }
        })
        .children(children)
}
