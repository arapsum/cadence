use gpui::{Hsla, div, hsla, prelude::*, px, rgb};

use crate::domain::CategoryColor;

pub(crate) const DAY_HEADER_HEIGHT: f32 = 68.0;
pub(crate) const TIME_GUTTER_WIDTH: f32 = 78.0;
pub(crate) const MIN_COLUMN_WIDTH: f32 = 132.0;
pub(crate) const PIXELS_PER_MINUTE: f32 = 1.5;
pub(crate) const PLANE_HEIGHT: f32 = 24.0 * 60.0 * PIXELS_PER_MINUTE;

pub(crate) fn category_dot(color: Option<CategoryColor>) -> impl IntoElement {
    div()
        .w(px(7.0))
        .h(px(7.0))
        .rounded_full()
        .bg(category_palette(color.unwrap_or(CategoryColor::Blue), false).0)
}

pub(crate) fn category_palette(color: CategoryColor, dark: bool) -> (Hsla, Hsla, Hsla) {
    match (color, dark) {
        (CategoryColor::Lime, false) => (
            rgb(0xB8F35A).into(),
            rgb(0x17230A).into(),
            rgb(0x8FCB2C).into(),
        ),
        (CategoryColor::Lime, true) => (
            hsla(0.23, 0.62, 0.28, 1.0),
            hsla(0.2, 0.84, 0.88, 1.0),
            hsla(0.23, 0.7, 0.52, 1.0),
        ),
        (CategoryColor::Yellow, false) => (
            rgb(0xFFE04A).into(),
            rgb(0x352C00).into(),
            rgb(0xD2AB00).into(),
        ),
        (CategoryColor::Yellow, true) => (
            hsla(0.13, 0.75, 0.3, 1.0),
            hsla(0.12, 0.9, 0.9, 1.0),
            hsla(0.13, 0.8, 0.58, 1.0),
        ),
        (CategoryColor::Coral, false) => (
            rgb(0xFF9BA1).into(),
            rgb(0x3A0E13).into(),
            rgb(0xE56C77).into(),
        ),
        (CategoryColor::Coral, true) => (
            hsla(0.99, 0.65, 0.32, 1.0),
            hsla(0.0, 0.82, 0.92, 1.0),
            hsla(0.99, 0.78, 0.6, 1.0),
        ),
        (CategoryColor::Violet, false) => (
            rgb(0xC9A0F2).into(),
            rgb(0x241236).into(),
            rgb(0x9A67D4).into(),
        ),
        (CategoryColor::Violet, true) => (
            hsla(0.76, 0.55, 0.35, 1.0),
            hsla(0.76, 0.75, 0.94, 1.0),
            hsla(0.76, 0.68, 0.65, 1.0),
        ),
        (CategoryColor::Cyan, false) => (
            rgb(0x63D9E9).into(),
            rgb(0x062D34).into(),
            rgb(0x26AEBE).into(),
        ),
        (CategoryColor::Cyan, true) => (
            hsla(0.52, 0.68, 0.32, 1.0),
            hsla(0.52, 0.86, 0.9, 1.0),
            hsla(0.52, 0.75, 0.62, 1.0),
        ),
        (CategoryColor::Blue, false) => (
            rgb(0x86B8EF).into(),
            rgb(0x0C2340).into(),
            rgb(0x4C8DD4).into(),
        ),
        (CategoryColor::Blue, true) => (
            hsla(0.59, 0.65, 0.34, 1.0),
            hsla(0.59, 0.84, 0.92, 1.0),
            hsla(0.59, 0.76, 0.64, 1.0),
        ),
    }
}
