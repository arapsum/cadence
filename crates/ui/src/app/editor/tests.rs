use std::{cell::RefCell, fs, rc::Rc};

use gpui::{
    AppContext as _, Entity, Modifiers, ScrollDelta, ScrollWheelEvent, TestAppContext, TouchPhase,
    point, px,
};
use gpui_component::Root;
use jiff::{
    Timestamp,
    civil::{Date, Time},
};
use uuid::Uuid;

use super::super::{settings_window, state::CadenceView};
use super::{
    RecurrenceScope,
    form::{EventEditor, TimeOption, end_time_options_after},
};
use crate::{
    calendar::CalendarViewMode,
    domain::{
        DateRange, Event, EventDraft, EventId, OccurrenceId, RecurrenceException, RecurrenceRule,
        RecurrenceSeries, RecurrenceSeriesId,
    },
    editor::{EditorMode, FormDraft},
    store::{InMemoryRepository, StorageClient, TimetableRepository, default_categories},
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

fn assert_dirty_event_form_can_be_discarded(
    calendar: &Entity<CadenceView>,
    cx: &mut gpui::VisualTestContext,
) {
    let before = calendar.read_with(cx, |view, _| view.repository.snapshot().unwrap());
    cx.simulate_input("Unsaved");
    cx.simulate_keystrokes("escape");
    cx.update(|window, app| window.draw(app).clear(app));
    assert!(cx.update(gpui_component::WindowExt::has_active_dialog));
    cx.simulate_keystrokes("enter");
    cx.update(|window, app| window.draw(app).clear(app));
    assert!(!cx.update(gpui_component::WindowExt::has_active_dialog));
    assert_eq!(
        calendar.read_with(cx, |view, _| view.repository.snapshot().unwrap()),
        before
    );
}

fn assert_event_editor_dialog_is_rendered(cx: &mut gpui::VisualTestContext) {
    assert!(cx.update(gpui_component::WindowExt::has_active_dialog));
    assert!(cx.update(|window, app| Root::render_dialog_layer(window, app).is_some()));
    cx.update(|window, app| window.draw(app).clear(app));
    assert!(cx.debug_bounds("event-editor-form").is_some());
}

fn assert_inspector_actions_are_non_mutating(
    calendar: &Entity<CadenceView>,
    event_id: OccurrenceId,
    event_date: Date,
    cx: &mut gpui::VisualTestContext,
) {
    calendar.update_in(cx, |view, window, app| {
        view.inspect_event(event_id, event_date, window, app);
    });
    cx.update(|window, app| window.draw(app).clear(app));
    assert!(cx.debug_bounds("event-inspector-details").is_some());
    let before = calendar.read_with(cx, |view, _| view.repository.snapshot().unwrap());

    let duplicate = cx
        .debug_bounds("duplicate-event")
        .expect("duplicate action was rendered");
    cx.simulate_click(duplicate.center(), Modifiers::none());
    assert_event_editor_dialog_is_rendered(cx);
    assert_eq!(
        calendar.read_with(cx, |view, _| view.repository.snapshot().unwrap()),
        before
    );
    cx.update(gpui_component::WindowExt::close_all_dialogs);

    calendar.update_in(cx, |view, window, app| {
        view.inspect_event(event_id, event_date, window, app);
    });
    cx.update(|window, app| window.draw(app).clear(app));
    let delete = cx
        .debug_bounds("delete-event")
        .expect("delete action was rendered");
    cx.simulate_click(delete.center(), Modifiers::none());
    cx.update(|window, app| window.draw(app).clear(app));
    assert!(cx.update(gpui_component::WindowExt::has_active_dialog));
    cx.simulate_keystrokes("escape");
    cx.update(|window, app| window.draw(app).clear(app));
    assert!(cx.debug_bounds("event-inspector-details").is_some());
    assert_eq!(
        calendar.read_with(cx, |view, _| view.repository.snapshot().unwrap()),
        before
    );
    cx.update(gpui_component::WindowExt::close_all_dialogs);
}

#[gpui::test]
fn dirty_event_form_discard_preserves_snapshot(cx: &mut TestAppContext) {
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
    cx.update(|window, app| window.draw(app).clear(app));
    assert!(cx.debug_bounds("event-editor-form").is_some());

    assert_dirty_event_form_can_be_discarded(&calendar, cx);
}

#[gpui::test]
fn inspector_actions_preserve_snapshot_when_cancelled(cx: &mut TestAppContext) {
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

    let (event_id, event_date) = calendar.read_with(cx, |view, _| {
        let event = view
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.surface(view.state.view_mode()).events.first())
            .expect("the seeded calendar contains an event");
        (event.id(), event.date())
    });
    assert_inspector_actions_are_non_mutating(&calendar, event_id, event_date, cx);
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
    calendar.update_in(cx, |_, window, app| {
        CadenceView::open_about(window, app);
    });
    assert!(cx.update(gpui_component::WindowExt::has_active_dialog));
    assert!(cx.update(|window, app| Root::render_dialog_layer(window, app).is_some()));

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
fn half_hour_slot_opens_editor(cx: &mut TestAppContext) {
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
        view.initialize_scroll(CalendarViewMode::Week, (0.0, 0.0));
    });
    cx.update(|window, app| window.draw(app).clear(app));
    let slot = cx
        .debug_bounds("week-half-hour-slot")
        .expect("the first buffered date should expose a free half-hour slot");
    cx.simulate_click(slot.center(), Modifiers::none());
    assert!(cx.update(gpui_component::WindowExt::has_active_dialog));
    cx.update(|window, app| window.draw(app).clear(app));
    assert!(cx.debug_bounds("event-editor-form").is_some());

    cx.update(gpui_component::WindowExt::close_all_dialogs);
}

