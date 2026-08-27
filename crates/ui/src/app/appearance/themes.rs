use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Hsla, IntoElement, Render, SharedString,
    Subscription, Window, div, prelude::*, px,
};
use gpui_component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, StyledExt as _, ThemeMode,
    ThemeRegistry,
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
};

use crate::{
    app::state::CadenceView,
    store::{AppearanceMode, AppearancePreferences},
};

use super::AppearancePreviewState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ThemeFilter {
    All,
    Light,
    Dark,
}

impl ThemeFilter {
    fn matches(self, mode: ThemeMode) -> bool {
        match self {
            Self::All => true,
            Self::Light => mode == ThemeMode::Light,
            Self::Dark => mode == ThemeMode::Dark,
        }
    }
}

#[derive(Clone)]
struct ThemeCard {
    name: SharedString,
    mode: ThemeMode,
    swatches: Vec<Hsla>,
}

fn theme_cards(cx: &App) -> Vec<ThemeCard> {
    ThemeRegistry::global(cx)
        .sorted_themes()
        .into_iter()
        .map(|theme| ThemeCard {
            name: theme.name.clone(),
            mode: theme.mode,
            swatches: theme_swatches(theme),
        })
        .collect()
}

fn theme_swatches(theme: &gpui_component::ThemeConfig) -> Vec<Hsla> {
    let colors = &theme.colors;
    [
        colors.background.as_deref(),
        colors.foreground.as_deref(),
        colors.primary.as_deref(),
        colors.accent.as_deref(),
        colors.success.as_deref(),
        colors.warning.as_deref(),
        colors.danger.as_deref(),
    ]
    .into_iter()
    .filter_map(|color| color.and_then(|value| gpui_component::try_parse_color(value).ok()))
    .collect()
}

/// Browse bundled themes with mode filters and reversible application previews.
pub struct ThemeBrowser {
    owner: gpui::WeakEntity<CadenceView>,
    preview: Entity<AppearancePreviewState>,
    search: Entity<InputState>,
    filter: ThemeFilter,
    themes: Vec<ThemeCard>,
    focus: FocusHandle,
    theme_focus: Vec<FocusHandle>,
    subscriptions: Vec<Subscription>,
}

impl ThemeBrowser {
    pub fn new(
        owner: &Entity<CadenceView>,
        preview: Entity<AppearancePreviewState>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let search = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search themes…")
                .default_value("")
        });
        let themes = theme_cards(cx);
        let focus = cx.focus_handle();
        let theme_focus = themes.iter().map(|_| cx.focus_handle()).collect::<Vec<_>>();
        let mut browser = Self {
            owner: owner.downgrade(),
            preview,
            search,
            filter: ThemeFilter::All,
            themes,
            focus,
            theme_focus,
            subscriptions: Vec::new(),
        };
        let search_entity = browser.search.clone();
        browser
            .subscriptions
            .push(cx.subscribe(&search_entity, |_, _, _: &InputEvent, cx| cx.notify()));
        let focus_handles = browser.theme_focus.clone();
        for (index, handle) in focus_handles.iter().enumerate() {
            let preview = browser.preview.clone();
            browser
                .subscriptions
                .push(cx.on_focus(handle, window, move |_, _, cx| {
                    preview.update(cx, |state, cx| {
                        if let Some((name, mode)) = theme_cards_for_focus(index, cx) {
                            state.preview_theme(&name, mode, cx);
                        }
                    });
                }));
        }
        let preview = browser.preview.clone();
        browser
            .subscriptions
            .push(cx.on_focus_out(&browser.focus, window, move |_, _, _, cx| {
                preview.update(cx, AppearancePreviewState::restore);
            }));
        browser
    }
}

fn theme_cards_for_focus(index: usize, cx: &App) -> Option<(String, ThemeMode)> {
    ThemeRegistry::global(cx)
        .sorted_themes()
        .get(index)
        .map(|theme| (theme.name.to_string(), theme.mode))
}

