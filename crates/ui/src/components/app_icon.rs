use std::sync::{Arc, LazyLock};

use gpui::{Image, ImageFormat, IntoElement, Pixels, Styled as _, img};

const CADENCE_ICON_SVG: &[u8] = include_bytes!("../../assets/cadence-icon.svg");

static CADENCE_ICON: LazyLock<Arc<Image>> = LazyLock::new(|| {
    Arc::new(Image::from_bytes(
        ImageFormat::Svg,
        CADENCE_ICON_SVG.to_vec(),
    ))
});

pub fn render(size: Pixels) -> impl IntoElement {
    img(CADENCE_ICON.clone()).size(size)
}
