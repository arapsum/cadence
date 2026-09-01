pub mod actions;
pub mod appearance;
mod bulk_delete;
pub mod categories;
pub mod day;
pub mod editor;
pub mod event_card;
pub mod grid;
pub mod guidance;
pub mod history;
pub mod interaction;
pub mod presentation;
mod runtime;
mod settings_window;
pub mod sidebar;
pub mod state;
pub mod style;
pub mod surface;
pub mod toolbar;
pub mod view;
pub mod week;
pub mod workspace;

use gpui::{
    App, AppContext as _, AsyncApp, Entity, Window, WindowBounds, WindowDecorations, WindowHandle,
    WindowOptions, px, size,
};
use gpui_component::{Root, TitleBar};

pub fn init(cx: &mut App) {
    appearance::register_themes(cx);
    runtime::init(cx);
    settings_window::init(cx);
    actions::bind(cx);
}

pub fn open_main_window(cx: &AsyncApp) -> gpui::Result<WindowHandle<Root>> {
    let options = cx.update(|cx| main_window_options(cx));
    cx.open_window(options, mount_main_window)
}

pub fn open_main_window_with_app(cx: &mut App) -> gpui::Result<WindowHandle<Root>> {
    let options = main_window_options(cx);
    cx.open_window(options, mount_main_window)
}

pub fn close_main_window(window: &mut Window, cx: &mut App) {
    runtime::close_main_window(window, cx);
}

fn mount_main_window(window: &mut Window, cx: &mut App) -> Entity<Root> {
    window.set_window_title("Cadence");
    if let Some(view) = runtime::existing_main_view(cx) {
        mount_view(view, window, cx)
    } else {
        mount(window, cx)
    }
}

fn main_window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::centered(size(px(1480.), px(880.)), cx)),
        window_min_size: Some(size(px(640.), px(480.))),
        window_decorations: Some(WindowDecorations::Client),
        app_id: Some(crate::APPLICATION_ID.to_owned()),
        ..TitleBar::window_options()
    }
}

pub fn mount(window: &mut Window, cx: &mut App) -> Entity<Root> {
    let view = cx.new(|cx| state::CadenceView::new(window, cx));
    mount_view(view, window, cx)
}

fn mount_view(view: Entity<state::CadenceView>, window: &mut Window, cx: &mut App) -> Entity<Root> {
    runtime::install(window.window_handle(), view.clone(), cx);
    window.on_window_should_close(cx, runtime::should_close_main_window);
    cx.new(|cx| Root::new(view, window, cx))
}
