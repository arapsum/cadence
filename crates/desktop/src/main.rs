use std::env;

use gpui::{App, QuitMode};

fn main() {
    if handle_cli() {
        return;
    }

    let app = gpui_platform::application()
        .with_assets(gpui_component_assets::Assets)
        .with_quit_mode(QuitMode::Explicit);

    app.run(|cx: &mut App| {
        cx.set_app_identity(cadence_ui::APPLICATION_ID, cadence_ui::APPLICATION_NAME);
        cadence_ui::init(cx);

        cx.spawn(async move |cx| {
            cadence_ui::open_main_window(cx).expect("Failed to open Cadence window");
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
