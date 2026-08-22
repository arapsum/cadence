use cadence_core::{
    domain::{
        Category, CategoryColor, CategoryId, DateRange, Event, EventDraft, EventId,
        RecurrenceException, RecurrenceRule, RecurrenceSeries, RecurrenceSeriesId, Settings,
        WeekStart, WeekdaySet,
    },
    store::{InMemoryRepository, TimetableRepository, seed_sample_week},
};
use jiff::{
    Timestamp,
    civil::{Date, Time},
};
use uuid::Uuid;

const fn date(year: i16, month: i8, day: i8) -> Date {
    Date::constant(year, month, day)
}

const fn time(hour: i8, minute: i8) -> Time {
    Time::constant(hour, minute, 0, 0)
}

fn category(id: u128, name: &str) -> Category {
    Category::new(
        CategoryId::from_uuid(Uuid::from_u128(id)),
        name,
        CategoryColor::Lime,
        true,
    )
    .unwrap()
}

fn event(id: u128, category_id: CategoryId, day: Date, start: i8, end: i8) -> Event {
    Event::new(
        EventId::from_uuid(Uuid::from_u128(id)),
        EventDraft::new(
            format!("Event {id}"),
            day,
            time(start, 0),
            time(end, 0),
            category_id,
            None,
        ),
        Timestamp::from_second(0).unwrap(),
    )
    .unwrap()
}

#[test]
fn day_and_week_queries_share_the_same_events() {
    let mut repository = InMemoryRepository::new(Settings::default());
    let category = category(1, "Work");
    let category_id = category.id();
    repository.create_category(category).unwrap();
    repository
        .create_event(event(2, category_id, date(2024, 3, 4), 9, 10))
        .unwrap();
    repository
        .create_event(event(3, category_id, date(2024, 3, 5), 11, 12))
        .unwrap();

    let week = DateRange::week(date(2024, 3, 5), WeekStart::Monday).unwrap();
    let day = DateRange::day(date(2024, 3, 4)).unwrap();
    let week_events = repository.events(week).unwrap();
    let day_events = repository.events(day).unwrap();
    assert_eq!(week_events.len(), 2);
    assert_eq!(day_events, vec![week_events[0].clone()]);
}

#[test]
fn repository_preserves_boundaries_and_referential_integrity() {
    let mut repository = InMemoryRepository::with_defaults();
    repository.create_category(category(9, "Fallback")).unwrap();
    let category = category(10, "Reading");
    let category_id = category.id();
    repository.create_category(category.clone()).unwrap();
    let event = event(11, category_id, date(2024, 3, 4), 9, 10);
    repository.create_event(event.clone()).unwrap();

    assert!(repository.create_event(event.clone()).is_err());
    assert!(repository.delete_category(category_id).is_err());
    assert_eq!(
        repository
            .delete_event(EventId::from_uuid(Uuid::from_u128(11)))
            .unwrap(),
        event
    );
    assert_eq!(repository.delete_category(category_id).unwrap(), category);
}

#[test]
fn categories_require_unique_names_and_keep_one_category() {
    let mut repository = InMemoryRepository::with_defaults();
    let first = category(600, "Focus");
    let second = category(601, "Career");
    repository.create_category(first.clone()).unwrap();
    repository.create_category(second.clone()).unwrap();

    let duplicate = Category::new(
        CategoryId::from_uuid(Uuid::from_u128(602)),
        "  focus  ",
        CategoryColor::Blue,
        true,
    )
    .unwrap();
    assert!(matches!(
        repository.create_category(duplicate),
        Err(cadence_core::domain::RepositoryError::DuplicateCategoryName)
    ));

    let mut renamed = second.clone();
    renamed.revise("FOCUS", CategoryColor::Cyan, true).unwrap();
    assert!(matches!(
        repository.update_category(renamed),
        Err(cadence_core::domain::RepositoryError::DuplicateCategoryName)
    ));

    repository.delete_category(first.id()).unwrap();
    assert!(matches!(
        repository.delete_category(second.id()),
        Err(cadence_core::domain::RepositoryError::LastCategory)
    ));
}

