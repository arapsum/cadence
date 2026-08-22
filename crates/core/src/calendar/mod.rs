//! Calendar state and layout primitives shared by Cadence views.
//!
//! This module deliberately has no GPUI dependency. Day and week surfaces can
//! therefore be tested with ordinary Rust tests while the application layer
//! remains focused on translating those primitives into pixels and interactions.

mod interaction;
mod layout;
mod state;

pub use interaction::{ResizeEdge, propose_move, propose_resize};
pub use layout::{LayoutError, LayoutMetrics, PositionedEvent, layout_events};
pub use state::{CalendarState, CalendarViewMode, CategoryFilter};
