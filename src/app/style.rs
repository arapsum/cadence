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
    if dark {
        dark_category_palette(color)
    } else {
        light_category_palette(color)
    }
}

fn light_category_palette(color: CategoryColor) -> (Hsla, Hsla, Hsla) {
    match color {
        CategoryColor::Lime => (
            rgb(0x00F2_F8E7).into(),
            rgb(0x0017_230A).into(),
            rgb(0x00B8_DD7A).into(),
        ),
        CategoryColor::Yellow => (
            rgb(0x00FF_F8DF).into(),
            rgb(0x0035_2C00).into(),
            rgb(0x00E4_C66D).into(),
        ),
        CategoryColor::Coral => (
            rgb(0x00FF_EEEF).into(),
            rgb(0x003A_0E13).into(),
            rgb(0x00EC_A6AA).into(),
        ),
        CategoryColor::Violet => (
            rgb(0x00F4_EEFB).into(),
            rgb(0x0024_1236).into(),
            rgb(0x00C6_A3E5).into(),
        ),
        CategoryColor::Cyan => (
            rgb(0x00E9_F8FA).into(),
            rgb(0x0006_2D34).into(),
            rgb(0x007E_CAD3).into(),
        ),
        CategoryColor::Blue => (
            rgb(0x00EC_F5FC).into(),
            rgb(0x000C_2340).into(),
            rgb(0x0089_BCE6).into(),
        ),
        CategoryColor::Orange => (
            rgb(0x00FF_F1E6).into(),
            rgb(0x0039_1A03).into(),
            rgb(0x00E9_A56C).into(),
        ),
        CategoryColor::Rose => (
            rgb(0x00FF_EBF2).into(),
            rgb(0x003B_0A22).into(),
            rgb(0x00E8_A0BC).into(),
        ),
        CategoryColor::Magenta => (
            rgb(0x00FA_EBFA).into(),
            rgb(0x0032_0A37).into(),
            rgb(0x00DC_9AD8).into(),
        ),
        CategoryColor::Indigo => (
            rgb(0x00EE_EFFF).into(),
            rgb(0x0013_163F).into(),
            rgb(0x00A8_AFE9).into(),
        ),
        CategoryColor::Teal => (
            rgb(0x00E5_F8F3).into(),
            rgb(0x0006_332C).into(),
            rgb(0x0075_C9B3).into(),
        ),
        CategoryColor::Slate => (
            rgb(0x00F0_F3F7).into(),
            rgb(0x001C_2631).into(),
            rgb(0x00AD_B9C6).into(),
        ),
    }
}

const fn dark_category_palette(color: CategoryColor) -> (Hsla, Hsla, Hsla) {
    match color {
        CategoryColor::Lime => (
            hsla(0.23, 0.28, 0.17, 1.0),
            hsla(0.2, 0.42, 0.86, 1.0),
            hsla(0.23, 0.42, 0.42, 1.0),
        ),
        CategoryColor::Yellow => (
            hsla(0.13, 0.3, 0.18, 1.0),
            hsla(0.12, 0.44, 0.88, 1.0),
            hsla(0.13, 0.46, 0.45, 1.0),
        ),
        CategoryColor::Coral => (
            hsla(0.99, 0.28, 0.18, 1.0),
            hsla(0.0, 0.44, 0.88, 1.0),
            hsla(0.99, 0.44, 0.46, 1.0),
        ),
        CategoryColor::Violet => (
            hsla(0.76, 0.26, 0.19, 1.0),
            hsla(0.76, 0.42, 0.9, 1.0),
            hsla(0.76, 0.4, 0.49, 1.0),
        ),
        CategoryColor::Cyan => (
            hsla(0.52, 0.28, 0.18, 1.0),
            hsla(0.52, 0.43, 0.88, 1.0),
            hsla(0.52, 0.44, 0.45, 1.0),
        ),
        CategoryColor::Blue => (
            hsla(0.59, 0.27, 0.19, 1.0),
            hsla(0.59, 0.42, 0.9, 1.0),
            hsla(0.59, 0.43, 0.48, 1.0),
        ),
        CategoryColor::Orange => (
            hsla(0.08, 0.3, 0.18, 1.0),
            hsla(0.08, 0.45, 0.9, 1.0),
            hsla(0.08, 0.48, 0.46, 1.0),
        ),
        CategoryColor::Rose => (
            hsla(0.94, 0.28, 0.18, 1.0),
            hsla(0.94, 0.42, 0.9, 1.0),
            hsla(0.94, 0.44, 0.47, 1.0),
        ),
        CategoryColor::Magenta => (
            hsla(0.86, 0.27, 0.18, 1.0),
            hsla(0.86, 0.42, 0.9, 1.0),
            hsla(0.86, 0.44, 0.48, 1.0),
        ),
        CategoryColor::Indigo => (
            hsla(0.66, 0.27, 0.19, 1.0),
            hsla(0.66, 0.42, 0.9, 1.0),
            hsla(0.66, 0.42, 0.5, 1.0),
        ),
        CategoryColor::Teal => (
            hsla(0.47, 0.27, 0.18, 1.0),
            hsla(0.47, 0.42, 0.88, 1.0),
            hsla(0.47, 0.43, 0.46, 1.0),
        ),
        CategoryColor::Slate => (
            hsla(0.58, 0.12, 0.2, 1.0),
            hsla(0.58, 0.2, 0.9, 1.0),
            hsla(0.58, 0.22, 0.5, 1.0),
        ),
    }
}
