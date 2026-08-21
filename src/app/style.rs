use gpui::{Hsla, Pixels, div, hsla, prelude::*, px, rgb};

use crate::domain::CategoryColor;

pub(super) const DAY_HEADER_HEIGHT: f32 = 68.0;
pub(super) const TIME_GUTTER_WIDTH: f32 = 78.0;
pub(super) const MIN_COLUMN_WIDTH: f32 = 132.0;
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
            rgb(0x00B8_F35A).into(),
            rgb(0x0017_230A).into(),
            rgb(0x008F_CB2C).into(),
        ),
        (CategoryColor::Lime, true) => (
            hsla(0.23, 0.62, 0.28, 1.0),
            hsla(0.2, 0.84, 0.88, 1.0),
            hsla(0.23, 0.7, 0.52, 1.0),
        ),
        (CategoryColor::Yellow, false) => (
            rgb(0x00FF_E04A).into(),
            rgb(0x0035_2C00).into(),
            rgb(0x00D2_AB00).into(),
        ),
        (CategoryColor::Yellow, true) => (
            hsla(0.13, 0.75, 0.3, 1.0),
            hsla(0.12, 0.9, 0.9, 1.0),
            hsla(0.13, 0.8, 0.58, 1.0),
        ),
        (CategoryColor::Coral, false) => (
            rgb(0x00FF_9BA1).into(),
            rgb(0x003A_0E13).into(),
            rgb(0x00E5_6C77).into(),
        ),
        (CategoryColor::Coral, true) => (
            hsla(0.99, 0.65, 0.32, 1.0),
            hsla(0.0, 0.82, 0.92, 1.0),
            hsla(0.99, 0.78, 0.6, 1.0),
        ),
        (CategoryColor::Violet, false) => (
            rgb(0x00C9_A0F2).into(),
            rgb(0x0024_1236).into(),
            rgb(0x009A_67D4).into(),
        ),
        (CategoryColor::Violet, true) => (
            hsla(0.76, 0.55, 0.35, 1.0),
            hsla(0.76, 0.75, 0.94, 1.0),
            hsla(0.76, 0.68, 0.65, 1.0),
        ),
        (CategoryColor::Cyan, false) => (
            rgb(0x0063_D9E9).into(),
            rgb(0x0006_2D34).into(),
            rgb(0x0026_AEBE).into(),
        ),
        (CategoryColor::Cyan, true) => (
            hsla(0.52, 0.68, 0.32, 1.0),
            hsla(0.52, 0.86, 0.9, 1.0),
            hsla(0.52, 0.75, 0.62, 1.0),
        ),
        (CategoryColor::Blue, false) => (
            rgb(0x0086_B8EF).into(),
            rgb(0x000C_2340).into(),
            rgb(0x004C_8DD4).into(),
        ),
        (CategoryColor::Blue, true) => (
            hsla(0.59, 0.65, 0.34, 1.0),
            hsla(0.59, 0.84, 0.92, 1.0),
            hsla(0.59, 0.76, 0.64, 1.0),
        ),
    }
}
