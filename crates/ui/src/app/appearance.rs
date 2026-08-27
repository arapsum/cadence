mod catalog;
mod preview;
mod themes;
mod typography;

use crate::store::AppearancePreferences;

pub(super) use preview::AppearancePreviewState;
pub(super) use themes::ThemeBrowser;
pub(super) use typography::TypographyBrowser;

pub(super) fn register_themes(cx: &mut gpui::App) {
    catalog::register_themes(cx);
}

pub(super) fn apply(
    preferences: &AppearancePreferences,
    window: Option<&mut gpui::Window>,
    cx: &mut gpui::App,
) {
    catalog::apply(preferences, window, cx);
}

pub(super) fn normalize(
    preferences: &AppearancePreferences,
    cx: &gpui::App,
) -> AppearancePreferences {
    catalog::normalize(preferences, cx)
}

#[cfg(test)]
pub(super) const BUNDLED_THEMES: &[&str] = catalog::BUNDLED_THEMES;

#[cfg(test)]
pub(super) fn resolved_font_size(preferred: u16) -> u16 {
    catalog::resolved_font_size(preferred)
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use gpui_component::{ThemeMode, ThemeRegistry, ThemeSet};

    use super::{BUNDLED_THEMES, normalize, register_themes, resolved_font_size};
    use crate::store::{AppearanceMode, AppearancePreferences};

    #[test]
    fn bundled_theme_sets_parse_with_at_least_one_theme() {
        assert_eq!(BUNDLED_THEMES.len(), 21);
        for source in BUNDLED_THEMES {
            let set: ThemeSet = serde_json::from_str(source).expect("bundled theme is valid JSON");
            assert!(!set.themes.is_empty());
        }
    }

    #[test]
    fn unsupported_font_sizes_fall_back_to_the_default_preset() {
        assert_eq!(resolved_font_size(14), 14);
        assert_eq!(resolved_font_size(16), 16);
        assert_eq!(resolved_font_size(18), 18);
        assert_eq!(resolved_font_size(15), 16);
        assert_eq!(resolved_font_size(24), 16);
    }

    #[gpui::test]
    fn registering_the_catalog_exposes_both_appearance_modes(cx: &TestAppContext) {
        cx.update(gpui_component::init);
        cx.update(|cx| {
            register_themes(cx);
            let registry = ThemeRegistry::global(cx);
            assert!(registry.themes().len() >= 30);
            assert!(
                registry
                    .themes()
                    .values()
                    .any(|theme| theme.mode == ThemeMode::Light)
            );
            assert!(
                registry
                    .themes()
                    .values()
                    .any(|theme| theme.mode == ThemeMode::Dark)
            );
        });
    }

    #[gpui::test]
    fn normalization_replaces_unavailable_appearance_values(cx: &TestAppContext) {
        cx.update(gpui_component::init);
        cx.update(|cx| {
            register_themes(cx);
            let normalized = normalize(
                &AppearancePreferences {
                    mode: AppearanceMode::Dark,
                    light_theme: "Missing light".to_owned(),
                    dark_theme: "Missing dark".to_owned(),
                    font_family: "Missing font".to_owned(),
                    font_size: 15,
                },
                cx,
            );
            assert_eq!(normalized.light_theme, "Default Light");
            assert_eq!(normalized.dark_theme, "Default Dark");
            assert_eq!(normalized.font_family, ".SystemUIFont");
            assert_eq!(normalized.font_size, 16);
        });
    }
}
