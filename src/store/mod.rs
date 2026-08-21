mod memory;
mod sqlite;
mod worker;

use crate::domain::{Category, CategoryId, DateRange, Event, EventId, RepositoryError, Settings};

pub use memory::{InMemoryRepository, default_categories, seed_sample_week};
pub use sqlite::{SqliteRepository, StorageError, data_directory, database_path};
pub use worker::{StorageClient, StorageSnapshot};

/// Preferences restored when Cadence starts.
#[derive(Debug, Clone, Copy, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct AppPreferences {
    /// Calendar surface selected at the previous shutdown.
    pub view_mode: CalendarViewModePreference,
    /// Category selected at the previous shutdown, or `None` for all categories.
    pub category_filter: Option<CategoryId>,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            view_mode: CalendarViewModePreference::Week,
            category_filter: None,
        }
    }
}

/// Serializable calendar view preference.
#[derive(Debug, Clone, Copy, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum CalendarViewModePreference {
    /// Focused one-day view.
    Day,
    /// Seven-day view.
    Week,
}

/// Complete persisted application state used by the worker and backup format.
#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PersistenceSnapshot {
    /// Calendar settings.
    pub settings: Settings,
    /// Startup mode and category filter.
    pub preferences: AppPreferences,
    /// Stored categories.
    pub categories: Vec<Category>,
    /// Stored events.
    pub events: Vec<Event>,
}

/// Storage contract shared by the in-memory and `SQLite` repositories.
pub trait TimetableRepository {
    /// Looks up one event by identifier.
    ///
    /// # Parameters
    ///
    /// - `id`: Event identifier to find.
    ///
    /// # Returns
    ///
    /// The matching event, or `None` when it is not stored.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - The repository cannot read its event store.
    fn event(&self, id: EventId) -> Result<Option<Event>, RepositoryError>;

    /// Lists events whose dates belong to a range.
    ///
    /// # Parameters
    ///
    /// - `range`: Date range to query.
    ///
    /// # Returns
    ///
    /// Events contained by `range`.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - The repository cannot read its event store.
    fn events(&self, range: DateRange) -> Result<Vec<Event>, RepositoryError>;

    /// Stores a new event.
    ///
    /// # Parameters
    ///
    /// - `event`: Event to store.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the event is stored.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - An event with the same identifier already exists.
    /// - The event references a category that is not stored.
    /// - The repository cannot write its event store.
    fn create_event(&mut self, event: Event) -> Result<(), RepositoryError>;

    /// Replaces an existing event.
    ///
    /// # Parameters
    ///
    /// - `event`: Revised event to store.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the event is replaced.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - The event does not exist.
    /// - The event references a category that is not stored.
    /// - The repository cannot write its event store.
    fn update_event(&mut self, event: Event) -> Result<(), RepositoryError>;

    /// Removes an event by identifier.
    ///
    /// # Parameters
    ///
    /// - `id`: Event identifier to remove.
    ///
    /// # Returns
    ///
    /// The removed event.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - The event does not exist.
    /// - The repository cannot write its event store.
    fn delete_event(&mut self, id: EventId) -> Result<Event, RepositoryError>;

    /// Lists all stored categories.
    ///
    /// # Returns
    ///
    /// The stored categories.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - The repository cannot read its category store.
    fn categories(&self) -> Result<Vec<Category>, RepositoryError>;

    /// Looks up one category by identifier.
    ///
    /// # Parameters
    ///
    /// - `id`: Category identifier to find.
    ///
    /// # Returns
    ///
    /// The matching category, or `None` when it is not stored.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - The repository cannot read its category store.
    fn category(&self, id: CategoryId) -> Result<Option<Category>, RepositoryError>;

    /// Stores a new category.
    ///
    /// # Parameters
    ///
    /// - `category`: Category to store.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the category is stored.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - A category with the same identifier already exists.
    /// - The repository cannot write its category store.
    fn create_category(&mut self, category: Category) -> Result<(), RepositoryError>;

    /// Replaces an existing category.
    ///
    /// # Parameters
    ///
    /// - `category`: Revised category to store.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the category is replaced.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - The category does not exist.
    /// - The repository cannot write its category store.
    fn update_category(&mut self, category: Category) -> Result<(), RepositoryError>;

    /// Removes a category by identifier.
    ///
    /// # Parameters
    ///
    /// - `id`: Category identifier to remove.
    ///
    /// # Returns
    ///
    /// The removed category.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - The category does not exist.
    /// - Events still reference the category.
    /// - The repository cannot write its category store.
    fn delete_category(&mut self, id: CategoryId) -> Result<Category, RepositoryError>;

    /// Returns the current application settings.
    ///
    /// # Returns
    ///
    /// The stored application settings.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - The repository cannot read its settings store.
    fn settings(&self) -> Result<Settings, RepositoryError>;

    /// Replaces the current application settings.
    ///
    /// # Parameters
    ///
    /// - `settings`: Settings to store.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the settings are stored.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - The repository cannot write its settings store.
    fn replace_settings(&mut self, settings: Settings) -> Result<(), RepositoryError>;

    /// Returns the persisted startup preferences.
    ///
    /// # Returns
    ///
    /// The stored view mode and category filter.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - The repository cannot read its preferences store.
    fn preferences(&self) -> Result<AppPreferences, RepositoryError>;

    /// Replaces the persisted startup preferences.
    ///
    /// # Parameters
    ///
    /// - `preferences`: Startup preferences to store.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the preferences are stored.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - The repository cannot write its preferences store.
    fn replace_preferences(&mut self, preferences: AppPreferences) -> Result<(), RepositoryError>;

    /// Returns a deterministic copy of all persisted data.
    ///
    /// # Returns
    ///
    /// Settings, preferences, categories, and events suitable for export.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - Any persisted table cannot be read.
    fn snapshot(&self) -> Result<PersistenceSnapshot, RepositoryError> {
        Ok(PersistenceSnapshot {
            settings: self.settings()?,
            preferences: self.preferences()?,
            categories: self.categories()?,
            events: self.events(DateRange::new(
                jiff::civil::Date::MIN,
                jiff::civil::Date::MAX,
            )?)?,
        })
    }
}
