mod form;
mod inspector;
mod recurrence;
mod undo;
mod workflow;

#[cfg(test)]
mod tests;

pub(in crate::app) use recurrence::RecurrenceScope;

use gpui::{Context, Window};
use gpui_component::{WindowExt as _, notification::Notification};

use super::state::CadenceView;

impl CadenceView {
    pub(in crate::app) fn show_error(
        &mut self,
        message: impl Into<String>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let message = message.into();
        self.error = Some(message.clone());
        window.push_notification(Notification::error(message), cx);
        cx.notify();
    }
}
