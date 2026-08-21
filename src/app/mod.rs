pub(super) mod actions;
pub(super) mod day;
pub(super) mod editor;
pub(super) mod event_card;
pub(super) mod grid;
pub(super) mod history;
pub(super) mod interaction;
pub(super) mod presentation;
pub(super) mod state;
pub(super) mod style;
pub(super) mod surface;
pub(super) mod toolbar;
pub(super) mod view;
pub(super) mod week;

use gpui::{App, AppContext as _, WindowBounds, WindowDecorations, WindowOptions, px, size};
use gpui_component::Root;

use crate::components::title_bar::CadenceTitleBar;

/// Starts the desktop `gpui` application.
///
/// # Panics
///
/// Panics when:
///
/// - The Cadence window cannot be opened.
pub fn run() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(|cx: &mut App| {
        gpui_component::init(cx);
        actions::bind(cx);

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
