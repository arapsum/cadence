use std::{cell::RefCell, rc::Rc};

use gpui::{AppContext as _, Entity, Modifiers, TestAppContext};
use gpui_component::Root;
use jiff::civil::Time;

use super::super::state::CadenceView;
use super::{form::TimeOption, form::end_time_options_after};

#[test]
fn end_time_options_begin_at_the_next_available_slot() {
    let options = [
        TimeOption {
            time: Time::constant(11, 0, 0, 0),
            label: "11:00 AM".into(),
        },
        TimeOption {
            time: Time::constant(11, 15, 0, 0),
            label: "11:15 AM".into(),
        },
        TimeOption {
            time: Time::constant(11, 30, 0, 0),
            label: "11:30 AM".into(),
        },
    ];

    let end_options = end_time_options_after(&options, Time::constant(11, 15, 0, 0));

    assert_eq!(
        end_options.first().map(|option| option.time),
        Some(Time::constant(11, 30, 0, 0))
    );
}

#[gpui::test]
fn event_entry_points_render_their_dialogs(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);

    let calendar = Rc::new(RefCell::new(None::<Entity<CadenceView>>));
    let captured_calendar = Rc::clone(&calendar);
    let (_, cx) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| CadenceView::new(window, cx));
        captured_calendar.replace(Some(view.clone()));
        Root::new(view, window, cx)
    });
    let calendar = calendar
        .borrow()
        .clone()
        .expect("calendar view was captured while building the root");

    cx.update(|window, app| window.draw(app).clear(app));
    let new_event = cx
        .debug_bounds("new-event")
        .expect("new event button was rendered");
    cx.simulate_click(new_event.center(), Modifiers::none());

    assert!(cx.update(gpui_component::WindowExt::has_active_dialog));
    assert!(cx.update(|window, app| Root::render_dialog_layer(window, app).is_some()));
    cx.update(|window, app| window.draw(app).clear(app));
    assert!(cx.debug_bounds("event-editor-form").is_some());

    cx.update(gpui_component::WindowExt::close_all_dialogs);
    let (event_id, event_date) = calendar.read_with(cx, |view, _| {
        let event = view
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.events.first())
            .expect("the seeded calendar contains an event");
        (event.id(), event.date())
    });
    calendar.update_in(cx, |view, window, app| {
        view.inspect_event(event_id, event_date, window, app);
    });
    cx.update(|window, app| window.draw(app).clear(app));
    assert!(cx.debug_bounds("event-inspector-details").is_some());
}
