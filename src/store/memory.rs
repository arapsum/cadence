use std::collections::HashMap;

use jiff::{
    Timestamp,
    civil::{Date, Time},
};
use uuid::Uuid;

use crate::domain::{
    Category, CategoryColor, CategoryId, DateRange, Event, EventDraft, EventId, RepositoryError,
    Settings, WeekStart,
};

use super::TimetableRepository;

/// In-memory implementation of the timetable repository contract.
///
/// # Fields
///
/// - `events`: Events keyed by stable identifier.
/// - `categories`: Categories keyed by stable identifier.
/// - `settings`: Current application settings.
#[derive(Debug, Clone, Default)]
pub struct InMemoryRepository {
    events: HashMap<EventId, Event>,
    categories: HashMap<CategoryId, Category>,
    settings: Settings,
}

impl InMemoryRepository {
    /// Creates an empty in-memory repository.
    ///
    /// # Parameters
    ///
    /// - `settings`: Initial application settings.
    ///
    /// # Returns
    ///
    /// An empty repository configured with `settings`.
    #[must_use]
    pub fn new(settings: Settings) -> Self {
        Self {
            events: HashMap::new(),
            categories: HashMap::new(),
            settings,
        }
    }

    /// Creates an empty repository with default settings.
    ///
    /// # Returns
    ///
    /// An empty repository configured with `Settings::default()`.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(Settings::default())
    }

    /// Reports whether the repository has no events or categories.
    ///
    /// # Returns
    ///
    /// `true` when both the event and category stores are empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty() && self.categories.is_empty()
    }
}

impl TimetableRepository for InMemoryRepository {
    fn event(&self, id: EventId) -> Result<Option<Event>, RepositoryError> {
        Ok(self.events.get(&id).cloned())
    }

    fn events(&self, range: DateRange) -> Result<Vec<Event>, RepositoryError> {
        let mut events = self
            .events
            .values()
            .filter(|event| range.contains(event.date()))
            .cloned()
            .collect::<Vec<_>>();
        events.sort_by_key(|event| {
            (
                event.date(),
                event.start_time(),
                event.end_time(),
                event.id(),
            )
        });
        Ok(events)
    }

    fn create_event(&mut self, event: Event) -> Result<(), RepositoryError> {
        if self.events.contains_key(&event.id()) {
            return Err(RepositoryError::DuplicateEvent);
        }
        self.ensure_category(event.category_id())?;
        self.events.insert(event.id(), event);
        Ok(())
    }

    fn update_event(&mut self, event: Event) -> Result<(), RepositoryError> {
        if !self.events.contains_key(&event.id()) {
            return Err(RepositoryError::EventNotFound);
        }
        self.ensure_category(event.category_id())?;
        self.events.insert(event.id(), event);
        Ok(())
    }

    fn delete_event(&mut self, id: EventId) -> Result<Event, RepositoryError> {
        self.events
            .remove(&id)
            .ok_or(RepositoryError::EventNotFound)
    }

    fn categories(&self) -> Result<Vec<Category>, RepositoryError> {
        let mut categories = self.categories.values().cloned().collect::<Vec<_>>();
        categories.sort_by_key(|category| (category.name().to_ascii_lowercase(), category.id()));
        Ok(categories)
    }

    fn category(&self, id: CategoryId) -> Result<Option<Category>, RepositoryError> {
        Ok(self.categories.get(&id).cloned())
    }

    fn create_category(&mut self, category: Category) -> Result<(), RepositoryError> {
        if self.categories.contains_key(&category.id()) {
            return Err(RepositoryError::DuplicateCategory);
        }
        self.categories.insert(category.id(), category);
        Ok(())
    }

    fn update_category(&mut self, category: Category) -> Result<(), RepositoryError> {
        if !self.categories.contains_key(&category.id()) {
            return Err(RepositoryError::CategoryNotFound);
        }
        self.categories.insert(category.id(), category);
        Ok(())
    }

    fn delete_category(&mut self, id: CategoryId) -> Result<Category, RepositoryError> {
        if self.events.values().any(|event| event.category_id() == id) {
            return Err(RepositoryError::CategoryInUse);
        }
        self.categories
            .remove(&id)
            .ok_or(RepositoryError::CategoryNotFound)
    }

