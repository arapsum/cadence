use std::env;

use gpui::{App, WindowBounds, WindowDecorations, WindowOptions, px, size};
use gpui_component::TitleBar;

fn main() {
    if handle_cli() {
        return;
    }

    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(|cx: &mut App| {
        cx.set_app_identity(cadence_ui::APPLICATION_ID, cadence_ui::APPLICATION_NAME);
        cadence_ui::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1480.), px(880.)), cx)),
            window_min_size: Some(size(px(640.), px(480.))),
            window_decorations: Some(WindowDecorations::Client),
            app_id: Some(cadence_ui::APPLICATION_ID.to_owned()),
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

fn handle_cli() -> bool {
    let Some(argument) = env::args_os().nth(1) else {
        return false;
    };

    match argument.to_str() {
        Some("--version" | "-V") => {
            let info = cadence_ui::BuildInfo::current();
            println!(
                "{} {} ({})",
                cadence_ui::APPLICATION_NAME,
                info.version,
                info.commit
            );
            true
        }
        Some("--help" | "-h") => {
            println!("Cadence — a local-first desktop timetable");
            println!();
            println!("Usage: cadence [OPTIONS]");
            println!();
            println!("Options:");
            println!("  -h, --help       Show this help message");
            println!("  -V, --version    Show the application version");
            true
        }
        _ => false,
    }
}
