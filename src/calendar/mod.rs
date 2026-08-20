//! Calendar state and layout primitives shared by Cadence views.
//!
//! This module deliberately has no GPUI dependency. The week view can therefore
//! be tested with ordinary Rust tests while the application layer remains focused
//! on translating those primitives into pixels and interactions.

mod layout;
mod state;

pub use layout::{LayoutError, LayoutMetrics, PositionedEvent, layout_week};
pub use state::{CalendarState, CategoryFilter};