#[gpui::test]
fn settings_entry_point_opens_one_separate_window(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    cx.update(settings_window::init);

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
    let main_window_id = cx.read(|app| {
        app.windows()
            .into_iter()
            .next()
            .expect("main window is open")
            .window_id()
    });

    cx.update(|window, app| {
        calendar.update(app, |_, app| CadenceView::open_settings(window, app));
    });
    cx.run_until_parked();
    assert_eq!(cx.read(|app| app.windows().len()), 2);
    assert!(!cx.update(gpui_component::WindowExt::has_active_dialog));

    calendar.update_in(cx, |_, window, app| {
        CadenceView::open_settings(window, app);
    });
    cx.run_until_parked();
    assert_eq!(cx.read(|app| app.windows().len()), 2);

    let settings = cx.read(|app| {
        app.windows()
            .into_iter()
            .find(|window| window.window_id() != main_window_id)
            .expect("settings window is open")
    });
    settings
        .update(&mut cx.cx, |_, window, _| window.remove_window())
        .expect("settings window can close independently");
    cx.run_until_parked();
    assert_eq!(cx.read(|app| app.windows().len()), 1);

    cx.update(|window, app| {
        calendar.update(app, |_, app| CadenceView::open_settings(window, app));
    });
    cx.run_until_parked();
    assert_eq!(cx.read(|app| app.windows().len()), 2);

    drop(calendar);
    cx.update(|window, _| window.remove_window());
    cx.run_until_parked();
    assert_eq!(cx.read(|app| app.windows().len()), 0);
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
        let events = view.visible_surface_events(surface);
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
fn day_plan_event_selection_remains_available(cx: &mut TestAppContext) {
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
    let week_panel = cx
        .debug_bounds("week-workspace-panel")
        .expect("the Week workspace was rendered");
    assert!(
        week_panel.size.height > px(400.0),
        "the Week workspace should fill the available calendar height"
    );
    let day_header = cx
        .debug_bounds("calendar-day-header")
        .expect("the selected week header was rendered");
    cx.simulate_click(day_header.center(), Modifiers::none());
    assert!(calendar.read_with(cx, |view, _| view.day_plan_open));

    calendar.update_in(cx, |view, _, _| {
        view.initialize_scroll(CalendarViewMode::Day, (0.0, 500.0));
    });
    cx.update(|window, app| window.draw(app).clear(app));
    assert!(cx.debug_bounds("day-plan-sheet").is_some());
    assert!(
        cx.debug_bounds("calendar-event-card").is_some(),
        "the opened day plan should lay out its events"
    );
    let day_surface = cx
        .debug_bounds("day-calendar-surface")
        .expect("the Day plan surface was rendered");
    assert!(
        day_surface.size.height > px(400.0),
        "the Day plan surface should fill the sheet body"
    );
    let occurrence = calendar.read_with(cx, |view, _| {
        view.snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.surface(CalendarViewMode::Day).events.first())
            .expect("the opened day plan contains an event")
            .id()
    });
    calendar.update_in(cx, |view, _, app| {
        view.toggle_event_selection_from_shortcut(CalendarViewMode::Day, occurrence, app);
    });

    assert!(calendar.read_with(cx, |view, _| view.is_bulk_selecting()));
    assert_eq!(
        calendar.read_with(cx, |view, _| view.bulk_selection_count()),
        1
    );

    calendar.update_in(cx, |view, _, app| {
        view.toggle_event_selection_from_shortcut(CalendarViewMode::Day, occurrence, app);
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

fn make_editor(
    view: &CadenceView,
    draft: &FormDraft,
    window: &mut gpui::Window,
    app: &mut gpui::App,
) -> Entity<EventEditor> {
    let categories = view.repository.categories().unwrap();
    let settings = view.settings.clone();
    app.new(|editor_cx| {
        EventEditor::new(
            EditorMode::Create,
            draft,
            categories,
            &settings,
            window,
            editor_cx,
        )
    })
}

#[gpui::test]
fn editor_commit_records_a_valid_form(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);

    let calendar = Rc::new(RefCell::new(None::<Entity<CadenceView>>));
    let captured_calendar = Rc::clone(&calendar);
    let (_, cx) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| CadenceView::new(window, cx));
        captured_calendar.borrow_mut().replace(view.clone());
        Root::new(view, window, cx)
    });
    let calendar = calendar.borrow().clone().expect("calendar view");
    let draft = calendar.read_with(cx, |view, _| {
        let category_id = view
            .repository
            .categories()
            .unwrap()
            .first()
            .expect("seed category")
            .id();
        FormDraft {
            title: "Committed event".to_owned(),
            notes: String::new(),
            date: view.state.selected_date(),
            start_time: Time::constant(6, 0, 0, 0),
            end_time: Time::constant(7, 0, 0, 0),
            category_id: Some(category_id),
            recurrence: None,
            ends_on: None,
            reminder: None,
        }
    });
    let editor = calendar.update_in(cx, |view, window, app| {
        make_editor(view, &draft, window, app)
    });

    let (committed, form, error) = calendar.update_in(cx, |view, window, app| {
        let form = editor.read_with(app, EventEditor::form);
        (
            view.commit_editor(&editor, window, app),
            form,
            view.error.clone(),
        )
    });
    assert!(
        committed,
        "editor commit rejected form {form:?}; error={error:?}"
    );
    assert!(calendar.read_with(cx, |view, _| {
        view.repository
            .snapshot()
            .unwrap()
            .events
            .iter()
            .any(|event| event.title() == "Committed event")
    }));
}

