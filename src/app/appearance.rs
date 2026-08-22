use gpui::{
    App, AppContext as _, Context, Entity, IntoElement, Render, SharedString, Subscription, Window,
    div, prelude::*, px,
};
use gpui_component::{
    ActiveTheme as _, Disableable as _, IconName, IndexPath, Sizable as _, StyledExt as _, Theme,
    ThemeMode, ThemeRegistry,
    button::Button,
    select::{Select, SelectEvent, SelectItem, SelectState},
};

use crate::store::{AppearanceMode, AppearancePreferences};

use super::state::CadenceView;

const BUNDLED_THEMES: &[&str] = &[
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

fn resolved_font_size(preferred: u16) -> u16 {
    AppearancePreferences::FONT_SIZES
        .contains(&preferred)
        .then_some(preferred)
        .unwrap_or(16)
}

#[derive(Clone)]
struct AppearanceModeOption {
    mode: AppearanceMode,
    label: SharedString,
}

impl SelectItem for AppearanceModeOption {
    type Value = AppearanceMode;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.mode
    }
}

#[derive(Clone)]
struct ThemeOption {
    name: SharedString,
    mode: ThemeMode,
}

impl SelectItem for ThemeOption {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.name.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.name
    }

    fn render(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .size(px(8.0))
                    .rounded_full()
                    .bg(if self.mode.is_dark() {
                        cx.theme().primary
                    } else {
                        cx.theme().accent
                    }),
            )
            .child(self.name.clone())
    }
}

#[derive(Clone)]
struct FontOption {
    family: SharedString,
}

impl SelectItem for FontOption {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.family.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.family
    }

    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .font_family(self.family.clone())
            .child(self.family.clone())
    }
}

#[derive(Clone)]
struct FontSizeOption {
    size: u16,
    label: SharedString,
}

impl SelectItem for FontSizeOption {
    type Value = u16;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.size
    }
}

/// Stateful appearance controls embedded in the settings page.
pub(super) struct AppearanceControls {
    owner: gpui::WeakEntity<CadenceView>,
    mode: Entity<SelectState<Vec<AppearanceModeOption>>>,
    light_theme: Entity<SelectState<Vec<ThemeOption>>>,
    dark_theme: Entity<SelectState<Vec<ThemeOption>>>,
    font_family: Entity<SelectState<Vec<FontOption>>>,
    font_size: Entity<SelectState<Vec<FontSizeOption>>>,
    subscriptions: Vec<Subscription>,
}

