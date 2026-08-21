use gpui::{App, Context, IntoElement, SharedString, Window, div, prelude::*, px};
use gpui_component::{
    ActiveTheme as _, Disableable as _, IconName, Sizable as _, StyledExt as _, Theme, ThemeMode,
    button::{Button, ButtonVariants as _},
    menu::{DropdownMenu as _, PopupMenu, PopupMenuItem},
    select::{Select, SelectItem},
    tab::{Tab, TabBar},
};

use crate::calendar::{CalendarViewMode, CategoryFilter};
use crate::{domain::CategoryColor, store::TimetableRepository};

use super::{state::CadenceView, style::category_dot};

#[derive(Clone)]
pub(super) struct FilterOption {
    pub(super) filter: CategoryFilter,
    pub(super) label: SharedString,
    pub(super) color: Option<CategoryColor>,
}

impl FilterOption {
    pub(super) fn all() -> Self {
        Self {
            filter: CategoryFilter::All,
            label: "All categories".into(),
            color: None,
        }
    }
}

impl SelectItem for FilterOption {
    type Value = CategoryFilter;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.filter
    }

    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(category_dot(self.color))
            .child(self.label.clone())
    }
}

pub(super) fn render_titlebar_history(
    view: &CadenceView,
    cx: &Context<'_, CadenceView>,
) -> gpui::AnyElement {
    let interactive = view.is_interactive();
    let undo_button = Button::new("undo")
        .ghost()
        .small()
        .icon(IconName::Undo2)
        .disabled(!interactive || !view.history.can_undo())
        .tooltip("Undo (Ctrl/Cmd+Z)")
        .on_click(cx.listener(|this, _, window, cx| this.undo(window, cx)))
        .into_any_element();
    let redo_button = Button::new("redo")
        .ghost()
        .small()
        .icon(IconName::Redo2)
        .disabled(!interactive || !view.history.can_redo())
        .tooltip("Redo (Ctrl/Cmd+Shift+Z)")
        .on_click(cx.listener(|this, _, window, cx| this.redo(window, cx)))
        .into_any_element();
    div()
        .flex()
        .items_center()
        .gap_1()
        .when(
            matches!(
                view.persistence_state,
                super::state::PersistenceState::Writing
            ),
            |this| {
                this.child(
                    div()
                        .mr_1()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Saving…"),
                )
            },
        )
        .child(undo_button)
        .child(redo_button)
        .into_any_element()
}

pub(super) fn render_titlebar_actions(
    view: &CadenceView,
    window: &Window,
    cx: &Context<'_, CadenceView>,
) -> gpui::AnyElement {
    let width = window.viewport_size().width.as_f32();
    let show_filter = width >= 960.0;
    let show_today = width >= 760.0;
    let show_navigation = width >= 1_160.0;
    let interactive = view.is_interactive();
    let filter = Select::new(&view.category_filter)
        .w(px(200.0))
        .appearance(false)
        .disabled(!interactive)
        .placeholder("Filter categories");
    let today_button = Button::new("today")
        .outline()
        .small()
        .disabled(!interactive)
        .label("Today")
        .on_click(cx.listener(|this, _, _, cx| this.go_to_today(cx)));
    let new_event_button = Button::new("new-event")
        .debug_selector(|| "new-event".into())
        .primary()
        .small()
        .disabled(!interactive)
        .label("New event")
        .tooltip("New event (Ctrl/Cmd+N)")
        .on_click(cx.listener(|this, _, window, cx| {
            cx.stop_propagation();
            this.new_event(window, cx);
        }));

    div()
        .flex()
        .items_center()
        .gap_2()
        .child(render_mode_control(view, cx))
        .when(show_filter, |this| this.child(filter))
        .when(show_today, |this| this.child(today_button))
        .child(new_event_button)
        .when(show_navigation, |this| {
            this.child(render_navigation(view, false, cx))
        })
        .child(render_overflow_menu(
            view,
            show_filter,
            show_today,
            show_navigation,
            cx,
        ))
        .into_any_element()
}

fn render_mode_control(view: &CadenceView, cx: &Context<'_, CadenceView>) -> gpui::AnyElement {
    TabBar::new("calendar-view-mode")
        .segmented()
        .small()
        .selected_index(match view.state.view_mode() {
            CalendarViewMode::Day => 0,
            CalendarViewMode::Week => 1,
        })
        .on_click(cx.listener(|this, index: &usize, _, cx| {
            this.set_view_mode(
                if *index == 0 {
                    CalendarViewMode::Day
                } else {
                    CalendarViewMode::Week
                },
                cx,
            );
        }))
        .child(Tab::new().label("Day"))
        .child(Tab::new().label("Week"))
        .into_any_element()
}

fn render_overflow_menu(
    view: &CadenceView,
    show_filter: bool,
    show_today: bool,
    show_navigation: bool,
    cx: &Context<'_, CadenceView>,
) -> impl IntoElement {
    let owner = cx.entity().downgrade();
    let interactive = view.is_interactive();
    let is_dark = cx.theme().mode.is_dark();
    let selected_filter = view.state.category_filter();
    let filters = std::iter::once(FilterOption::all())
        .chain(
            view.repository
                .categories()
                .unwrap_or_default()
                .into_iter()
                .filter(crate::domain::Category::is_visible)
                .map(|category| FilterOption {
                    filter: CategoryFilter::Only(category.id()),
                    label: category.name().into(),
                    color: Some(category.color_token()),
                }),
        )
        .collect::<Vec<_>>();

    Button::new("titlebar-more")
        .ghost()
        .small()
        .icon(IconName::Ellipsis)
        .tooltip("More timetable actions")
        .dropdown_menu(move |menu, _, _| {
            let menu = add_navigation_items(
                menu,
                owner.clone(),
                interactive,
                show_navigation,
                show_today,
            );
            let menu = add_filter_items(
                menu,
                &owner,
                interactive,
                show_filter,
                selected_filter,
                &filters,
            );
            add_secondary_items(menu, owner.clone(), interactive, is_dark)
        })
}

