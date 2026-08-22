use gpui::{Context, Window};
use gpui_component::{WindowExt as _, notification::Notification};
use jiff::{Timestamp, civil::Date};

use crate::{
    domain::{
        EventDraft, OccurrenceId, RecurrenceException, RecurrenceExceptionKind, RecurrenceSeries,
        RecurrenceSeriesId,
    },
    editor::FormDraft,
    store::TimetableRepository,
};

use super::super::{
    history::{CalendarChange, ChangeKind},
    state::CadenceView,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(in crate::app) enum RecurrenceScope {
    This,
    Following,
}

pub(in crate::app::editor) fn draft_to_domain(draft: &FormDraft) -> EventDraft {
    EventDraft::new(
        draft.title.clone(),
        draft.date,
        draft.start_time,
        draft.end_time,
        draft
            .category_id
            .expect("form validation supplies a category before commit"),
        Some(draft.notes.clone()),
    )
    .with_reminder(draft.reminder)
}

impl CadenceView {
    pub(in crate::app) fn apply_recurring_edit(
        &mut self,
        series_id: RecurrenceSeriesId,
        original_date: Date,
        draft: &FormDraft,
        scope: RecurrenceScope,
        timestamp: Timestamp,
    ) -> Result<OccurrenceId, String> {
        let repository_rollback = self.repository.clone();
        let result =
            self.apply_recurring_edit_inner(series_id, original_date, draft, scope, timestamp);
        if result.is_err() {
            self.repository = repository_rollback;
        }
        result
    }

    fn apply_recurring_edit_inner(
        &mut self,
        series_id: RecurrenceSeriesId,
        original_date: Date,
        draft: &FormDraft,
        scope: RecurrenceScope,
        timestamp: Timestamp,
    ) -> Result<OccurrenceId, String> {
        let series = self
            .repository
            .series(series_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "That recurring series is no longer available.".to_owned())?;
        match scope {
            RecurrenceScope::This => self.apply_single_occurrence_edit(
                series_id,
                original_date,
                draft_to_domain(draft),
                timestamp,
            ),
            RecurrenceScope::Following => {
                self.apply_following_edit(series_id, original_date, draft, series, timestamp)
            }
        }
    }

    fn apply_single_occurrence_edit(
        &mut self,
        series_id: RecurrenceSeriesId,
        original_date: Date,
        draft: EventDraft,
        timestamp: Timestamp,
    ) -> Result<OccurrenceId, String> {
        let exception = RecurrenceException::modified(series_id, original_date, draft, timestamp)
            .map_err(|error| error.to_string())?;
        self.repository
            .upsert_exception(exception)
            .map_err(|error| error.to_string())?;
        Ok(OccurrenceId::Recurring {
            series_id,
            original_date,
        })
    }

    fn apply_following_edit(
        &mut self,
        series_id: RecurrenceSeriesId,
        original_date: Date,
        draft: &FormDraft,
        mut series: RecurrenceSeries,
        timestamp: Timestamp,
    ) -> Result<OccurrenceId, String> {
        let rule = draft.recurrence.unwrap_or_else(|| series.rule());
        let ends_on = draft.ends_on;
        let exceptions = self
            .repository
            .recurrence_exceptions()
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter(|exception| {
                exception.series_id() == series_id && exception.original_date() >= original_date
            })
            .collect::<Vec<_>>();

        if original_date == series.template().date {
            series
                .revise(draft_to_domain(draft), rule, ends_on, timestamp)
                .map_err(|error| error.to_string())?;
            let selected_date = series.template().date;
            self.repository
                .update_series(series.clone())
                .map_err(|error| error.to_string())?;
            self.rehome_exceptions(series_id, exceptions, &series, timestamp)?;
            return Ok(OccurrenceId::Recurring {
                series_id,
                original_date: selected_date,
            });
        }

        let predecessor_end = original_date
            .yesterday()
            .map_err(|error| error.to_string())?;
        let predecessor_end = series
            .ends_on()
            .map_or(predecessor_end, |end| end.min(predecessor_end));
        let predecessor_template = series.template();
        series
            .revise(
                predecessor_template,
                series.rule(),
                Some(predecessor_end),
                timestamp,
            )
            .map_err(|error| error.to_string())?;
        self.repository
            .update_series(series)
            .map_err(|error| error.to_string())?;

        let successor = RecurrenceSeries::new(
            RecurrenceSeriesId::new(),
            draft_to_domain(draft),
            rule,
            ends_on,
            timestamp,
        )
        .map_err(|error| error.to_string())?;
        let successor_id = successor.id();
        self.repository
            .create_series(successor.clone())
            .map_err(|error| error.to_string())?;
        self.rehome_exceptions(series_id, exceptions, &successor, timestamp)?;
        Ok(OccurrenceId::Recurring {
            series_id: successor_id,
            original_date: draft.date,
        })
    }

    fn rehome_exceptions(
        &mut self,
        series_id: RecurrenceSeriesId,
        exceptions: Vec<RecurrenceException>,
        series: &RecurrenceSeries,
        timestamp: Timestamp,
    ) -> Result<(), String> {
        let successor_id = series.id();
        for exception in exceptions {
            self.repository
                .delete_exception(series_id, exception.original_date())
                .map_err(|error| error.to_string())?;
            if !series.contains_date(exception.original_date()) {
                continue;
            }
            let replacement = match exception.kind() {
                RecurrenceExceptionKind::Cancelled => {
                    RecurrenceException::cancelled(successor_id, exception.original_date())
                }
                RecurrenceExceptionKind::Modified(replacement) => RecurrenceException::modified(
                    successor_id,
                    exception.original_date(),
                    replacement.clone(),
                    timestamp,
                )
                .map_err(|error| error.to_string())?,
            };
            self.repository
                .upsert_exception(replacement)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub(in crate::app::editor) fn delete_recurring(
        &mut self,
        series_id: RecurrenceSeriesId,
        original_date: Date,
        scope: RecurrenceScope,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let rollback = self.rollback_view_state();
        let before = match self.repository.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.show_error(error.to_string(), window, cx);
                return;
            }
        };
        let repository_rollback = self.repository.clone();
        let result = (|| -> Result<(), String> {
            match scope {
                RecurrenceScope::This => self
                    .repository
                    .upsert_exception(RecurrenceException::cancelled(series_id, original_date))
                    .map_err(|error| error.to_string()),
                RecurrenceScope::Following => {
                    let Some(mut series) = self
                        .repository
                        .series(series_id)
                        .map_err(|error| error.to_string())?
                    else {
                        return Err("That recurring series is no longer available.".to_owned());
                    };
                    if original_date == series.template().date {
                        self.repository
                            .delete_series(series_id)
                            .map_err(|error| error.to_string())?;
                    } else {
                        let end = original_date
                            .yesterday()
                            .map_err(|error| error.to_string())?;
                        let template = series.template();
                        let ends_on = series.ends_on().map_or(end, |existing| existing.min(end));
                        series
                            .revise(template, series.rule(), Some(ends_on), Timestamp::now())
                            .map_err(|error| error.to_string())?;
                        self.repository
                            .update_series(series)
                            .map_err(|error| error.to_string())?;
                        let exceptions = self
                            .repository
                            .recurrence_exceptions()
                            .map_err(|error| error.to_string())?;
                        for exception in exceptions.into_iter().filter(|exception| {
                            exception.series_id() == series_id
                                && exception.original_date() >= original_date
                        }) {
                            self.repository
                                .delete_exception(series_id, exception.original_date())
                                .map_err(|error| error.to_string())?;
                        }
                    }
                    Ok(())
                }
            }
        })();
        if let Err(error) = result {
            self.repository = repository_rollback;
            self.show_error(error, window, cx);
            return;
        }
        let after = match self.repository.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.show_error(error.to_string(), window, cx);
                return;
            }
        };
        self.state.clear_selection();
        self.refresh_snapshot();
        self.persist_snapshot(
            before.clone(),
            rollback,
            super::super::state::HistoryEffect::Record(CalendarChange::Snapshot {
                before: Box::new(before),
                after: Box::new(after),
                kind: ChangeKind::Delete,
            }),
            cx,
        );
        window.push_notification(Notification::success("Recurring event deleted"), cx);
        cx.notify();
    }
}
