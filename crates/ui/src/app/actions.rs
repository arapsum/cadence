use gpui::{App, KeyBinding};

/// Keyboard context used by the calendar surface actions.
pub(super) const CALENDAR_CONTEXT: &str = "CadenceCalendar";
/// Keyboard context active while the rolling week surface has focus.
pub(super) const WEEK_VIEWPORT_CONTEXT: &str = "CadenceWeekViewport";

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
    PreviousPeriod,
    NextPeriod,
    GoToToday,
    NewEvent,
    OpenAgenda,
    OpenSettings,
    OpenAbout,
    Undo,
    Redo,
    SelectAllEvents,
    DeleteSelectedEvents,
    CancelManipulation,
    SlideWeekBackward,
    SlideWeekForward,
    ScrollWeekDown,
    ScrollWeekUp,
);

pub(super) fn bind(cx: &mut App) {
    cx.bind_keys([
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
        KeyBinding::new("cmd-a", SelectAllEvents, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("ctrl-a", SelectAllEvents, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("delete", DeleteSelectedEvents, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("backspace", DeleteSelectedEvents, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("escape", CancelManipulation, Some(CALENDAR_CONTEXT)),
        KeyBinding::new("h", SlideWeekBackward, Some(WEEK_VIEWPORT_CONTEXT)),
        KeyBinding::new("l", SlideWeekForward, Some(WEEK_VIEWPORT_CONTEXT)),
        KeyBinding::new("j", ScrollWeekDown, Some(WEEK_VIEWPORT_CONTEXT)),
        KeyBinding::new("k", ScrollWeekUp, Some(WEEK_VIEWPORT_CONTEXT)),
    ]);
}
