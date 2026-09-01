use gpui::{AnyWindowHandle, App, AppContext, BorrowAppContext, Global, Subscription, Window};
use gpui_component::WindowExt as _;

use super::state::CadenceView;

const SAVE_IN_PROGRESS_MESSAGE: &str =
    "Cadence is still saving. Try closing again when the save finishes.";

#[derive(Default)]
struct MainWindowRegistry {
    handle: Option<AnyWindowHandle>,
    view: Option<gpui::Entity<CadenceView>>,
    #[allow(dead_code)]
    close_subscription: Option<Subscription>,
}

impl Global for MainWindowRegistry {}

/// Installs integrations that belong to the application lifetime rather than a
/// particular main-window entity.
pub(super) fn init(cx: &mut App) {
    if cx.has_global::<MainWindowRegistry>() {
        return;
    }

    cx.set_global(MainWindowRegistry::default());
    cx.on_system_notification_response(|response, app| {
        let tag = response.tag.to_string();
        route_notification_response(&tag, app);
    });

    let close_subscription = cx.on_window_closed(|cx, closed_window_id| {
        cx.update_global::<MainWindowRegistry, _>(|registry, _| {
            if registry
                .handle
                .is_some_and(|handle| handle.window_id() == closed_window_id)
            {
                registry.handle = None;
            }
        });
    });
    cx.update_global::<MainWindowRegistry, _>(|registry, _| {
        registry.close_subscription = Some(close_subscription);
    });

    #[cfg(all(target_os = "linux", not(test)))]
    install_tray(cx);
}

/// Installs application-level integrations that need to outlive a single view
/// render by registering the current main window with the application registry.
pub(super) fn install(window: AnyWindowHandle, view: gpui::Entity<CadenceView>, cx: &mut App) {
    cx.update_global::<MainWindowRegistry, _>(|registry, _| {
        registry.handle = Some(window);
        registry.view = Some(view);
    });
}

pub(super) fn existing_main_view(cx: &App) -> Option<gpui::Entity<CadenceView>> {
    cx.try_global::<MainWindowRegistry>()
        .and_then(|registry| registry.view.clone())
}

fn current_main_window<C: AppContext>(
    cx: &C,
) -> Option<(AnyWindowHandle, gpui::Entity<CadenceView>)> {
    cx.read_global(|registry: &MainWindowRegistry, _| registry.handle.zip(registry.view.clone()))
}

fn route_notification_response(tag: &str, cx: &mut App) {
    if cx
        .try_global::<MainWindowRegistry>()
        .is_none_or(|registry| registry.handle.is_none())
    {
        let _ = super::open_main_window_with_app(cx);
    }

    let Some((window, view)) = current_main_window(cx) else {
        return;
    };
    if deliver_notification_response(tag, window, &view, cx) {
        return;
    }

    cx.update_global::<MainWindowRegistry, _>(|registry, _| {
        if registry
            .handle
            .is_some_and(|handle| handle.window_id() == window.window_id())
        {
            registry.handle = None;
        }
    });

    if super::open_main_window_with_app(cx).is_ok()
        && let Some((window, view)) = current_main_window(cx)
    {
        let _ = deliver_notification_response(tag, window, &view, cx);
    }
}

fn deliver_notification_response(
    tag: &str,
    window: AnyWindowHandle,
    view: &gpui::Entity<CadenceView>,
    cx: &mut App,
) -> bool {
    window
        .update(cx, |_, window, cx| {
            window.activate_window();
            view.update(cx, |view, cx| {
                view.handle_notification_response(tag, window, cx);
            });
            true
        })
        .unwrap_or(false)
}

fn persistence_allows_close(cx: &mut App) -> bool {
    let view = cx
        .try_global::<MainWindowRegistry>()
        .and_then(|registry| registry.view.clone());
    view.is_none_or(|view| {
        view.update(cx, |view, _| {
            !matches!(
                view.persistence_state,
                super::state::PersistenceState::Writing
            )
        })
    })
}

/// Handles the main title-bar close control without terminating the tray task.
pub(super) fn close_main_window(window: &mut Window, cx: &mut App) {
    if persistence_allows_close(cx) {
        window.remove_window();
    } else {
        window.push_notification(SAVE_IN_PROGRESS_MESSAGE, cx);
    }
}