#[gpui::test]
fn editor_rejects_overlap_without_mutating_snapshot_and_accepts_adjacent_event(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_component::init);

    let calendar = Rc::new(RefCell::new(None::<Entity<CadenceView>>));
    let captured_calendar = Rc::clone(&calendar);
    let (_, cx) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| CadenceView::new(window, cx));
        captured_calendar.borrow_mut().replace(view.clone());
        Root::new(view, window, cx)
    });
    let calendar = calendar.borrow().clone().expect("calendar view");
    let (date, category_id) = calendar.update_in(cx, |view, _, _| {
        view.repository = InMemoryRepository::with_defaults();
        for category in default_categories() {
            view.repository.create_category(category).unwrap();
        }
        let category_id = view
            .repository
            .categories()
            .unwrap()
            .first()
            .expect("default category")
            .id();
        let event = Event::new(
            EventId::from_uuid(Uuid::from_u128(0x300)),
            EventDraft::new(
                "Existing",
                view.state.selected_date(),
                Time::constant(9, 0, 0, 0),
                Time::constant(10, 0, 0, 0),
                category_id,
                None,
            ),
            Timestamp::from_second(1_700_000_200).unwrap(),
        )
        .unwrap();
        view.repository.create_event(event).unwrap();
        view.refresh_snapshot();
        (view.state.selected_date(), category_id)
    });
    let before = calendar.read_with(cx, |view, _| view.repository.snapshot().unwrap());

    let overlapping = FormDraft {
        title: "Overlap".to_owned(),
        notes: String::new(),
        date,
        start_time: Time::constant(9, 30, 0, 0),
        end_time: Time::constant(10, 30, 0, 0),
        category_id: Some(category_id),
        recurrence: None,
        ends_on: None,
        reminder: None,
    };
    let editor = calendar.update_in(cx, |view, window, app| {
        make_editor(view, &overlapping, window, app)
    });
    assert!(!calendar.update_in(cx, |view, window, app| {
        view.commit_editor(&editor, window, app)
    }));
    assert_eq!(
        calendar.read_with(cx, |view, _| view.repository.snapshot().unwrap()),
        before
    );

    let adjacent = FormDraft {
        title: "Adjacent".to_owned(),
        start_time: Time::constant(10, 0, 0, 0),
        end_time: Time::constant(11, 0, 0, 0),
        ..overlapping
    };
    let editor = calendar.update_in(cx, |view, window, app| {
        make_editor(view, &adjacent, window, app)
    });
    assert!(calendar.update_in(cx, |view, window, app| {
        view.commit_editor(&editor, window, app)
    }));
    assert!(calendar.read_with(cx, |view, _| {
        view.repository
            .snapshot()
            .unwrap()
            .events
            .iter()
            .any(|event| event.title() == "Adjacent")
    }));
}

