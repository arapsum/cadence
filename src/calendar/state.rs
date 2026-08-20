use jiff::civil::Date;

use crate::domain::{CalendarError, CategoryId, EventId, WeekStart};

/// The category selector shown above a calendar surface.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum CategoryFilter {
    All,
    Only(CategoryId),
}

/// Small, view-independent state machine for navigating a calendar week.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CalendarState {
    selected_date: Date,
    week_start: WeekStart,
    category_filter: CategoryFilter,
    selected_event: Option<EventId>,
}

impl CalendarState {
    pub fn new(selected_date: Date, week_start: WeekStart) -> Self {
        Self {
            selected_date,
            week_start,
            category_filter: CategoryFilter::All,
            selected_event: None,
        }
    }

    pub fn selected_date(self) -> Date {
        self.selected_date
    }

    pub fn week_start(self) -> WeekStart {
        self.week_start
    }

    pub fn category_filter(self) -> CategoryFilter {
        self.category_filter
    }

    pub fn selected_event(self) -> Option<EventId> {
        self.selected_event
    }

    pub fn set_week_start(&mut self, week_start: WeekStart) {
        self.week_start = week_start;
    }

    pub fn set_category_filter(&mut self, category_filter: CategoryFilter) {
        self.category_filter = category_filter;
    }

    pub fn select_event(&mut self, event_id: EventId) {
        self.selected_event = Some(event_id);
    }

    pub fn clear_selection(&mut self) {
        self.selected_event = None;
    }

    pub fn go_to_today(&mut self, today: Date) {
        self.selected_date = today;
    }

    pub fn previous_week(&mut self) -> Result<(), CalendarError> {
        let mut date = self.selected_date;
        for _ in 0..7 {
            date = date.yesterday()?;
        }
        self.selected_date = date;
        self.selected_event = None;
        Ok(())
    }

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
