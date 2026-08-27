use gpui::{
    App, Context, Entity, IntoElement, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement as _, Subscription, Window, div, prelude::*, px,
};
use gpui_component::{
    ActiveTheme as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    input::{Input, InputEvent, InputState, Textarea, TextareaState},
    select::{Select, SelectItem, SelectState},
};
use jiff::civil::{Time, Weekday};

use crate::{
    domain::{
        Category, CategoryId, RecurrenceRule, ReminderOffset, Settings, WeekdaySet, format_time,
    },
    editor::{EditorMode, FormDraft, FormErrors, chrono_date, jiff_date, time_options},
};

use super::super::style::category_dot;

#[derive(Clone)]
pub(in crate::app::editor) struct CategoryOption {
    pub(in crate::app::editor) category: Category,
    id: CategoryId,
}

impl SelectItem for CategoryOption {
    type Value = CategoryId;

    fn title(&self) -> SharedString {
        self.category.name().into()
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }

    fn render(&self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(category_dot(Some(self.category.color_token()), cx.theme()))
            .child(self.category.name().to_owned())
    }
}

#[derive(Clone)]
pub(in crate::app::editor) struct TimeOption {
    pub(in crate::app::editor) time: Time,
    pub(in crate::app::editor) label: SharedString,
}

#[derive(Clone)]
pub(in crate::app::editor) struct RepeatOption {
    rule: Option<RecurrenceRule>,
    label: SharedString,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WeeklyDaysSource {
    Automatic,
    Customized,
}

impl SelectItem for RepeatOption {
    type Value = Option<RecurrenceRule>;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.rule
    }
}

#[derive(Clone)]
pub(in crate::app::editor) struct ReminderOption {
    reminder: Option<ReminderOffset>,
    label: SharedString,
}

impl SelectItem for ReminderOption {
    type Value = Option<ReminderOffset>;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.reminder
    }
}

impl SelectItem for TimeOption {
    type Value = Time;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.time
    }
}

pub(in crate::app::editor) struct EventEditor {
    mode: EditorMode,
    initial: FormDraft,
    title: Entity<InputState>,
    notes: Entity<TextareaState>,
    date: Entity<DatePickerState>,
    start_time: Entity<SelectState<Vec<TimeOption>>>,
    end_time: Entity<SelectState<Vec<TimeOption>>>,
    category: Entity<SelectState<Vec<CategoryOption>>>,
    repeat: Entity<SelectState<Vec<RepeatOption>>>,
    reminder: Entity<SelectState<Vec<ReminderOption>>>,
    ends_on: Entity<DatePickerState>,
    ends_enabled: bool,
    weekly_days: WeekdaySet,
    weekly_days_source: WeeklyDaysSource,
    all_time_options: Vec<TimeOption>,
    errors: FormErrors,
    focus_title: bool,
    reveal_end_date: bool,
    form_scroll: ScrollHandle,
    subscriptions: Vec<Subscription>,
}

fn time_options_for(draft: &FormDraft, settings: &Settings) -> Vec<TimeOption> {
    time_options(
        settings.snap_interval().minutes(),
        settings.day_start(),
        &[draft.start_time, draft.end_time],
    )
    .iter()
    .copied()
    .map(|time| TimeOption {
        time,
        label: format_time(time, settings.clock_format()).into(),
    })
    .collect()
}

fn category_options_for(
    categories: Vec<Category>,
    draft: &FormDraft,
) -> (Vec<CategoryOption>, Option<usize>) {
    let options = categories
        .into_iter()
        .map(|category| CategoryOption {
            id: category.id(),
            category,
        })
        .collect::<Vec<_>>();
    let index = draft
        .category_id
        .and_then(|id| options.iter().position(|option| option.category.id() == id));
    (options, index)
}

