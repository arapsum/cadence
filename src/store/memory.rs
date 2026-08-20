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

#[derive(Debug, Clone, Default)]
pub struct InMemoryRepository {
    events: HashMap<EventId, Event>,
    categories: HashMap<CategoryId, Category>,
    settings: Settings,
}

impl InMemoryRepository {
    pub fn new(settings: Settings) -> Self {
        Self {
            events: HashMap::new(),
            categories: HashMap::new(),
            settings,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(Settings::default())
    }

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

/// Populate a repository with a repeatable week resembling the supplied UI reference.
pub fn seed_sample_week(
    repository: &mut InMemoryRepository,
    date: Date,
    timestamp: Timestamp,
) -> Result<Date, RepositoryError> {
    let week_start = crate::domain::start_of_week(date, WeekStart::Sunday)
        .map_err(|error| RepositoryError::InvalidEntity(error.to_string()))?;

    let categories = [
        ("Workout", CategoryColor::Lime),
        ("Reading", CategoryColor::Yellow),
        ("Learning", CategoryColor::Violet),
        ("Writing", CategoryColor::Cyan),
        ("Crafting", CategoryColor::Blue),
    ];
    let category_ids = categories
        .iter()
        .enumerate()
        .map(|(index, (name, color))| {
            let id = CategoryId::from_uuid(Uuid::from_u128(index as u128 + 1));
            let category = Category::new(id, *name, *color, true)
                .map_err(|error| RepositoryError::InvalidEntity(error.to_string()))?;
            repository.create_category(category)?;
            Ok::<_, RepositoryError>(id)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let workout = category_ids[0];
    let reading = category_ids[1];
    let learning = category_ids[2];
    let writing = category_ids[3];
    let crafting = category_ids[4];

    let samples = [
        (
            0,
            6,
            0,
            6,
            30,
            workout,
            "Morning Routine - Stretch Out",
            None,
        ),
        (
            0,
            7,
            0,
            8,
            0,
            reading,
            "Continue Reading The Design Of Everyday Things",
            None,
        ),
        (1, 9, 0, 10, 0, workout, "Gym Day - leg day session", None),
        (
            1,
            10,
            0,
            10,
            30,
            reading,
            "Plan the next reading block",
            None,
        ),
        (
            2,
            6,
            0,
            6,
            30,
            workout,
            "Morning Routine - Stretch Out",
            None,
        ),
        (2, 9, 0, 10, 0, workout, "Gym Day - chest day session", None),
        (
            2,
            13,
            0,
            13,
            30,
            crafting,
            "Videocoding Market Place App",
            None,
        ),
        (3, 7, 0, 8, 0, workout, "Gym Day - back day session", None),
        (
            3,
            7,
            30,
            8,
            30,
            learning,
            "How to make a michelin fried rice",
            Some("Overlapping sample"),
        ),
        (
            3,
            11,
            0,
            12,
            0,
            learning,
            "How to make a michelin fried rice",
            None,
        ),
        (
            4,
            9,
            0,
            10,
            0,
            workout,
            "Gym Day - shoulder day session",
            None,
        ),
        (
            6,
            7,
            0,
            8,
            0,
            writing,
            "Content idea mapping and creation",
            None,
        ),
    ];

    for (
        index,
        (day_offset, start_hour, start_minute, end_hour, end_minute, category_id, title, notes),
    ) in samples.into_iter().enumerate()
    {
        let date = add_days(week_start, day_offset)
            .map_err(|error| RepositoryError::InvalidEntity(error.to_string()))?;
        let draft = EventDraft::new(
            title,
            date,
            Time::constant(start_hour, start_minute, 0, 0),
            Time::constant(end_hour, end_minute, 0, 0),
            category_id,
            notes.map(str::to_owned),
        );
        let id = EventId::from_uuid(Uuid::from_u128(100 + index as u128));
        let event = Event::new(id, draft, timestamp)
            .map_err(|error| RepositoryError::InvalidEntity(error.to_string()))?;
        repository.create_event(event)?;
    }

    Ok(week_start)
}

fn add_days(date: Date, days: u8) -> Result<Date, jiff::Error> {
    let mut result = date;
    for _ in 0..days {
        result = result.tomorrow()?;
    }
    Ok(result)
}
