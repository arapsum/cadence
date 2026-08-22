pub(super) mod actions;
pub(super) mod categories;
pub(super) mod day;
pub(super) mod editor;
pub(super) mod event_card;
pub(super) mod grid;
pub(super) mod guidance;
pub(super) mod history;
pub(super) mod interaction;
pub(super) mod presentation;
pub(super) mod sidebar;
pub(super) mod state;
pub(super) mod style;
pub(super) mod surface;
pub(super) mod toolbar;
pub(super) mod view;
pub(super) mod week;
pub(super) mod workspace;

use gpui::{App, AppContext as _, WindowBounds, WindowDecorations, WindowOptions, px, size};
use gpui_component::{Root, WindowExt as _};

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
        cx.set_app_identity("io.github.arapsum.Cadence", "Cadence");
        gpui_component::init(cx);
        actions::bind(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1480.), px(880.)), cx)),
            window_min_size: Some(size(px(640.), px(480.))),
            window_decorations: Some(WindowDecorations::Client),
            ..CadenceTitleBar::window_options()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                window.set_window_title("Cadence");

                let view = cx.new(|cx| state::CadenceView::new(window, cx));
                let close_view = view.downgrade();
                window.on_window_should_close(cx, move |window, cx| {
                    let can_close = close_view
                        .update(cx, |view, _| {
                            !matches!(view.persistence_state, state::PersistenceState::Writing)
                        })
                        .unwrap_or(true);
                    if !can_close {
                        window.push_notification(
                            "Cadence is still saving. Try closing again when the save finishes.",
                            cx,
                        );
                    }
                    can_close
                });
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open Cadence window");
        })
        .detach();
    });
}
