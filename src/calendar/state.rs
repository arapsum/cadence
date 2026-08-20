use jiff::civil::Date;

use crate::domain::{CalendarError, CategoryId, EventId, WeekStart};

/// The category selector shown above a calendar surface.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum CategoryFilter {
    All,
    Only(CategoryId),
}

/// Small, view-independent state machine for navigating a calendar week.
///
/// # Fields
///
/// - `selected_date`: Date used to anchor the active calendar range.
/// - `week_start`: First weekday used when deriving calendar ranges.
/// - `category_filter`: Category filter applied to the active surface.
/// - `selected_event`: Optional event currently selected by the user.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CalendarState {
    selected_date: Date,
    week_start: WeekStart,
    category_filter: CategoryFilter,
    selected_event: Option<EventId>,
}

impl CalendarState {
    /// Creates calendar navigation state.
    ///
    /// # Parameters
    ///
    /// - `selected_date`: Date used to anchor the active calendar range.
    /// - `week_start`: First weekday used when deriving calendar ranges.
    ///
    /// # Returns
    ///
    /// Calendar state with no event selected and the `All` category filter.
    #[must_use]
    pub const fn new(selected_date: Date, week_start: WeekStart) -> Self {
        Self {
            selected_date,
            week_start,
            category_filter: CategoryFilter::All,
            selected_event: None,
        }
    }

    /// Returns the date anchoring the active calendar range.
    #[must_use]
    pub const fn selected_date(self) -> Date {
        self.selected_date
    }

    /// Returns the configured first weekday.
    #[must_use]
    pub const fn week_start(self) -> WeekStart {
        self.week_start
    }

    /// Returns the active category filter.
    #[must_use]
    pub const fn category_filter(self) -> CategoryFilter {
        self.category_filter
    }

    /// Returns the selected event, when one exists.
    #[must_use]
    pub const fn selected_event(self) -> Option<EventId> {
        self.selected_event
    }

    /// Changes the first weekday used by calendar ranges.
    ///
    /// # Parameters
    ///
    /// - `week_start`: New first weekday.
    pub const fn set_week_start(&mut self, week_start: WeekStart) {
        self.week_start = week_start;
    }

    /// Changes the active category filter.
    ///
    /// # Parameters
    ///
    /// - `category_filter`: New category filter.
    pub const fn set_category_filter(&mut self, category_filter: CategoryFilter) {
        self.category_filter = category_filter;
    }

    /// Selects an event.
    ///
    /// # Parameters
    ///
    /// - `event_id`: Identifier of the event to select.
    pub const fn select_event(&mut self, event_id: EventId) {
        self.selected_event = Some(event_id);
    }

    /// Clears the selected event.
    pub const fn clear_selection(&mut self) {
        self.selected_event = None;
    }

    /// Moves the active range anchor to today.
    ///
    /// # Parameters
    ///
    /// - `today`: Date to use as the active range anchor.
    pub const fn go_to_today(&mut self, today: Date) {
        self.selected_date = today;
    }

    /// Moves the active range one week backward.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the selected date is moved and any event selection is cleared.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - Date arithmetic cannot produce the previous week.
    pub fn previous_week(&mut self) -> Result<(), CalendarError> {
        let mut date = self.selected_date;
        for _ in 0..7 {
            date = date.yesterday()?;
        }
        self.selected_date = date;
        self.selected_event = None;
        Ok(())
    }

    /// Moves the active range one week forward.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the selected date is moved and any event selection is cleared.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - Date arithmetic cannot produce the next week.
    pub fn next_week(&mut self) -> Result<(), CalendarError> {
        let mut date = self.selected_date;
        for _ in 0..7 {
            date = date.tomorrow()?;
        }
        self.selected_date = date;
        self.selected_event = None;
        Ok(())
    }
}
