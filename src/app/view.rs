use gpui::{Context, IntoElement, Render, Window, div, prelude::*};
use gpui_component::{ActiveTheme as _, StyledExt as _};

use crate::components::title_bar::CadenceTitleBar;

use super::{state::CadenceView, toolbar, week};

impl Render for CadenceView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let error = self.error.clone();
        div()
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
            .child(week::render(self, window, cx))
    }
}
