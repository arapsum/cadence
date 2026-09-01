use gpui::{Context, IntoElement, Render, Window, div, prelude::*, px};
use gpui_component::{
    ActiveTheme as _, FocusTrapElement as _, IconName, Root, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    skeleton::Skeleton,
};

use crate::components::title_bar::CadenceTitleBar;

use super::{
    actions,
    state::{CadenceView, PersistenceState},
    toolbar, workspace,
};

impl Render for CadenceView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let error = self.error.clone();
        let sheet_layer = Root::render_sheet_layer(window, cx);
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);
        div()
            .key_context(actions::CALENDAR_CONTEXT)
            .on_action(cx.listener(|this, _: &actions::PreviousPeriod, _, cx| {
                this.shift_period(false, cx);
            }))
            .on_action(cx.listener(|this, _: &actions::NextPeriod, _, cx| {
                this.shift_period(true, cx);
            }))
            .on_action(cx.listener(|this, _: &actions::GoToToday, _, cx| {
                this.go_to_today(cx);
            }))
            .on_action(cx.listener(|this, _: &actions::NewEvent, window, cx| {
                this.new_event(window, cx);
            }))
            .on_action(cx.listener(|this, _: &actions::OpenAgenda, window, cx| {
                this.open_agenda(window, cx);
            }))
            .on_action(cx.listener(|_, _: &actions::OpenSettings, window, cx| {
                Self::open_settings(window, cx);
            }))
            .on_action(cx.listener(|_, _: &actions::OpenAbout, window, cx| {
                Self::open_about(window, cx);
            }))
            .on_action(cx.listener(|this, _: &actions::Undo, window, cx| {
                this.undo(window, cx);
            }))
            .on_action(cx.listener(|this, _: &actions::Redo, window, cx| {
                this.redo(window, cx);
            }))
            .on_action(cx.listener(|this, _: &actions::SelectAllEvents, _, cx| {
                this.select_all_visible_events(cx);
            }))
            .on_action(
                cx.listener(|this, _: &actions::DeleteSelectedEvents, window, cx| {
                    this.confirm_delete_selected(window, cx);
                }),
            )
            .on_action(
                cx.listener(|this, _: &actions::CancelManipulation, window, cx| {
                    if this.day_plan_open && this.manipulation.is_none() {
                        this.close_day_plan(window, cx);
                    } else if this.is_bulk_selecting() {
                        this.cancel_event_selection(cx);
                    } else {
                        this.cancel_manipulation(window, cx);
                    }
                }),
            )
            .v_flex()
            .relative()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                CadenceTitleBar::new("Cadence")
                    .close_to_tray()
                    .leading(toolbar::render_titlebar_history(self, cx))
                    .controls(toolbar::render_titlebar_actions(self, window, cx)),
            )
            .when_some(error, |this, error| {
                this.child(
                    div()
                        .mx_4()
                        .mb_3()
                        .p_3()
                        .rounded_md()
                        .bg(cx.theme().danger.opacity(0.12))
                        .text_color(cx.theme().danger)
                        .text_sm()
                        .child(error),
                )
            })
            .child(render_content(self, window, cx))
            .children(sheet_layer)
            .children(dialog_layer)
            .children(notification_layer)
    }
}

fn render_day_plan_sheet(
    view: &mut CadenceView,
    window: &Window,
    cx: &mut Context<'_, CadenceView>,
) -> gpui::AnyElement {
    let owner = cx.entity().downgrade();
    let title = view
        .state
        .selected_date()
        .strftime("%A, %B %-d")
        .to_string();
    let focus = view.day_plan_focus.clone();

    div()
        .absolute()
        .inset_0()
        .child(
            div()
                .id("day-plan-sheet-overlay")
                .absolute()
                .inset_0()
                .occlude()
                .bg(cx.theme().background.opacity(0.52))
                .on_click(move |_, window, app| {
                    owner
                        .update(app, |view, cx| view.close_day_plan(window, cx))
                        .ok();
                }),
        )
        .child(
            div()
                .id("day-plan-sheet")
                .debug_selector(|| "day-plan-sheet".into())
                .absolute()
                .top(px(0.0))
                .right_0()
                .bottom_0()
                .w(px(440.0))
                .min_w(px(360.0))
                .v_flex()
                .occlude()
                .bg(cx.theme().background)
                .border_l_1()
                .border_color(cx.theme().border)
                .shadow_xl()
                .track_focus(&focus)
                .on_click(|_, _, app| app.stop_propagation())
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
                        .child(
                            div()
                                .v_flex()
                                .gap_0()
                                .child(div().text_sm().font_semibold().child("Day plan"))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(title),
                                ),
                        )
                        .child(
                            Button::new("close-day-plan-sheet")
                                .ghost()
                                .small()
                                .icon(IconName::Close)
                                .tooltip("Close day plan (Escape)")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.close_day_plan(window, cx);
                                })),
                        ),
                )
                .child(
                    div()
                        .v_flex()
                        .flex_1()
                        .min_h_0()
                        .child(super::day::render(view, window, cx)),
                )
                .focus_trap("day-plan-sheet-focus", &focus),
        )
        .into_any_element()
}