    fn settings(&self) -> Result<Settings, RepositoryError> {
        Ok(self.settings.clone())
    }

    fn replace_settings(&mut self, settings: Settings) -> Result<(), RepositoryError> {
        self.settings = settings;
        Ok(())
    }
}

impl InMemoryRepository {
    fn ensure_category(&self, id: CategoryId) -> Result<(), RepositoryError> {
        if self.categories.contains_key(&id) {
            Ok(())
        } else {
            Err(RepositoryError::CategoryNotFound)
        }
    }
}

#[derive(Clone, Copy)]
struct SampleSlot {
    start_hour: i8,
    start_minute: i8,
    end_hour: i8,
    end_minute: i8,
    category_index: usize,
    title: &'static str,
    notes: Option<&'static str>,
}

const ROUTINE: usize = 0;
const FOCUS: usize = 1;
const BREAK: usize = 2;
const CAREER: usize = 3;
const INTERVIEW: usize = 4;
const PLANNING: usize = 5;

const fn slot(
    start_hour: i8,
    start_minute: i8,
    end_hour: i8,
    end_minute: i8,
    category_index: usize,
    title: &'static str,
) -> SampleSlot {
    SampleSlot {
        start_hour,
        start_minute,
        end_hour,
        end_minute,
        category_index,
        title,
        notes: None,
    }
}

const fn slot_with_notes(
    start_hour: i8,
    start_minute: i8,
    end_hour: i8,
    end_minute: i8,
    category_index: usize,
    title: &'static str,
    notes: &'static str,
) -> SampleSlot {
    SampleSlot {
        start_hour,
        start_minute,
        end_hour,
        end_minute,
        category_index,
        title,
        notes: Some(notes),
    }
}

