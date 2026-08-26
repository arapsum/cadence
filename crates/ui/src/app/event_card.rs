use gpui::{
    App, Context, Hsla, IntoElement, KeyDownEvent, MouseButton, Pixels, Point, Render,
    StatefulInteractiveElement as _, Window, div, prelude::*, px,
};
use gpui_component::{ActiveTheme as _, StyledExt as _, tooltip::Tooltip};

use crate::{
    calendar::PositionedEvent,
    domain::{Category, EventOccurrence, format_time},
};

use super::{
    interaction::{DragPayload, ManipulationKind},
    state::CadenceView,
    style::category_palette,
    surface::SurfaceMode,
};

#[allow(clippy::too_many_lines)]
pub(super) struct EventCardProps<'a> {
    pub(super) view: &'a CadenceView,
    pub(super) event: &'a EventOccurrence,
    pub(super) category: &'a Category,
    pub(super) conflicted: bool,
    pub(super) position: PositionedEvent,
    pub(super) column_width: f32,
    pub(super) mode: SurfaceMode,
    pub(super) cx: &'a Context<'a, CadenceView>,
}

#[allow(clippy::too_many_lines)]
pub(super) fn render(props: &EventCardProps<'_>) -> gpui::AnyElement {
    let EventCardProps {
        view,
        event,
        category,
        conflicted,
        position,
        column_width,
        mode,
        cx,
    } = *props;
    let bulk_mode = view.is_bulk_selecting();
    let bulk_selectable = view.bulk_selection_surface() == Some(mode.calendar_mode())
        && view.state.view_mode() == mode.calendar_mode();
    let selected = if bulk_mode {
        view.is_bulk_selected(event.id())
    } else {
        view.state.selected_event() == Some(event.id())
    };
    let dark = cx.theme().mode.is_dark();
    let (background, foreground, border) = category_palette(category.color_token(), dark);
    let lane_count = f32::from(position.lane_count().max(1));
    let width = column_width * f32::from(position.lane_span()) / lane_count - 8.0;
    let day_offset = f32::from(position.day_offset());
    let lane = f32::from(position.lane());
    let left = day_offset.mul_add(column_width, column_width * lane / lane_count + 4.0);
    let occurrence_id = event.id();
    let title = event.title().to_owned();
    let category_name = category.name().to_owned();
    let event_time = format!(
        "{} – {}",
        format_time(event.start_time(), view.settings.clock_format()),
        format_time(event.end_time(), view.settings.clock_format())
    );
    let tooltip_text = event.notes().map_or_else(
        || format!("{title}\n{category_name} · {event_time}"),
        |notes| format!("{title}\n{category_name} · {event_time}\n{notes}"),
    );
    let tooltip_text = if conflicted {
        format!("{tooltip_text}\nWarning: overlaps another event")
    } else {
        tooltip_text
    };
    let element_key = format!("{occurrence_id:?}");
    let event_date = event.date();
    let test_card = matches!(mode, SurfaceMode::Day)
        && event_date == view.state.selected_date()
        && event.start_time().hour() == 7
        && event.start_time().minute() == 30;
    let accessibility_label = format!("{title}, {category_name}, {event_date}, {event_time}");
    let compact = position.height() < 42.0;
    let tall = position.height() >= 68.0;
    let roomy = position.height() >= 96.0;
    let show_notes = matches!(mode, SurfaceMode::Day) && position.height() >= 112.0;
    let notes = event.notes().filter(|_| show_notes).map(str::to_owned);
    let details = render_details(title, &event_time, compact, tall, roomy, notes);
    let state = view;
    let view = cx.entity().downgrade();
    let key_view = view.clone();
    let select_view = view.clone();
    let drag_view = view.clone();
    let resize_start_view = view.clone();
    let resize_end_view = view.clone();
    let range_start = state
        .surface_snapshot(mode.calendar_mode())
        .map_or(event_date, |snapshot| snapshot.range.start());
    let move_payload = DragPayload {
        surface: mode.calendar_mode(),
        occurrence_id,
        kind: ManipulationKind::Move,
        original_day: position.day_offset(),
        range_start,
    };
    let resize_start_payload = DragPayload {
        surface: mode.calendar_mode(),
        occurrence_id,
        kind: ManipulationKind::Resize(crate::calendar::ResizeEdge::Start),
        original_day: position.day_offset(),
        range_start,
    };
    let resize_end_payload = DragPayload {
        surface: mode.calendar_mode(),
        occurrence_id,
        kind: ManipulationKind::Resize(crate::calendar::ResizeEdge::End),
        original_day: position.day_offset(),
        range_start,
    };
    let avatar_title = event.title().to_owned();
    let avatar_time = event_time.clone();
    let avatar_background = background;
    let avatar_foreground = foreground;
    let active = state.manipulation.as_ref().is_some_and(|manipulation| {
        manipulation.occurrence_id() == occurrence_id
            && manipulation.surface() == mode.calendar_mode()
    });
    div()
        .id(format!("{}-event-card-{element_key}", mode.key()))
        .role(gpui::Role::Button)
        .aria_label(accessibility_label)
        .aria_selected(selected)
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
        .border_color(if conflicted {
            cx.theme().warning
        } else if selected {
            cx.theme().foreground
        } else {
            border
        })
        .when(selected, gpui::Styled::border_2)
        .bg(background)
        .text_color(foreground)
        .overflow_hidden()
        .when(bulk_mode, gpui::Styled::cursor_pointer)
        .when(!bulk_mode, gpui::Styled::cursor_grab)
        .when(bulk_mode && !bulk_selectable, |this| {
            this.opacity(0.68).cursor_default()
        })
        .when(active, |this| this.opacity(0.34))
        .tab_index(if !bulk_mode || bulk_selectable {
            0isize
        } else {
            -1isize
        })
        .focus(|this| this.border_color(cx.theme().foreground))
        .hover(|this| this.opacity(0.92))
        .when(test_card, |this| {
            this.debug_selector(|| "calendar-event-card".into())
        })
        .tooltip(move |window, cx| Tooltip::new(tooltip_text.clone()).build(window, cx))
        .on_key_down(
            move |event: &KeyDownEvent, window, app| match event.keystroke.key.as_str() {
                "enter" | "return" | "space" if bulk_mode => {
                    if !bulk_selectable {
                        return;
                    }
                    app.stop_propagation();
                    select_view
                        .update(app, |view, cx| {
                            view.toggle_event_selection(mode.calendar_mode(), occurrence_id, cx);
                        })
                        .ok();
                }
                "enter" | "return" => {
                    app.stop_propagation();
                    key_view
                        .update(app, |view, cx| {
                            view.activate_surface(mode.calendar_mode(), cx);
                            view.inspect_event(occurrence_id, event_date, window, cx);
                        })
                        .ok();
                }
                "left" | "up" => {
                    app.stop_propagation();
                    window.focus_prev(app);
                }
                "right" | "down" => {
                    app.stop_propagation();
                    window.focus_next(app);
                }
                _ => {}
            },
        )
        .when(!bulk_mode, |this| {
            this.on_drag(move_payload, move |payload, offset, _, app| {
                drag_view
                    .update(app, |view, cx| view.begin_manipulation(payload, offset, cx))
                    .ok();
                app.new(|_| {
                    DragAvatar::new(
                        avatar_title.clone(),
                        avatar_time.clone(),
                        avatar_background,
                        avatar_foreground,
                        offset,
                    )
                })
            })
        })
        .on_click(move |event, window, app| {
            app.stop_propagation();
            view.update(app, |this, cx| {
                if event.modifiers().secondary() && event.click_count() == 1 {
                    this.toggle_event_selection_from_shortcut(
                        mode.calendar_mode(),
                        occurrence_id,
                        cx,
                    );
                } else if bulk_mode {
                    if bulk_selectable && event.standard_click() {
                        this.toggle_event_selection(mode.calendar_mode(), occurrence_id, cx);
                    }
                } else if event.standard_click() && event.click_count() >= 2 {
                    this.activate_surface(mode.calendar_mode(), cx);
                    this.inspect_event(occurrence_id, event_date, window, cx);
                } else if event.standard_click() {
                    this.activate_surface(mode.calendar_mode(), cx);
                    this.select_event(occurrence_id, event_date, cx);
                }
            })
            .ok();
        })
        .child(resize_handle(ResizeHandleProps {
            id: format!("{}-event-resize-start-{element_key}", mode.key()),
            payload: resize_start_payload,
            view: resize_start_view,
            start: true,
            background,
            foreground,
            height: position.height(),
            event_title: event.title().to_owned(),
            event_time: event_time.clone(),
            disabled: bulk_mode,
        }))
        .child(render_category_header(
            category_name,
            foreground,
            conflicted,
            cx,
        ))
        .child(details)
        .child(resize_handle(ResizeHandleProps {
            id: format!("{}-event-resize-end-{element_key}", mode.key()),
            payload: resize_end_payload,
            view: resize_end_view,
            start: false,
            background,
            foreground,
            height: position.height(),
            event_title: event.title().to_owned(),
            event_time,
            disabled: bulk_mode,
        }))
        .into_any_element()
}

