use gpui::{Context, IntoElement, Render, Window, div, prelude::*};
use gpui_component::{ActiveTheme as _, StyledExt as _};

use crate::calendar::CalendarViewMode;
use crate::components::title_bar::CadenceTitleBar;

use super::{actions, day, state::CadenceView, toolbar, week};

impl Render for CadenceView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let error = self.error.clone();
        div()
            .key_context(actions::CALENDAR_CONTEXT)
            .on_action(cx.listener(|this, _: &actions::ShowDay, _, cx| {
                this.set_view_mode(CalendarViewMode::Day, cx);
            }))
            .on_action(cx.listener(|this, _: &actions::ShowWeek, _, cx| {
                this.set_view_mode(CalendarViewMode::Week, cx);
            }))
            .on_action(cx.listener(|this, _: &actions::PreviousPeriod, _, cx| {
                this.shift_period(false, cx);
            }))
            .on_action(cx.listener(|this, _: &actions::NextPeriod, _, cx| {
                this.shift_period(true, cx);
            }))
            .on_action(cx.listener(|this, _: &actions::GoToToday, _, cx| {
                this.go_to_today(cx);
            }))
            .v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(CadenceTitleBar::new("Cadence"))
            .child(toolbar::render(self, window, cx))
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
            .child(match self.state.view_mode() {
                CalendarViewMode::Day => day::render(self, window, cx).into_any_element(),
                CalendarViewMode::Week => week::render(self, window, cx).into_any_element(),
            })
    }
}
