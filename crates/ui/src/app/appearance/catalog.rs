use gpui::{App, Window, px};
use gpui_component::{Theme, ThemeMode, ThemeRegistry};

use crate::store::{AppearanceMode, AppearancePreferences};

pub(super) const BUNDLED_THEMES: &[&str] = &[
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/adventure.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/alduin.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/asciinema.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/aurora.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/ayu.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/catppuccin.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/everforest.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/fahrenheit.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/flexoki.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/gruvbox.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/harper.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/hybrid.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/jellybeans.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/kibble.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/macos-classic.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/mellifluous.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/molokai.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/solarized.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/spaceduck.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/tokyonight.json"
    )),
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/themes/twilight.json"
    )),
];

/// Registers Cadence's pinned `gpui-component` theme catalog.
pub(super) fn register_themes(cx: &mut App) {
    let registry = ThemeRegistry::global_mut(cx);
    for source in BUNDLED_THEMES {
        if let Err(error) = registry.load_themes_from_str(source) {
            eprintln!("Cadence could not load a bundled theme: {error}");
        }
    }
}

/// Applies persisted appearance preferences to the active GPUI theme.
pub(super) fn apply(
    preferences: &AppearancePreferences,
    mut window: Option<&mut Window>,
    cx: &mut App,
) {
    let preferences = normalize(preferences, cx);
    cx.set_window_appearance(match preferences.mode {
        AppearanceMode::System => None,
        AppearanceMode::Light => Some(gpui::WindowAppearance::Light),
        AppearanceMode::Dark => Some(gpui::WindowAppearance::Dark),
    });

    let system_mode = window.as_ref().map_or_else(
        || ThemeMode::from(cx.window_appearance()),
        |window| ThemeMode::from(window.appearance()),
    );
    let mode = match preferences.mode {
        AppearanceMode::System => system_mode,
        AppearanceMode::Light => ThemeMode::Light,
        AppearanceMode::Dark => ThemeMode::Dark,
    };

    let light_theme = configured_theme(&preferences.light_theme, ThemeMode::Light, cx);
    let dark_theme = configured_theme(&preferences.dark_theme, ThemeMode::Dark, cx);
    {
        let theme = Theme::global_mut(cx);
        theme.light_theme = light_theme;
        theme.dark_theme = dark_theme;
    }

    Theme::change(mode, window.take(), cx);

    let font_family = resolved_font_family(&preferences.font_family, cx);
    let font_size = resolved_font_size(preferences.font_size);
    let theme = Theme::global_mut(cx);
    theme.font_family = font_family.into();
    theme.font_size = px(f32::from(font_size));
    Theme::sync_base(cx);
    cx.refresh_windows();
}

/// Resolves persisted appearance values against the currently available catalog and fonts.
pub(super) fn normalize(preferences: &AppearancePreferences, cx: &App) -> AppearancePreferences {
    AppearancePreferences {
        mode: preferences.mode,
        light_theme: resolved_theme_name(&preferences.light_theme, ThemeMode::Light, cx).to_owned(),
        dark_theme: resolved_theme_name(&preferences.dark_theme, ThemeMode::Dark, cx).to_owned(),
        font_family: resolved_font_family(&preferences.font_family, cx),
        font_size: resolved_font_size(preferences.font_size),
    }
}

fn configured_theme(
    name: &str,
    mode: ThemeMode,
    cx: &App,
) -> std::rc::Rc<gpui_component::ThemeConfig> {
    ThemeRegistry::global(cx)
        .themes()
        .get(name)
        .filter(|config| config.mode == mode)
        .cloned()
        .unwrap_or_else(|| match mode {
            ThemeMode::Light => ThemeRegistry::global(cx).default_light_theme().clone(),
            ThemeMode::Dark => ThemeRegistry::global(cx).default_dark_theme().clone(),
        })
}

fn resolved_font_family(preferred: &str, cx: &App) -> String {
    if preferred == ".SystemUIFont"
        || cx
            .text_system()
            .all_font_names()
            .iter()
            .any(|font| font == preferred)
    {
        preferred.to_owned()
    } else {
        ".SystemUIFont".to_owned()
    }
}

pub(super) fn resolved_font_size(preferred: u16) -> u16 {
    if AppearancePreferences::FONT_SIZES.contains(&preferred) {
        preferred
    } else {
        16
    }
}

fn resolved_theme_name<'a>(preferred: &'a str, mode: ThemeMode, cx: &App) -> &'a str {
    if ThemeRegistry::global(cx)
        .themes()
        .get(preferred)
        .is_some_and(|theme| theme.mode == mode)
    {
        preferred
    } else {
        match mode {
            ThemeMode::Light => "Default Light",
            ThemeMode::Dark => "Default Dark",
        }
    }
}
