use gpui::{App, Context, Entity, Subscription, Window};
use gpui_component::ThemeMode;

use crate::{
    app::state::CadenceView,
    store::{AppearanceMode, AppearancePreferences},
};

/// Coordinates preview state shared by the Themes and Typography pages.
pub struct AppearancePreviewState {
    owner: gpui::WeakEntity<CadenceView>,
    committed: AppearancePreferences,
    last_observed: AppearancePreferences,
    preview: Option<AppearancePreferences>,
    subscriptions: Vec<Subscription>,
}

impl AppearancePreviewState {
    pub fn new(
        owner: &Entity<CadenceView>,
        initial: AppearancePreferences,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let mut state = Self {
            owner: owner.downgrade(),
            committed: initial.clone(),
            last_observed: initial,
            preview: None,
            subscriptions: Vec::new(),
        };
        state
            .subscriptions
            .push(cx.observe_in(owner, window, |state, owner, _, cx| {
                let appearance = owner.read(cx).appearance.clone();
                if appearance != state.last_observed {
                    state.committed = appearance.clone();
                    state.last_observed = appearance;
                    state.preview = None;
                    cx.notify();
                }
            }));
        state
    }

    pub(super) fn effective(&self) -> &AppearancePreferences {
        self.preview.as_ref().unwrap_or(&self.committed)
    }

    pub(super) const fn is_previewing(&self) -> bool {
        self.preview.is_some()
    }

    pub(super) fn preview_theme(
        &mut self,
        name: &str,
        mode: ThemeMode,
        cx: &mut Context<'_, Self>,
    ) {
        let mut candidate = self.committed.clone();
        match mode {
            ThemeMode::Light => name.clone_into(&mut candidate.light_theme),
            ThemeMode::Dark => name.clone_into(&mut candidate.dark_theme),
        }
        candidate.mode = match mode {
            ThemeMode::Light => AppearanceMode::Light,
            ThemeMode::Dark => AppearanceMode::Dark,
        };
        self.preview = Some(candidate.clone());
        self.owner
            .update(cx, |view, cx| view.preview_appearance(&candidate, cx))
            .ok();
        cx.notify();
    }

    pub(super) fn preview_font_family(&mut self, family: &str, cx: &mut Context<'_, Self>) {
        let mut candidate = self.committed.clone();
        family.clone_into(&mut candidate.font_family);
        self.preview = Some(candidate.clone());
        self.owner
            .update(cx, |view, cx| view.preview_appearance(&candidate, cx))
            .ok();
        cx.notify();
    }

    pub(super) fn preview_font_size(&mut self, size: u16, cx: &mut Context<'_, Self>) {
        let mut candidate = self.committed.clone();
        candidate.font_size = size;
        self.preview = Some(candidate.clone());
        self.owner
            .update(cx, |view, cx| view.preview_appearance(&candidate, cx))
            .ok();
        cx.notify();
    }

    pub(super) fn preview_mode(&mut self, mode: AppearanceMode, cx: &mut Context<'_, Self>) {
        let mut candidate = self.committed.clone();
        candidate.mode = mode;
        self.preview = Some(candidate.clone());
        self.owner
            .update(cx, |view, cx| view.preview_appearance(&candidate, cx))
            .ok();
        cx.notify();
    }

    fn commit(&mut self, candidate: &AppearancePreferences, cx: &mut Context<'_, Self>) {
        self.preview = None;
        self.committed = candidate.clone();
        self.last_observed = candidate.clone();
        self.owner
            .update(cx, |view, cx| view.commit_appearance(candidate, cx))
            .ok();
        cx.notify();
    }

    pub(super) fn commit_theme(&mut self, name: &str, mode: ThemeMode, cx: &mut Context<'_, Self>) {
        let mut candidate = self.committed.clone();
        match mode {
            ThemeMode::Light => name.clone_into(&mut candidate.light_theme),
            ThemeMode::Dark => name.clone_into(&mut candidate.dark_theme),
        }
        self.commit(&candidate, cx);
    }

    pub(super) fn commit_font_family(&mut self, family: &str, cx: &mut Context<'_, Self>) {
        let mut candidate = self.committed.clone();
        family.clone_into(&mut candidate.font_family);
        self.commit(&candidate, cx);
    }

    pub(super) fn commit_font_size(&mut self, size: u16, cx: &mut Context<'_, Self>) {
        let mut candidate = self.committed.clone();
        candidate.font_size = size;
        self.commit(&candidate, cx);
    }

    pub(super) fn commit_mode(&mut self, mode: AppearanceMode, cx: &mut Context<'_, Self>) {
        let mut candidate = self.committed.clone();
        candidate.mode = mode;
        self.commit(&candidate, cx);
    }

    pub(super) fn reset_themes(&mut self, cx: &mut Context<'_, Self>) {
        let defaults = AppearancePreferences::default();
        let mut candidate = self.committed.clone();
        candidate.mode = defaults.mode;
        candidate.light_theme = defaults.light_theme;
        candidate.dark_theme = defaults.dark_theme;
        self.commit(&candidate, cx);
    }

    pub(super) fn reset_typography(&mut self, cx: &mut Context<'_, Self>) {
        let defaults = AppearancePreferences::default();
        let mut candidate = self.committed.clone();
        candidate.font_family = defaults.font_family;
        candidate.font_size = defaults.font_size;
        self.commit(&candidate, cx);
    }

    pub(super) fn restore(&mut self, cx: &mut Context<'_, Self>) {
        if self.preview.take().is_some() {
            self.owner.update(cx, CadenceView::restore_appearance).ok();
            cx.notify();
        }
    }

    pub fn restore_with_app(&mut self, cx: &mut App) {
        if self.preview.take().is_some() {
            self.owner.update(cx, CadenceView::restore_appearance).ok();
        }
    }
}
