use gpui::{Context, IntoElement, Window, div, prelude::*, px};
use gpui_component::{
    ActiveTheme as _, StyledExt as _,
    resizable::{ResizablePanelGroup, resizable_panel},
};

use crate::calendar::CalendarViewMode;

use super::{day, sidebar, state::CadenceView, week};

#[derive(Debug, Eq, PartialEq)]
struct WorkspacePanelWidths {
    day: u16,
    week_min: u16,
}

const fn workspace_panel_widths() -> WorkspacePanelWidths {
    WorkspacePanelWidths {
        day: 360,
        week_min: 600,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WorkspaceLayout {
    Expanded,
    Compact,
    Single,
}

impl WorkspaceLayout {
    pub(super) const fn for_width(width: f32) -> Self {
        if width >= 1_320.0 {
            Self::Expanded
        } else if width >= 1_040.0 {
            Self::Compact
        } else {
            Self::Single
        }
    }

    pub(super) const fn shows_both(self) -> bool {
        !matches!(self, Self::Single)
    }
}

pub(super) fn render(
    view: &mut CadenceView,
    window: &Window,
    cx: &mut Context<'_, CadenceView>,
) -> impl IntoElement {
    let layout = WorkspaceLayout::for_width(window.viewport_size().width.as_f32());
    let sidebar_collapsed = layout != WorkspaceLayout::Expanded;
    let content = if layout.shows_both() {
        let widths = workspace_panel_widths();
        ResizablePanelGroup::new("calendar-workspace-panels")
            .child(
                resizable_panel()
                    .size(px(f32::from(widths.day)))
                    .size_range(px(f32::from(widths.day))..px(f32::from(widths.day)))
                    .pr(px(6.0))
                    .child(render_panel(view, window, CalendarViewMode::Day, cx)),
            )
            .child(
                resizable_panel()
                    .size_range(px(f32::from(widths.week_min))..gpui::Pixels::MAX)
                    .pl(px(6.0))
                    .child(render_panel(view, window, CalendarViewMode::Week, cx)),
            )
            .into_any_element()
    } else {
        render_panel(view, window, view.state.view_mode(), cx)
    };

    div()
        .flex()
        .flex_1()
        .min_h_0()
        .bg(cx.theme().muted.opacity(0.22))
        .child(sidebar::render(view, sidebar_collapsed, cx))
        .child(div().flex_1().min_w_0().min_h_0().p_3().child(content))
}

fn render_panel(
    view: &mut CadenceView,
    window: &Window,
    mode: CalendarViewMode,
    cx: &mut Context<'_, CadenceView>,
) -> gpui::AnyElement {
    let active = view.state.view_mode() == mode;
    let title = match mode {
        CalendarViewMode::Day => "Day plan",
        CalendarViewMode::Week => "Week overview",
    };
    let range = match mode {
        CalendarViewMode::Day => view
            .state
            .selected_date()
            .strftime("%A, %B %-d")
            .to_string(),
        CalendarViewMode::Week => {
            view.surface_snapshot(CalendarViewMode::Week)
                .map_or_else(String::new, |surface| {
                    format!(
                        "{} – {}",
                        surface.range.start().strftime("%b %-d"),
                        surface
                            .range
                            .end()
                            .yesterday()
                            .unwrap_or_else(|_| surface.range.start())
                            .strftime("%b %-d, %Y")
                    )
                })
        }
    };
    let surface = match mode {
        CalendarViewMode::Day => day::render(view, window, cx).into_any_element(),
        CalendarViewMode::Week => week::render(view, window, cx).into_any_element(),
    };

    div()
        .id(match mode {
            CalendarViewMode::Day => "day-workspace-panel",
            CalendarViewMode::Week => "week-workspace-panel",
        })
        .v_flex()
        .size_full()
        .min_w_0()
        .overflow_hidden()
        .rounded_lg()
        .border_1()
        .border_color(if active {
            cx.theme().primary.opacity(0.72)
        } else {
            cx.theme().border
        })
        .bg(cx.theme().background)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .h(px(48.0))
                .flex_shrink_0()
                .px_4()
                .border_b_1()
                .border_color(cx.theme().border.opacity(0.72))
                .child(div().text_sm().font_semibold().child(title))
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(range),
                ),
        )
        .child(surface)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{WorkspaceLayout, WorkspacePanelWidths, workspace_panel_widths};

    #[test]
    fn workspace_gives_the_week_panel_more_room_than_the_day_panel() {
        let widths = workspace_panel_widths();

        assert_eq!(
            widths,
            WorkspacePanelWidths {
                day: 360,
                week_min: 600,
            }
        );
        assert!(widths.week_min > widths.day);
    }

    #[test]
    fn workspace_breakpoints_select_expected_layouts() {
        assert_eq!(
            WorkspaceLayout::for_width(1_500.0),
            WorkspaceLayout::Expanded
        );
        assert_eq!(
            WorkspaceLayout::for_width(1_100.0),
            WorkspaceLayout::Compact
        );
        assert_eq!(WorkspaceLayout::for_width(900.0), WorkspaceLayout::Single);
    }
}