impl AppearanceControls {
    pub(super) fn new(
        owner: gpui::WeakEntity<CadenceView>,
        initial: AppearancePreferences,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let modes = vec![
            AppearanceModeOption {
                mode: AppearanceMode::System,
                label: "System".into(),
            },
            AppearanceModeOption {
                mode: AppearanceMode::Light,
                label: "Light".into(),
            },
            AppearanceModeOption {
                mode: AppearanceMode::Dark,
                label: "Dark".into(),
            },
        ];
        let themes = ThemeRegistry::global(cx).sorted_themes();
        let light_themes = themes
            .iter()
            .filter(|theme| theme.mode == ThemeMode::Light)
            .map(|theme| ThemeOption {
                name: theme.name.clone(),
                mode: theme.mode,
            })
            .collect::<Vec<_>>();
        let dark_themes = themes
            .iter()
            .filter(|theme| theme.mode == ThemeMode::Dark)
            .map(|theme| ThemeOption {
                name: theme.name.clone(),
                mode: theme.mode,
            })
            .collect::<Vec<_>>();
        let font_families = font_options(cx);
        let font_sizes = vec![
            FontSizeOption {
                size: 14,
                label: "Small · 14 px".into(),
            },
            FontSizeOption {
                size: 16,
                label: "Default · 16 px".into(),
            },
            FontSizeOption {
                size: 18,
                label: "Large · 18 px".into(),
            },
        ];
        let light_theme = resolved_theme_name(&initial.light_theme, ThemeMode::Light, cx);
        let dark_theme = resolved_theme_name(&initial.dark_theme, ThemeMode::Dark, cx);
        let font_family = resolved_font_family(&initial.font_family, cx);
        let mode_index = match initial.mode {
            AppearanceMode::System => 0,
            AppearanceMode::Light => 1,
            AppearanceMode::Dark => 2,
        };
        let mode = cx.new(|cx| {
            SelectState::new(
                modes,
                Some(IndexPath::default().row(mode_index)),
                window,
                cx,
            )
        });
        let light_theme_state = cx.new(|cx| {
            SelectState::new(
                light_themes.clone(),
                selected_index(&light_themes, |theme| theme.name.as_ref() == light_theme),
                window,
                cx,
            )
            .searchable(true)
        });
        let dark_theme_state = cx.new(|cx| {
            SelectState::new(
                dark_themes.clone(),
                selected_index(&dark_themes, |theme| theme.name.as_ref() == dark_theme),
                window,
                cx,
            )
            .searchable(true)
        });
        let font_state = cx.new(|cx| {
            SelectState::new(
                font_families.clone(),
                selected_index(&font_families, |font| font.family.as_ref() == font_family),
                window,
                cx,
            )
            .searchable(true)
        });
        let size_state = cx.new(|cx| {
            SelectState::new(
                font_sizes.clone(),
                selected_index(&font_sizes, |option| {
                    option.size == resolved_font_size(initial.font_size)
                }),
                window,
                cx,
            )
        });
        let mut controls = Self {
            owner,
            mode,
            light_theme: light_theme_state,
            dark_theme: dark_theme_state,
            font_family: font_state,
            font_size: size_state,
            subscriptions: Vec::new(),
        };
        controls.subscribe(cx);
        controls
    }

    fn subscribe(&mut self, cx: &mut Context<'_, Self>) {
        let owner = self.owner.clone();
        self.subscriptions.push(cx.subscribe(
            &self.mode,
            move |_, _, event: &SelectEvent<Vec<AppearanceModeOption>>, cx| {
                if let SelectEvent::Confirm(Some(mode)) = event {
                    owner
                        .update(cx, |view, cx| view.set_appearance_mode(*mode, cx))
                        .ok();
                }
            },
        ));

        let owner = self.owner.clone();
        self.subscriptions.push(cx.subscribe(
            &self.light_theme,
            move |_, _, event: &SelectEvent<Vec<ThemeOption>>, cx| {
                if let SelectEvent::Confirm(Some(theme)) = event {
                    owner
                        .update(cx, |view, cx| {
                            view.set_light_theme(theme.to_string(), cx);
                        })
                        .ok();
                }
            },
        ));

        let owner = self.owner.clone();
        self.subscriptions.push(cx.subscribe(
            &self.dark_theme,
            move |_, _, event: &SelectEvent<Vec<ThemeOption>>, cx| {
                if let SelectEvent::Confirm(Some(theme)) = event {
                    owner
                        .update(cx, |view, cx| {
                            view.set_dark_theme(theme.to_string(), cx);
                        })
                        .ok();
                }
            },
        ));

        let owner = self.owner.clone();
        self.subscriptions.push(cx.subscribe(
            &self.font_family,
            move |_, _, event: &SelectEvent<Vec<FontOption>>, cx| {
                if let SelectEvent::Confirm(Some(font)) = event {
                    owner
                        .update(cx, |view, cx| {
                            view.set_font_family(font.to_string(), cx);
                        })
                        .ok();
                }
            },
        ));

        let owner = self.owner.clone();
        self.subscriptions.push(cx.subscribe(
            &self.font_size,
            move |_, _, event: &SelectEvent<Vec<FontSizeOption>>, cx| {
                if let SelectEvent::Confirm(Some(size)) = event {
                    owner
                        .update(cx, |view, cx| view.set_font_size(*size, cx))
                        .ok();
                }
            },
        ));
    }

