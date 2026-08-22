use std::collections::HashMap;

use gpui::{Context, IntoElement, div, prelude::*, px};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    menu::{DropdownMenu as _, PopupMenuItem},
    sidebar::{Sidebar, SidebarCollapsible, SidebarGroup, SidebarMenu, SidebarMenuItem},
};

use crate::{
    calendar::{CalendarViewMode, CategoryFilter},
    domain::{CategoryColor, CategoryId},
};

use super::{presentation::local_date_time, state::CadenceView, style::category_palette};

#[allow(clippy::too_many_lines)]
pub(super) fn render(
    view: &CadenceView,
    collapsed: bool,
    cx: &Context<'_, CadenceView>,
) -> impl IntoElement {
    let owner = cx.entity().downgrade();
    let day_owner = owner.clone();
    let week_owner = owner.clone();
    let agenda_owner = owner.clone();
    let settings_owner = owner.clone();
    let active_mode = view.state.view_mode();
    let interactive = view.is_interactive();
    let categories = view
        .snapshot
        .as_ref()
        .map_or(&[][..], |snapshot| snapshot.categories.as_slice());
    let only_category = categories.len() == 1;
    let summary = DailySummary::from_view(view);

    let navigation = SidebarGroup::new("Views").child(
        SidebarMenu::new()
            .child(
                SidebarMenuItem::new("Day")
                    .icon(IconName::Calendar)
                    .active(active_mode == CalendarViewMode::Day)
                    .disable(!interactive)
                    .on_click(move |_, _, app| {
                        day_owner
                            .update(app, |view, cx| {
                                view.set_view_mode(CalendarViewMode::Day, cx);
                            })
                            .ok();
                    }),
            )
            .child(
                SidebarMenuItem::new("Week")
                    .icon(IconName::LayoutDashboard)
                    .active(active_mode == CalendarViewMode::Week)
                    .disable(!interactive)
                    .on_click(move |_, _, app| {
                        week_owner
                            .update(app, |view, cx| {
                                view.set_view_mode(CalendarViewMode::Week, cx);
                            })
                            .ok();
                    }),
            )
            .child(
                SidebarMenuItem::new("Agenda")
                    .icon(IconName::Inbox)
                    .disable(!interactive)
                    .on_click(move |_, window, app| {
                        agenda_owner
                            .update(app, |view, cx| view.open_agenda(window, cx))
                            .ok();
                    }),
            )
            .child(
                SidebarMenuItem::new("Settings")
                    .icon(IconName::Settings)
                    .on_click(move |_, window, app| {
                        settings_owner
                            .update(app, |view, cx| view.open_settings(window, cx))
                            .ok();
                    }),
            ),
    );

    let new_category_owner = owner.clone();
    let category_items = std::iter::once(
        SidebarMenuItem::new("New category")
            .icon(IconName::Plus)
            .disable(!interactive)
            .on_click(move |_, window, app| {
                new_category_owner
                    .update(app, |view, cx| view.new_category(window, cx))
                    .ok();
            }),
    )
    .chain(
        categories
            .iter()
            .filter(|category| category.is_visible())
            .map(|category| {
                let category_id = category.id();
                let filter_owner = owner.clone();
                let edit_owner = owner.clone();
                let delete_owner = owner.clone();
                let filter = CategoryFilter::Only(category.id());
                let next_filter = if view.state.category_filter() == filter {
                    CategoryFilter::All
                } else {
                    filter
                };
                SidebarMenuItem::new(category.name().to_owned())
                    .icon(Icon::new(IconName::Minus).text_color(
                        category_palette(category.color_token(), cx.theme().mode.is_dark()).2,
                    ))
                    .active(view.state.category_filter() == filter)
                    .disable(!interactive)
                    .suffix(move |_, _| {
                        let edit_owner = edit_owner.clone();
                        let delete_owner = delete_owner.clone();
                        Button::new(format!("sidebar-category-actions-{category_id}"))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Ellipsis)
                            .tooltip("Category actions")
                            .dropdown_menu(move |menu, _, _| {
                                let edit_owner = edit_owner.clone();
                                let delete_owner = delete_owner.clone();
                                menu.item(PopupMenuItem::new("Edit category").on_click(
                                    move |_, window, app| {
                                        edit_owner
                                            .update(app, |view, cx| {
                                                view.edit_category(category_id, window, cx);
                                            })
                                            .ok();
                                    },
                                ))
                                .item(
                                    PopupMenuItem::new("Delete category")
                                        .disabled(only_category)
                                        .on_click(move |_, window, app| {
                                            delete_owner
                                                .update(app, |view, cx| {
                                                    view.confirm_delete_category(
                                                        category_id,
                                                        window,
                                                        cx,
                                                    );
                                                })
                                                .ok();
                                        }),
                                )
                            })
                    })
                    .on_click(move |_, window, app| {
                        filter_owner
                            .update(app, |view, cx| {
                                view.category_filter.update(cx, |select, cx| {
                                    select.set_selected_value(&next_filter, window, cx);
                                });
                            })
                            .ok();
                    })
            }),
    );
    let categories_group =
        SidebarGroup::new("Categories").child(SidebarMenu::new().children(category_items));

    Sidebar::new("cadence-sidebar")
        .w(px(252.0))
        .collapsible(SidebarCollapsible::Icon)
        .collapsed(collapsed)
        .header(render_plan(view, collapsed, cx))
        .child(navigation)
        .child(categories_group)
        .footer(render_summary(&summary, collapsed, cx))
}