fn add_navigation_items(
    menu: PopupMenu,
    owner: gpui::WeakEntity<CadenceView>,
    interactive: bool,
    show_navigation: bool,
    show_today: bool,
) -> PopupMenu {
    let menu = if show_navigation {
        menu
    } else {
        let previous_owner = owner.clone();
        let next_owner = owner.clone();
        menu.item(
            PopupMenuItem::new("Previous period")
                .icon(IconName::ChevronLeft)
                .disabled(!interactive)
                .on_click(move |_, _, cx| {
                    previous_owner
                        .update(cx, |view, cx| view.shift_period(false, cx))
                        .ok();
                }),
        )
        .item(
            PopupMenuItem::new("Next period")
                .icon(IconName::ChevronRight)
                .disabled(!interactive)
                .on_click(move |_, _, cx| {
                    next_owner
                        .update(cx, |view, cx| view.shift_period(true, cx))
                        .ok();
                }),
        )
    };
    if show_today {
        menu
    } else {
        menu.item(
            PopupMenuItem::new("Today")
                .icon(IconName::Calendar)
                .disabled(!interactive)
                .on_click(move |_, _, cx| {
                    owner.update(cx, CadenceView::go_to_today).ok();
                }),
        )
    }
}

fn add_filter_items(
    menu: PopupMenu,
    owner: &gpui::WeakEntity<CadenceView>,
    interactive: bool,
    show_filter: bool,
    selected_filter: CategoryFilter,
    filters: &[FilterOption],
) -> PopupMenu {
    if show_filter {
        return menu;
    }
    filters.iter().fold(
        menu.separator()
            .item(PopupMenuItem::label("Filter categories")),
        |menu, option| {
            let filter_owner = owner.clone();
            let filter = option.filter;
            menu.item(
                PopupMenuItem::new(option.label.clone())
                    .checked(filter == selected_filter)
                    .disabled(!interactive)
                    .on_click(move |_, window, cx| {
                        filter_owner
                            .update(cx, |view, cx| {
                                view.category_filter.update(cx, |select, cx| {
                                    select.set_selected_value(&filter, window, cx);
                                });
                            })
                            .ok();
                    }),
            )
        },
    )
}

fn add_secondary_items(
    menu: PopupMenu,
    owner: gpui::WeakEntity<CadenceView>,
    interactive: bool,
    is_dark: bool,
) -> PopupMenu {
    let agenda_owner = owner.clone();
    let export_owner = owner.clone();
    let settings_owner = owner;
    menu.separator()
        .item(
            PopupMenuItem::new("Agenda")
                .icon(IconName::Calendar)
                .disabled(!interactive)
                .on_click(move |_, window, cx| {
                    agenda_owner
                        .update(cx, |view, cx| view.open_agenda(window, cx))
                        .ok();
                }),
        )
        .item(
            PopupMenuItem::new("Export backup")
                .icon(IconName::File)
                .disabled(!interactive)
                .on_click(move |_, window, cx| {
                    export_owner
                        .update(cx, |view, cx| view.export_backup(window, cx))
                        .ok();
                }),
        )
        .item(
            PopupMenuItem::new("Settings")
                .icon(IconName::Settings)
                .on_click(move |_, window, cx| {
                    settings_owner
                        .update(cx, |view, cx| view.open_settings(window, cx))
                        .ok();
                }),
        )
        .separator()
        .item(
            PopupMenuItem::new(if is_dark {
                "Use light theme"
            } else {
                "Use dark theme"
            })
            .icon(if is_dark {
                IconName::Sun
            } else {
                IconName::Moon
            })
            .on_click(|_, window, cx| {
                Theme::change(
                    if cx.theme().mode.is_dark() {
                        ThemeMode::Light
                    } else {
                        ThemeMode::Dark
                    },
                    Some(window),
                    cx,
                );
            }),
        )
}

fn render_navigation(
    view: &CadenceView,
    compact: bool,
    cx: &Context<'_, CadenceView>,
) -> gpui::AnyElement {
    div()
        .flex()
        .items_center()
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        .child(
            Button::new("previous-period")
                .ghost()
                .icon(IconName::ChevronLeft)
                .tooltip("Previous period (Alt+Left)")
                .on_click(cx.listener(|this, _, _, cx| this.shift_period(false, cx))),
        )
        .child(
            div()
                .min_w(px(if compact { 166.0 } else { 180.0 }))
                .px_3()
                .text_center()
                .text_sm()
                .font_medium()
                .border_l_1()
                .border_r_1()
                .border_color(cx.theme().border)
                .child(view.range_label()),
        )
        .child(
            Button::new("next-period")
                .ghost()
                .icon(IconName::ChevronRight)
                .tooltip("Next period (Alt+Right)")
                .on_click(cx.listener(|this, _, _, cx| this.shift_period(true, cx))),
        )
        .into_any_element()
}
