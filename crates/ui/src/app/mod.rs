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
mod settings_window;
pub mod sidebar;
pub mod state;
pub mod style;
pub mod surface;
pub mod toolbar;
pub mod view;
pub mod week;
pub mod workspace;

use gpui::{App, AppContext as _, Entity, Window};
use gpui_component::{Root, WindowExt as _};

pub fn init(cx: &mut App) {
    appearance::register_themes(cx);
    settings_window::init(cx);
    actions::bind(cx);
}

pub fn mount(window: &mut Window, cx: &mut App) -> Entity<Root> {
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
}
