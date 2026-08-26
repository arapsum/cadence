use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Global, IntoElement, Render, Subscription,
    WeakEntity, Window, WindowBounds, WindowDecorations, WindowHandle, WindowId, WindowOptions,
    div, prelude::*, px, size,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Root, StyledExt as _, TitleBar, WindowExt as _,
    group_box::GroupBoxVariant,
    setting::{SettingField, SettingGroup, SettingItem, SettingPage, Settings},
};

use crate::{APPLICATION_ID, domain::ClockFormat};

use super::{appearance::AppearanceControls, categories::CategoryManager, state::CadenceView};
use crate::components::title_bar::CadenceTitleBar;

const SETTINGS_WINDOW_CONTEXT: &str = "CadenceSettings";

#[derive(Default)]
struct SettingsWindowRegistry {
    handle: Option<WindowHandle<Root>>,
    owner_window_id: Option<WindowId>,
    _close_subscription: Option<Subscription>,
}

impl Global for SettingsWindowRegistry {}

pub(super) fn init(cx: &mut App) {
    if cx.has_global::<SettingsWindowRegistry>() {
        return;
    }

    cx.set_global(SettingsWindowRegistry::default());
    let close_subscription = cx.on_window_closed(|cx, closed_window_id| {
        let settings_to_close = cx.update_global::<SettingsWindowRegistry, _>(|registry, _| {
            if registry
                .handle
                .is_some_and(|handle| handle.window_id() == closed_window_id)
            {
                registry.handle = None;
                registry.owner_window_id = None;
                return None;
            }

            if registry.owner_window_id == Some(closed_window_id) {
                registry.owner_window_id = None;
                return registry.handle.take();
            }

            None
        });
        if let Some(settings) = settings_to_close {
            cx.defer(move |cx| {
                settings
                    .update(cx, |_, window, _| window.remove_window())
                    .ok();
            });
        }
    });
    cx.update_global::<SettingsWindowRegistry, _>(|registry, _| {
        registry._close_subscription = Some(close_subscription);
    });
}

pub(super) struct SettingsWindow {
    owner: WeakEntity<CadenceView>,
    category_manager: Entity<CategoryManager>,
    appearance_controls: Entity<AppearanceControls>,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl SettingsWindow {
    fn new(owner: Entity<CadenceView>, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let appearance = owner.read(cx).appearance.clone();
        let category_manager = cx.new(|cx| CategoryManager::new(owner.clone(), cx));
        let appearance_controls =
            cx.new(|cx| AppearanceControls::new(owner.clone(), &appearance, window, cx));
        let focus_handle = cx.focus_handle();

        let owner_observer = cx.observe_in(&owner, window, |_, _, _, cx| cx.notify());
        let owner_release = cx.observe_release_in(&owner, window, |_, _, window, _| {
            window.remove_window();
        });

        focus_handle.focus(window, cx);

        Self {
            owner: owner.downgrade(),
            category_manager,
            appearance_controls,
            focus_handle,
            _subscriptions: vec![owner_observer, owner_release],
        }
    }

    fn pages(&self, cx: &App) -> [SettingPage; 4] {
        let (week_start, clock_format) = self
            .owner
            .read_with(cx, |view, _| {
                let clock_format = match view.settings.clock_format() {
                    ClockFormat::TwelveHour => "12-hour",
                    ClockFormat::TwentyFourHour => "24-hour",
                };
                (
                    format!("{:?}", view.settings.week_starts_on()),
                    clock_format,
                )
            })
            .unwrap_or_else(|_| ("Monday".to_owned(), "24-hour"));

        [
            general_settings_page(&self.owner, week_start, clock_format),
            appearance_settings_page(self.appearance_controls.clone()),
            notifications_settings_page(&self.owner),
            categories_settings_page(self.category_manager.clone()),
        ]
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let settings = Settings::new("cadence-settings")
            .sidebar_width(px(190.0))
            .with_group_variant(GroupBoxVariant::Outline)
            .pages(self.pages(cx));

        div()
            .id("settings-window")
            .key_context(SETTINGS_WINDOW_CONTEXT)
            .track_focus(&self.focus_handle)
            .v_flex()
            .relative()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(CadenceTitleBar::new("Settings"))
            .child(div().flex_1().min_h_0().overflow_hidden().child(settings))
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}

impl CadenceView {
    /// Opens or activates the single Settings utility window.
    pub(in crate::app) fn open_settings(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let owner = cx.entity();
        let origin_window = window.window_handle();
        let origin = origin_window.downcast::<Root>();
        open_settings_window(owner, origin_window.window_id(), origin, cx);
    }
}

fn open_settings_window(
    owner: Entity<CadenceView>,
    owner_window_id: WindowId,
    origin: Option<WindowHandle<Root>>,
    cx: &mut App,
) {
    let existing = cx.global::<SettingsWindowRegistry>().handle;
    if let Some(existing) = existing
        && cx
            .windows()
            .iter()
            .any(|window| window.window_id() == existing.window_id())
    {
        cx.defer(move |cx| {
            existing
                .update(cx, |_, window, _| window.activate_window())
                .ok();
        });
        return;
    }

    cx.update_global::<SettingsWindowRegistry, _>(|registry, _| registry.handle = None);
    cx.defer(move |cx| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(900.0), px(650.0)), cx)),
            window_min_size: Some(size(px(640.0), px(480.0))),
            window_decorations: Some(WindowDecorations::Client),
            app_id: Some(APPLICATION_ID.to_owned()),
            ..TitleBar::window_options()
        };

        match cx.open_window(options, move |window, cx| {
            window.set_window_title("Cadence — Settings");
            let settings_window = cx.new(|cx| SettingsWindow::new(owner.clone(), window, cx));
            cx.new(|cx| Root::new(settings_window, window, cx))
        }) {
            Ok(handle) => {
                cx.update_global::<SettingsWindowRegistry, _>(|registry, _| {
                    registry.handle = Some(handle);
                    registry.owner_window_id = Some(owner_window_id);
                });
            }
            Err(error) => {
                eprintln!("Cadence could not open the Settings window: {error:#}");
                if let Some(origin) = origin {
                    origin
                        .update(cx, |_, window, cx| {
                            window.push_notification(
                                "Cadence could not open the Settings window.",
                                cx,
                            );
                        })
                        .ok();
                }
            }
        }
    });
}

