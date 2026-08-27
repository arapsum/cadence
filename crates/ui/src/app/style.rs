use gpui::{Hsla, Pixels, div, prelude::*, px};
use gpui_component::Theme;

use crate::domain::CategoryColor;

pub(super) const DAY_HEADER_HEIGHT: f32 = 68.0;
pub(super) const TIME_GUTTER_WIDTH: f32 = 78.0;
pub(super) const MIN_COLUMN_WIDTH: f32 = 92.0;
pub(super) const PIXELS_PER_MINUTE: f32 = 1.5;
pub(super) const PLANE_HEIGHT: f32 = 24.0 * 60.0 * PIXELS_PER_MINUTE;

const DIALOG_CENTER_NUDGE: Pixels = px(20.0);
const CATEGORY_SURFACE_ALPHAS: [f32; 10] =
    [0.18, 0.16, 0.14, 0.12, 0.1, 0.08, 0.06, 0.04, 0.02, 0.0];
const CATEGORY_ACCENT_BLEND_WEIGHTS: [f32; 11] =
    [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
const MIN_TEXT_CONTRAST: f32 = 4.5;
const MIN_ACCENT_CONTRAST: f32 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CategoryPalette {
    pub(super) surface: Hsla,
    pub(super) foreground: Hsla,
    pub(super) border: Hsla,
    pub(super) indicator: Hsla,
}

pub(super) fn dialog_margin_top(available_height: Pixels, dialog_height: Pixels) -> Pixels {
    ((available_height - dialog_height) / 2.0 - DIALOG_CENTER_NUDGE).max(px(0.0))
}

pub(super) fn category_dot(color: Option<CategoryColor>, theme: &Theme) -> impl IntoElement {
    let palette = category_palette(color.unwrap_or(CategoryColor::Blue), theme);
    div()
        .w(px(7.0))
        .h(px(7.0))
        .rounded_full()
        .bg(palette.indicator)
}

pub(super) fn category_palette(color: CategoryColor, theme: &Theme) -> CategoryPalette {
    let seed = category_seed(color, theme);
    let surface = category_surface(seed, theme.background, theme.foreground);
    let indicator = ensure_contrast(
        seed,
        &[theme.background, surface],
        theme.foreground,
        MIN_ACCENT_CONTRAST,
    );

    CategoryPalette {
        surface,
        foreground: theme.foreground,
        border: indicator,
        indicator,
    }
}

fn category_seed(color: CategoryColor, theme: &Theme) -> Hsla {
    match color {
        CategoryColor::Lime => theme.green,
        CategoryColor::Yellow => theme.yellow,
        CategoryColor::Coral => theme.red_light,
        CategoryColor::Violet => theme.magenta_light,
        CategoryColor::Cyan => theme.cyan_light,
        CategoryColor::Blue => theme.blue,
        CategoryColor::Orange => theme.yellow_light,
        CategoryColor::Rose => theme.red,
        CategoryColor::Magenta => theme.magenta,
        CategoryColor::Indigo => theme.blue_light,
        CategoryColor::Teal => theme.cyan,
        CategoryColor::Slate => theme.muted_foreground,
    }
}

fn category_surface(seed: Hsla, background: Hsla, foreground: Hsla) -> Hsla {
    for alpha in CATEGORY_SURFACE_ALPHAS {
        let candidate = background.blend(seed.alpha(alpha));
        if contrast_ratio(foreground, candidate) >= MIN_TEXT_CONTRAST {
            return candidate;
        }
    }
    background
}

fn ensure_contrast(seed: Hsla, backgrounds: &[Hsla], toward: Hsla, minimum: f32) -> Hsla {
    for weight in CATEGORY_ACCENT_BLEND_WEIGHTS {
        let candidate = seed.blend(toward.alpha(weight));
        if backgrounds
            .iter()
            .all(|background| contrast_ratio(candidate, *background) >= minimum)
        {
            return candidate;
        }
    }
    toward
}

fn contrast_ratio(first: Hsla, second: Hsla) -> f32 {
    let first = relative_luminance(first);
    let second = relative_luminance(second);
    let lighter = first.max(second);
    let darker = first.min(second);
    (lighter + 0.05) / (darker + 0.05)
}

fn relative_luminance(color: Hsla) -> f32 {
    let color = color.to_rgb();
    0.2126_f32.mul_add(
        linearize(color.r),
        0.7152_f32.mul_add(linearize(color.g), 0.0722 * linearize(color.b)),
    )
}

fn linearize(component: f32) -> f32 {
    if component <= 0.04045 {
        component / 12.92
    } else {
        ((component + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use gpui::hsla;
    use gpui_component::{Theme, ThemeColor, ThemeSet};

    use super::{
        MIN_ACCENT_CONTRAST, MIN_TEXT_CONTRAST, category_palette, category_seed, contrast_ratio,
    };
    use crate::domain::CategoryColor;

    #[test]
    fn category_tokens_map_to_semantic_theme_hues() {
        let theme = test_theme();
        let expected = [
            (CategoryColor::Lime, theme.green),
            (CategoryColor::Yellow, theme.yellow),
            (CategoryColor::Coral, theme.red_light),
            (CategoryColor::Violet, theme.magenta_light),
            (CategoryColor::Cyan, theme.cyan_light),
            (CategoryColor::Blue, theme.blue),
            (CategoryColor::Orange, theme.yellow_light),
            (CategoryColor::Rose, theme.red),
            (CategoryColor::Magenta, theme.magenta),
            (CategoryColor::Indigo, theme.blue_light),
            (CategoryColor::Teal, theme.cyan),
            (CategoryColor::Slate, theme.muted_foreground),
        ];

        for (color, expected_seed) in expected {
            assert_eq!(category_seed(color, &theme), expected_seed);
        }
    }

    #[test]
    fn every_bundled_theme_produces_accessible_category_roles() {
        for source in super::super::appearance::BUNDLED_THEMES {
            let theme_set = serde_json::from_str::<ThemeSet>(source).expect("bundled theme parses");
            for config in theme_set.themes {
                let theme_name = config.name.clone();
                let mut theme = Theme::default();
                theme.apply_config(&Rc::new(config));

                for color in CategoryColor::ALL {
                    let palette = category_palette(color, &theme);
                    assert!(
                        contrast_ratio(palette.foreground, palette.surface) >= MIN_TEXT_CONTRAST,
                        "{theme_name} {color:?} text contrast"
                    );
                    assert!(
                        contrast_ratio(palette.indicator, theme.background) >= MIN_ACCENT_CONTRAST,
                        "{theme_name} {color:?} indicator contrast"
                    );
                    assert!(
                        contrast_ratio(palette.border, palette.surface) >= MIN_ACCENT_CONTRAST,
                        "{theme_name} {color:?} border contrast"
                    );
                }
            }
        }
    }

    #[test]
    fn palette_tracks_theme_changes_without_changing_the_category_token() {
        let mut theme = test_theme();
        let before = category_palette(CategoryColor::Blue, &theme);
        theme.blue = hsla(0.92, 0.8, 0.48, 1.0);
        let after = category_palette(CategoryColor::Blue, &theme);

        assert_ne!(before, after);
    }

    fn test_theme() -> Theme {
        Theme::from(&ThemeColor {
            background: hsla(0.0, 0.0, 0.98, 1.0),
            foreground: hsla(0.0, 0.0, 0.08, 1.0),
            muted_foreground: hsla(0.58, 0.12, 0.38, 1.0),
            red: hsla(0.0, 0.72, 0.48, 1.0),
            red_light: hsla(0.02, 0.68, 0.58, 1.0),
            green: hsla(0.32, 0.62, 0.36, 1.0),
            green_light: hsla(0.28, 0.58, 0.5, 1.0),
            blue: hsla(0.6, 0.72, 0.48, 1.0),
            blue_light: hsla(0.65, 0.68, 0.58, 1.0),
            yellow: hsla(0.14, 0.76, 0.42, 1.0),
            yellow_light: hsla(0.09, 0.72, 0.54, 1.0),
            magenta: hsla(0.86, 0.64, 0.48, 1.0),
            magenta_light: hsla(0.77, 0.6, 0.58, 1.0),
            cyan: hsla(0.49, 0.62, 0.38, 1.0),
            cyan_light: hsla(0.53, 0.58, 0.52, 1.0),
            ..ThemeColor::default()
        })
    }
}
