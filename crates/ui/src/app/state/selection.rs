use std::collections::BTreeSet;

use gpui::Context;

use crate::{calendar::CalendarViewMode, domain::OccurrenceId};

use super::CadenceView;

/// The transient event-selection mode owned by the calendar surface.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub(in crate::app) enum EventSelection {
    /// Normal single-event selection is active.
    #[default]
    Single,
    /// Multiple occurrences can be selected on one captured surface.
    Bulk {
        /// Surface whose visible occurrences may be selected.
        surface: CalendarViewMode,
        /// Stable occurrence identities currently selected.
        selected: BTreeSet<OccurrenceId>,
    },
}

impl EventSelection {
    pub(in crate::app) const fn surface(&self) -> Option<CalendarViewMode> {
        match self {
            Self::Single => None,
            Self::Bulk { surface, .. } => Some(*surface),
        }
    }

    pub(in crate::app) const fn selected(&self) -> Option<&BTreeSet<OccurrenceId>> {
        match self {
            Self::Single => None,
            Self::Bulk { selected, .. } => Some(selected),
        }
    }

    pub(in crate::app) const fn selected_mut(&mut self) -> Option<&mut BTreeSet<OccurrenceId>> {
        match self {
            Self::Single => None,
            Self::Bulk { selected, .. } => Some(selected),
        }
    }
}

impl CadenceView {
    /// Reports whether the calendar is in multiple-event selection mode.
    pub(in crate::app) const fn is_bulk_selecting(&self) -> bool {
        matches!(self.event_selection, EventSelection::Bulk { .. })
    }

    /// Returns the surface captured when multiple-event selection began.
    pub(in crate::app) const fn bulk_selection_surface(&self) -> Option<CalendarViewMode> {
        self.event_selection.surface()
    }

    /// Returns the number of currently selected occurrences.
    pub(in crate::app) fn bulk_selection_count(&self) -> usize {
        self.event_selection.selected().map_or(0, BTreeSet::len)
    }

    /// Returns the number of visible occurrences eligible for bulk selection.
    pub(in crate::app) fn bulk_selectable_count(&self) -> usize {
        self.bulk_selection_surface()
            .and_then(|surface| self.surface_snapshot(surface))
            .map_or(0, |snapshot| snapshot.events.len())
    }

    /// Returns the number of visible occurrences on the active surface.
    pub(in crate::app) fn bulk_selectable_count_for_active_surface(&self) -> usize {
        self.bulk_selectable_count_for(self.state.view_mode())
    }

    /// Reports whether every visible occurrence on the captured surface is selected.
    pub(in crate::app) fn bulk_all_selected(&self) -> bool {
        let Some(surface) = self.bulk_selection_surface() else {
            return false;
        };
        let Some(snapshot) = self.surface_snapshot(surface) else {
            return false;
        };
        !snapshot.events.is_empty()
            && snapshot
                .events
                .iter()
                .all(|event| self.is_bulk_selected(event.id()))
    }

    /// Returns whether `occurrence_id` is selected for bulk deletion.
    pub(in crate::app) fn is_bulk_selected(&self, occurrence_id: OccurrenceId) -> bool {
        self.event_selection
            .selected()
            .is_some_and(|selected| selected.contains(&occurrence_id))
    }

    /// Returns a stable copy of the selected occurrence identities.
    pub(in crate::app) fn selected_occurrences(&self) -> Vec<OccurrenceId> {
        self.event_selection
            .selected()
            .map_or_else(Vec::new, |selected| selected.iter().copied().collect())
    }

    /// Starts selection mode for the active surface when it has visible events.
    pub(in crate::app) fn begin_event_selection(&mut self, cx: &mut Context<'_, Self>) {
        if !self.is_interactive()
            || self.is_bulk_selecting()
            || self.bulk_selectable_count_for(self.state.view_mode()) == 0
        {
            return;
        }
        self.state.clear_selection();
        self.event_selection = EventSelection::Bulk {
            surface: self.state.view_mode(),
            selected: BTreeSet::new(),
        };
        cx.notify();
    }

    /// Leaves selection mode and clears all transient event selection.
    pub(in crate::app) fn cancel_event_selection(&mut self, cx: &mut Context<'_, Self>) {
        if !self.is_bulk_selecting() {
            return;
        }
        self.event_selection = EventSelection::Single;
        self.state.clear_selection();
        cx.notify();
    }

    /// Toggles one occurrence when it belongs to the captured active surface.
    pub(in crate::app) fn toggle_event_selection(
        &mut self,
        surface: CalendarViewMode,
        occurrence_id: OccurrenceId,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.is_interactive()
            || self.bulk_selection_surface() != Some(surface)
            || self.state.view_mode() != surface
            || !self.surface_snapshot(surface).is_some_and(|snapshot| {
                snapshot
                    .events
                    .iter()
                    .any(|event| event.id() == occurrence_id)
            })
        {
            return;
        }
        if let Some(selected) = self.event_selection.selected_mut()
            && !selected.insert(occurrence_id)
        {
            selected.remove(&occurrence_id);
        }
        cx.notify();
    }

    /// Enters bulk selection when needed and toggles an occurrence from a
    /// modifier-assisted pointer click.
    pub(in crate::app) fn toggle_event_selection_from_shortcut(
        &mut self,
        surface: CalendarViewMode,
        occurrence_id: OccurrenceId,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.is_interactive() {
            return;
        }
        if !self.is_bulk_selecting() {
            self.activate_surface(surface, cx);
            self.begin_event_selection(cx);
        }
        self.toggle_event_selection(surface, occurrence_id, cx);
    }

    /// Selects every visible occurrence, or clears the set when all are selected.
    pub(in crate::app) fn select_all_visible_events(&mut self, cx: &mut Context<'_, Self>) {
        let Some(surface) = self.bulk_selection_surface() else {
            return;
        };
        let Some(snapshot) = self.surface_snapshot(surface) else {
            return;
        };
        let ids = snapshot
            .events
            .iter()
            .map(cadence_core::domain::EventOccurrence::id)
            .collect::<BTreeSet<_>>();
        if let Some(selected) = self.event_selection.selected_mut() {
            if !ids.is_empty() && selected == &ids {
                selected.clear();
            } else {
                *selected = ids;
            }
        }
        cx.notify();
    }

    fn bulk_selectable_count_for(&self, surface: CalendarViewMode) -> usize {
        self.surface_snapshot(surface)
            .map_or(0, |snapshot| snapshot.events.len())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::EventSelection;
    use crate::{calendar::CalendarViewMode, domain::OccurrenceId};

    #[test]
    fn selection_mode_records_surface_and_selected_occurrence() {
        let selection = EventSelection::Bulk {
            surface: CalendarViewMode::Week,
            selected: BTreeSet::from([OccurrenceId::Standalone(uuid::Uuid::from_u128(1).into())]),
        };

        assert_eq!(selection.surface(), Some(CalendarViewMode::Week));
        assert_eq!(selection.selected().map(BTreeSet::len), Some(1));
    }
}