struct ResizeHandleProps {
    id: String,
    payload: DragPayload,
    view: gpui::WeakEntity<CadenceView>,
    start: bool,
    background: Hsla,
    foreground: Hsla,
    height: f32,
    event_title: String,
    event_time: String,
    disabled: bool,
}

fn resize_handle(props: ResizeHandleProps) -> gpui::AnyElement {
    let ResizeHandleProps {
        id,
        payload,
        view,
        start,
        background,
        foreground,
        height,
        event_title,
        event_time,
        disabled,
    } = props;
    let handle_view = view;
    div()
        .id(id)
        .absolute()
        .left(px(0.0))
        .right(px(0.0))
        .when(start, |this| this.top(px(0.0)))
        .when(!start, |this| this.bottom(px(0.0)))
        .h(px(10.0_f32.min((height - 4.0).max(4.0))))
        .when(!disabled, gpui::Styled::cursor_ns_resize)
        .bg(foreground.opacity(0.08))
        .hover(|this| this.bg(foreground.opacity(0.2)))
        .on_mouse_down(
            MouseButton::Left,
            |_: &gpui::MouseDownEvent, _: &mut Window, app: &mut App| {
                app.stop_propagation();
            },
        )
        .when(!disabled, |this| {
            this.on_drag(payload, move |payload, offset, _, app| {
                handle_view
                    .update(app, |view, cx| view.begin_manipulation(payload, offset, cx))
                    .ok();
                app.new(|_| {
                    DragAvatar::new(
                        event_title.clone(),
                        event_time.clone(),
                        background,
                        foreground,
                        offset,
                    )
                })
            })
        })
        .into_any_element()
}

