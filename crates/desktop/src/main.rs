use gpui::{App, WindowBounds, WindowDecorations, WindowOptions, px, size};
use gpui_component::TitleBar;

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(|cx: &mut App| {
        cx.set_app_identity("io.github.arapsum.Cadence", "Cadence");
        cadence_ui::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1480.), px(880.)), cx)),
            window_min_size: Some(size(px(640.), px(480.))),
            window_decorations: Some(WindowDecorations::Client),
            ..TitleBar::window_options()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                window.set_window_title("Cadence");
                cadence_ui::mount(window, cx)
            })
            .expect("Failed to open Cadence window");
        })
        .detach();
    });
}
