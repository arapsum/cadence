use gpui::{AnyWindowHandle, App, WeakEntity};

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

#[cfg(all(target_os = "linux", not(test)))]
#[derive(Clone, Copy, Debug)]
enum TrayCommand {
    Show,
    Quit,
}

#[cfg(all(target_os = "linux", not(test)))]
struct CadenceTray {
    sender: async_channel::Sender<TrayCommand>,
}

#[cfg(all(target_os = "linux", not(test)))]
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
        let Ok(_tray_handle) = tray.spawn().await else {
            eprintln!("Cadence could not start the desktop tray service");
            return;
        };

        while let Ok(command) = receiver.recv().await {
            match command {
                TrayCommand::Show => {
                    let _ = window.update(cx, |_, window, _| window.activate_window());
                }
                TrayCommand::Quit => {
                    cx.update(|cx| cx.quit());
                    break;
                }
            }
        }
    })
    .detach();
}