impl Render for ThemeBrowser {
    #[allow(clippy::too_many_lines)]
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let interactive = self
            .owner
            .upgrade()
            .is_some_and(|owner| owner.read(cx).is_interactive());
        let query = self.search.read(cx).value().to_lowercase();
        let visible = self
            .themes
            .iter()
            .enumerate()
            .filter(|(_, theme)| {
                self.filter.matches(theme.mode)
                    && (query.is_empty() || theme.name.to_lowercase().contains(&query))
            })
            .collect::<Vec<_>>();
        let visible_count = visible.len();
        let preview = self.preview.read(cx);
        let effective = preview.effective().clone();
        let previewing = preview.is_previewing();
        let browser = cx.entity().downgrade();
        let filter_button = |filter: ThemeFilter, label: &'static str| {
            let selected = self.filter == filter;
            let browser = browser.clone();
            Button::new(format!("theme-filter-{label}"))
                .small()
                .when(selected, Button::primary)
                .when(!selected, Button::outline)
                .label(label)
                .disabled(!interactive)
                .on_click(move |_, _, cx| {
                    browser
                        .update(cx, |this, cx| {
                            this.filter = filter;
                            cx.notify();
                        })
                        .ok();
                })
        };
        div()
            .id("themes-browser")
            .track_focus(&self.focus)
            .v_flex()
            .gap_4()
            .w_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(Input::new(&self.search).appearance(false).w(px(300.0)))
                    .child(
                        div()
                            .ml_auto()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{visible_count} available")),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_xs().font_medium().child("Appearance mode"))
                    .children(
                        [
                            (AppearanceMode::System, "System"),
                            (AppearanceMode::Light, "Light"),
                            (AppearanceMode::Dark, "Dark"),
                        ]
                        .into_iter()
                        .map(|(mode, label)| {
                            let selected = effective.mode == mode;
                            let preview_enter = self.preview.clone();
                            let preview_exit = self.preview.clone();
                            let preview_click = self.preview.clone();
                            Button::new(format!("appearance-mode-{label}"))
                                .small()
                                .when(selected, Button::primary)
                                .when(!selected, Button::outline)
                                .label(label)
                                .disabled(!interactive)
                                .on_hover(move |hovered, _, cx| {
                                    if *hovered {
                                        preview_enter.update(cx, |state, cx| {
                                            state.preview_mode(mode, cx);
                                        });
                                    } else {
                                        preview_exit.update(cx, AppearancePreviewState::restore);
                                    }
                                })
                                .on_click(move |_, _, cx| {
                                    if interactive {
                                        preview_click
                                            .update(cx, |state, cx| state.commit_mode(mode, cx));
                                    }
                                })
                        }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(filter_button(ThemeFilter::All, "All"))
                    .child(filter_button(ThemeFilter::Light, "Light"))
                    .child(filter_button(ThemeFilter::Dark, "Dark"))
                    .child(
                        div()
                            .ml_auto()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Hover a theme to preview it"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_3()
                    .children(visible.into_iter().map(|(index, theme)| {
                        render_theme_card(
                            index,
                            theme,
                            &effective,
                            previewing,
                            interactive,
                            &self.preview,
                            &self.theme_focus[index],
                            browser.clone(),
                            cx,
                        )
                    })),
            )
            .when(visible_count == 0, |this| {
                this.child(
                    div()
                        .py_8()
                        .w_full()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("No bundled themes match this search."),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pt_2()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Theme changes are saved when you click a card."),
                    )
                    .child(
                        Button::new("reset-themes")
                            .outline()
                            .small()
                            .icon(IconName::Undo2)
                            .label("Reset themes")
                            .disabled(!interactive)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.preview
                                    .update(cx, AppearancePreviewState::reset_themes);
                            })),
                    ),
            )
            .when(window.viewport_size().width < px(700.0), |this| {
                this.w_full()
            })
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn render_theme_card(
    index: usize,
    theme: &ThemeCard,
    appearance: &AppearancePreferences,
    previewing: bool,
    interactive: bool,
    preview: &Entity<AppearancePreviewState>,
    focus: &FocusHandle,
    _browser: gpui::WeakEntity<ThemeBrowser>,
    cx: &Context<'_, ThemeBrowser>,
) -> impl IntoElement {
    let is_selected = match theme.mode {
        ThemeMode::Light => appearance.light_theme == theme.name,
        ThemeMode::Dark => appearance.dark_theme == theme.name,
    };
    let is_preview = previewing
        && match theme.mode {
            ThemeMode::Light => appearance.light_theme == theme.name,
            ThemeMode::Dark => appearance.dark_theme == theme.name,
        };
    let name = theme.name.to_string();
    let mode = theme.mode;
    let preview_enter = preview.clone();
    let preview_exit = preview.clone();
    let preview_click = preview.clone();
    let preview_key = preview.clone();
    let hover_name = name.clone();
    let click_name = name.clone();
    let key_name = name;
    let swatches = theme.swatches.clone();
    let card_width = px(248.0);
    div()
        .id(format!("theme-card-{}", theme.name))
        .track_focus(focus)
        .tab_index(0)
        .w(card_width)
        .min_h(px(154.0))
        .v_flex()
        .gap_2()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(if is_selected || is_preview {
            cx.theme().primary
        } else {
            cx.theme().border
        })
        .bg(if is_preview {
            cx.theme().primary.opacity(0.1)
        } else {
            cx.theme().secondary.opacity(0.28)
        })
        .cursor_pointer()
        .focus(|this| {
            this.border_color(cx.theme().primary)
                .bg(cx.theme().secondary)
        })
        .hover(|this| this.border_color(cx.theme().primary.opacity(0.85)))
        .aria_label(format!("{} {} theme", theme.name, mode_label(mode)))
        .aria_selected(is_selected)
        .on_hover(move |hovered, _, cx| {
            if *hovered {
                preview_enter.update(cx, |state, cx| state.preview_theme(&hover_name, mode, cx));
            } else {
                preview_exit.update(cx, AppearancePreviewState::restore);
            }
        })
        .on_click(move |_, _, cx| {
            if interactive {
                preview_click.update(cx, |state, cx| state.commit_theme(&click_name, mode, cx));
            }
        })
        .on_key_down(move |event, _, cx| {
            if interactive && matches!(event.keystroke.key.as_str(), "enter" | "return" | "space") {
                cx.stop_propagation();
                preview_key.update(cx, |state, cx| state.commit_theme(&key_name, mode, cx));
            }
        })
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().size(px(8.0)).rounded_full().bg(cx.theme().primary))
                .child(
                    div()
                        .flex_1()
                        .text_sm()
                        .font_medium()
                        .child(theme.name.clone()),
                )
                .when(is_selected, |this| {
                    this.child(Icon::new(IconName::Check).small())
                }),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(mode_label(mode)),
        )
        .child(
            div()
                .flex()
                .gap_1()
                .children(
                    swatches
                        .into_iter()
                        .enumerate()
                        .map(|(swatch_index, color)| {
                            div()
                                .id(format!("theme-swatch-{index}-{swatch_index}"))
                                .size(px(14.0))
                                .rounded_full()
                                .bg(color)
                                .border_1()
                                .border_color(cx.theme().border.opacity(0.7))
                                .into_any_element()
                        }),
                ),
        )
        .child(
            div()
                .mt_auto()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(if is_selected {
                    "Selected"
                } else {
                    "Click to use"
                }),
        )
}

fn mode_label(mode: ThemeMode) -> &'static str {
    if mode.is_dark() { "Dark" } else { "Light" }
}