fn render_content(
    view: &mut CadenceView,
    window: &Window,
    cx: &mut Context<'_, CadenceView>,
) -> gpui::AnyElement {
    match &view.persistence_state {
        PersistenceState::Opening => render_opening_skeleton(cx),
        PersistenceState::Recovery(error) => {
            let retry = cx.listener(|this, _, window, cx| this.retry_storage(window, cx));
            let archive =
                cx.listener(|this, _, window, cx| this.archive_and_start_fresh(window, cx));
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .p_6()
                .child(
                    div()
                        .max_w(px(520.0))
                        .v_flex()
                        .gap_3()
                        .rounded_lg()
                        .border_1()
                        .border_color(cx.theme().border)
                        .p_6()
                        .bg(cx.theme().secondary)
                        .child(
                            div()
                                .text_lg()
                                .font_semibold()
                                .child("Timetable needs attention"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(error.user_message()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("Database: {}", view.storage_path.display())),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(
                                    Button::new("storage-retry")
                                        .outline()
                                        .label("Retry")
                                        .on_click(retry),
                                )
                                .child(
                                    Button::new("storage-reveal")
                                        .outline()
                                        .label("Reveal data folder")
                                        .on_click({
                                            let path = view.storage_path.clone();
                                            move |_, _, cx| {
                                                cx.reveal_path(
                                                    path.parent().unwrap_or_else(|| {
                                                        std::path::Path::new(".")
                                                    }),
                                                );
                                            }
                                        }),
                                )
                                .child(
                                    Button::new("storage-fresh")
                                        .danger()
                                        .label("Archive and start fresh")
                                        .on_click(archive),
                                ),
                        ),
                )
                .into_any_element()
        }
        PersistenceState::Ready | PersistenceState::Writing => div()
            .relative()
            .v_flex()
            .flex_1()
            .min_h_0()
            .child(workspace::render(view, window, cx))
            .when(view.day_plan_open, |this| {
                this.child(render_day_plan_sheet(view, window, cx))
            })
            .into_any_element(),
    }
}

fn render_opening_skeleton(cx: &Context<'_, CadenceView>) -> gpui::AnyElement {
    let day_headers = (0..7).map(|_| {
        div()
            .flex_1()
            .child(Skeleton::new().secondary().h_5().w_20().max_w_full())
            .into_any_element()
    });
    let time_rows = (0..6).map(|_| {
        div()
            .flex()
            .gap_2()
            .h(px(72.0))
            .child(
                div()
                    .w(px(64.0))
                    .flex_shrink_0()
                    .child(Skeleton::new().secondary().h_4().w_16().max_w_full()),
            )
            .children((0..7).map(|_| {
                div()
                    .flex_1()
                    .h_full()
                    .rounded_md()
                    .overflow_hidden()
                    .child(Skeleton::new().secondary().h_full().w_full())
                    .into_any_element()
            }))
            .into_any_element()
    });

    div()
        .flex_1()
        .p_4()
        .overflow_hidden()
        .child(
            div()
                .v_flex()
                .gap_3()
                .h_full()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().border)
                .p_4()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(Skeleton::new().h_4().w_4().rounded_full())
                        .child("Opening timetable…"),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(div().w(px(64.0)).flex_shrink_0())
                        .children(day_headers),
                )
                .children(time_rows),
        )
        .into_any_element()
}