/// Handles native window-manager close requests for the main window.
pub(super) fn should_close_main_window(window: &mut Window, cx: &mut App) -> bool {
    let can_close = persistence_allows_close(cx);
    if !can_close {
        window.push_notification(SAVE_IN_PROGRESS_MESSAGE, cx);
    }
    can_close
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrayCommand {
    Show,
    Quit,
}

#[cfg(target_os = "linux")]
struct CadenceTray {
    sender: async_channel::Sender<TrayCommand>,
}

#[cfg(target_os = "linux")]
impl ksni::Tray for CadenceTray {
    fn id(&self) -> String {
        crate::APPLICATION_ID.to_owned()
    }

    fn title(&self) -> String {
        crate::APPLICATION_NAME.to_owned()
    }

    fn icon_name(&self) -> String {
        crate::APPLICATION_ID.to_owned()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.sender.try_send(TrayCommand::Show);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;

        vec![
            StandardItem {
                label: "Show Cadence".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.sender.try_send(TrayCommand::Show);
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: "Quit Cadence".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.sender.try_send(TrayCommand::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

#[cfg(all(target_os = "linux", not(test)))]
fn install_tray(cx: &App) {
    use ksni::TrayMethods as _;

    let (sender, receiver) = async_channel::unbounded();
    let tray = CadenceTray { sender };
    cx.spawn(async move |cx| {
        let Ok(tray_handle) = tray.spawn().await else {
            eprintln!("Cadence could not start the desktop tray service");
            return;
        };

        let shutdown_handle = tray_handle.clone();
        cx.update(|cx| {
            cx.on_app_quit(move |_| {
                let shutdown_handle = shutdown_handle.clone();
                async move {
                    shutdown_handle.shutdown().await;
                }
            })
            .detach();
        });

        while let Ok(command) = receiver.recv().await {
            if !dispatch_tray_command_async(command, cx) {
                break;
            }
        }
    })
    .detach();
}

#[cfg(all(target_os = "linux", not(test)))]
fn dispatch_tray_command_async(command: TrayCommand, cx: &mut gpui::AsyncApp) -> bool {
    match command {
        TrayCommand::Show => {
            if let Some((window, _)) = current_main_window(cx) {
                if window
                    .update(cx, |_, window, _| window.activate_window())
                    .is_ok()
                {
                    return true;
                }
                cx.update_global::<MainWindowRegistry, _>(|registry, _| {
                    if registry
                        .handle
                        .is_some_and(|handle| handle.window_id() == window.window_id())
                    {
                        registry.handle = None;
                    }
                });
            }

            if let Err(error) = super::open_main_window(cx) {
                eprintln!("Cadence could not reopen the main window: {error:#}");
            }
            true
        }
        TrayCommand::Quit => {
            if let Some((window, _)) = current_main_window(cx) {
                let _ = window.update(cx, |_, window, _| window.remove_window());
            }
            cx.update(|cx| cx.quit());
            false
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
fn dispatch_tray_command<C: AppContext>(
    command: TrayCommand,
    window: AnyWindowHandle,
    cx: &mut C,
) -> bool {
    match command {
        TrayCommand::Show => {
            let _ = window.update(cx, |_, window, _| window.activate_window());
            true
        }
        TrayCommand::Quit => {
            let _ = window.update(cx, |_, window, cx| {
                window.remove_window();
                cx.quit();
            });
            false
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use gpui::TestAppContext;
    use ksni::Tray as _;

    use super::*;

    #[test]
    fn tray_menu_callbacks_enqueue_show_and_quit_commands() {
        let (sender, receiver) = async_channel::unbounded();
        let mut tray = CadenceTray { sender };
        let mut menu = tray.menu();

        let ksni::MenuItem::Standard(show) = menu.remove(0) else {
            panic!("Show menu item should be standard");
        };
        assert_eq!(show.label, "Show Cadence");
        (show.activate)(&mut tray);
        assert_eq!(receiver.try_recv(), Ok(TrayCommand::Show));

        assert!(matches!(menu.remove(0), ksni::MenuItem::Separator));

        let ksni::MenuItem::Standard(quit) = menu.remove(0) else {
            panic!("Quit menu item should be standard");
        };
        assert_eq!(quit.label, "Quit Cadence");
        (quit.activate)(&mut tray);
        assert_eq!(receiver.try_recv(), Ok(TrayCommand::Quit));
    }

    #[gpui::test]
    fn show_dispatch_activates_the_owned_window(cx: &mut TestAppContext) {
        let first = cx.add_empty_window();
        let main_window = first
            .read(|app| app.windows().into_iter().next())
            .expect("main window should be open");
        let second = first.cx.add_empty_window();
        let second_window = second
            .read(|app| {
                app.windows()
                    .into_iter()
                    .find(|window| *window != main_window)
            })
            .expect("second window should be open");
        second.update(|window, _| window.activate_window());
        second.run_until_parked();
        assert_eq!(second.read(gpui::App::active_window), Some(second_window));

        assert!(dispatch_tray_command(
            TrayCommand::Show,
            main_window,
            &mut second.cx
        ));
        second.run_until_parked();

        assert_eq!(second.read(gpui::App::active_window), Some(main_window));
    }

    #[gpui::test]
    fn quit_dispatch_removes_the_owned_window_and_stops_dispatch(cx: &mut TestAppContext) {
        let cx = cx.add_empty_window();
        let main_window = cx
            .read(|app| app.windows().into_iter().next())
            .expect("main window should be open");
        assert_eq!(cx.read(|app| app.windows().len()), 1);

        assert!(!dispatch_tray_command(
            TrayCommand::Quit,
            main_window,
            &mut cx.cx
        ));
        cx.run_until_parked();

        assert_eq!(cx.read(|app| app.windows().len()), 0);
    }
}
