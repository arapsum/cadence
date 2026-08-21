use gpui::{Context, IntoElement, Window, div, prelude::*, px};
use gpui_component::{
    ActiveTheme as _, Placement, StyledExt as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
};

use crate::{domain::format_time, store::TimetableRepository};

use super::{
    state::{CadenceView, HistoryEffect},
    style::category_dot,
    toolbar::FilterOption,
};

impl CadenceView {
    /// Opens the chronological agenda for the events currently in view.
    pub(in crate::app) fn open_agenda(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(snapshot) = self.snapshot.clone() else {
            return;
        };
        let events = snapshot.events;
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
        let notifications_enabled = self.notifications_enabled;
        let reduce_motion = self.reduce_motion;
        let categories = self.repository.categories().unwrap_or_default();
        let general_description = format!(
            "Week starts on {:?} · {} clock",
            self.settings.week_starts_on(),
            match self.settings.clock_format() {
                crate::domain::ClockFormat::TwelveHour => "12-hour",
                crate::domain::ClockFormat::TwentyFourHour => "24-hour",
            }
        );
        let owner = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            let notification_owner = owner.clone();
            let motion_owner = owner.clone();
            dialog
                .title("Settings")
                .w(px(520.0))
                .child(
                    div()
                        .v_flex()
                        .gap_5()
                        .p_1()
                        .child(settings_section(
                            "General",
                            general_description.clone(),
                            Button::new("toggle-reduce-motion")
                                .outline()
                                .label(if reduce_motion { "Reduce motion: On" } else { "Reduce motion: Off" })
                                .on_click(move |_, _, cx| {
                                    notification_owner.update(cx, |view, cx| view.set_reduce_motion(!view.reduce_motion, cx)).ok();
                                }),
                        ))
                        .child(settings_section(
                            "Notifications",
                            "Reminders are delivered only while Cadence is running. Operating-system permissions still apply.".to_owned(),
                            Button::new("toggle-notifications")
                                .outline()
                                .label(if notifications_enabled { "Desktop reminders: On" } else { "Desktop reminders: Off" })
                                .on_click(move |_, _, cx| {
                                    motion_owner.update(cx, |view, cx| view.set_notifications(!view.notifications_enabled, cx)).ok();
                                }),
                        ))
                        .child(
                            div()
                                .v_flex()
                                .gap_2()
                                .child(div().font_medium().child("Categories"))
                                .child(div().text_sm().text_color(cx.theme().muted_foreground).child("Categories carry a text label as well as a color."))
                                .children(categories.iter().map(|category| {
                                    let category_id = category.id();
                                    let category_owner = owner.clone();
                                    div().flex().items_center().gap_2()
                                        .child(category_dot(Some(category.color_token())))
                                        .child(div().flex_1().child(category.name().to_owned()))
                                        .child(
                                            Button::new(format!("toggle-category-{category_id}"))
                                                .outline()
                                                .label(if category.is_visible() { "Hide" } else { "Show" })
                                                .on_click(move |_, window, cx| {
                                                    category_owner.update(cx, |view, cx| view.toggle_category_visibility(category_id, window, cx)).ok();
                                                }),
                                        )
                                })),
                        ),
                )
                .footer(
                    div().flex().justify_end().child(
                        Button::new("settings-close").primary().label("Done").on_click(|_, window, cx| window.close_dialog(cx)),
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

    fn toggle_category_visibility(
        &mut self,
        id: crate::domain::CategoryId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Ok(before) = self.repository.snapshot() else {
            return;
        };
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
        if category.is_visible()
            && categories
                .iter()
                .filter(|category| category.is_visible())
                .count()
                == 1
        {
            self.show_error("Keep at least one category visible.", window, cx);
            return;
        }
        category.set_visible(!category.is_visible());
        if self.repository.update_category(category).is_err() {
            return;
        }
        self.sync_category_filter(window, cx);
        self.refresh_snapshot();
        self.persist_snapshot(before, self.rollback_view_state(), HistoryEffect::None, cx);
        cx.notify();
    }

    fn sync_category_filter(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
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

fn settings_section(title: &'static str, description: String, control: Button) -> impl IntoElement {
    div()
        .v_flex()
        .gap_2()
        .child(div().font_medium().child(title))
        .child(div().text_sm().child(description))
        .child(control)
}
