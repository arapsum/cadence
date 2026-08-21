use gpui::{Context, IntoElement, Window, div, prelude::*};
use gpui_component::{ActiveTheme as _, StyledExt as _};

use super::{presentation::local_date_time, state::CadenceView, surface};

pub(super) fn render(
    view: &mut CadenceView,
    window: &Window,
    cx: &mut Context<'_, CadenceView>,
) -> impl IntoElement {
    let (today, now_time) = local_date_time(view.now, &view.settings);
    let selected_date = view.state.selected_date();
    let summary = view.snapshot.as_ref().map(|snapshot| {
        let mut current = snapshot
            .events
            .iter()
            .filter(|event| {
                event.date() == selected_date
                    && selected_date == today
                    && event.start_time() <= now_time
                    && now_time < event.end_time()
            })
            .collect::<Vec<_>>();
        current.sort_by_key(|event| (event.end_time(), event.title()));
        let next = snapshot
            .events
            .iter()
            .filter(|event| event.date() == selected_date && event.start_time() > now_time)
            .min_by_key(|event| (event.start_time(), event.end_time()));
        (current, next)
    });

    div()
        .v_flex()
        .flex_1()
        .min_h_0()
        .child(render_guidance(selected_date == today, summary, cx))
        .child(surface::render(view, window, surface::SurfaceMode::Day, cx))
}

fn render_guidance(
    is_today: bool,
    summary: Option<(
        Vec<&crate::domain::EventOccurrence>,
        Option<&crate::domain::EventOccurrence>,
    )>,
    cx: &Context<'_, CadenceView>,
) -> impl IntoElement {
    let (headline, detail) = if !is_today {
        (
            "Live guidance follows Today",
            "Return to Today to see what is next.".to_owned(),
        )
    } else if let Some((current, next)) = summary {
        current.first().map_or_else(
            || {
                let detail = next.map_or_else(
                    || "Clear for the rest of the day.".to_owned(),
                    |event| format!("Free now · Next: {}", event.title()),
                );
                ("Free now", detail)
            },
            |event| {
                let extra = current.len().saturating_sub(1);
                let detail = if extra == 0 {
                    format!("Now: {}", event.title())
                } else {
                    format!(
                        "Now: {} and {extra} overlapping event{}",
                        event.title(),
                        if extra == 1 { "" } else { "s" }
                    )
                };
                let next = next.map_or_else(
                    || "Clear for the rest of the day.".to_owned(),
                    |event| format!("Next: {}", event.title()),
                );
                ("On your plan", format!("{detail} · {next}"))
            },
        )
    } else {
        ("Free now", "No events are visible for this day.".to_owned())
    };

    div()
        .mx_4()
        .mb_3()
        .px_3()
        .py_2()
        .flex()
        .items_center()
        .gap_3()
        .rounded_md()
        .bg(cx.theme().secondary)
        .child(div().text_sm().font_medium().child(headline))
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(detail),
        )
}
