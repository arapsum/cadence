mod event_card;
mod grid;
mod presentation;
mod state;
mod style;
mod toolbar;
mod view;
mod week;

use gpui::{App, AppContext as _, WindowBounds, WindowDecorations, WindowOptions, px, size};
use gpui_component::Root;

use crate::components::title_bar::CadenceTitleBar;

pub fn run() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(|cx: &mut App| {
        gpui_component::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1180.), px(760.)), cx)),
            window_min_size: Some(size(px(640.), px(480.))),
            window_decorations: Some(WindowDecorations::Client),
            ..CadenceTitleBar::window_options()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                window.set_window_title("Cadence");

                let view = cx.new(|cx| state::CadenceView::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open Cadence window");
        })
        .detach();
    });
}
