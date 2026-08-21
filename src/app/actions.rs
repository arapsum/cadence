use gpui::{App, KeyBinding};

/// Keyboard context used by the calendar surface actions.
pub(super) const CALENDAR_CONTEXT: &str = "CadenceCalendar";

macro_rules! cadence_actions {
    ($($name:ident),* $(,)?) => {
        $(
            #[derive(Clone, PartialEq, Eq, Default, Debug, gpui::Action)]
            #[action(namespace = cadence)]
            pub struct $name;
        )*
    };
}

cadence_actions!(
    ShowDay,
    ShowWeek,
    PreviousPeriod,
    NextPeriod,
    GoToToday,
    NewEvent,
    OpenAgenda,
    ExportBackup,
    OpenSettings,
    Undo,
    Redo,
    CancelManipulation,
);

pub(super) fn bind(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-1", ShowDay, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("ctrl-1", ShowDay, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("cmd-2", ShowWeek, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("ctrl-2", ShowWeek, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("alt-left", PreviousPeriod, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("alt-right", NextPeriod, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("cmd-t", GoToToday, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("ctrl-t", GoToToday, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("cmd-n", NewEvent, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("ctrl-n", NewEvent, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("cmd-f", OpenAgenda, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("ctrl-f", OpenAgenda, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("cmd-,", OpenSettings, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("ctrl-,", OpenSettings, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("cmd-z", Undo, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("ctrl-z", Undo, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("cmd-shift-z", Redo, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("ctrl-shift-z", Redo, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("ctrl-y", Redo, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("escape", CancelManipulation, Some(CALENDAR_CONTEXT)),
    ]);
}