fn repeat_options_for(
    recurrence: Option<RecurrenceRule>,
    weekly_days: WeekdaySet,
) -> (Vec<RepeatOption>, Option<gpui_component::IndexPath>) {
    let options = vec![
        RepeatOption {
            rule: None,
            label: "Never".into(),
        },
        RepeatOption {
            rule: Some(RecurrenceRule::Daily),
            label: "Daily".into(),
        },
        RepeatOption {
            rule: Some(RecurrenceRule::Weekdays),
            label: "Weekdays".into(),
        },
        RepeatOption {
            rule: Some(RecurrenceRule::Weekly(weekly_days)),
            label: weekly_repeat_label(weekly_days),
        },
    ];
    let index = recurrence
        .map(|rule| match rule {
            RecurrenceRule::Daily => 1,
            RecurrenceRule::Weekdays => 2,
            RecurrenceRule::Weekly(_) => 3,
        })
        .map(gpui_component::IndexPath::new);
    (options, index)
}

fn initial_weekly_days(draft: &FormDraft) -> (WeekdaySet, WeeklyDaysSource) {
    match draft.recurrence {
        Some(RecurrenceRule::Weekly(days)) => (days, WeeklyDaysSource::Customized),
        _ => (
            WeekdaySet::one(draft.date.weekday()),
            WeeklyDaysSource::Automatic,
        ),
    }
}

fn weekly_days_after_date_change(
    source: WeeklyDaysSource,
    current: WeekdaySet,
    date: jiff::civil::Date,
) -> WeekdaySet {
    match source {
        WeeklyDaysSource::Automatic => WeekdaySet::one(date.weekday()),
        WeeklyDaysSource::Customized => current,
    }
}

fn weekly_repeat_label(days: WeekdaySet) -> SharedString {
    let selected = days.days();
    match selected.as_slice() {
        [] => "Weekly".into(),
        [day] => format!("Weekly on {}", weekday_name(*day)).into(),
        _ => format!(
            "Weekly on {}",
            selected
                .iter()
                .map(|day| weekday_short_name(*day))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .into(),
    }
}

const fn weekday_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Monday => "Monday",
        Weekday::Tuesday => "Tuesday",
        Weekday::Wednesday => "Wednesday",
        Weekday::Thursday => "Thursday",
        Weekday::Friday => "Friday",
        Weekday::Saturday => "Saturday",
        Weekday::Sunday => "Sunday",
    }
}

const fn weekday_short_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Monday => "Mon",
        Weekday::Tuesday => "Tue",
        Weekday::Wednesday => "Wed",
        Weekday::Thursday => "Thu",
        Weekday::Friday => "Fri",
        Weekday::Saturday => "Sat",
        Weekday::Sunday => "Sun",
    }
}

fn reminder_options_for(
    draft: &FormDraft,
) -> (Vec<ReminderOption>, Option<gpui_component::IndexPath>) {
    let options = [
        None,
        Some(0),
        Some(5),
        Some(10),
        Some(15),
        Some(30),
        Some(60),
    ]
    .into_iter()
    .map(|minutes| ReminderOption {
        reminder: minutes
            .map(|minutes| ReminderOffset::new(minutes).expect("fixed reminder offset is valid")),
        label: match minutes {
            None => "No reminder".into(),
            Some(0) => "At start".into(),
            Some(minutes) => format!("{minutes} minutes before").into(),
        },
    })
    .collect::<Vec<_>>();
    let index = options
        .iter()
        .position(|option| option.reminder == draft.reminder)
        .map(gpui_component::IndexPath::new);
    (options, index)
}

