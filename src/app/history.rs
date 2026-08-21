use std::collections::VecDeque;

use crate::domain::{Event, EventDraft, EventId};
use crate::store::PersistenceSnapshot;

const HISTORY_LIMIT: usize = 100;

/// User-facing operation represented by an event-history entry.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) enum ChangeKind {
    /// A new event was created.
    Create,
    /// An event's editable fields were changed.
    Edit,
    /// An event was moved to another date or time.
    Move,
    /// An event's duration was changed.
    Resize,
    /// An event was deleted.
    Delete,
}

impl ChangeKind {
    /// Returns the concise label used by history notifications.
    ///
    /// # Returns
    ///
    /// A lowercase operation label suitable for a status message.
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Create => "create event",
            Self::Edit => "edit event",
            Self::Move => "move event",
            Self::Resize => "resize event",
            Self::Delete => "delete event",
        }
    }
}

/// Reversible mutation captured by the session history.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(super) enum EventChange {
    /// An event was inserted into the repository.
    Create { event: Event },
    /// An event was changed from one draft to another.
    Update {
        id: EventId,
        before: EventDraft,
        after: EventDraft,
        kind: ChangeKind,
    },
    /// An event was removed from the repository.
    Delete { event: Event },
    /// A recurring-series or cross-entity mutation captured as a state patch.
    Snapshot {
        before: PersistenceSnapshot,
        after: PersistenceSnapshot,
        kind: ChangeKind,
    },
}

impl EventChange {
    /// Returns the operation represented by this change.
    ///
    /// # Returns
    ///
    /// The operation kind used by notifications and history controls.
    pub(super) const fn kind(&self) -> ChangeKind {
        match self {
            Self::Create { .. } => ChangeKind::Create,
            Self::Update { kind, .. } | Self::Snapshot { kind, .. } => *kind,
            Self::Delete { .. } => ChangeKind::Delete,
        }
    }
}

/// Bounded, session-only undo and redo stacks for event mutations.
#[derive(Debug)]
pub(super) struct EventHistory {
    undo: VecDeque<EventChange>,
    redo: VecDeque<EventChange>,
}

impl Default for EventHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHistory {
    /// Creates an empty history with bounded undo and redo capacity.
    ///
    /// # Returns
    ///
    /// A history with no undoable or redoable changes.
    pub(super) fn new() -> Self {
        Self {
            undo: VecDeque::with_capacity(HISTORY_LIMIT),
            redo: VecDeque::with_capacity(HISTORY_LIMIT),
        }
    }

    /// Reports whether an undo operation is available.
    ///
    /// # Returns
    ///
    /// `true` when the undo stack contains a change.
    pub(super) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Reports whether a redo operation is available.
    ///
    /// # Returns
    ///
    /// `true` when the redo stack contains a change.
    pub(super) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Peeks at the most recent undoable change without moving it.
    ///
    /// # Returns
    ///
    /// The top undo entry, or `None` when the stack is empty.
    pub(super) fn peek_undo(&self) -> Option<&EventChange> {
        self.undo.back()
    }

    /// Peeks at the most recent redoable change without moving it.
    ///
    /// # Returns
    ///
    /// The top redo entry, or `None` when the stack is empty.
    pub(super) fn peek_redo(&self) -> Option<&EventChange> {
        self.redo.back()
    }

    /// Records a committed change and clears any redo entries.
    ///
    /// # Parameters
    ///
    /// - `change`: Mutation that was successfully persisted.
    pub(super) fn record(&mut self, change: EventChange) {
        if self.undo.len() == HISTORY_LIMIT {
            self.undo.pop_front();
        }
        self.undo.push_back(change);
        self.redo.clear();
    }

    /// Moves a matching top entry between two history stacks.
    ///
    /// # Parameters
    ///
    /// - `from`: Stack whose top entry should be moved.
    /// - `to`: Stack that receives the moved entry.
    /// - `expected`: Entry that must currently be on top of `from`.
    ///
    /// # Returns
    ///
    /// `true` when `expected` was moved; `false` when the source stack's top
    /// entry did not match.
    ///
    /// # Panics
    ///
    /// Panics when:
    ///
    /// - The source stack changes between the top-entry check and removal.
    fn move_top(
        from: &mut VecDeque<EventChange>,
        to: &mut VecDeque<EventChange>,
        expected: &EventChange,
    ) -> bool {
        if from.back() != Some(expected) {
            return false;
        }
        let change = from.pop_back().expect("entry was checked");
        if to.len() == HISTORY_LIMIT {
            to.pop_front();
        }
        to.push_back(change);
        true
    }

    /// Completes an undo after its repository operation succeeds.
    ///
    /// # Parameters
    ///
    /// - `change`: Change that was applied in reverse.
    ///
    /// # Returns
    ///
    /// `true` when the matching undo entry moved to the redo stack.
    pub(super) fn finish_undo(&mut self, change: &EventChange) -> bool {
        Self::move_top(&mut self.undo, &mut self.redo, change)
    }

    /// Completes a redo after its repository operation succeeds.
    ///
    /// # Parameters
    ///
    /// - `change`: Change that was applied again.
    ///
    /// # Returns
    ///
    /// `true` when the matching redo entry moved to the undo stack.
    pub(super) fn finish_redo(&mut self, change: &EventChange) -> bool {
        Self::move_top(&mut self.redo, &mut self.undo, change)
    }

    #[cfg(test)]
    fn lengths(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }
}

#[cfg(test)]
mod tests {
    use jiff::{Timestamp, civil::Date};
    use uuid::Uuid;

    use super::{ChangeKind, EventChange, EventHistory};
    use crate::domain::{CategoryId, Event, EventDraft};

    fn event(id: u128) -> Event {
        Event::new(
            Uuid::from_u128(id).into(),
            EventDraft::new(
                "Event",
                Date::constant(2026, 8, 21),
                jiff::civil::Time::constant(9, 0, 0, 0),
                jiff::civil::Time::constant(10, 0, 0, 0),
                CategoryId::from_uuid(Uuid::from_u128(1)),
                None,
            ),
            Timestamp::from_second(0).expect("valid timestamp"),
        )
        .expect("valid event")
    }

    #[test]
    fn recording_a_new_change_clears_redo() {
        let mut history = EventHistory::new();
        let first = EventChange::Create { event: event(1) };
        let second = EventChange::Create { event: event(2) };
        history.record(first.clone());
        assert!(history.finish_undo(&first));
        assert!(history.can_redo());
        history.record(second);
        assert_eq!(history.lengths(), (1, 0));
    }

    #[test]
    fn undo_and_redo_move_one_entry_between_stacks() {
        let mut history = EventHistory::new();
        let change = EventChange::Update {
            id: event(1).id(),
            before: event(1).draft(),
            after: event(1).draft(),
            kind: ChangeKind::Move,
        };
        history.record(change.clone());
        assert!(history.finish_undo(&change));
        assert_eq!(history.lengths(), (0, 1));
        assert!(history.finish_redo(&change));
        assert_eq!(history.lengths(), (1, 0));
    }
}
