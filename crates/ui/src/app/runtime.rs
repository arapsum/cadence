use gpui::{AnyWindowHandle, App, AppContext, WeakEntity};

use super::state::CadenceView;

/// Installs application-level integrations that need to outlive a single view
/// render, including notification activation routing and the Linux tray.
pub(super) fn install(window: AnyWindowHandle, view: WeakEntity<CadenceView>, cx: &App) {
    cx.on_system_notification_response(move |response, app| {
        let tag = response.tag.to_string();
        let _ = window.update(app, |_, window, app| {
            window.activate_window();
            let _ = view.update(app, |view, cx| {
                view.handle_notification_response(&tag, window, cx);
            });
        });
    });

    #[cfg(all(target_os = "linux", not(test)))]
    install_tray(window, cx);
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
fn install_tray(window: AnyWindowHandle, cx: &App) {
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
            if !dispatch_tray_command(command, window, cx) {
                break;
            }
        }
    })
    .detach();
}

#[cfg(target_os = "linux")]
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
