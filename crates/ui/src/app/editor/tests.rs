use std::{cell::RefCell, rc::Rc};

use gpui::{
    AppContext as _, Entity, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, ScrollDelta,
    ScrollWheelEvent, TestAppContext, TouchPhase, point, px,
};
use gpui_component::Root;
use jiff::civil::Time;

use super::super::state::CadenceView;
use super::{form::TimeOption, form::end_time_options_after};
use crate::{
    calendar::CalendarViewMode,
    domain::RecurrenceRule,
    editor::{EditorMode, FormDraft},
};

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
    calendar.update_in(cx, |view, window, app| {
        view.open_settings(window, app);
    });
    cx.update(|window, app| window.draw(app).clear(app));
    assert!(cx.update(gpui_component::WindowExt::has_active_dialog));
    cx.update(gpui_component::WindowExt::close_all_dialogs);
    calendar.update_in(cx, |_, window, app| {
        CadenceView::open_about(window, app);
    });
    assert!(cx.update(gpui_component::WindowExt::has_active_dialog));
    assert!(cx.update(|window, app| Root::render_dialog_layer(window, app).is_some()));

    cx.update(gpui_component::WindowExt::close_all_dialogs);

    let (event_id, event_date) = calendar.read_with(cx, |view, _| {
        let event = view
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.surface(view.state.view_mode()).events.first())
            .expect("the seeded calendar contains an event");
        (event.id(), event.date())
    });
    calendar.update_in(cx, |view, window, app| {
        view.inspect_event(event_id, event_date, window, app);
    });
    cx.update(|window, app| window.draw(app).clear(app));
    assert!(cx.debug_bounds("event-inspector-details").is_some());

    cx.update(gpui_component::WindowExt::close_all_dialogs);
    let recurring_draft = calendar.read_with(cx, |view, _| {
        let event = view
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.surface(view.state.view_mode()).events.first())
            .expect("the seeded calendar contains an event");
        let mut draft = FormDraft::from_occurrence(event);
        draft.recurrence = Some(RecurrenceRule::Daily);
        draft
    });
    calendar.update_in(cx, |view, window, app| {
        view.open_editor(EditorMode::Create, &recurring_draft, window, app);
    });
    cx.update(|window, app| window.draw(app).clear(app));
    let form = cx
        .debug_bounds("event-editor-form")
        .expect("recurring event form was rendered");
    cx.simulate_event(ScrollWheelEvent {
        position: form.center(),
        delta: ScrollDelta::Pixels(point(px(0.0), px(-1_000.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
    });
    cx.update(|window, app| window.draw(app).clear(app));
    let toggle_end_date = cx
        .debug_bounds("toggle-repeat-end")
        .expect("repeat end-date button was rendered");
    cx.simulate_click(toggle_end_date.center(), Modifiers::none());
    cx.update(|window, app| window.draw(app).clear(app));

    let end_date = cx
        .debug_bounds("repeat-end-date")
        .expect("enabling a recurrence end date should render its date picker");
    let form = cx
        .debug_bounds("event-editor-form")
        .expect("event form remained rendered");
    assert!(
        end_date.intersects(&form),
        "the newly rendered end-date picker should be revealed in the form viewport"
    );
}

#[gpui::test]
fn bulk_selection_tracks_visible_occurrences(cx: &mut TestAppContext) {
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
    let (surface, first_id, visible_count) = calendar.read_with(cx, |view, _| {
        let surface = view.state.view_mode();
        let events = &view
            .snapshot
            .as_ref()
            .expect("the seeded calendar has a snapshot")
            .surface(surface)
            .events;
        (
            surface,
            events
                .first()
                .expect("the seeded calendar has an event")
                .id(),
            events.len(),
        )
    });

    calendar.update_in(cx, |view, _, app| {
        view.begin_event_selection(app);
    });
    assert_eq!(
        calendar.read_with(cx, |view, _| view.bulk_selectable_count()),
        visible_count
    );
    assert!(calendar.read_with(cx, |view, _| view.is_bulk_selecting()));

    calendar.update_in(cx, |view, _, app| {
        view.toggle_event_selection(surface, first_id, app);
    });
    assert_eq!(
        calendar.read_with(cx, |view, _| view.bulk_selection_count()),
        1
    );

    calendar.update_in(cx, |view, _, app| {
        view.select_all_visible_events(app);
    });
    assert_eq!(
        calendar.read_with(cx, |view, _| view.bulk_selection_count()),
        visible_count
    );
    assert!(calendar.read_with(cx, |view, _| view.bulk_all_selected()));

    calendar.update_in(cx, |view, _, app| {
        view.select_all_visible_events(app);
    });
    assert_eq!(
        calendar.read_with(cx, |view, _| view.bulk_selection_count()),
        0
    );

    calendar.update_in(cx, |view, _, app| {
        view.cancel_event_selection(app);
    });
    assert!(!calendar.read_with(cx, |view, _| view.is_bulk_selecting()));
}

#[gpui::test]
fn secondary_click_enters_and_toggles_bulk_selection(cx: &mut TestAppContext) {
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

    calendar.update_in(cx, |view, _, _| {
        view.initialize_scroll(CalendarViewMode::Day, (0.0, 500.0));
    });
    cx.update(|window, app| window.draw(app).clear(app));
    let event_card = cx
        .debug_bounds("calendar-event-card")
        .expect("an event card was rendered");
    cx.simulate_click(event_card.center(), Modifiers::secondary_key());

    assert!(calendar.read_with(cx, |view, _| view.is_bulk_selecting()));
    assert_eq!(
        calendar.read_with(cx, |view, _| view.bulk_selection_count()),
        1
    );

    cx.update(|window, app| window.draw(app).clear(app));
    let event_card = cx
        .debug_bounds("calendar-event-card")
        .expect("the selected event card was rendered");
    cx.simulate_click(event_card.center(), Modifiers::secondary_key());

    assert_eq!(
        calendar.read_with(cx, |view, _| view.bulk_selection_count()),
        0
    );

    calendar.update_in(cx, |view, _, app| {
        view.cancel_event_selection(app);
    });
    cx.update(|window, app| window.draw(app).clear(app));
    let event_card = cx
        .debug_bounds("calendar-event-card")
        .expect("the event card was rendered for the double-click check");
    let modifiers = Modifiers::secondary_key();
    cx.simulate_event(MouseDownEvent {
        position: event_card.center(),
        modifiers,
        button: MouseButton::Left,
        click_count: 2,
        first_mouse: false,
    });
    cx.simulate_event(MouseUpEvent {
        position: event_card.center(),
        modifiers,
        button: MouseButton::Left,
        click_count: 2,
    });

    assert!(!cx.update(gpui_component::WindowExt::has_active_dialog));
    assert!(!calendar.read_with(cx, |view, _| view.is_bulk_selecting()));
}
