use gpui::{Context, ElementId, IntoElement, StatefulInteractiveElement as _, div, prelude::*, px};
use gpui_component::{ActiveTheme as _, StyledExt as _, tooltip::Tooltip};

use crate::{
    calendar::PositionedEvent,
    domain::{Category, Event, format_time},
};

use super::{state::CadenceView, style::category_palette};

pub(crate) fn render(
    view: &CadenceView,
    event: &Event,
    category: &Category,
    position: PositionedEvent,
    column_width: f32,
    cx: &mut Context<CadenceView>,
) -> gpui::AnyElement {
    let selected = view.state.selected_event() == Some(event.id());
    let dark = cx.theme().mode.is_dark();
    let (background, foreground, border) = category_palette(category.color_token(), dark);
    let lane_count = position.lane_count().max(1) as f32;
    let width = column_width * position.lane_span() as f32 / lane_count - 8.0;
    let left = position.day_offset() as f32 * column_width
        + column_width * position.lane() as f32 / lane_count
        + 4.0;
    let event_id = event.id();
    let title = event.title().to_owned();
    let category_name = category.name().to_owned();
    let event_time = format!(
        "{} – {}",
        format_time(event.start_time(), view.settings.clock_format()),
        format_time(event.end_time(), view.settings.clock_format())
    );
    let tooltip_text = match event.notes() {
        Some(notes) => format!("{title}\n{category_name} · {event_time}\n{notes}"),
        None => format!("{title}\n{category_name} · {event_time}"),
    };
    let compact = position.height() < 42.0;
    let tall = position.height() >= 68.0;
    let roomy = position.height() >= 96.0;
    let view = cx.entity().downgrade();
    div()
        .id(ElementId::NamedInteger(
            "event-card".into(),
            event_id.as_uuid().as_u128() as u64,
        ))
        .absolute()
        .top(px(position.top() + 4.0))
        .left(px(left))
        .w(px(width.max(38.0)))
        .h(px((position.height() - 8.0).max(18.0)))
        .v_flex()
        .when(compact, |this| this.px_1().py_0().gap_0())
        .when(!compact && !roomy, |this| this.p_1().gap_1())
        .when(roomy, |this| this.p_2().gap_1())
        .rounded_md()
        .border_1()
        .border_color(if selected {
            cx.theme().foreground
        } else {
            border
        })
        .when(selected, |this| this.border_2())
        .bg(background)
        .text_color(foreground)
        .overflow_hidden()
        .cursor_pointer()
        .tab_index(0)
        .focus(|this| this.border_color(cx.theme().foreground))
        .hover(|this| this.opacity(0.92))
        .tooltip(move |window, cx| Tooltip::new(tooltip_text.clone()).build(window, cx))
        .on_click(move |_, _, app| {
            app.stop_propagation();
            view.update(app, |this, cx| this.select_event(event_id, cx))
                .ok();
        })
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .text_xs()
                .font_medium()
                .child(
                    div()
                        .rounded_full()
                        .px_1()
                        .bg(foreground.opacity(0.16))
                        .child(category_name),
                )
                .child(
                    div()
                        .w(px(4.0))
                        .h(px(4.0))
                        .rounded_full()
                        .bg(foreground.opacity(0.65)),
                ),
        )
        .when(tall, |this| {
            this.child(
                div()
                    .text_sm()
                    .font_medium()
                    .when(roomy, |this| this.line_clamp(2))
                    .when(!roomy, |this| this.truncate())
                    .child(title.clone()),
            )
            .child(div().text_xs().opacity(0.78).child(event_time.clone()))
        })
        .when(!compact && !tall, |this| {
            this.child(div().text_xs().font_medium().truncate().child(title))
        })
        .into_any_element()
}
