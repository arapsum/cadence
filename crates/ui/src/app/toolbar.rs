use gpui::{App, Context, IntoElement, SharedString, Window, div, prelude::*, px};
use gpui_component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    menu::{DropdownMenu as _, PopupMenu, PopupMenuItem},
    select::{Select, SelectItem},
    tab::{Tab, TabBar},
};

use crate::calendar::{CalendarViewMode, CategoryFilter};
use crate::{
    domain::CategoryColor,
    store::{AppearanceMode, TimetableRepository},
};

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
    let interactive = view.is_interactive() && !view.is_bulk_selecting();
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
    if view.is_bulk_selecting() {
        return render_bulk_selection_actions(view, cx);
    }
    let width = window.viewport_size().width.as_f32();
    let show_filter = width >= 960.0;
    let show_today = width >= 760.0;
    let show_navigation = width >= 1_160.0;
    let interactive = view.is_interactive();
    let filter = Select::new(&view.category_filter)
        .w(px(156.0))
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

    let leading = div()
        .flex()
        .items_center()
        .gap_2()
        .child(render_mode_control(view, cx))
        .when(show_filter, |this| this.child(filter));
    let trailing = div()
        .flex()
        .items_center()
        .gap_2()
        .when(show_navigation, |this| {
            this.child(render_navigation(view, true, cx))
        })
        .when(show_today && !show_navigation, |this| {
            this.child(today_button)
        })
        .child(new_event_button)
        .child(render_overflow_menu(
            view,
            show_filter,
            show_today,
            show_navigation,
            cx,
        ));

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .w_full()
        .child(leading)
        .child(trailing)
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

fn render_bulk_selection_actions(
    view: &CadenceView,
    cx: &Context<'_, CadenceView>,
) -> gpui::AnyElement {
    let selected_count = view.bulk_selection_count();
    let all_selected = view.bulk_all_selected();
    let selectable_count = view.bulk_selectable_count();
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_medium()
                .text_color(cx.theme().foreground)
                .child(format!("{selected_count} selected")),
        )
        .child(
            Button::new("bulk-select-all")
                .outline()
                .small()
                .label(if all_selected {
                    "Clear all"
                } else {
                    "Select all"
                })
                .disabled(selectable_count == 0)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.select_all_visible_events(cx);
                })),
        )
        .child(
            Button::new("bulk-delete-selected")
                .danger()
                .small()
                .label(format!("Delete {selected_count}"))
                .disabled(selected_count == 0)
                .on_click(cx.listener(|this, _, window, cx| {
                    this.confirm_delete_selected(window, cx);
                })),
        )
        .child(
            Button::new("bulk-cancel")
                .ghost()
                .small()
                .label("Cancel")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.cancel_event_selection(cx);
                })),
        )
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
    let selectable_events = view.bulk_selectable_count_for_active_surface();
    let appearance_mode = view.appearance.mode;
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
        .dropdown_menu(move |menu, window, cx| {
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
            add_secondary_items(
                menu,
                &owner,
                interactive,
                selectable_events > 0,
                appearance_mode,
                window,
                cx,
            )
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
    owner: &gpui::WeakEntity<CadenceView>,
    interactive: bool,
    selectable_events: bool,
    appearance_mode: AppearanceMode,
    window: &mut Window,
    cx: &mut gpui::Context<'_, PopupMenu>,
) -> PopupMenu {
    let agenda_owner = owner.clone();
    let export_owner = owner.clone();
    let appearance_owner = owner.clone();
    let settings_owner = owner.clone();
    let about_owner = owner.clone();
    let selection_owner = owner.clone();
    let menu = menu
        .separator()
        .item(
            PopupMenuItem::new("Select events")
                .disabled(!interactive || !selectable_events)
                .on_click(move |_, _, cx| {
                    selection_owner
                        .update(cx, CadenceView::begin_event_selection)
                        .ok();
                }),
        )
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
        );
    let menu = menu.item(
        PopupMenuItem::new("About Cadence")
            .icon(IconName::Info)
            .on_click(move |_, window, cx| {
                about_owner
                    .update(cx, |_, cx| CadenceView::open_about(window, cx))
                    .ok();
            }),
    );
    menu.submenu_with_icon(
        Some(Icon::new(IconName::Palette)),
        "Appearance",
        window,
        cx,
        move |menu, _, _| {
            add_appearance_items(menu, &appearance_owner, interactive, appearance_mode)
        },
    )
}

fn add_appearance_items(
    menu: PopupMenu,
    owner: &gpui::WeakEntity<CadenceView>,
    interactive: bool,
    selected: AppearanceMode,
) -> PopupMenu {
    [
        (AppearanceMode::System, "System"),
        (AppearanceMode::Light, "Light"),
        (AppearanceMode::Dark, "Dark"),
    ]
    .into_iter()
    .fold(menu, |menu, (mode, label)| {
        let mode_owner = owner.clone();
        menu.item(
            PopupMenuItem::new(label)
                .checked(mode == selected)
                .disabled(!interactive)
                .on_click(move |_, _, cx| {
                    mode_owner
                        .update(cx, |view, cx| view.set_appearance_mode(mode, cx))
                        .ok();
                }),
        )
    })
}

fn render_navigation(
    view: &CadenceView,
    compact: bool,
    cx: &Context<'_, CadenceView>,
) -> gpui::AnyElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .rounded_md()
                .border_1()
                .border_color(cx.theme().border)
                .child(
                    Button::new("previous-period")
                        .ghost()
                        .small()
                        .icon(IconName::ChevronLeft)
                        .tooltip("Previous period (Alt+Left)")
                        .on_click(cx.listener(|this, _, _, cx| this.shift_period(false, cx))),
                )
                .child(
                    Button::new("navigation-today")
                        .ghost()
                        .small()
                        .label("Today")
                        .on_click(cx.listener(|this, _, _, cx| this.go_to_today(cx))),
                )
                .child(
                    Button::new("next-period")
                        .ghost()
                        .small()
                        .icon(IconName::ChevronRight)
                        .tooltip("Next period (Alt+Right)")
                        .on_click(cx.listener(|this, _, _, cx| this.shift_period(true, cx))),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .min_w(px(if compact { 164.0 } else { 176.0 }))
                .h(px(32.0))
                .px_3()
                .text_center()
                .text_sm()
                .font_medium()
                .rounded_md()
                .border_1()
                .border_color(cx.theme().border)
                .child(Icon::new(IconName::Calendar).small())
                .child(view.range_label()),
        )
        .into_any_element()
}
