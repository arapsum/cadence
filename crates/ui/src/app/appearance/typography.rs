use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Hsla, IntoElement, ListAlignment,
    ListState, Render, Subscription, Window, div, list, prelude::*, px,
};
use gpui_component::{
    ActiveTheme as _, Disableable as _, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
    scroll::ScrollableElement as _,
};

use crate::{app::state::CadenceView, store::AppearancePreferences};

use super::{AppearancePreviewState, catalog::resolved_font_size};

fn font_options(cx: &App) -> Vec<String> {
    let mut names = cx.text_system().all_font_names();
    if !names.iter().any(|name| name == ".SystemUIFont") {
        names.push(".SystemUIFont".to_owned());
    }
    names.sort_unstable_by_key(|name| name.to_lowercase());
    names.dedup();
    names
}

/// Browse installed font families and preview typography changes globally.
pub struct TypographyBrowser {
    owner: gpui::WeakEntity<CadenceView>,
    preview: Entity<AppearancePreviewState>,
    search: Entity<InputState>,
    fonts: Vec<String>,
    focus: FocusHandle,
    font_focus: Vec<FocusHandle>,
    list_state: ListState,
    subscriptions: Vec<Subscription>,
}

impl TypographyBrowser {
    pub fn new(
        owner: &Entity<CadenceView>,
        preview: Entity<AppearancePreviewState>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let search = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search fonts…")
                .default_value("")
        });
        let fonts = font_options(cx);
        let focus = cx.focus_handle();
        let font_focus = fonts.iter().map(|_| cx.focus_handle()).collect::<Vec<_>>();
        let list_state = ListState::new(0, ListAlignment::Top, px(42.0));
        let mut browser = Self {
            owner: owner.downgrade(),
            preview,
            search,
            fonts,
            focus,
            font_focus,
            list_state,
            subscriptions: Vec::new(),
        };
        let search_entity = browser.search.clone();
        browser
            .subscriptions
            .push(cx.subscribe(&search_entity, |_, _, _: &InputEvent, cx| cx.notify()));
        let focus_handles = browser.font_focus.clone();
        let fonts_for_focus = browser.fonts.clone();
        for (index, handle) in focus_handles.iter().enumerate() {
            let preview = browser.preview.clone();
            let family = fonts_for_focus.get(index).cloned();
            browser
                .subscriptions
                .push(cx.on_focus(handle, window, move |_, _, cx| {
                    if let Some(family) = family.as_deref() {
                        preview.update(cx, |state, cx| state.preview_font_family(family, cx));
                    }
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

impl Render for TypographyBrowser {
    #[allow(clippy::too_many_lines)]
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let interactive = self
            .owner
            .upgrade()
            .is_some_and(|owner| owner.read(cx).is_interactive());
        let query = self.search.read(cx).value().to_lowercase();
        let filtered = self
            .fonts
            .iter()
            .enumerate()
            .filter(|(_, family)| query.is_empty() || family.to_lowercase().contains(&query))
            .map(|(index, family)| (index, family.clone()))
            .collect::<Vec<_>>();
        if self.list_state.item_count() != filtered.len() {
            self.list_state.reset(filtered.len());
        }
        let preview = self.preview.read(cx);
        let effective = preview.effective().clone();
        let previewing = preview.is_previewing();
        let filtered_for_list = filtered.clone();
        let preview_entity = self.preview.clone();
        let focus_handles = self.font_focus.clone();
        let browser = cx.entity().downgrade();
        div()
            .id("typography-browser")
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
                            .child(format!("{} fonts", filtered.len())),
                    ),
            )
            .child(
                div()
                    .v_flex()
                    .gap_2()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .p_3()
                    .font_family(effective.font_family.clone())
                    .text_size(px(f32::from(resolved_font_size(effective.font_size))))
                    .child("Aa  Calendar preview")
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Morning routine · 08:00 AM – 09:00 AM"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_xs().font_medium().child("Font size"))
                    .children([14_u16, 16, 18].into_iter().map(|size| {
                        let selected = effective.font_size == size;
                        let preview_enter = preview_entity.clone();
                        let preview_exit = preview_entity.clone();
                        let preview_click = preview_entity.clone();
                        Button::new(format!("font-size-{size}"))
                            .small()
                            .when(selected, Button::primary)
                            .when(!selected, Button::outline)
                            .label(format!("{size} px"))
                            .disabled(!interactive)
                            .on_hover(move |hovered, _, cx| {
                                if *hovered {
                                    preview_enter
                                        .update(cx, |state, cx| state.preview_font_size(size, cx));
                                } else {
                                    preview_exit.update(cx, AppearancePreviewState::restore);
                                }
                            })
                            .on_click(move |_, _, cx| {
                                if interactive {
                                    preview_click
                                        .update(cx, |state, cx| state.commit_font_size(size, cx));
                                }
                            })
                    })),
            )
            .child(
                div()
                    .h(px(300.0))
                    .min_h(px(180.0))
                    .w_full()
                    .rounded_lg()
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        list(self.list_state.clone(), move |item_index, _, cx| {
                            let Some((index, family)) = filtered_for_list.get(item_index).cloned()
                            else {
                                return div().into_any_element();
                            };
                            render_font_row(
                                index,
                                &family,
                                &effective,
                                previewing,
                                interactive,
                                &preview_entity,
                                &focus_handles[index],
                                browser.clone(),
                                cx,
                            )
                            .into_any_element()
                        })
                        .size_full(),
                    )
                    .vertical_scrollbar(&self.list_state),
            )
            .when(filtered.is_empty(), |this| {
                this.child(
                    div()
                        .py_4()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("No installed fonts match this search."),
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
                            .child("Hover or focus a font to preview it."),
                    )
                    .child(
                        Button::new("reset-typography")
                            .outline()
                            .small()
                            .icon(IconName::Undo2)
                            .label("Reset typography")
                            .disabled(!interactive)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.preview
                                    .update(cx, AppearancePreviewState::reset_typography);
                            })),
                    ),
            )
            .when(window.viewport_size().width < px(700.0), |this| {
                this.w_full()
            })
    }
}

