mod memory;
mod sqlite;
mod worker;

use crate::domain::{
    Category, CategoryId, DateRange, Event, EventId, EventOccurrence, OccurrenceId,
    RecurrenceException, RecurrenceSeries, RecurrenceSeriesId, RepositoryError, Settings,
    expand_series,
};

pub use memory::{InMemoryRepository, default_categories, seed_sample_week};
pub use sqlite::{SqliteRepository, StorageError, data_directory, database_path};
pub use worker::{StorageClient, StorageSnapshot};

/// Preferences restored when Cadence starts.
#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct AppPreferences {
    /// Calendar surface selected at the previous shutdown.
    pub view_mode: CalendarViewModePreference,
    /// Category selected at the previous shutdown, or `None` for all categories.
    pub category_filter: Option<CategoryId>,
    /// Whether desktop reminders may be delivered while Cadence is running.
    #[serde(default)]
    pub notifications_enabled: bool,
    /// Whether non-essential interface animation is reduced.
    #[serde(default)]
    pub reduce_motion: bool,
    /// Appearance and typography selected by the user.
    #[serde(default)]
    pub appearance: AppearancePreferences,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            view_mode: CalendarViewModePreference::Week,
            category_filter: None,
            notifications_enabled: false,
            reduce_motion: false,
            appearance: AppearancePreferences::default(),
        }
    }
}

/// Appearance mode selected for the Cadence window.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppearanceMode {
    /// Follow the operating system appearance.
    #[default]
    System,
    /// Always use the configured light theme.
    Light,
    /// Always use the configured dark theme.
    Dark,
}

/// Persisted theme and typography preferences.
#[derive(Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct AppearancePreferences {
    /// Whether the active mode follows the system or is explicitly selected.
    #[serde(default)]
    pub mode: AppearanceMode,
    /// Theme applied whenever the active mode is light.
    #[serde(default = "default_light_theme")]
    pub light_theme: String,
    /// Theme applied whenever the active mode is dark.
    #[serde(default = "default_dark_theme")]
    pub dark_theme: String,
    /// Installed font family used for application text.
    #[serde(default = "default_font_family")]
    pub font_family: String,
    /// Base application font size in logical pixels.
    #[serde(default = "default_font_size")]
    pub font_size: u16,
}

impl AppearancePreferences {
    /// Supported application font-size presets.
    pub const FONT_SIZES: [u16; 3] = [14, 16, 18];
}

impl Default for AppearancePreferences {
    fn default() -> Self {
        Self {
            mode: AppearanceMode::System,
            light_theme: default_light_theme(),
            dark_theme: default_dark_theme(),
            font_family: default_font_family(),
            font_size: default_font_size(),
        }
    }
}

fn default_light_theme() -> String {
    "Default Light".to_owned()
}

fn default_dark_theme() -> String {
    "Default Dark".to_owned()
}

fn default_font_family() -> String {
    ".SystemUIFont".to_owned()
}