impl EventEditor {
    pub(in crate::app::editor) fn new(
        mode: EditorMode,
        draft: &FormDraft,
        categories: Vec<Category>,
        settings: &Settings,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let title = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("e.g. Morning routine")
                .default_value(draft.title.clone())
        });
        let notes = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("Optional notes")
                .default_value(draft.notes.clone())
        });
        let date = cx.new(|cx| {
            let mut state = DatePickerState::new(window, cx).date_format("%B %-d, %Y");
            state.set_date(chrono_date(draft.date), window, cx);
            state
        });
        let all_time_options = time_options_for(draft, settings);
        let start_options = all_time_options.clone();
        let end_options = end_time_options_after(&all_time_options, draft.start_time);
        let start_index = start_options
            .iter()
            .position(|option| option.time == draft.start_time)
            .map(gpui_component::IndexPath::new);
        let end_index = end_options
            .iter()
            .position(|option| option.time == draft.end_time)
            .map(gpui_component::IndexPath::new);
        let (category_options, category_index) = category_options_for(categories, draft);

        let start_time = cx.new(|cx| SelectState::new(start_options, start_index, window, cx));
        let end_time = cx.new(|cx| SelectState::new(end_options, end_index, window, cx));
        let category = cx.new(|cx| {
            SelectState::new(
                category_options,
                category_index.map(gpui_component::IndexPath::new),
                window,
                cx,
            )
        });

        let (weekly_days, weekly_days_source) = initial_weekly_days(draft);
        let (repeat_options, repeat_index) = repeat_options_for(draft.recurrence, weekly_days);
        let repeat = cx.new(|cx| SelectState::new(repeat_options, repeat_index, window, cx));
        let (reminder_options, reminder_index) = reminder_options_for(draft);
        let reminder = cx.new(|cx| SelectState::new(reminder_options, reminder_index, window, cx));
        let ends_enabled = draft.ends_on.is_some();
        let ends_on = cx.new(|cx| {
            let mut state = DatePickerState::new(window, cx).date_format("%B %-d, %Y");
            state.set_date(chrono_date(draft.ends_on.unwrap_or(draft.date)), window, cx);
            state
        });

        Self {
            mode,
            initial: draft.clone(),
            title,
            notes,
            date,
            start_time,
            end_time,
            category,
            repeat,
            reminder,
            ends_on,
            ends_enabled,
            weekly_days,
            weekly_days_source,
            all_time_options,
            errors: FormErrors::default(),
            focus_title: true,
            reveal_end_date: false,
            form_scroll: ScrollHandle::new(),
            subscriptions: Vec::new(),
        }
    }

    pub(in crate::app::editor) fn subscribe(
        &mut self,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        let title = self.title.clone();
        self.subscriptions
            .push(cx.subscribe(&title, |this, _, _: &InputEvent, cx| {
                this.errors.title = None;
                cx.notify();
            }));
        let notes = self.notes.clone();
        self.subscriptions
            .push(cx.subscribe(&notes, |_, _, _: &InputEvent, cx| {
                cx.notify();
            }));
        let date = self.date.clone();
        self.subscriptions.push(cx.subscribe_in(
            &date,
            window,
            |this, _, _: &DatePickerEvent, window, cx| {
                let date = this.selected_date(cx);
                this.weekly_days =
                    weekly_days_after_date_change(this.weekly_days_source, this.weekly_days, date);
                this.refresh_repeat_options(window, cx);
                this.errors.date = None;
                this.errors.conflict = None;
                cx.notify();
            },
        ));
        let start_time = self.start_time.clone();
        self.subscriptions.push(cx.subscribe_in(
            &start_time,
            window,
            |this, _, event: &gpui_component::select::SelectEvent<Vec<TimeOption>>, window, cx| {
                this.errors.start_time = None;
                this.errors.end_time = None;
                this.errors.conflict = None;
                if let gpui_component::select::SelectEvent::Confirm(Some(start_time)) = event {
                    this.update_end_time_options(*start_time, window, cx);
                }
                cx.notify();
            },
        ));
        let end_time = self.end_time.clone();
        self.subscriptions.push(cx.subscribe(
            &end_time,
            |this, _, _: &gpui_component::select::SelectEvent<Vec<TimeOption>>, cx| {
                this.errors.end_time = None;
                this.errors.conflict = None;
                cx.notify();
            },
        ));
        let category = self.category.clone();
        self.subscriptions.push(cx.subscribe(
            &category,
            |this, _, _: &gpui_component::select::SelectEvent<Vec<CategoryOption>>, cx| {
                this.errors.category = None;
                cx.notify();
            },
        ));
        let repeat = self.repeat.clone();
        self.subscriptions.push(cx.subscribe(
            &repeat,
            |this, _, _: &gpui_component::select::SelectEvent<Vec<RepeatOption>>, cx| {
                cx.notify();
                this.errors.recurrence = None;
                this.errors.conflict = None;
            },
        ));
        let reminder = self.reminder.clone();
        self.subscriptions.push(cx.subscribe(
            &reminder,
            |_, _, _: &gpui_component::select::SelectEvent<Vec<ReminderOption>>, cx| {
                cx.notify();
            },
        ));
        let ends_on = self.ends_on.clone();
        self.subscriptions
            .push(cx.subscribe(&ends_on, |this, _, _: &DatePickerEvent, cx| {
                this.errors.ends_on = None;
                this.errors.conflict = None;
                cx.notify();
            }));
    }

    pub(in crate::app::editor) fn form(&self, cx: &App) -> FormDraft {
        FormDraft {
            title: self.title.read(cx).value().to_string(),
            notes: self.notes.read(cx).value().to_string(),
            date: self
                .date
                .read(cx)
                .date()
                .start()
                .map_or(self.initial.date, jiff_date),
            start_time: self
                .start_time
                .read(cx)
                .selected_value()
                .copied()
                .unwrap_or(self.initial.start_time),
            end_time: self
                .end_time
                .read(cx)
                .selected_value()
                .copied()
                .unwrap_or(self.initial.end_time),
            category_id: self.category.read(cx).selected_value().copied(),
            recurrence: self
                .repeat
                .read(cx)
                .selected_value()
                .copied()
                .flatten()
                .map(|rule| match rule {
                    RecurrenceRule::Weekly(_) => RecurrenceRule::Weekly(self.weekly_days),
                    other => other,
                }),
            ends_on: self.ends_enabled.then(|| {
                self.ends_on
                    .read(cx)
                    .date()
                    .start()
                    .map_or(self.initial.date, jiff_date)
            }),
            reminder: self.reminder.read(cx).selected_value().copied().flatten(),
        }
    }

    pub(in crate::app::editor) fn is_dirty(&self, cx: &App) -> bool {
        self.form(cx) != self.initial
    }

    pub(in crate::app::editor) const fn mode(&self) -> EditorMode {
        self.mode
    }

    pub(in crate::app::editor) fn set_errors(&mut self, errors: FormErrors) {
        self.errors = errors;
    }

    fn update_end_time_options(
        &self,
        start_time: Time,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let current_end_time = self.end_time.read(cx).selected_value().copied();
        let options = end_time_options_after(&self.all_time_options, start_time);
        let selected_end_time = current_end_time
            .filter(|end_time| *end_time > start_time)
            .or_else(|| options.first().map(|option| option.time));

        self.end_time.update(cx, |end_time, cx| {
            end_time.set_items(options, window, cx);
            if let Some(selected_end_time) = selected_end_time {
                end_time.set_selected_value(&selected_end_time, window, cx);
            } else {
                end_time.set_selected_index(None, window, cx);
            }
        });
    }

    fn selected_date(&self, cx: &App) -> jiff::civil::Date {
        self.date
            .read(cx)
            .date()
            .start()
            .map_or(self.initial.date, jiff_date)
    }

    fn refresh_repeat_options(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let recurrence = self.repeat.read(cx).selected_value().copied().flatten();
        let (options, index) = repeat_options_for(recurrence, self.weekly_days);
        self.repeat.update(cx, |repeat, cx| {
            repeat.set_items(options, window, cx);
            repeat.set_selected_index(index, window, cx);
        });
    }

    fn toggle_end_date(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.ends_enabled = !self.ends_enabled;
        self.errors.conflict = None;
        if self.ends_enabled {
            self.reveal_end_date = true;
            self.ends_on.update(cx, |date, cx| {
                date.set_date(
                    chrono_date(self.initial.ends_on.unwrap_or(self.initial.date)),
                    window,
                    cx,
                );
            });
        }
        cx.notify();
    }

    fn toggle_weekday(&mut self, day: Weekday, window: &mut Window, cx: &mut Context<'_, Self>) {
        if let Ok(days) = self.weekly_days.toggled(day) {
            self.weekly_days = days;
            self.weekly_days_source = WeeklyDaysSource::Customized;
            self.refresh_repeat_options(window, cx);
            self.errors.conflict = None;
            cx.notify();
        }
    }
}

