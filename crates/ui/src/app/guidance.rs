use gpui::{Context, IntoElement, Window, div, prelude::*, px};
use gpui_component::{
    ActiveTheme as _, Placement, StyledExt as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
};

use crate::{APPLICATION_NAME, BuildInfo};
use crate::{domain::format_time, store::TimetableRepository};

use super::{
    history::{CalendarChange, ChangeKind},
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
                            category_dot(Some(category.color_token()), cx.theme())
                                .into_any_element()
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

    /// Opens the application identity and support-information dialog.
    pub(in crate::app) fn open_about(window: &mut Window, cx: &mut Context<'_, Self>) {
        let build = BuildInfo::current();
        window.open_dialog(cx, move |dialog, _, cx| {
            dialog
                .title(format!("About {APPLICATION_NAME}"))
                .w(px(420.0))
                .child(
                    div()
                        .id("about-cadence-details")
                        .v_flex()
                        .items_center()
                        .gap_3()
                        .p_6()
                        .child(crate::components::app_icon::render(px(72.0)))
                        .child(div().text_xl().font_semibold().child(APPLICATION_NAME))
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .text_center()
                                .child("A local-first timetable for planning a day and understanding a week."),
                        )
                        .child(
                            div()
                                .v_flex()
                                .items_center()
                                .gap_1()
                                .pt_2()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("Version {}", build.version))
                                .child(format!("Build {}", build.commit))
                                .child(format!("GPUI {}", build.gpui_revision)),
                        ),
                )
                .footer(
                    div().flex().justify_end().child(
                        Button::new("about-close")
                            .primary()
                            .label("Done")
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                    ),
                )
        });
    }

    pub(in crate::app) fn set_notifications(&mut self, enabled: bool, cx: &mut Context<'_, Self>) {
        let Ok(before) = self.repository.snapshot() else {
            return;
        };
        self.notifications_enabled = enabled;
        let _ = self.repository.replace_preferences(self.preferences());
        self.persist_snapshot(before, self.rollback_view_state(), HistoryEffect::None, cx);
        cx.notify();
    }

    pub(in crate::app) fn set_reduce_motion(&mut self, enabled: bool, cx: &mut Context<'_, Self>) {
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

    /// Commits a complete appearance preference candidate from the Settings window.
    pub(in crate::app) fn commit_appearance(
        &mut self,
        appearance: &crate::store::AppearancePreferences,
        cx: &mut Context<'_, Self>,
    ) {
        self.update_appearance(|current| *current = appearance.clone(), cx);
    }

    /// Applies a temporary appearance candidate without touching persistence.
    pub(in crate::app) fn preview_appearance(
        &self,
        appearance: &crate::store::AppearancePreferences,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.is_interactive() {
            return;
        }
        let appearance = super::appearance::normalize(appearance, cx);
        super::appearance::apply(&appearance, None, cx);
        cx.notify();
    }

    /// Restores the last committed appearance after a temporary preview ends.
    #[allow(clippy::needless_pass_by_ref_mut)]
    pub(in crate::app) fn restore_appearance(&mut self, cx: &mut Context<'_, Self>) {
        super::appearance::apply(&self.appearance, None, cx);
        cx.notify();
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
                before: Box::new(before),
                after: Box::new(after),
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
