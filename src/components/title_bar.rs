use gpui::{
    App, Context, Decorations, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, Render, RenderOnce, SharedString, StatefulInteractiveElement as _,
    Styled as _, Window, WindowControlArea, WindowOptions, div, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, InteractiveElementExt as _, Sizable as _, StyledExt as _,
    h_flex,
};

const TITLE_BAR_HEIGHT: gpui::Pixels = px(34.);

#[derive(IntoElement)]
pub(crate) struct CadenceTitleBar {
    title: SharedString,
}

impl CadenceTitleBar {
    pub(crate) fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
        }
    }

    pub(crate) fn window_options() -> WindowOptions {
        gpui_component::TitleBar::window_options()
    }
}

struct DragState {
    should_move: bool,
}

impl Render for DragState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(Clone, Copy)]
enum ControlKind {
    Minimize,
    Restore,
    Maximize,
    Close,
}

impl ControlKind {
    fn id(self) -> &'static str {
        match self {
            Self::Minimize => "window-minimize",
            Self::Restore => "window-restore",
            Self::Maximize => "window-maximize",
            Self::Close => "window-close",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Minimize => IconName::WindowMinimize,
            Self::Restore => IconName::WindowRestore,
            Self::Maximize => IconName::WindowMaximize,
            Self::Close => IconName::WindowClose,
        }
    }

    fn control_area(self) -> WindowControlArea {
        match self {
            Self::Minimize => WindowControlArea::Min,
            Self::Restore | Self::Maximize => WindowControlArea::Max,
            Self::Close => WindowControlArea::Close,
        }
    }

    fn is_close(self) -> bool {
        matches!(self, Self::Close)
    }
}

#[derive(IntoElement)]
struct WindowControl {
    kind: ControlKind,
}

impl WindowControl {
    fn new(kind: ControlKind) -> Self {
        Self { kind }
    }
}

impl RenderOnce for WindowControl {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let kind = self.kind;
        let is_close = kind.is_close();
        let hover_background = if is_close {
            cx.theme().danger
        } else {
            cx.theme().secondary_hover
        };
        let active_background = if is_close {
            cx.theme().danger_active
        } else {
            cx.theme().secondary_active
        };
        let hover_foreground = if is_close {
            cx.theme().danger_foreground
        } else {
            cx.theme().foreground
        };

        div()
            .id(kind.id())
            .group("window-control")
            .flex()
            .items_center()
            .justify_center()
            .w(px(20.))
            .h(px(20.))
            .rounded_full()
            .flex_shrink_0()
            .cursor_pointer()
            .text_color(cx.theme().muted_foreground)
            .hover(|style| style.bg(hover_background).text_color(hover_foreground))
            .active(|style| style.bg(active_background).text_color(hover_foreground))
            .when(cfg!(target_os = "windows"), |this| {
                this.window_control_area(kind.control_area())
            })
            .when(!cfg!(target_os = "windows"), |this| {
                this.on_mouse_down(MouseButton::Left, |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                })
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    match kind {
                        ControlKind::Minimize => window.minimize_window(),
                        ControlKind::Restore | ControlKind::Maximize => window.zoom_window(),
                        ControlKind::Close => window.remove_window(),
                    }
                })
            })
            .child(Icon::new(kind.icon()).small())
    }
}

#[derive(IntoElement)]
struct WindowControls;

impl RenderOnce for WindowControls {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        if cfg!(target_os = "macos") || cfg!(target_family = "wasm") {
            return h_flex().id("window-controls");
        }

        #[cfg(target_os = "linux")]
        if !matches!(window.window_decorations(), Decorations::Client { .. }) {
            return h_flex().id("window-controls");
        }

        let supported = window.window_controls();

        h_flex()
            .id("window-controls")
            .items_center()
            .gap_3()
            .px_3()
            .h_full()
            .flex_shrink_0()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .when(supported.minimize && window.is_minimizable(), |this| {
                this.child(WindowControl::new(ControlKind::Minimize))
            })
            .when(supported.maximize && window.is_resizable(), |this| {
                this.child(WindowControl::new(if window.is_maximized() {
                    ControlKind::Restore
                } else {
                    ControlKind::Maximize
                }))
            })
            .child(WindowControl::new(ControlKind::Close))
    }
}

impl RenderOnce for CadenceTitleBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let drag_state = window.use_state(cx, |_, _| DragState { should_move: false });
        let supports_maximize = window.window_controls().maximize && window.is_resizable();
        let supports_window_menu = window.window_controls().window_menu;

        div()
            .id("cadence-title-bar")
            .flex()
            .items_center()
            .justify_between()
            .h(TITLE_BAR_HEIGHT)
            .flex_shrink_0()
            .border_b_1()
            .border_color(cx.theme().title_bar_border)
            .bg(cx.theme().title_bar)
            .window_control_area(WindowControlArea::Drag)
            .when(cfg!(target_os = "macos"), |this| this.pl(px(80.)))
            .when(!cfg!(target_os = "macos"), |this| this.pl_3())
            .when(cfg!(target_os = "linux") && supports_maximize, |this| {
                this.on_double_click(|_, window, _| window.zoom_window())
            })
            .when(cfg!(target_os = "macos"), |this| {
                this.on_double_click(|_, window, _| window.titlebar_double_click())
            })
            .when(cfg!(target_os = "linux") && supports_window_menu, |this| {
                this.on_mouse_down(MouseButton::Right, |event, window, _| {
                    window.show_window_menu(event.position)
                })
            })
            .on_mouse_down_out(window.listener_for(&drag_state, |state, _, _, _| {
                state.should_move = false;
            }))
            .on_mouse_down(
                MouseButton::Left,
                window.listener_for(&drag_state, |state, _, _, _| {
                    state.should_move = true;
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                window.listener_for(&drag_state, |state, _, _, _| {
                    state.should_move = false;
                }),
            )
            .on_mouse_move(window.listener_for(&drag_state, |state, _, window, _| {
                if state.should_move {
                    state.should_move = false;
                    window.start_window_move();
                }
            }))
            .child(
                div()
                    .id("window-title")
                    .flex()
                    .items_center()
                    .h_full()
                    .text_sm()
                    .font_medium()
                    .child(self.title),
            )
            .child(WindowControls)
    }
}