    fn reset(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.owner
            .update(cx, |view, cx| view.reset_appearance(cx))
            .ok();
        self.mode.update(cx, |state, cx| {
            state.set_selected_index(Some(IndexPath::default().row(0)), window, cx);
        });
        self.light_theme.update(cx, |state, cx| {
            state.set_selected_value(&"Default Light".into(), window, cx);
        });
        self.dark_theme.update(cx, |state, cx| {
            state.set_selected_value(&"Default Dark".into(), window, cx);
        });
        self.font_family.update(cx, |state, cx| {
            state.set_selected_value(&".SystemUIFont".into(), window, cx);
        });
        self.font_size.update(cx, |state, cx| {
            state.set_selected_value(&16, window, cx);
        });
    }
}

impl Render for AppearanceControls {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let (appearance, interactive) = self
            .owner
            .upgrade()
            .map(|owner| {
                let view = owner.read(cx);
                (view.appearance.clone(), view.is_interactive())
            })
            .unwrap_or_else(|| (AppearancePreferences::default(), false));
        let reset = cx.listener(|this, _, window, cx| this.reset(window, cx));
        let field_width = px(240.0);
        div()
            .v_flex()
            .gap_3()
            .child(appearance_field(
                "Appearance mode",
                Select::new(&self.mode)
                    .w(field_width)
                    .appearance(false)
                    .disabled(!interactive),
            ))
            .child(appearance_field(
                "Light theme",
                Select::new(&self.light_theme)
                    .w(field_width)
                    .appearance(false)
                    .disabled(!interactive),
            ))
            .child(appearance_field(
                "Dark theme",
                Select::new(&self.dark_theme)
                    .w(field_width)
                    .appearance(false)
                    .disabled(!interactive),
            ))
            .child(appearance_field(
                "Font family",
                Select::new(&self.font_family)
                    .w(field_width)
                    .appearance(false)
                    .disabled(!interactive),
            ))
            .child(appearance_field(
                "Font size",
                Select::new(&self.font_size)
                    .w(field_width)
                    .appearance(false)
                    .disabled(!interactive),
            ))
            .child(
                div()
                    .v_flex()
                    .gap_1()
                    .rounded_md()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().muted.opacity(0.2))
                    .p_3()
                    .font_family(appearance.font_family)
                    .text_size(px(f32::from(resolved_font_size(appearance.font_size))))
                    .child("Aa  Calendar preview")
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Morning routine · 08:00 AM – 09:00 AM"),
                    ),
            )
            .child(
                Button::new("reset-appearance")
                    .outline()
                    .small()
                    .icon(IconName::Undo2)
                    .label("Reset appearance")
                    .disabled(!interactive)
                    .on_click(reset),
            )
            .when(window.viewport_size().width < px(640.0), |this| {
                this.w_full()
            })
    }
}

fn appearance_field<E>(label: &'static str, control: E) -> impl IntoElement
where
    E: IntoElement,
{
    div()
        .v_flex()
        .gap_1()
        .child(div().text_xs().child(label))
        .child(control)
}

fn font_options(cx: &App) -> Vec<FontOption> {
    let mut names = cx.text_system().all_font_names();
    if !names.iter().any(|name| name == ".SystemUIFont") {
        names.push(".SystemUIFont".to_owned());
    }
    names.sort_unstable_by_key(|name| name.to_lowercase());
    names.dedup();
    names
        .into_iter()
        .map(|family| FontOption {
            family: family.into(),
        })
        .collect()
}

fn selected_index<T>(items: &[T], predicate: impl Fn(&T) -> bool) -> Option<IndexPath> {
    items
        .iter()
        .position(predicate)
        .map(|index| IndexPath::default().row(index))
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

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use gpui_component::ThemeSet;

    use super::{
        BUNDLED_THEMES, ThemeMode, ThemeRegistry, normalize, register_themes, resolved_font_size,
    };
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
    fn registering_the_catalog_exposes_both_appearance_modes(cx: &mut TestAppContext) {
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
    fn normalization_replaces_unavailable_appearance_values(cx: &mut TestAppContext) {
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