/// Populate a repository with the weekday blocks from the planning screenshot.
///
/// # Parameters
///
/// - `repository`: Repository to populate.
/// - `date`: Date used to locate the sample week.
/// - `timestamp`: Creation timestamp assigned to sample events.
///
/// # Returns
///
/// The Sunday date that starts the populated sample week.
///
/// # Errors
///
/// Returns an error when:
///
/// - Date arithmetic cannot produce the requested week.
/// - A sample category or event is invalid.
/// - The repository rejects a sample category or event.
///
/// # Panics
///
/// Panics when:
///
/// - A built-in sample category index cannot fit in the supported `u16` range.
pub fn seed_sample_week(
    repository: &mut InMemoryRepository,
    date: Date,
    timestamp: Timestamp,
) -> Result<Date, RepositoryError> {
    let week_start = crate::domain::start_of_week(date, WeekStart::Sunday)
        .map_err(|error| RepositoryError::InvalidEntity(error.to_string()))?;

    let categories = [
        ("Routine", CategoryColor::Lime),
        ("Focus", CategoryColor::Violet),
        ("Break", CategoryColor::Yellow),
        ("Career", CategoryColor::Cyan),
        ("Interview", CategoryColor::Coral),
        ("Planning", CategoryColor::Blue),
    ];
    let category_ids = categories
        .iter()
        .enumerate()
        .map(|(index, (name, color))| {
            let id = CategoryId::from_uuid(Uuid::from_u128(
                u128::from(u16::try_from(index).expect("category index fits in u16")) + 1,
            ));
            let category = Category::new(id, *name, *color, true)
                .map_err(|error| RepositoryError::InvalidEntity(error.to_string()))?;
            repository.create_category(category)?;
            Ok::<_, RepositoryError>(id)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut event_index = 100u128;
    for day_offset in 0..7u8 {
        let date = add_days(week_start, day_offset)
            .map_err(|error| RepositoryError::InvalidEntity(error.to_string()))?;
        for slot in sample_schedule(day_offset) {
            let draft = EventDraft::new(
                slot.title,
                date,
                Time::constant(slot.start_hour, slot.start_minute, 0, 0),
                Time::constant(slot.end_hour, slot.end_minute, 0, 0),
                category_ids[slot.category_index],
                slot.notes.map(str::to_owned),
            );
            let id = EventId::from_uuid(Uuid::from_u128(event_index));
            let event = Event::new(id, draft, timestamp)
                .map_err(|error| RepositoryError::InvalidEntity(error.to_string()))?;
            repository.create_event(event)?;
            event_index += 1;
        }
    }

    Ok(week_start)
}

fn sample_schedule(day_offset: u8) -> Vec<SampleSlot> {
    match day_offset {
        0 => sunday_schedule(),
        1 => common_schedule(
            "Portfolio",
            "System Design",
            "Behavioural + technical interview prep",
            "Applications",
            "Shutdown + plan",
        ),
        2 => common_schedule(
            "Portfolio",
            "Coding / DSA",
            "DSA interview prep",
            "Applications",
            "Shutdown + plan",
        ),
        3 => common_schedule(
            "Portfolio",
            "System Design",
            "Backend / database interview prep",
            "Applications",
            "Shutdown + plan",
        ),
        4 => common_schedule(
            "Portfolio",
            "Coding / DSA",
            "DSA + live coding",
            "Applications",
            "Shutdown + plan",
        ),
        5 => common_schedule(
            "Portfolio / GitHub cleanup",
            "System Design + review",
            "Mock interview + weekly review",
            "Applications + follow-ups",
            "Weekly review",
        ),
        6 => saturday_schedule(),
        _ => unreachable!("sample schedule only supports seven days"),
    }
}

fn common_schedule(
    focus_title: &'static str,
    technical_title: &'static str,
    interview_title: &'static str,
    afternoon_title: &'static str,
    closing_title: &'static str,
) -> Vec<SampleSlot> {
    vec![
        slot(7, 30, 8, 0, ROUTINE, "Breakfast + plan"),
        slot_with_notes(
            8,
            0,
            10,
            0,
            FOCUS,
            "Thesis",
            "Protect this deep-work block and leave a short progress note before the break.",
        ),
        slot(10, 0, 10, 20, BREAK, "Break"),
        slot(10, 20, 11, 50, FOCUS, technical_title),
        slot(11, 50, 12, 30, CAREER, "Job discovery"),
        slot(12, 30, 13, 30, ROUTINE, "Lunch"),
        slot(13, 30, 14, 45, CAREER, afternoon_title),
        slot(14, 45, 15, 0, BREAK, "Break"),
        slot(15, 0, 16, 15, INTERVIEW, interview_title),
        slot(16, 15, 16, 30, BREAK, "Break"),
        slot(16, 30, 18, 0, CAREER, focus_title),
        slot(18, 0, 18, 20, PLANNING, closing_title),
        slot(18, 30, 21, 30, ROUTINE, "Personal time"),
    ]
}

fn sunday_schedule() -> Vec<SampleSlot> {
    vec![
        slot(7, 30, 8, 0, ROUTINE, "Rest"),
        slot(18, 0, 18, 20, PLANNING, "Weekly planning"),
        slot(18, 30, 21, 30, ROUTINE, "Rest"),
    ]
}

fn saturday_schedule() -> Vec<SampleSlot> {
    vec![
        slot(7, 30, 8, 0, ROUTINE, "Slow start"),
        slot_with_notes(
            8,
            0,
            10,
            0,
            FOCUS,
            "Thesis / weekly catch-up",
            "Review the week's open questions, then choose the smallest useful next step.",
        ),
        slot(10, 0, 10, 20, BREAK, "Break"),
        slot(10, 20, 11, 50, FOCUS, "Coding / project build"),
        slot(11, 50, 12, 30, CAREER, "Job search review"),
        slot(12, 30, 13, 30, ROUTINE, "Lunch"),
        slot(13, 30, 14, 45, CAREER, "Optional applications"),
        slot(14, 45, 15, 0, BREAK, "Break"),
        slot(15, 0, 16, 15, INTERVIEW, "Interview weak spots"),
        slot(16, 15, 16, 30, BREAK, "Break"),
        slot(16, 30, 18, 0, CAREER, "Long project session"),
        slot(18, 0, 18, 20, PLANNING, "Finish / commit"),
        slot(18, 30, 21, 30, ROUTINE, "Personal time"),
    ]
}

fn add_days(date: Date, days: u8) -> Result<Date, jiff::Error> {
    let mut result = date;
    for _ in 0..days {
        result = result.tomorrow()?;
    }
    Ok(result)
}
