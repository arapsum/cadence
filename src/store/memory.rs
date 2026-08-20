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

#[derive(Clone, Copy)]
struct SampleSlot {
    start_hour: i8,
    start_minute: i8,
    end_hour: i8,
    end_minute: i8,
    category_index: usize,
    title: &'static str,
}

/// Populate a repository with the weekday blocks from the planning screenshot.
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
            let id = CategoryId::from_uuid(Uuid::from_u128(index as u128 + 1));
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
                None,
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
    const ROUTINE: usize = 0;
    const FOCUS: usize = 1;
    const BREAK: usize = 2;
    const CAREER: usize = 3;
    const INTERVIEW: usize = 4;
    const PLANNING: usize = 5;

    let common = |focus_title: &'static str,
                  technical_title: &'static str,
                  interview_title: &'static str,
                  afternoon_title: &'static str,
                  closing_title: &'static str| {
        vec![
            SampleSlot {
                start_hour: 7,
                start_minute: 30,
                end_hour: 8,
                end_minute: 0,
                category_index: ROUTINE,
                title: "Breakfast + plan",
            },
            SampleSlot {
                start_hour: 8,
                start_minute: 0,
                end_hour: 10,
                end_minute: 0,
                category_index: FOCUS,
                title: "Thesis",
            },
            SampleSlot {
                start_hour: 10,
                start_minute: 0,
                end_hour: 10,
                end_minute: 20,
                category_index: BREAK,
                title: "Break",
            },
            SampleSlot {
                start_hour: 10,
                start_minute: 20,
                end_hour: 11,
                end_minute: 50,
                category_index: FOCUS,
                title: technical_title,
            },
            SampleSlot {
                start_hour: 11,
                start_minute: 50,
                end_hour: 12,
                end_minute: 30,
                category_index: CAREER,
                title: "Job discovery",
            },
            SampleSlot {
                start_hour: 12,
                start_minute: 30,
                end_hour: 13,
                end_minute: 30,
                category_index: ROUTINE,
                title: "Lunch",
            },
            SampleSlot {
                start_hour: 13,
                start_minute: 30,
                end_hour: 14,
                end_minute: 45,
                category_index: CAREER,
                title: afternoon_title,
            },
            SampleSlot {
                start_hour: 14,
                start_minute: 45,
                end_hour: 15,
                end_minute: 0,
                category_index: BREAK,
                title: "Break",
            },
            SampleSlot {
                start_hour: 15,
                start_minute: 0,
                end_hour: 16,
                end_minute: 15,
                category_index: INTERVIEW,
                title: interview_title,
            },
            SampleSlot {
                start_hour: 16,
                start_minute: 15,
                end_hour: 16,
                end_minute: 30,
                category_index: BREAK,
                title: "Break",
            },
            SampleSlot {
                start_hour: 16,
                start_minute: 30,
                end_hour: 18,
                end_minute: 0,
                category_index: CAREER,
                title: focus_title,
            },
            SampleSlot {
                start_hour: 18,
                start_minute: 0,
                end_hour: 18,
                end_minute: 20,
                category_index: PLANNING,
                title: closing_title,
            },
            SampleSlot {
                start_hour: 18,
                start_minute: 30,
                end_hour: 21,
                end_minute: 30,
                category_index: ROUTINE,
                title: "Personal time",
            },
        ]
    };

    match day_offset {
        0 => vec![
            SampleSlot {
                start_hour: 7,
                start_minute: 30,
                end_hour: 8,
                end_minute: 0,
                category_index: ROUTINE,
                title: "Rest",
            },
            SampleSlot {
                start_hour: 18,
                start_minute: 0,
                end_hour: 18,
                end_minute: 20,
                category_index: PLANNING,
                title: "Weekly planning",
            },
            SampleSlot {
                start_hour: 18,
                start_minute: 30,
                end_hour: 21,
                end_minute: 30,
                category_index: ROUTINE,
                title: "Rest",
            },
        ],
        1 => common(
            "Portfolio",
            "System Design",
            "Behavioural + technical interview prep",
            "Applications",
            "Shutdown + plan",
        ),
        2 => common(
            "Portfolio",
            "Coding / DSA",
            "DSA interview prep",
            "Applications",
            "Shutdown + plan",
        ),
        3 => common(
            "Portfolio",
            "System Design",
            "Backend / database interview prep",
            "Applications",
            "Shutdown + plan",
        ),
        4 => common(
            "Portfolio",
            "Coding / DSA",
            "DSA + live coding",
            "Applications",
            "Shutdown + plan",
        ),
        5 => common(
            "Portfolio / GitHub cleanup",
            "System Design + review",
            "Mock interview + weekly review",
            "Applications + follow-ups",
            "Weekly review",
        ),
        6 => vec![
            SampleSlot {
                start_hour: 7,
                start_minute: 30,
                end_hour: 8,
                end_minute: 0,
                category_index: ROUTINE,
                title: "Slow start",
            },
            SampleSlot {
                start_hour: 8,
                start_minute: 0,
                end_hour: 10,
                end_minute: 0,
                category_index: FOCUS,
                title: "Thesis / weekly catch-up",
            },
            SampleSlot {
                start_hour: 10,
                start_minute: 0,
                end_hour: 10,
                end_minute: 20,
                category_index: BREAK,
                title: "Break",
            },
            SampleSlot {
                start_hour: 10,
                start_minute: 20,
                end_hour: 11,
                end_minute: 50,
                category_index: FOCUS,
                title: "Coding / project build",
            },
            SampleSlot {
                start_hour: 11,
                start_minute: 50,
                end_hour: 12,
                end_minute: 30,
                category_index: CAREER,
                title: "Job search review",
            },
            SampleSlot {
                start_hour: 12,
                start_minute: 30,
                end_hour: 13,
                end_minute: 30,
                category_index: ROUTINE,
                title: "Lunch",
            },
            SampleSlot {
                start_hour: 13,
                start_minute: 30,
                end_hour: 14,
                end_minute: 45,
                category_index: CAREER,
                title: "Optional applications",
            },
            SampleSlot {
                start_hour: 14,
                start_minute: 45,
                end_hour: 15,
                end_minute: 0,
                category_index: BREAK,
                title: "Break",
            },
            SampleSlot {
                start_hour: 15,
                start_minute: 0,
                end_hour: 16,
                end_minute: 15,
                category_index: INTERVIEW,
                title: "Interview weak spots",
            },
            SampleSlot {
                start_hour: 16,
                start_minute: 15,
                end_hour: 16,
                end_minute: 30,
                category_index: BREAK,
                title: "Break",
            },
            SampleSlot {
                start_hour: 16,
                start_minute: 30,
                end_hour: 18,
                end_minute: 0,
                category_index: CAREER,
                title: "Long project session",
            },
            SampleSlot {
                start_hour: 18,
                start_minute: 0,
                end_hour: 18,
                end_minute: 20,
                category_index: PLANNING,
                title: "Finish / commit",
            },
            SampleSlot {
                start_hour: 18,
                start_minute: 30,
                end_hour: 21,
                end_minute: 30,
                category_index: ROUTINE,
                title: "Personal time",
            },
        ],
        _ => unreachable!(),
    }
}

fn add_days(date: Date, days: u8) -> Result<Date, jiff::Error> {
    let mut result = date;
    for _ in 0..days {
        result = result.tomorrow()?;
    }
    Ok(result)
}
