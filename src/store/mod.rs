mod memory;

use crate::domain::{Category, CategoryId, DateRange, Event, EventId, RepositoryError, Settings};

pub use memory::{InMemoryRepository, seed_sample_week};

/// Storage contract shared by the in-memory and future `SQLite` repositories.
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
}