pub(in crate::app::editor) fn end_time_options_after(
    options: &[TimeOption],
    start_time: Time,
) -> Vec<TimeOption> {
    options
        .iter()
        .filter(|option| option.time > start_time)
        .cloned()
        .collect()
}

impl EventEditor {
    fn weekday_buttons(&self, cx: &Context<'_, Self>) -> Vec<gpui::AnyElement> {
        let owner = cx.entity().downgrade();
        [
            (Weekday::Monday, "M"),
            (Weekday::Tuesday, "T"),
            (Weekday::Wednesday, "W"),
            (Weekday::Thursday, "T"),
            (Weekday::Friday, "F"),
            (Weekday::Saturday, "S"),
            (Weekday::Sunday, "S"),
        ]
        .into_iter()
        .map(|(day, label)| {
            let owner = owner.clone();
            let active = self.weekly_days.contains(day);
            Button::new(format!("repeat-weekday-{day:?}"))
                .label(label)
                .when(active, Button::primary)
                .when(!active, Button::outline)
                .on_click(move |_, window, app| {
                    owner
                        .update(app, |editor, cx| editor.toggle_weekday(day, window, cx))
                        .ok();
                })
                .into_any_element()
        })
        .collect()
    }

    fn repeat_fields(
        &self,
        errors: FormErrors,
        weekday_buttons: Vec<gpui::AnyElement>,
        cx: &Context<'_, Self>,
    ) -> impl IntoElement {
        let selected_repeat = self.repeat.read(cx).selected_value().copied().flatten();
        let toggle_owner = cx.entity().downgrade();
        div().when(selected_repeat.is_some(), |this| {
            this.when(
                matches!(selected_repeat, Some(RecurrenceRule::Weekly(_))),
                |this| {
                    this.child(
                        div()
                            .v_flex()
                            .gap_1()
                            .child(div().text_sm().child("Weekly days"))
                            .child(div().flex().gap_1().children(weekday_buttons)),
                    )
                },
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_sm().child("End date"))
                    .child(
                        Button::new("toggle-repeat-end")
                            .debug_selector(|| "toggle-repeat-end".into())
                            .outline()
                            .label(if self.ends_enabled {
                                "Remove end date"
                            } else {
                                "Add end date"
                            })
                            .on_click(move |_, window, app| {
                                toggle_owner
                                    .update(app, |editor, cx| {
                                        editor.toggle_end_date(window, cx);
                                    })
                                    .ok();
                            }),
                    ),
            )
            .when(self.ends_enabled, |this| {
                this.child(
                    div()
                        .debug_selector(|| "repeat-end-date".into())
                        .child(field(
                            "Ends on",
                            DatePicker::new(&self.ends_on).placeholder("Choose an end date"),
                            errors.ends_on,
                        )),
                )
            })
        })
    }
}