struct DragAvatar {
    title: String,
    event_time: String,
    background: Hsla,
    foreground: Hsla,
    offset: Point<Pixels>,
}

impl DragAvatar {
    const fn new(
        title: String,
        event_time: String,
        background: Hsla,
        foreground: Hsla,
        offset: Point<Pixels>,
    ) -> Self {
        Self {
            title,
            event_time,
            background,
            foreground,
            offset,
        }
    }
}

impl Render for DragAvatar {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let width = px(220.0);
        let height = px(56.0);
        div()
            .absolute()
            .left(self.offset.x - width / 2.0)
            .top(self.offset.y - height / 2.0)
            .w(width)
            .h(height)
            .v_flex()
            .justify_center()
            .gap_1()
            .rounded_md()
            .border_1()
            .border_color(self.foreground.opacity(0.5))
            .bg(self.background.opacity(0.9))
            .text_color(self.foreground)
            .px_2()
            .shadow_md()
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .truncate()
                    .child(self.title.clone()),
            )
            .child(div().text_xs().opacity(0.8).child(self.event_time.clone()))
    }
}

fn render_category_header(
    category_name: String,
    foreground: Hsla,
    conflicted: bool,
    cx: &Context<'_, CadenceView>,
) -> gpui::AnyElement {
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
        )
        .when(conflicted, |this| {
            this.child(div().ml_auto().text_color(cx.theme().warning).child("!"))
        })
        .into_any_element()
}

fn render_details(
    title: String,
    event_time: &str,
    compact: bool,
    tall: bool,
    roomy: bool,
    notes: Option<String>,
) -> gpui::AnyElement {
    div()
        .when(tall, |this| {
            this.child(
                div()
                    .text_sm()
                    .font_medium()
                    .when(roomy, |this| this.line_clamp(2))
                    .when(!roomy, gpui::Styled::truncate)
                    .child(title.clone()),
            )
            .child(div().text_xs().opacity(0.78).child(event_time.to_owned()))
        })
        .when_some(notes, |this, notes| {
            this.child(div().text_xs().opacity(0.72).line_clamp(2).child(notes))
        })
        .when(!compact && !tall, |this| {
            this.child(div().text_xs().font_medium().truncate().child(title))
        })
        .into_any_element()
}
