use gpui::{App, Entity, Window};
use gpui_component::Root;

pub(crate) use cadence_core::{calendar, domain, editor, store};

mod app;
mod components;

/// Initializes Cadence's GPUI components, themes, and application actions.
///
/// # Parameters
///
/// - `cx`: Application context receiving Cadence's global registrations.
pub fn init(cx: &mut App) {
    gpui_component::init(cx);
    app::init(cx);
}

/// Creates Cadence's root entity and installs its save-aware close behavior.
///
/// # Parameters
///
/// - `window`: Window receiving the Cadence root view.
/// - `cx`: Application context used to create the root entity.
///
/// # Returns
///
/// The `gpui_component` root that renders the Cadence workspace.
pub fn mount(window: &mut Window, cx: &mut App) -> Entity<Root> {
    app::mount(window, cx)
}