fn configure_predecessor_series(
    view: &mut CadenceView,
    timestamp: Timestamp,
    start: Date,
) -> (RecurrenceSeriesId, RecurrenceException) {
    view.repository = InMemoryRepository::with_defaults();
    for category in default_categories() {
        view.repository.create_category(category).unwrap();
    }
    let category_id = view
        .repository
        .categories()
        .unwrap()
        .first()
        .expect("default category")
        .id();
    let series = RecurrenceSeries::new(
        RecurrenceSeriesId::from_uuid(Uuid::from_u128(0x100)),
        EventDraft::new(
            "Original",
            start,
            Time::constant(9, 0, 0, 0),
            Time::constant(10, 0, 0, 0),
            category_id,
            Some("before".to_owned()),
        ),
        RecurrenceRule::Daily,
        Some(Date::constant(2026, 8, 31)),
        timestamp,
    )
    .unwrap();
    let series_id = series.id();
    view.repository.create_series(series).unwrap();
    let predecessor_exception =
        RecurrenceException::cancelled(series_id, Date::constant(2026, 8, 19));
    view.repository
        .upsert_exception(predecessor_exception.clone())
        .unwrap();
    view.refresh_snapshot();
    (series_id, predecessor_exception)
}

fn apply_following_edit_and_assert(
    view: &mut CadenceView,
    series_id: RecurrenceSeriesId,
    predecessor_exception: RecurrenceException,
    split: Date,
    timestamp: Timestamp,
    start: Date,
) -> RecurrenceSeriesId {
    let category_id = view
        .repository
        .categories()
        .unwrap()
        .first()
        .expect("default category")
        .id();
    let draft = FormDraft {
        title: "Changed".to_owned(),
        notes: "after".to_owned(),
        date: split,
        start_time: Time::constant(11, 0, 0, 0),
        end_time: Time::constant(12, 0, 0, 0),
        category_id: Some(category_id),
        recurrence: Some(RecurrenceRule::Daily),
        ends_on: Some(Date::constant(2026, 8, 31)),
        reminder: None,
    };
    let occurrence = view
        .apply_recurring_edit(
            series_id,
            split,
            &draft,
            RecurrenceScope::Following,
            timestamp,
        )
        .unwrap();
    let successor_id = occurrence
        .recurring()
        .expect("following edit creates a recurring successor")
        .0;

    let predecessor = view.repository.series(series_id).unwrap().unwrap();
    assert_eq!(predecessor.template().title, "Original");
    assert_eq!(
        predecessor.template().start_time,
        Time::constant(9, 0, 0, 0)
    );
    assert_eq!(predecessor.ends_on(), Some(Date::constant(2026, 8, 23)));
    assert_eq!(
        view.repository.recurrence_exceptions().unwrap(),
        vec![predecessor_exception]
    );

    let occurrences = view
        .repository
        .occurrences(DateRange::new(start, Date::constant(2026, 8, 27)).unwrap())
        .unwrap();
    assert!(occurrences.iter().any(|occurrence| {
        occurrence.date() == Date::constant(2026, 8, 18) && occurrence.title() == "Original"
    }));
    assert!(
        !occurrences
            .iter()
            .any(|occurrence| occurrence.date() == Date::constant(2026, 8, 19))
    );
    assert!(
        occurrences
            .iter()
            .any(|occurrence| { occurrence.date() == split && occurrence.title() == "Changed" })
    );
    successor_id
}

