use gpui::{
    AnyElement, App, Context, Decorations, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, Render, RenderOnce, SharedString, StatefulInteractiveElement as _,
    Styled as _, Window, WindowControlArea, div, hsla, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, InteractiveElementExt as _, Sizable as _, StyledExt as _,
    h_flex,
};

const TITLE_BAR_HEIGHT: gpui::Pixels = px(60.);

/// Cadence's custom title bar and native-style window controls.
#[derive(IntoElement)]
pub struct CadenceTitleBar {
    title: SharedString,
    leading: Option<AnyElement>,
    controls: Option<AnyElement>,
    brand_width: gpui::Pixels,
}

impl CadenceTitleBar {
    /// Creates a title bar with the supplied window title.
    ///
    /// # Parameters
    ///
    /// - `title`: Text shown in the title bar.
    ///
    /// # Returns
    ///
    /// A title bar configured with `title`.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            leading: None,
            controls: None,
            brand_width: px(252.0),
        }
    }

    /// Adds controls next to the window title.
    ///
    /// # Parameters
    ///
    /// - `controls`: Application controls that should follow the title.
    ///
    /// # Returns
    ///
    /// A title bar containing `controls` beside its title.
    #[must_use]
    pub fn leading(mut self, controls: impl IntoElement) -> Self {
        self.leading = Some(controls.into_any_element());
        self
    }

    /// Adds application controls before the window-management buttons.
    ///
    /// # Parameters
    ///
    /// - `controls`: Interactive application controls for the trailing title-bar region.
    ///
    /// # Returns
    ///
    /// A title bar containing `controls` before the window controls.
    #[must_use]
    pub fn controls(mut self, controls: impl IntoElement) -> Self {
        self.controls = Some(controls.into_any_element());
        self
    }
}

struct DragState {
    should_move: bool,
}

impl Render for DragState {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
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
    const fn id(self) -> &'static str {
        match self {
            Self::Minimize => "window-minimize",
            Self::Restore => "window-restore",
            Self::Maximize => "window-maximize",
            Self::Close => "window-close",
        }
    }

    const fn icon(self) -> IconName {
        match self {
            Self::Minimize => IconName::WindowMinimize,
            Self::Restore => IconName::WindowRestore,
            Self::Maximize => IconName::WindowMaximize,
            Self::Close => IconName::WindowClose,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Minimize => "Minimize window",
            Self::Restore => "Restore window",
            Self::Maximize => "Maximize window",
            Self::Close => "Close window",
        }
    }

    const fn control_area(self) -> WindowControlArea {
        match self {
            Self::Minimize => WindowControlArea::Min,
            Self::Restore | Self::Maximize => WindowControlArea::Max,
            Self::Close => WindowControlArea::Close,
        }
    }

    const fn is_close(self) -> bool {
        matches!(self, Self::Close)
    }
}

#[derive(IntoElement)]
struct WindowControl {
    kind: ControlKind,
}

impl WindowControl {
    const fn new(kind: ControlKind) -> Self {
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
            .role(gpui::Role::Button)
            .aria_label(kind.label())
            .group("window-control")
            .flex()
            .items_center()
            .relative()
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
    #[allow(clippy::too_many_lines)]
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let drag_state = window.use_state(cx, |_, _| DragState { should_move: false });
        let supports_maximize = window.window_controls().maximize && window.is_resizable();
        let supports_window_menu = window.window_controls().window_menu;
        let has_controls = self.controls.is_some();
        let brand_color = if cx.theme().mode.is_dark() {
            hsla(0.91, 0.34, 0.76, 1.0)
        } else {
            hsla(0.91, 0.42, 0.25, 1.0)
        };

        div()
            .id("cadence-title-bar")
            .flex()
            .items_center()
            .h(TITLE_BAR_HEIGHT)
            .flex_shrink_0()
            .border_b_1()
            .border_color(cx.theme().title_bar_border)
            .bg(cx.theme().title_bar)
            .window_control_area(WindowControlArea::Drag)
            .when(cfg!(target_os = "macos"), |this| this.pl(px(80.)))
            .when(!cfg!(target_os = "macos"), left_padding)
            .when(cfg!(target_os = "linux") && supports_maximize, |this| {
                this.on_double_click(|_, window, _| window.zoom_window())
            })
            .when(cfg!(target_os = "macos"), |this| {
                this.on_double_click(|_, window, _| window.titlebar_double_click())
            })
            .when(cfg!(target_os = "linux") && supports_window_menu, |this| {
                this.on_mouse_down(MouseButton::Right, |event, window, _| {
                    window.show_window_menu(event.position);
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
                    .gap_1()
                    .w(self.brand_width)
                    .h_full()
                    .flex_shrink_0()
                    .border_r_1()
                    .border_color(cx.theme().title_bar_border)
                    .text_base()
                    .font_semibold()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(28.0))
                            .rounded_md()
                            .border_1()
                            .border_color(brand_color.opacity(0.32))
                            .bg(brand_color.opacity(0.1))
                            .text_color(brand_color)
                            .child(Icon::new(IconName::Calendar).small()),
                    )
                    .child(self.title)
                    .when_some(self.leading, |this, leading| {
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .ml_1()
                                .h_full()
                                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                    cx.stop_propagation();
                                })
                                .child(leading),
                        )
                    }),
            )
            .when_some(self.controls, |this, controls| {
                this.child(
                    div()
                        .id("title-bar-controls")
                        .flex()
                        .flex_1()
                        .min_w_0()
                        .items_center()
                        .px_4()
                        .h_full()
                        .child(
                            div()
                                .flex()
                                .w_full()
                                .items_center()
                                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                    cx.stop_propagation();
                                })
                                .child(controls),
                        ),
                )
            })
            .when(!has_controls, |this| this.child(div().flex_1()))
            .child(WindowControls)
    }
}

fn left_padding<T: gpui_component::StyledExt>(style: T) -> T {
    style.pl_3()
}