impl Render for EventEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        if self.focus_title {
            self.focus_title = false;
            let title = self.title.clone();
            window.defer(cx, move |window, cx| {
                title.update(cx, |title, cx| title.focus(window, cx));
            });
        }
        if self.reveal_end_date {
            self.reveal_end_date = false;
            let form_scroll = self.form_scroll.clone();
            window.defer(cx, move |_, cx| {
                form_scroll.scroll_to_bottom();
                cx.refresh_windows();
            });
        }

        let errors = self.errors.clone();
        let weekday_buttons = self.weekday_buttons(cx);
        let repeat_fields = self.repeat_fields(errors.clone(), weekday_buttons, cx);
        div()
            .id("event-editor-form")
            .debug_selector(|| "event-editor-form".into())
            .v_flex()
            .gap_4()
            .max_h(px(560.0))
            .track_scroll(&self.form_scroll)
            .overflow_y_scroll()
            .child(field("Title", Input::new(&self.title), errors.title))
            .child(field("Notes", Textarea::new(&self.notes).h(px(84.0)), None))
            .child(field(
                "Date",
                DatePicker::new(&self.date).placeholder("Choose a date"),
                errors.date,
            ))
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(field(
                        "Start time",
                        Select::new(&self.start_time)
                            .w(px(148.0))
                            .placeholder("Start time"),
                        errors.start_time,
                    ))
                    .child(field(
                        "End time",
                        Select::new(&self.end_time)
                            .w(px(148.0))
                            .placeholder("End time"),
                        errors.end_time,
                    )),
            )
            .when_some(errors.conflict, |this, error| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(gpui::rgb(0x00D9_304F))
                        .child(error),
                )
            })
            .child(field(
                "Category",
                Select::new(&self.category)
                    .w_full()
                    .placeholder("Choose a category"),
                errors.category,
            ))
            .child(field(
                "Reminder",
                Select::new(&self.reminder)
                    .w_full()
                    .placeholder("Choose a reminder"),
                None,
            ))
            .child(field(
                "Repeats",
                Select::new(&self.repeat)
                    .w_full()
                    .placeholder("Does this repeat?"),
                errors.recurrence,
            ))
            .child(repeat_fields)
    }
}

