mod calendar;
mod lifecycle;
mod manipulation;
mod persistence;
mod viewport;

pub(in crate::app) use persistence::{HistoryEffect, PersistenceState};
pub(in crate::app) use viewport::RollbackViewState;

use std::{collections::HashSet, path::PathBuf};

use gpui::{Entity, ScrollHandle, Subscription};
use gpui_component::select::SelectState;
use jiff::Timestamp;

use crate::{
    calendar::CalendarState,
    domain::Settings,
    store::{InMemoryRepository, StorageClient},
};

use super::{
    history::EventHistory, interaction::Manipulation, presentation::CalendarSnapshot,
    toolbar::FilterOption,
};

use persistence::PendingWrite;

pub(super) struct CadenceView {
    pub(super) repository: InMemoryRepository,
    pub(super) storage: StorageClient,
    pub(super) storage_path: PathBuf,
    pub(super) persistence_state: PersistenceState,
    pending_write: Option<PendingWrite>,
    pub(super) manipulation: Option<Manipulation>,
    manipulation_rollback: Option<RollbackViewState>,
    pub(super) history: EventHistory,
    pub(super) settings: Settings,
    pub(super) state: CalendarState,
    pub(super) category_filter: Entity<SelectState<Vec<FilterOption>>>,
    pub(super) scroll_handle: ScrollHandle,
    pub(super) snapshot: Option<CalendarSnapshot>,
    pub(super) now: Timestamp,
    pub(super) scroll_initialized: bool,
    pub(super) pending_scroll_minutes: Option<f32>,
    pub(super) error: Option<String>,
    pub(super) last_category: Option<crate::domain::CategoryId>,
    pub(super) notifications_enabled: bool,
    pub(super) reduce_motion: bool,
    pub(super) delivered_reminders: HashSet<String>,
    pub(super) subscriptions: Vec<Subscription>,
}