fn general_settings_page(
    owner: &WeakEntity<CadenceView>,
    week_start: String,
    clock_format: &'static str,
) -> SettingPage {
    let motion_reader = owner.clone();
    let motion_writer = owner.clone();
    SettingPage::new("General")
        .icon(Icon::new(IconName::Settings2))
        .default_open(true)
        .resettable(false)
        .description("Calendar display and accessibility preferences.")
        .groups([
            SettingGroup::new().title("Calendar").items([
                SettingItem::new(
                    "Week starts on",
                    SettingField::render(move |_, _, cx| {
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(week_start.clone())
                    }),
                )
                .description("The first column shown in week view."),
                SettingItem::new(
                    "Clock format",
                    SettingField::render(move |_, _, cx| {
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(clock_format)
                    }),
                )
                .description("How event and time-grid labels are displayed."),
            ]),
            SettingGroup::new().title("Accessibility").item(
                SettingItem::new(
                    "Reduce motion",
                    SettingField::switch(
                        move |cx| {
                            motion_reader
                                .read_with(cx, |view, _| view.reduce_motion)
                                .unwrap_or(false)
                        },
                        move |enabled, cx| {
                            motion_writer
                                .update(cx, |view, cx| view.set_reduce_motion(enabled, cx))
                                .ok();
                        },
                    ),
                )
                .description("Minimise non-essential interface animation."),
            ),
        ])
}

fn notifications_settings_page(owner: &WeakEntity<CadenceView>) -> SettingPage {
    let notification_reader = owner.clone();
    let notification_writer = owner.clone();
    SettingPage::new("Notifications")
        .icon(Icon::new(IconName::Bell))
        .resettable(false)
        .description("Control reminders delivered while Cadence is running.")
        .group(
            SettingGroup::new().title("Desktop reminders").item(
                SettingItem::new(
                    "Enable notifications",
                    SettingField::switch(
                        move |cx| {
                            notification_reader
                                .read_with(cx, |view, _| view.notifications_enabled)
                                .unwrap_or(false)
                        },
                        move |enabled, cx| {
                            notification_writer
                                .update(cx, |view, cx| view.set_notifications(enabled, cx))
                                .ok();
                        },
                    ),
                )
                .description("Operating-system notification permissions still apply to reminders."),
            ),
        )
}

fn categories_settings_page(manager: Entity<CategoryManager>) -> SettingPage {
    SettingPage::new("Categories")
        .icon(Icon::new(IconName::Palette))
        .resettable(false)
        .description("Create, edit, hide, and safely remove calendar categories.")
        .group(
            SettingGroup::new()
                .title("Calendar categories")
                .item(SettingItem::render(move |_, _, _| manager.clone())),
        )
}

fn appearance_settings_page(manager: Entity<AppearanceControls>) -> SettingPage {
    SettingPage::new("Appearance")
        .icon(Icon::new(IconName::Palette))
        .resettable(false)
        .description("Choose the theme, appearance mode, and application typography.")
        .group(
            SettingGroup::new()
                .title("Theme and typography")
                .item(SettingItem::render(move |_, _, _| manager.clone())),
        )
}