fn field<E: IntoElement>(label: &'static str, input: E, error: Option<String>) -> impl IntoElement {
    div()
        .v_flex()
        .gap_1()
        .flex_1()
        .child(div().text_sm().child(label))
        .child(input)
        .when_some(error, |this, error| {
            this.child(
                div()
                    .text_xs()
                    .text_color(gpui::rgb(0x00D9_304F))
                    .child(error),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::Date;

    fn draft(date: Date, recurrence: Option<RecurrenceRule>) -> FormDraft {
        FormDraft {
            title: String::new(),
            notes: String::new(),
            date,
            start_time: Time::constant(10, 0, 0, 0),
            end_time: Time::constant(11, 45, 0, 0),
            category_id: None,
            recurrence,
            ends_on: None,
            reminder: None,
        }
    }

    #[test]
    fn weekly_recurrence_defaults_to_the_event_date() {
        let date = Date::constant(2026, 8, 17);
        let (days, source) = initial_weekly_days(&draft(date, None));

        assert_eq!(days, WeekdaySet::one(Weekday::Monday));
        assert_eq!(source, WeeklyDaysSource::Automatic);
    }

    #[test]
    fn existing_weekly_recurrence_keeps_its_selected_days() {
        let date = Date::constant(2026, 8, 17);
        let selected = WeekdaySet::one(Weekday::Wednesday);
        let (days, source) =
            initial_weekly_days(&draft(date, Some(RecurrenceRule::Weekly(selected))));

        assert_eq!(days, selected);
        assert_eq!(source, WeeklyDaysSource::Customized);
    }

    #[test]
    fn automatic_weekly_day_follows_date_but_custom_days_do_not() {
        let monday = Date::constant(2026, 8, 17);
        let tuesday = Date::constant(2026, 8, 18);
        let custom = WeekdaySet::one(Weekday::Monday);

        assert_eq!(
            weekly_days_after_date_change(WeeklyDaysSource::Automatic, custom, tuesday),
            WeekdaySet::one(Weekday::Tuesday)
        );
        assert_eq!(
            weekly_days_after_date_change(WeeklyDaysSource::Customized, custom, tuesday),
            custom
        );
        assert_eq!(
            weekly_days_after_date_change(WeeklyDaysSource::Automatic, custom, monday),
            WeekdaySet::one(Weekday::Monday)
        );
    }

    #[test]
    fn weekly_option_label_identifies_single_and_multiple_days() {
        assert_eq!(
            weekly_repeat_label(WeekdaySet::one(Weekday::Monday)).to_string(),
            "Weekly on Monday"
        );
        let selected = WeekdaySet::one(Weekday::Monday)
            .toggled(Weekday::Wednesday)
            .expect("adding Wednesday leaves Monday selected");
        assert_eq!(
            weekly_repeat_label(selected).to_string(),
            "Weekly on Mon, Wed"
        );
    }

    #[test]
    fn refreshing_weekly_options_updates_the_rule_value_with_the_label() {
        let selected = WeekdaySet::one(Weekday::Monday);
        let updated = WeekdaySet::one(Weekday::Wednesday);
        let (options, index) = repeat_options_for(Some(RecurrenceRule::Weekly(selected)), updated);
        let option = options.get(3).expect("weekly option is present");

        assert_eq!(index, Some(gpui_component::IndexPath::new(3)));
        assert_eq!(option.rule, Some(RecurrenceRule::Weekly(updated)));
        assert_eq!(option.label.to_string(), "Weekly on Wednesday");
    }
}