#[gpui::test]
fn following_recurring_edit_keeps_predecessor_history_intact(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);

    let calendar = Rc::new(RefCell::new(None::<Entity<CadenceView>>));
    let captured_calendar = Rc::clone(&calendar);
    let (_, cx) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| CadenceView::new(window, cx));
        captured_calendar.borrow_mut().replace(view.clone());
        Root::new(view, window, cx)
    });
    let calendar = calendar.borrow().clone().expect("calendar view");
    let timestamp = Timestamp::from_second(1_700_000_000).unwrap();
    let start = Date::constant(2026, 8, 17);
    let split = Date::constant(2026, 8, 24);

    let (series_id, predecessor_exception) = calendar.update_in(cx, |view, _, _| {
        configure_predecessor_series(view, timestamp, start)
    });
    let successor_id = calendar.update_in(cx, |view, _, _| {
        apply_following_edit_and_assert(
            view,
            series_id,
            predecessor_exception,
            split,
            timestamp,
            start,
        )
    });

    calendar.read_with(cx, |view, _| {
        let successor = view.repository.series(successor_id).unwrap().unwrap();
        assert_eq!(successor.template().title, "Changed");
        assert_eq!(successor.template().date, split);
        assert_eq!(successor.rule(), RecurrenceRule::Daily);
    });
}

#[gpui::test]
fn deleting_one_recurring_occurrence_is_reversible_through_history(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);

    let calendar = Rc::new(RefCell::new(None::<Entity<CadenceView>>));
    let captured_calendar = Rc::clone(&calendar);
    let (_, cx) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|cx| CadenceView::new(window, cx));
        captured_calendar.borrow_mut().replace(view.clone());
        Root::new(view, window, cx)
    });
    let calendar = calendar.borrow().clone().expect("calendar view");
    let storage_path =
        std::env::temp_dir().join(format!("cadence-ui-history-{}.sqlite3", Uuid::now_v7()));
    calendar.update_in(cx, |view, _, _| {
        view.storage = StorageClient::spawn(storage_path.clone());
        view.storage_path = storage_path.clone();
        view.repository = InMemoryRepository::with_defaults();
        for category in default_categories() {
            view.repository.create_category(category).unwrap();
        }
        let category_id = view
            .repository
            .categories()
            .unwrap()
            .first()
            .expect("default category")
            .id();
        view.repository
            .create_series(
                RecurrenceSeries::new(
                    RecurrenceSeriesId::from_uuid(Uuid::from_u128(0x101)),
                    EventDraft::new(
                        "Routine",
                        Date::constant(2026, 8, 17),
                        Time::constant(8, 0, 0, 0),
                        Time::constant(9, 0, 0, 0),
                        category_id,
                        None,
                    ),
                    RecurrenceRule::Daily,
                    Some(Date::constant(2026, 8, 31)),
                    Timestamp::from_second(1_700_000_001).unwrap(),
                )
                .unwrap(),
            )
            .unwrap();
        view.refresh_snapshot();
    });

    let series_id = RecurrenceSeriesId::from_uuid(Uuid::from_u128(0x101));
    let original_date = Date::constant(2026, 8, 19);
    let before = calendar.read_with(cx, |view, _| view.repository.snapshot().unwrap());
    assert!(
        before
            .recurrence_exceptions
            .iter()
            .all(|exception| exception.original_date() != original_date)
    );

    calendar.update_in(cx, |view, window, app| {
        view.delete_recurring(series_id, original_date, RecurrenceScope::This, window, app);
    });
    cx.cx.run_until_parked();
    calendar.read_with(cx, |view, _| {
        assert!(
            view.repository
                .occurrences(
                    DateRange::new(original_date, original_date.tomorrow().unwrap()).unwrap()
                )
                .unwrap()
                .is_empty()
        );
        assert!(
            view.history.can_undo(),
            "history should record the committed delete; persistence={:?}, error={:?}",
            view.persistence_state,
            view.error
        );
    });

    calendar.update_in(cx, |view, window, app| {
        view.undo(window, app);
    });
    cx.cx.run_until_parked();
    calendar.read_with(cx, |view, _| {
        assert_eq!(
            view.repository.snapshot().unwrap(),
            before,
            "undo should restore the deleted occurrence and exception state"
        );
        assert!(
            !view
                .repository
                .occurrences(
                    DateRange::new(original_date, original_date.tomorrow().unwrap()).unwrap()
                )
                .unwrap()
                .is_empty()
        );
    });

    drop(calendar);
    let _ = fs::remove_file(&storage_path);
    let _ = fs::remove_file(storage_path.with_extension("sqlite3-journal"));
}