#[allow(clippy::too_many_arguments)]
fn render_font_row(
    index: usize,
    family: &str,
    appearance: &AppearancePreferences,
    previewing: bool,
    interactive: bool,
    preview: &Entity<AppearancePreviewState>,
    focus: &FocusHandle,
    _browser: gpui::WeakEntity<TypographyBrowser>,
    cx: &App,
) -> impl IntoElement {
    let selected = appearance.font_family == family;
    let is_preview = previewing && appearance.font_family == family;
    let hover_family = family.to_owned();
    let click_family = family.to_owned();
    let key_family = family.to_owned();
    let display_family = family.to_owned();
    let preview_enter = preview.clone();
    let preview_exit = preview.clone();
    let preview_click = preview.clone();
    let preview_key = preview.clone();
    div()
        .id(format!("font-row-{index}"))
        .track_focus(focus)
        .tab_index(0)
        .h(px(42.0))
        .w_full()
        .px_3()
        .flex()
        .items_center()
        .gap_2()
        .font_family(display_family)
        .rounded_md()
        .bg(if is_preview {
            cx.theme().primary.opacity(0.1)
        } else {
            Hsla::transparent_black()
        })
        .focus(|this| {
            this.bg(cx.theme().secondary)
                .border_1()
                .border_color(cx.theme().primary)
        })
        .hover(|this| this.bg(cx.theme().secondary.opacity(0.7)))
        .aria_label(format!("Font {family}"))
        .aria_selected(selected)
        .on_hover(move |hovered, _, cx| {
            if *hovered {
                preview_enter.update(cx, |state, cx| state.preview_font_family(&hover_family, cx));
            } else {
                preview_exit.update(cx, AppearancePreviewState::restore);
            }
        })
        .on_click(move |_, _, cx| {
            if interactive {
                preview_click.update(cx, |state, cx| state.commit_font_family(&click_family, cx));
            }
        })
        .on_key_down(move |event, _, cx| {
            if interactive && matches!(event.keystroke.key.as_str(), "enter" | "return" | "space") {
                cx.stop_propagation();
                preview_key.update(cx, |state, cx| state.commit_font_family(&key_family, cx));
            }
        })
        .child(div().flex_1().text_sm().child(family.to_owned()))
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(if selected { "Selected" } else { "Preview" }),
        )
}