const fn default_font_size() -> u16 {
    16
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
    /// Stored recurring series.
    pub recurrence_series: Vec<RecurrenceSeries>,
    /// Stored recurring occurrence exceptions.
    pub recurrence_exceptions: Vec<RecurrenceException>,
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

    /// Lists standalone and recurring occurrences intersecting `range`.
    ///
    /// # Errors
    ///
    /// Returns an error when any underlying event, series, or exception query fails.
    fn occurrences(&self, range: DateRange) -> Result<Vec<EventOccurrence>, RepositoryError> {
        let mut occurrences = self
            .events(range)?
            .iter()
            .map(EventOccurrence::standalone)
            .collect::<Vec<_>>();
        let exceptions = self.recurrence_exceptions()?;
        for series in self.recurrence_series()? {
            let series_exceptions = exceptions
                .iter()
                .filter(|exception| exception.series_id() == series.id())
                .cloned()
                .collect::<Vec<_>>();
            occurrences.extend(expand_series(&series, &series_exceptions, range));
        }
        occurrences.sort_by_key(|occurrence| {
            (
                occurrence.date(),
                occurrence.start_time(),
                occurrence.end_time(),
                occurrence.id(),
            )
        });
        Ok(occurrences)
    }

    /// Looks up one standalone or recurring occurrence.
    ///
    /// # Errors
    ///
    /// Returns an error when the repository cannot read the requested occurrence data.
    fn occurrence(&self, id: OccurrenceId) -> Result<Option<EventOccurrence>, RepositoryError> {
        match id {
            OccurrenceId::Standalone(event_id) => Ok(self
                .event(event_id)?
                .as_ref()
                .map(EventOccurrence::standalone)),
            OccurrenceId::Recurring {
                series_id,
                original_date,
            } => {
                let Some(series) = self.series(series_id)? else {
                    return Ok(None);
                };
                let exception = self.recurrence_exceptions()?.into_iter().find(|exception| {
                    exception.series_id() == series_id && exception.original_date() == original_date
                });
                match exception {
                    Some(exception) => match exception.kind() {
                        crate::domain::RecurrenceExceptionKind::Cancelled => Ok(None),
                        crate::domain::RecurrenceExceptionKind::Modified(draft) => Ok(Some(
                            EventOccurrence::recurring(series_id, original_date, draft.clone()),
                        )),
                    },
                    None if series.contains_date(original_date) => {
                        let mut draft = series.template();
                        draft.date = original_date;
                        Ok(Some(EventOccurrence::recurring(
                            series_id,
                            original_date,
                            draft,
                        )))
                    }
                    None => Ok(None),
                }
            }
        }
    }

    /// Lists all stored recurring series.
    ///
    /// # Errors
    ///
    /// Returns an error when the repository cannot read its recurring-series store.
    fn recurrence_series(&self) -> Result<Vec<RecurrenceSeries>, RepositoryError>;

    /// Looks up one recurring series.
    ///
    /// # Errors
    ///
    /// Returns an error when the repository cannot read its recurring-series store.
    fn series(&self, id: RecurrenceSeriesId) -> Result<Option<RecurrenceSeries>, RepositoryError>;

    /// Lists all persisted recurring exceptions.
    ///
    /// # Errors
    ///
    /// Returns an error when the repository cannot read its recurring-exception store.
    fn recurrence_exceptions(&self) -> Result<Vec<RecurrenceException>, RepositoryError>;

    /// Stores a new recurring series.
    ///
    /// # Errors
    ///
    /// Returns an error when the series already exists, references an unknown category, or
    /// cannot be written.
    fn create_series(&mut self, series: RecurrenceSeries) -> Result<(), RepositoryError>;

    /// Replaces an existing recurring series.
    ///
    /// # Errors
    ///
    /// Returns an error when the series does not exist, references an unknown category, or
    /// cannot be written.
    fn update_series(&mut self, series: RecurrenceSeries) -> Result<(), RepositoryError>;

    /// Removes a recurring series and its exceptions.
    ///
    /// # Errors
    ///
    /// Returns an error when the series does not exist or cannot be written.
    fn delete_series(
        &mut self,
        id: RecurrenceSeriesId,
    ) -> Result<RecurrenceSeries, RepositoryError>;

    /// Inserts or replaces one recurring exception.
    ///
    /// # Errors
    ///
    /// Returns an error when the owning series does not exist or the exception cannot be written.
    fn upsert_exception(&mut self, exception: RecurrenceException) -> Result<(), RepositoryError>;

    /// Removes one recurring exception, returning it when present.
    ///
    /// # Errors
    ///
    /// Returns an error when the exception cannot be removed.
    fn delete_exception(
        &mut self,
        series_id: RecurrenceSeriesId,
        original_date: jiff::civil::Date,
    ) -> Result<Option<RecurrenceException>, RepositoryError>;

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
            recurrence_series: self.recurrence_series()?,
            recurrence_exceptions: self.recurrence_exceptions()?,
        })
    }
}
