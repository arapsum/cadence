use jiff::civil::Date;

use crate::domain::{
    CalendarError, CategoryId, DateRange, EventId, WeekStart, next_day, next_week, previous_day,
    previous_week,
};

/// The category selector shown above a calendar surface.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum CategoryFilter {
    All,
    Only(CategoryId),
}

/// Calendar surface used to present the timetable.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum CalendarViewMode {
    Day,
    Week,
}

/// Small, view-independent state machine for navigating a calendar surface.
///
/// # Fields
///
/// - `selected_date`: Date used to anchor the active calendar range.
/// - `week_start`: First weekday used when deriving calendar ranges.
/// - `view_mode`: Active day or week surface.
/// - `category_filter`: Category filter applied to the active surface.
/// - `selected_event`: Optional event currently selected by the user.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CalendarState {
    selected_date: Date,
    week_start: WeekStart,
    view_mode: CalendarViewMode,
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
    /// - `view_mode`: Initial day or week surface.
    ///
    /// # Returns
    ///
    /// Calendar state with no event selected and the `All` category filter.
    #[must_use]
    pub const fn new(
        selected_date: Date,
        week_start: WeekStart,
        view_mode: CalendarViewMode,
    ) -> Self {
        Self {
            selected_date,
            week_start,
            view_mode,
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

    /// Returns the active calendar surface mode.
    #[must_use]
    pub const fn view_mode(self) -> CalendarViewMode {
        self.view_mode
    }

    /// Changes the active calendar surface mode.
    ///
    /// # Parameters
    ///
    /// - `view_mode`: New day or week surface.
    pub const fn set_view_mode(&mut self, view_mode: CalendarViewMode) {
        self.view_mode = view_mode;
    }

    /// Returns the date range represented by the active surface.
    ///
    /// # Returns
    ///
    /// A one-day or seven-day range anchored by the selected date.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - Date arithmetic cannot produce the active range.
    pub fn visible_range(self) -> Result<DateRange, CalendarError> {
        match self.view_mode {
            CalendarViewMode::Day => DateRange::day(self.selected_date),
            CalendarViewMode::Week => DateRange::week(self.selected_date, self.week_start),
        }
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

    /// Selects a date and clears the selected event.
    ///
    /// # Parameters
    ///
    /// - `date`: Date to select.
    pub const fn select_date(&mut self, date: Date) {
        self.selected_date = date;
        self.selected_event = None;
    }

    /// Selects an event and its date.
    ///
    /// # Parameters
    ///
    /// - `event_id`: Identifier of the event to select.
    /// - `date`: Date on which the event occurs.
    pub const fn select_event(&mut self, event_id: EventId, date: Date) {
        self.selected_date = date;
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
        self.selected_event = None;
    }

    /// Moves the active range one period backward.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the selected date is moved and any event selection is cleared.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - Date arithmetic cannot produce the previous period.
    pub fn previous_period(&mut self) -> Result<(), CalendarError> {
        self.selected_date = match self.view_mode {
            CalendarViewMode::Day => previous_day(self.selected_date)?,
            CalendarViewMode::Week => previous_week(self.selected_date, self.week_start)?,
        };
        self.selected_event = None;
        Ok(())
    }

    /// Moves the active range one period forward.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the selected date is moved and any event selection is cleared.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - Date arithmetic cannot produce the next period.
    pub fn next_period(&mut self) -> Result<(), CalendarError> {
        self.selected_date = match self.view_mode {
            CalendarViewMode::Day => next_day(self.selected_date)?,
            CalendarViewMode::Week => next_week(self.selected_date, self.week_start)?,
        };
        self.selected_event = None;
        Ok(())
    }
}
