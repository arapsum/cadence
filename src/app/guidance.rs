use gpui::{Context, IntoElement, Window, div, prelude::*, px};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Placement, StyledExt as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    group_box::GroupBoxVariant,
    setting::{SettingField, SettingGroup, SettingItem, SettingPage, Settings},
    window_paddings,
};

use crate::{domain::format_time, store::TimetableRepository};

use super::{
    appearance::AppearanceControls,
    categories::CategoryManager,
    history::{CalendarChange, ChangeKind},
    state::{CadenceView, HistoryEffect},
    style::{category_dot, dialog_margin_top},
    toolbar::FilterOption,
};

impl CadenceView {
    /// Opens the chronological agenda for the events currently in view.
    pub(in crate::app) fn open_agenda(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(snapshot) = self.snapshot.clone() else {
            return;
        };
        let events = snapshot.surface(self.state.view_mode()).events.clone();
        let categories = snapshot.categories;
        window.open_sheet_at(Placement::Right, cx, move |sheet, _, cx| {
            sheet.title("Agenda").size(px(420.0)).child(
                div()
                    .v_flex()
                    .gap_3()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Events in the current calendar range"),
                    )
                    .when(events.is_empty(), |this| {
                        this.child(
                            div()
                                .py_6()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child("No events here yet. Create one from an empty time slot."),
                        )
                    })
                    .children(events.iter().map(|event| {
                        let category = categories
                            .iter()
                            .find(|category| category.id() == event.category_id());
                        let category_name =
                            category.map_or("Uncategorised", |category| category.name());
                        let dot = category.map(|category| {
                            category_dot(Some(category.color_token())).into_any_element()
                        });
                        div()
                            .v_flex()
                            .gap_1()
                            .p_3()
                            .rounded_md()
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .children(dot)
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(category_name.to_owned()),
                                    )
                                    .child(
                                        div()
                                            .ml_auto()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(event.date().to_string()),
                                    ),
                            )
                            .child(div().font_medium().child(event.title().to_owned()))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!(
                                        "{} – {}",
                                        format_time(
                                            event.start_time(),
                                            crate::domain::ClockFormat::TwelveHour
                                        ),
                                        format_time(
                                            event.end_time(),
                                            crate::domain::ClockFormat::TwelveHour
                                        )
                                    )),
                            )
                    })),
            )
        });
    }

    /// Opens the persisted application-preferences dialog.
    pub(in crate::app) fn open_settings(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let week_start = format!("{:?}", self.settings.week_starts_on());
        let clock_format = match self.settings.clock_format() {
            crate::domain::ClockFormat::TwelveHour => "12-hour",
            crate::domain::ClockFormat::TwentyFourHour => "24-hour",
        };
        let owner = cx.entity().downgrade();
        let view = cx.entity();
        let category_manager = cx.new(|cx| CategoryManager::new(view, cx));
        let appearance_controls = cx
            .new(|cx| AppearanceControls::new(owner.clone(), self.appearance.clone(), window, cx));
        window.open_dialog(cx, move |dialog, dialog_window, _| {
            let viewport = dialog_window.viewport_size();
            let padding = window_paddings(dialog_window);
            let available_width = viewport.width - padding.left - padding.right;
            let available_height = viewport.height - padding.top - padding.bottom;
            let dialog_width = (available_width - px(48.0)).clamp(px(560.0), px(900.0));
            let dialog_height = (available_height - px(96.0)).clamp(px(440.0), px(620.0));
            let margin_top = dialog_margin_top(available_height, dialog_height);

            let general_page = general_settings_page(&owner, week_start.clone(), clock_format);
            let notifications_page = notifications_settings_page(&owner);
            let categories_page = categories_settings_page(category_manager.clone());
            let appearance_page = appearance_settings_page(appearance_controls.clone());

            let settings = Settings::new("cadence-settings")
                .sidebar_width(px(190.0))
                .with_group_variant(GroupBoxVariant::Outline)
                .pages([
                    general_page,
                    appearance_page,
                    notifications_page,
                    categories_page,
                ]);

            dialog
                .margin_top(margin_top)
                .title("Settings")
                .w(dialog_width)
                .h(dialog_height)
                .child(div().size_full().overflow_hidden().child(settings))
                .footer(
                    div().flex().justify_end().child(
                        Button::new("settings-close")
                            .primary()
                            .label("Done")
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                    ),
                )
        });
    }

    fn set_notifications(&mut self, enabled: bool, cx: &mut Context<'_, Self>) {
        let Ok(before) = self.repository.snapshot() else {
            return;
        };
        self.notifications_enabled = enabled;
        let _ = self.repository.replace_preferences(self.preferences());
        self.persist_snapshot(before, self.rollback_view_state(), HistoryEffect::None, cx);
        cx.notify();
    }

    fn set_reduce_motion(&mut self, enabled: bool, cx: &mut Context<'_, Self>) {
        let Ok(before) = self.repository.snapshot() else {
            return;
        };
        self.reduce_motion = enabled;
        cx.set_reduce_motion(enabled);
        let _ = self.repository.replace_preferences(self.preferences());
        self.persist_snapshot(before, self.rollback_view_state(), HistoryEffect::None, cx);
        cx.notify();
    }

    pub(in crate::app) fn set_appearance_mode(
        &mut self,
        mode: crate::store::AppearanceMode,
        cx: &mut Context<'_, Self>,
    ) {
        self.update_appearance(|appearance| appearance.mode = mode, cx);
    }

    pub(in crate::app) fn set_light_theme(&mut self, theme: String, cx: &mut Context<'_, Self>) {
        self.update_appearance(|appearance| appearance.light_theme = theme, cx);
    }

    pub(in crate::app) fn set_dark_theme(&mut self, theme: String, cx: &mut Context<'_, Self>) {
        self.update_appearance(|appearance| appearance.dark_theme = theme, cx);
    }

    pub(in crate::app) fn set_font_family(&mut self, family: String, cx: &mut Context<'_, Self>) {
        self.update_appearance(|appearance| appearance.font_family = family, cx);
    }

    pub(in crate::app) fn set_font_size(&mut self, size: u16, cx: &mut Context<'_, Self>) {
        if !crate::store::AppearancePreferences::FONT_SIZES.contains(&size) {
            return;
        }
        self.update_appearance(|appearance| appearance.font_size = size, cx);
    }

    pub(in crate::app) fn reset_appearance(&mut self, cx: &mut Context<'_, Self>) {
        self.update_appearance(
            |appearance| *appearance = crate::store::AppearancePreferences::default(),
            cx,
        );
    }

    fn update_appearance(
        &mut self,
        update: impl FnOnce(&mut crate::store::AppearancePreferences),
        cx: &mut Context<'_, Self>,
    ) {
        if !self.is_interactive() {
            return;
        }
        let Ok(before) = self.repository.snapshot() else {
            return;
        };
        self.appearance = super::appearance::normalize(&self.appearance, cx);
        update(&mut self.appearance);
        super::appearance::apply(&self.appearance, None, cx);
        let _ = self.repository.replace_preferences(self.preferences());
        self.persist_snapshot(before, self.rollback_view_state(), HistoryEffect::None, cx);
        cx.notify();
    }

    pub(in crate::app) fn set_category_visibility(
        &mut self,
        id: crate::domain::CategoryId,
        visible: bool,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Ok(before) = self.repository.snapshot() else {
            return;
        };
        let rollback = self.rollback_view_state();
        let Ok(categories) = self.repository.categories() else {
            return;
        };
        let Some(mut category) = categories
            .iter()
            .find(|category| category.id() == id)
            .cloned()
        else {
            return;
        };
        if !visible
            && category.is_visible()
            && categories
                .iter()
                .filter(|category| category.is_visible())
                .count()
                == 1
        {
            self.show_error("Keep at least one category visible.", window, cx);
            return;
        }
        category.set_visible(visible);
        if self.repository.update_category(category).is_err() {
            return;
        }
        self.sync_category_filter(window, cx);
        let _ = self.repository.replace_preferences(self.preferences());
        self.refresh_snapshot();
        let Ok(after) = self.repository.snapshot() else {
            return;
        };
        self.persist_snapshot(
            before.clone(),
            rollback,
            HistoryEffect::Record(CalendarChange::Snapshot {
                before,
                after,
                kind: ChangeKind::EditCategory,
            }),
            cx,
        );
        cx.notify();
    }

    pub(in crate::app) fn sync_category_filter(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let categories = self.repository.categories().unwrap_or_default();
        if let crate::calendar::CategoryFilter::Only(id) = self.state.category_filter()
            && !categories
                .iter()
                .any(|category| category.id() == id && category.is_visible())
        {
            self.state
                .set_category_filter(crate::calendar::CategoryFilter::All);
        }
        let options = std::iter::once(FilterOption::all())
            .chain(
                categories
                    .iter()
                    .filter(|category| category.is_visible())
                    .map(|category| FilterOption {
                        filter: crate::calendar::CategoryFilter::Only(category.id()),
                        label: category.name().into(),
                        color: Some(category.color_token()),
                    }),
            )
            .collect::<Vec<_>>();
        let selected = self.state.category_filter();
        self.category_filter.update(cx, |select, cx| {
            select.set_items(options, window, cx);
            select.set_selected_value(&selected, window, cx);
        });
    }
}

fn general_settings_page(
    owner: &gpui::WeakEntity<CadenceView>,
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
                                .update(cx, |view, cx| {
                                    view.set_reduce_motion(enabled, cx);
                                })
                                .ok();
                        },
                    ),
                )
                .description("Minimise non-essential interface animation."),
            ),
        ])
}

fn notifications_settings_page(owner: &gpui::WeakEntity<CadenceView>) -> SettingPage {
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
                                .update(cx, |view, cx| {
                                    view.set_notifications(enabled, cx);
                                })
                                .ok();
                        },
                    ),
                )
                .description("Operating-system notification permissions still apply to reminders."),
            ),
        )
}

fn categories_settings_page(manager: gpui::Entity<CategoryManager>) -> SettingPage {
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

fn appearance_settings_page(manager: gpui::Entity<AppearanceControls>) -> SettingPage {
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
