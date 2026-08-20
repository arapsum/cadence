use gpui::{App, Context, SharedString, Window, WindowDecorations, WindowOptions, div, prelude::*};
use gpui_component::{button::*, *};

struct CadenceView {
    text: SharedString,
}

impl Render for CadenceView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .size_full()
            .child(TitleBar::new().child(div().flex().items_center().h_full().child("Cadence")))
            .child(
                div()
                    .v_flex()
                    .gap_2()
                    .flex_1()
                    .justify_center()
                    .items_center()
                    .child(format!("Hello, {}!", self.text))
                    .child(
                        Button::new("ok")
                            .primary()
                            .label("Let's Go!")
                            .on_click(|_, _, _| println!("Clicked")),
                    ),
            )
    }
}

pub fn run() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(|cx: &mut App| {
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            let window_options = WindowOptions {
                window_decorations: Some(WindowDecorations::Client),
                ..TitleBar::window_options()
            };

            cx.open_window(window_options, |window, cx| {
                window.set_window_title("Cadence");

                let view = cx.new(|_| CadenceView {
                    text: "World".into(),
                });

                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
