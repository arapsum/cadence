mod calendar;
mod lifecycle;
mod manipulation;
mod persistence;
mod selection;
mod viewport;

pub(in crate::app) use persistence::{HistoryEffect, PersistenceState};
pub(in crate::app) use selection::EventSelection;
pub(in crate::app) use viewport::{RollbackViewState, ScrollInitialization, WEEK_VISIBLE_DAYS};

use std::{collections::HashMap, path::PathBuf};

use gpui::{Entity, FocusHandle, Subscription, Task};
use gpui_component::select::SelectState;
use jiff::Timestamp;

use crate::{
    calendar::CalendarState,
    domain::Settings,
    store::{AppearancePreferences, InMemoryRepository, StorageClient},
};

use super::{
    history::CalendarHistory, interaction::Manipulation, presentation::WorkspaceSnapshot,
    toolbar::FilterOption,
};

use persistence::PendingWrite;

#[allow(clippy::struct_excessive_bools)]
pub(super) struct CadenceView {
    pub(super) repository: InMemoryRepository,
    pub(super) storage: StorageClient,
    pub(super) storage_path: PathBuf,
    pub(super) persistence_state: PersistenceState,
    pending_write: Option<PendingWrite>,
    storage_task: Option<Task<()>>,
    pending_write_task: Option<Task<()>>,
    export_task: Option<Task<()>>,
    clock_task: Option<Task<()>>,
    pub(super) manipulation: Option<Manipulation>,
    manipulation_rollback: Option<RollbackViewState>,
    pub(super) history: CalendarHistory,
    pub(super) event_selection: EventSelection,
    pub(super) settings: Settings,
    pub(super) state: CalendarState,
    pub(super) day_plan_open: bool,
    pub(super) day_plan_focus: FocusHandle,
    pub(super) week_viewport_focus: FocusHandle,
    pub(super) day_plan_previous_focus: Option<FocusHandle>,
    pub(super) category_filter: Entity<SelectState<Vec<FilterOption>>>,
    pub(super) day_viewport: viewport::SurfaceViewportState,
    pub(super) week_viewport: viewport::SurfaceViewportState,
    pub(super) day_surface_width: f32,
    pub(super) week_surface_width: f32,
    /// First date represented by the logical seven-day week viewport.
    pub(super) week_visible_start: jiff::civil::Date,
    /// First date rendered in the rolling week buffer.
    pub(super) week_buffer_start: jiff::civil::Date,
    /// Prevents duplicate deferred scroll-window reconciliation callbacks.
    pub(super) week_scroll_sync_scheduled: bool,
    pub(super) snapshot: Option<WorkspaceSnapshot>,
    pub(super) now: Timestamp,
    pub(super) pending_scroll_minutes: Option<f32>,
    pub(super) error: Option<String>,
    pub(super) last_category: Option<crate::domain::CategoryId>,
    pub(super) notifications_enabled: bool,
    pub(super) reduce_motion: bool,
    pub(super) appearance: AppearancePreferences,
    pub(super) delivered_reminders: HashMap<String, ReminderTarget>,
    pub(super) reminder_check_at: Timestamp,
    pub(super) subscriptions: Vec<Subscription>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) struct ReminderTarget {
    pub(super) occurrence_id: crate::domain::OccurrenceId,
    pub(super) date: jiff::civil::Date,
}
