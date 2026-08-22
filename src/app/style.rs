use gpui::{Hsla, Pixels, div, hsla, prelude::*, px, rgb};

use crate::domain::CategoryColor;

pub(super) const DAY_HEADER_HEIGHT: f32 = 68.0;
pub(super) const TIME_GUTTER_WIDTH: f32 = 78.0;
pub(super) const MIN_COLUMN_WIDTH: f32 = 92.0;
pub(super) const PIXELS_PER_MINUTE: f32 = 1.5;
pub(super) const PLANE_HEIGHT: f32 = 24.0 * 60.0 * PIXELS_PER_MINUTE;

const DIALOG_CENTER_NUDGE: Pixels = px(20.0);

pub(super) fn dialog_margin_top(available_height: Pixels, dialog_height: Pixels) -> Pixels {
    ((available_height - dialog_height) / 2.0 - DIALOG_CENTER_NUDGE).max(px(0.0))
}

pub(super) fn category_dot(color: Option<CategoryColor>) -> impl IntoElement {
    div()
        .w(px(7.0))
        .h(px(7.0))
        .rounded_full()
        .bg(category_palette(color.unwrap_or(CategoryColor::Blue), false).0)
}

pub(super) fn category_palette(color: CategoryColor, dark: bool) -> (Hsla, Hsla, Hsla) {
    match (color, dark) {
        (CategoryColor::Lime, false) => (
            rgb(0x00F2_F8E7).into(),
            rgb(0x0017_230A).into(),
            rgb(0x00B8_DD7A).into(),
        ),
        (CategoryColor::Lime, true) => (
            hsla(0.23, 0.28, 0.17, 1.0),
            hsla(0.2, 0.42, 0.86, 1.0),
            hsla(0.23, 0.42, 0.42, 1.0),
        ),
        (CategoryColor::Yellow, false) => (
            rgb(0x00FF_F8DF).into(),
            rgb(0x0035_2C00).into(),
            rgb(0x00E4_C66D).into(),
        ),
        (CategoryColor::Yellow, true) => (
            hsla(0.13, 0.3, 0.18, 1.0),
            hsla(0.12, 0.44, 0.88, 1.0),
            hsla(0.13, 0.46, 0.45, 1.0),
        ),
        (CategoryColor::Coral, false) => (
            rgb(0x00FF_EEEF).into(),
            rgb(0x003A_0E13).into(),
            rgb(0x00EC_A6AA).into(),
        ),
        (CategoryColor::Coral, true) => (
            hsla(0.99, 0.28, 0.18, 1.0),
            hsla(0.0, 0.44, 0.88, 1.0),
            hsla(0.99, 0.44, 0.46, 1.0),
        ),
        (CategoryColor::Violet, false) => (
            rgb(0x00F4_EEFB).into(),
            rgb(0x0024_1236).into(),
            rgb(0x00C6_A3E5).into(),
        ),
        (CategoryColor::Violet, true) => (
            hsla(0.76, 0.26, 0.19, 1.0),
            hsla(0.76, 0.42, 0.9, 1.0),
            hsla(0.76, 0.4, 0.49, 1.0),
        ),
        (CategoryColor::Cyan, false) => (
            rgb(0x00E9_F8FA).into(),
            rgb(0x0006_2D34).into(),
            rgb(0x007E_CAD3).into(),
        ),
        (CategoryColor::Cyan, true) => (
            hsla(0.52, 0.28, 0.18, 1.0),
            hsla(0.52, 0.43, 0.88, 1.0),
            hsla(0.52, 0.44, 0.45, 1.0),
        ),
        (CategoryColor::Blue, false) => (
            rgb(0x00EC_F5FC).into(),
            rgb(0x000C_2340).into(),
            rgb(0x0089_BCE6).into(),
        ),
        (CategoryColor::Blue, true) => (
            hsla(0.59, 0.27, 0.19, 1.0),
            hsla(0.59, 0.42, 0.9, 1.0),
            hsla(0.59, 0.43, 0.48, 1.0),
        ),
    }
}