fn render_plan(
    view: &CadenceView,
    collapsed: bool,
    cx: &Context<'_, CadenceView>,
) -> gpui::AnyElement {
    if collapsed {
        return div()
            .flex()
            .justify_center()
            .py_3()
            .child(Icon::new(IconName::Calendar))
            .into_any_element();
    }
    let (today, now) = local_date_time(view.now, &view.settings);
    let selected = view.state.selected_date();
    let events = view
        .snapshot
        .as_ref()
        .map_or(&[][..], |snapshot| snapshot.summary_events.as_slice());
    let current = events
        .iter()
        .find(|event| selected == today && event.start_time() <= now && now < event.end_time());
    let next = events
        .iter()
        .filter(|event| selected != today || event.start_time() > now)
        .min_by_key(|event| event.start_time());
    let headline = current.map_or_else(|| "Free time".to_owned(), |event| event.title().to_owned());
    let detail = current.map_or_else(
        || {
            next.map_or_else(
                || "Nothing else scheduled".to_owned(),
                |event| format!("Next: {}", event.title()),
            )
        },
        |event| {
            format!(
                "Now · {} – {}",
                crate::domain::format_time(event.start_time(), view.settings.clock_format()),
                crate::domain::format_time(event.end_time(), view.settings.clock_format())
            )
        },
    );

    div()
        .v_flex()
        .gap_2()
        .w_full()
        .pb_2()
        .child(
            div()
                .text_xs()
                .font_semibold()
                .text_color(cx.theme().sidebar_foreground.opacity(0.74))
                .child("ON YOUR PLAN"),
        )
        .child(
            div()
                .v_flex()
                .gap_2()
                .p_3()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().background)
                .shadow_xs()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().size(px(7.0)).rounded_full().bg(cx.theme().success))
                        .child(div().text_sm().font_semibold().child(headline)),
                )
                .child(
                    div()
                        .pl_4()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(detail),
                ),
        )
        .into_any_element()
}

#[derive(Default)]
struct DailySummary {
    blocks: usize,
    occupied_minutes: i32,
    category_minutes: HashMap<CategoryId, (CategoryColor, i32)>,
}

impl DailySummary {
    fn from_view(view: &CadenceView) -> Self {
        let Some(snapshot) = &view.snapshot else {
            return Self::default();
        };
        let mut intervals = Vec::with_capacity(snapshot.summary_events.len());
        let mut category_minutes = HashMap::new();
        for event in &snapshot.summary_events {
            let start =
                i32::from(event.start_time().hour()) * 60 + i32::from(event.start_time().minute());
            let end =
                i32::from(event.end_time().hour()) * 60 + i32::from(event.end_time().minute());
            intervals.push((start, end));
            if let Some(category) = snapshot
                .categories
                .iter()
                .find(|category| category.id() == event.category_id())
            {
                let entry = category_minutes
                    .entry(category.id())
                    .or_insert_with(|| (category.color_token(), 0));
                entry.1 += (end - start).max(0);
            }
        }
        intervals.sort_unstable();
        let mut merged: Vec<(i32, i32)> = Vec::new();
        for (start, end) in intervals {
            if let Some(last) = merged.last_mut().filter(|last| start <= last.1) {
                last.1 = last.1.max(end);
            } else {
                merged.push((start, end));
            }
        }
        Self {
            blocks: snapshot.summary_events.len(),
            occupied_minutes: merged.iter().map(|(start, end)| end - start).sum(),
            category_minutes,
        }
    }
}

fn render_summary(
    summary: &DailySummary,
    collapsed: bool,
    cx: &Context<'_, CadenceView>,
) -> gpui::AnyElement {
    if collapsed {
        return div()
            .flex()
            .justify_center()
            .py_2()
            .child(Icon::new(IconName::ChartPie))
            .into_any_element();
    }
    let hours = summary.occupied_minutes / 60;
    let minutes = summary.occupied_minutes % 60;
    let total = summary
        .category_minutes
        .values()
        .map(|(_, minutes)| *minutes)
        .sum::<i32>()
        .max(1);
    let mut segments = summary
        .category_minutes
        .iter()
        .map(|(id, (color, minutes))| (*id, *color, *minutes))
        .collect::<Vec<_>>();
    segments.sort_unstable_by_key(|(id, _, _)| *id);
    div()
        .v_flex()
        .w_full()
        .gap_3()
        .p_4()
        .rounded_lg()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .shadow_xs()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(Icon::new(IconName::ChartPie).text_color(cx.theme().muted_foreground))
                .child(div().text_sm().font_semibold().child("Today at a glance")),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(format!(
                    "{} scheduled blocks\n{hours}h {minutes:02}m of planned time",
                    summary.blocks
                )),
        )
        .child(
            div()
                .flex()
                .h(px(6.0))
                .rounded_full()
                .overflow_hidden()
                .gap(px(2.0))
                .children(segments.into_iter().map(|(_, color, minutes)| {
                    let minutes = i16::try_from(minutes).expect("daily minutes fit in i16");
                    let total = i16::try_from(total).expect("daily minutes fit in i16");
                    div()
                        .h_full()
                        .w(gpui::relative(f32::from(minutes) / f32::from(total)))
                        .bg(category_palette(color, cx.theme().mode.is_dark()).2)
                        .into_any_element()
                })),
        )
        .into_any_element()
}
