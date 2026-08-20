use gpui::{
    App, Context, Entity, Window, WindowBounds, WindowDecorations, WindowOptions, div, prelude::*,
    px, size,
};
use gpui_component::{
    ActiveTheme as _, IconName, IndexPath, Root, StyledExt as _, Theme, ThemeMode,
    button::{Button, ButtonVariants as _},
    input::{Input, InputState},
    popover::Popover,
    select::{Select, SelectState},
};

use crate::components::title_bar::CadenceTitleBar;

struct CadenceView {
    event_title: Entity<InputState>,
    calendar_view: Entity<SelectState<Vec<&'static str>>>,
}

impl CadenceView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let event_title =
            cx.new(|cx| InputState::new(window, cx).placeholder("e.g. Morning routine"));
        let calendar_view = cx.new(|cx| {
            SelectState::new(
                vec!["Day", "Week"],
                Some(IndexPath::default().row(1)),
                window,
                cx,
            )
        });

        Self {
            event_title,
            calendar_view,
        }
    }
}

impl Render for CadenceView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dark = cx.theme().mode.is_dark();
        let theme_icon = if is_dark {
            IconName::Sun
        } else {
            IconName::Moon
        };
        let theme_label = if is_dark {
            "Use light theme"
        } else {
            "Use dark theme"
        };

        div()
            .v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(CadenceTitleBar::new("Cadence"))
            .child(
                div()
                    .id("milestone-zero-scroll-area")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .v_flex()
                            .w_full()
                            .max_w(px(960.))
                            .min_h(px(760.))
                            .mx_auto()
                            .p_8()
                            .gap_6()
                            .child(
                                div()
                                    .v_flex()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_3xl()
                                            .font_semibold()
                                            .child("Framework smoke test"),
                                    )
                                    .child(
                                        div()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(
                                                "Milestone 0 verifies Cadence's window, controls, \
                                                 focus, overlays, scrolling, and theme system.",
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .v_flex()
                                    .gap_5()
                                    .p_5()
                                    .rounded_lg()
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .bg(cx.theme().background)
                                    .child(
                                        div()
                                            .v_flex()
                                            .gap_1()
                                            .child(div().font_semibold().child("Core controls"))
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(
                                                        "Exercise each control before calendar work begins.",
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .v_flex()
                                            .gap_3()
                                            .child(
                                                Select::new(&self.calendar_view)
                                                    .w(px(180.))
                                                    .title_prefix("View: "),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_3()
                                                    .child(
                                                        Popover::new("milestone-zero-info")
                                                            .trigger(
                                                                Button::new("show-info")
                                                                    .outline()
                                                                    .label("Open popover"),
                                                            )
                                                            .w(px(300.))
                                                            .gap_2()
                                                            .child(
                                                                div()
                                                                    .font_semibold()
                                                                    .child("Overlay check"),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_sm()
                                                                    .text_color(
                                                                        cx.theme()
                                                                            .muted_foreground,
                                                                    )
                                                                    .child(
                                                                        "The popover should receive focus and dismiss \
                                                                         when you click elsewhere.",
                                                                    ),
                                                            ),
                                                    )
                                                    .child(
                                                        Button::new("toggle-theme")
                                                            .outline()
                                                            .icon(theme_icon)
                                                            .label(theme_label)
                                                            .on_click(|_, window, cx| {
                                                                let mode =
                                                                    if cx.theme().mode.is_dark() {
                                                                        ThemeMode::Light
                                                                    } else {
                                                                        ThemeMode::Dark
                                                                    };
                                                                Theme::change(
                                                                    mode,
                                                                    Some(window),
                                                                    cx,
                                                                );
                                                            }),
                                                    )
                                                    .child(
                                                        Button::new("primary-action")
                                                            .primary()
                                                            .label("Primary action"),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .v_flex()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_medium()
                                                    .child("Event title"),
                                            )
                                            .child(
                                                Input::new(&self.event_title)
                                                    .w_full()
                                                    .max_w(px(480.)),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(
                                                        "Click the field, type, select text, and use Tab \
                                                         to confirm keyboard focus.",
                                                    ),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .v_flex()
                                    .gap_3()
                                    .p_5()
                                    .rounded_lg()
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .child(div().font_semibold().child("Scroll checkpoint"))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(
                                                "If this card is reachable in a short window, vertical \
                                                 scrolling and clipping are working.",
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Cadence · Milestone 0 platform spike"),
                            ),
                    ),
            )
    }
}

pub fn run() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(|cx: &mut App| {
        gpui_component::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(920.), px(640.)), cx)),
            window_min_size: Some(size(px(640.), px(480.))),
            window_decorations: Some(WindowDecorations::Client),
            ..CadenceTitleBar::window_options()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                window.set_window_title("Cadence");

                let view = cx.new(|cx| CadenceView::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open Cadence window");
        })
        .detach();
    });
}