#[test]
fn category_deletion_checks_modified_recurrence_exceptions() {
    let mut repository = InMemoryRepository::with_defaults();
    let source = category(610, "Source");
    let target = category(611, "Target");
    repository.create_category(source.clone()).unwrap();
    repository.create_category(target.clone()).unwrap();
    let start = date(2026, 8, 17);
    let series = RecurrenceSeries::new(
        RecurrenceSeriesId::from_uuid(Uuid::from_u128(612)),
        EventDraft::new("Series", start, time(8, 0), time(9, 0), target.id(), None),
        RecurrenceRule::Daily,
        None,
        Timestamp::from_second(0).unwrap(),
    )
    .unwrap();
    repository.create_series(series.clone()).unwrap();
    repository
        .upsert_exception(
            RecurrenceException::modified(
                series.id(),
                start,
                EventDraft::new(
                    "One-off",
                    start,
                    time(10, 0),
                    time(11, 0),
                    source.id(),
                    None,
                ),
                Timestamp::from_second(1).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();

    assert!(matches!(
        repository.delete_category(source.id()),
        Err(cadence_core::domain::RepositoryError::CategoryInUse)
    ));
}

#[test]
fn event_lifecycle_supports_edit_duplicate_delete_and_restore() {
    let mut repository = InMemoryRepository::with_defaults();
    let category = category(20, "Focus");
    let category_id = category.id();
    repository.create_category(category).unwrap();

    let event_id = EventId::from_uuid(Uuid::from_u128(21));
    let created = event(21, category_id, date(2024, 3, 4), 9, 10);
    repository.create_event(created.clone()).unwrap();

    let mut revised = created;
    revised
        .revise(
            EventDraft::new(
                "Revised focus block",
                date(2024, 3, 5),
                time(10, 30),
                time(12, 0),
                category_id,
                Some("Updated notes".to_owned()),
            ),
            Timestamp::from_second(60).unwrap(),
        )
        .unwrap();
    repository.update_event(revised.clone()).unwrap();

    let stored = repository.event(event_id).unwrap().unwrap();
    assert_eq!(stored.title(), "Revised focus block");
    assert_eq!(stored.date(), date(2024, 3, 5));
    assert_eq!(stored.notes(), Some("Updated notes"));

    let duplicate = Event::new(
        EventId::from_uuid(Uuid::from_u128(22)),
        stored.draft(),
        Timestamp::from_second(120).unwrap(),
    )
    .unwrap();
    repository.create_event(duplicate).unwrap();
    assert_eq!(
        repository
            .events(DateRange::day(date(2024, 3, 5)).unwrap())
            .unwrap()
            .len(),
        2
    );

    let deleted = repository.delete_event(event_id).unwrap();
    assert_eq!(deleted, revised);
    assert!(repository.event(event_id).unwrap().is_none());

    repository.create_event(deleted.clone()).unwrap();
    assert_eq!(repository.event(event_id).unwrap(), Some(deleted));
}

#[test]
fn sample_week_contains_the_planning_screenshot_blocks() {
    let mut repository = InMemoryRepository::with_defaults();
    let week_start = seed_sample_week(
        &mut repository,
        date(2024, 3, 6),
        Timestamp::from_second(0).unwrap(),
    )
    .unwrap();
    let events = repository
        .events(DateRange::week(week_start, WeekStart::Sunday).unwrap())
        .unwrap();
    assert_eq!(events.len(), 81);
    assert!(
        events
            .iter()
            .any(|event| event.title() == "Breakfast + plan")
    );
    assert!(events.iter().any(|event| event.date() == date(2024, 3, 6)
        && event.title() == "Backend / database interview prep"));
    assert!(
        events
            .iter()
            .any(|event| event.date() == date(2024, 3, 9) && event.title() == "Finish / commit")
    );
    assert!(
        events
            .iter()
            .any(|event| event.title() == "Weekly planning")
    );
    assert!(events.iter().any(|event| event.notes().is_some()));
}

#[test]
fn recurring_occurrences_are_range_bounded_and_exceptions_are_persisted_in_memory() {
    let mut repository = InMemoryRepository::with_defaults();
    let category_id = category(500, "Focus").id();
    repository.create_category(category(500, "Focus")).unwrap();
    let start = date(2026, 8, 17);
    let series = RecurrenceSeries::new(
        RecurrenceSeriesId::from_uuid(Uuid::from_u128(500)),
        EventDraft::new(
            "Deep work",
            start,
            time(8, 0),
            time(9, 0),
            category_id,
            None,
        ),
        RecurrenceRule::Weekly(WeekdaySet::one(jiff::civil::Weekday::Monday)),
        None,
        Timestamp::from_second(0).unwrap(),
    )
    .unwrap();
    repository.create_series(series.clone()).unwrap();

    let range = DateRange::new(start, date(2026, 9, 1)).unwrap();
    assert_eq!(repository.occurrences(range).unwrap().len(), 3);

    repository
        .upsert_exception(RecurrenceException::cancelled(series.id(), start))
        .unwrap();
    assert_eq!(repository.occurrences(range).unwrap().len(), 2);
    assert!(
        repository
            .occurrence(cadence_core::domain::OccurrenceId::Recurring {
                series_id: series.id(),
                original_date: start,
            })
            .unwrap()
            .is_none()
    );

    let moved_date = date(2026, 8, 28);
    repository
        .upsert_exception(
            RecurrenceException::modified(
                series.id(),
                date(2026, 8, 24),
                EventDraft::new(
                    "Moved deep work",
                    moved_date,
                    time(10, 0),
                    time(11, 0),
                    category_id,
                    None,
                ),
                Timestamp::from_second(1).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
    let moved_id = cadence_core::domain::OccurrenceId::Recurring {
        series_id: series.id(),
        original_date: date(2026, 8, 24),
    };
    assert_eq!(
        repository.occurrence(moved_id).unwrap().unwrap().date(),
        moved_date
    );
}
