mod memory;

use crate::domain::{Category, CategoryId, DateRange, Event, EventId, RepositoryError, Settings};

pub use memory::{InMemoryRepository, seed_sample_week};

/// Storage contract shared by the in-memory and future SQLite repositories.
pub trait TimetableRepository {
    fn event(&self, id: EventId) -> Result<Option<Event>, RepositoryError>;
    fn events(&self, range: DateRange) -> Result<Vec<Event>, RepositoryError>;
    fn create_event(&mut self, event: Event) -> Result<(), RepositoryError>;
    fn update_event(&mut self, event: Event) -> Result<(), RepositoryError>;
    fn delete_event(&mut self, id: EventId) -> Result<Event, RepositoryError>;

    fn categories(&self) -> Result<Vec<Category>, RepositoryError>;
    fn category(&self, id: CategoryId) -> Result<Option<Category>, RepositoryError>;
    fn create_category(&mut self, category: Category) -> Result<(), RepositoryError>;
    fn update_category(&mut self, category: Category) -> Result<(), RepositoryError>;
    fn delete_category(&mut self, id: CategoryId) -> Result<Category, RepositoryError>;

    fn settings(&self) -> Result<Settings, RepositoryError>;
    fn replace_settings(&mut self, settings: Settings) -> Result<(), RepositoryError>;
}
