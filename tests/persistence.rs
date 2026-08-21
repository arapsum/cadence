use std::fs;

use cadence::{
    domain::{
        Category, CategoryColor, CategoryId, DateRange, Event, EventDraft, EventId, Settings,
        WeekStart,
    },
    store::{
        AppPreferences, CalendarViewModePreference, PersistenceSnapshot, SqliteRepository,
        StorageClient, StorageError, TimetableRepository,
    },
};
use jiff::{
    Timestamp,
    civil::{Date, Time},
};
use rusqlite::Connection;
use tempfile::tempdir;
use uuid::Uuid;

const fn date(year: i16, month: i8, day: i8) -> Date {
    Date::constant(year, month, day)
}

const fn time(hour: i8, minute: i8) -> Time {
    Time::constant(hour, minute, 0, 0)
}

fn category(id: u128) -> Category {
    Category::new(
        CategoryId::from_uuid(Uuid::from_u128(id)),
        "Focus",
        CategoryColor::Violet,
        true,
    )
    .unwrap()
}

fn event(id: u128, category_id: CategoryId) -> Event {
    Event::new(
        EventId::from_uuid(Uuid::from_u128(id)),
        EventDraft::new(
            "Persisted focus",
            date(2026, 8, 21),
            time(9, 0),
            time(10, 30),
            category_id,
            Some("Keep this note".to_owned()),
        ),
        Timestamp::from_second(10).unwrap(),
    )
    .unwrap()
}

#[test]
fn first_run_has_defaults_without_sample_events() {
    let directory = tempdir().unwrap();
    let repository = SqliteRepository::open(directory.path().join("cadence.sqlite3")).unwrap();

    assert_eq!(repository.categories().unwrap().len(), 6);
    assert!(
        repository
            .events(DateRange::week(date(2026, 8, 21), WeekStart::Sunday).unwrap())
            .unwrap()
            .is_empty()
    );
    assert_eq!(repository.preferences().unwrap(), AppPreferences::default());
}

#[test]
fn sqlite_round_trip_preserves_entities_and_preferences() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("cadence.sqlite3");
    let mut repository = SqliteRepository::open(&path).unwrap();
    let category = category(100);
    let category_id = category.id();
    repository.create_category(category.clone()).unwrap();
    repository.create_event(event(101, category_id)).unwrap();
    repository
        .replace_preferences(AppPreferences {
            view_mode: CalendarViewModePreference::Day,
            category_filter: Some(category_id),
        })
        .unwrap();
    drop(repository);

    let repository = SqliteRepository::open(path).unwrap();
    assert_eq!(repository.category(category_id).unwrap(), Some(category));
    assert_eq!(
        repository
            .event(EventId::from_uuid(Uuid::from_u128(101)))
            .unwrap()
            .unwrap()
            .notes(),
        Some("Keep this note")
    );
    assert_eq!(
        repository.preferences().unwrap().view_mode,
        CalendarViewModePreference::Day
    );
    assert_eq!(
        repository.preferences().unwrap().category_filter,
        Some(category_id)
    );
}

#[test]
fn failed_snapshot_write_keeps_last_committed_state() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("cadence.sqlite3");
    let mut repository = SqliteRepository::open(&path).unwrap();
    let category = category(200);
    repository.create_category(category.clone()).unwrap();
    repository.create_event(event(201, category.id())).unwrap();
    let before = repository.snapshot().unwrap();
    let invalid = PersistenceSnapshot {
        settings: Settings::default(),
        preferences: AppPreferences::default(),
        categories: vec![category],
        events: vec![event(202, CategoryId::from_uuid(Uuid::from_u128(999)))],
    };

    assert!(repository.replace_snapshot(&invalid).is_err());
    assert_eq!(repository.snapshot().unwrap(), before);
}

#[test]
fn version_one_database_migrates_to_current_schema() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("cadence.sqlite3");
    let connection = Connection::open(&path).unwrap();
    connection.execute_batch(
        "CREATE TABLE categories (id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL, color TEXT NOT NULL, is_visible INTEGER NOT NULL);
         CREATE TABLE events (id TEXT PRIMARY KEY NOT NULL, category_id TEXT NOT NULL REFERENCES categories(id), title TEXT NOT NULL, date TEXT NOT NULL, start_time TEXT NOT NULL, end_time TEXT NOT NULL, notes TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
         CREATE TABLE settings (id INTEGER PRIMARY KEY CHECK (id = 1), week_start TEXT NOT NULL, clock_format TEXT NOT NULL, timezone TEXT NOT NULL, snap_minutes INTEGER NOT NULL, day_start TEXT NOT NULL, day_end TEXT NOT NULL);
         INSERT INTO settings VALUES (1, 'sunday', '12h', 'Etc/UTC', 15, '06:00:00', '22:00:00');
         PRAGMA user_version = 1;",
    ).unwrap();
    drop(connection);

    let repository = SqliteRepository::open(path.clone()).unwrap();
    assert_eq!(repository.preferences().unwrap(), AppPreferences::default());
    let version: u32 = Connection::open(path)
        .unwrap()
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 2);
}

#[test]
fn corrupt_database_is_rejected_without_replacement() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("cadence.sqlite3");
    fs::write(&path, b"not a sqlite database").unwrap();

    let error = SqliteRepository::open(path).unwrap_err();
    assert!(matches!(
        error,
        StorageError::Sqlite(_) | StorageError::Corrupt(_)
    ));
}

#[test]
fn worker_exports_a_versioned_read_consistent_backup() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("cadence.sqlite3");
    let client = StorageClient::spawn(path);
    let loaded = client.load().recv_blocking().unwrap().unwrap();
    assert_eq!(loaded.categories.len(), 6);

    let backup = client.export_json().recv_blocking().unwrap().unwrap();
    let value: serde_json::Value = serde_json::from_str(&backup).unwrap();
    assert_eq!(value["format_version"], 1);
    assert_eq!(value["data"]["events"].as_array().unwrap().len(), 0);
    assert_eq!(value["data"]["categories"].as_array().unwrap().len(), 6);
}
