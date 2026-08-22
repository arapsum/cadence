use gpui::{Context, IntoElement, Window};

use super::{state::CadenceView, surface};

pub(super) fn render(
    view: &mut CadenceView,
    window: &Window,
    cx: &mut Context<'_, CadenceView>,
) -> impl IntoElement {
    surface::render(view, window, surface::SurfaceMode::Day, cx)
}
