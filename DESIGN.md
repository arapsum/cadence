# Cadence design record

## M2 week surface

The main canvas follows the supplied timetable reference: a quiet toolbar above
a rounded calendar frame, a fixed Time gutter, seven day columns, and a
full-midnight-to-midnight scroll plane. Sunday remains the default week start
and the default display format is 12-hour time.

The header and gutter are overlays on the same body ScrollHandle. Their
translated tracks use the body offset, so the week stays aligned while either
axis scrolls. A 132 px minimum column keeps event cards readable; narrower
windows scroll horizontally instead of compressing the week into unusable
slivers.

Events use semantic category tokens with light and dark palettes. Tall cards
show category, title, and time; shorter cards reduce content density. Hover and
keyboard focus use the same tooltip with the complete title/category/time/notes
payload. A click selects an event and strengthens its border.

## State and geometry

The GPUI layer owns transient interaction state, while calendar::state and
calendar::layout remain UI-independent. The layout engine partitions events by
day, uses end-exclusive intervals, assigns overlap lanes, and reserves a
minimum occupancy for very short events. The renderer maps the resulting
positions to the 1.5 px/minute 24-hour plane.

The current-day column receives a subtle tint and a green dot in its header. A
green line and dot track the local wall-clock time and refresh every 30 seconds.

## M3 shared Day/Week surface

Day and Week are two adapters around one calendar surface renderer. The domain
state owns the selected date and active mode; it derives a one-day range in Day
mode and the configured seven-day range in Week mode. The layout engine accepts
either range without changing event top offsets, heights, or overlap lanes.

The Day surface uses one full-width column, so tall cards can show an optional
two-line notes block. Week keeps seven columns at a 132 px minimum and exposes
horizontal scrolling when the window is narrower than the full plane. Both
surfaces use the same tracked vertical scroll handle and preserve its offset in
minutes when switching modes. Week reveals the selected day after a switch;
Day starts at horizontal offset zero.

The toolbar uses a segmented Day/Week `TabBar`. The calendar root registers
`ShowDay`, `ShowWeek`, `PreviousPeriod`, `NextPeriod`, and `GoToToday` actions in
the `CadenceCalendar` key context. Cmd/Ctrl+1 and Cmd/Ctrl+2 switch modes,
Alt+Left/Right navigate, and Cmd/Ctrl+T returns to today.

## Validation

Automated validation currently passes:

- cargo fmt --all -- --check
- cargo clippy --locked --all-targets --all-features -- -D warnings -W
  clippy::pedantic -W clippy::nursery -W rust-2018-idioms
- cargo test --locked --all-targets

The remaining release check is a Wayland visual pass at the baseline and
minimum window sizes, including horizontal scrolling and the seeded overlap.

## M4 event editor

The event workflow is intentionally split into an inspector and an editor
dialog. Selecting an event (with a pointer or Enter after keyboard focus) opens
the inspector first, keeping accidental edits out of the primary timetable
surface. The inspector exposes Delete, Duplicate, and Edit. Duplicate copies
the current editable values into a new, unsaved create draft; it does not write
until Save is confirmed.

The shared editor form contains title, notes, date, start time, end time, and
category. `src/editor.rs` keeps its draft and validation independent of GPUI;
`src/app/editor.rs` adapts those values to GPUI Component's Input, Textarea,
DatePicker, and Select entities. Existing off-grid times are retained as
additional select options so opening an event never silently changes its time.

Creation defaults are deterministic: an empty slot uses that day and hour with
a one-hour duration; New event uses the selected date and the next snapped local
time when the date is today, otherwise the configured day start. The start is
clamped so the default duration remains inside the configured display day.

The form validates before repository mutation and places messages beside the
invalid field. A dirty Cancel, Escape, or close request opens a discard
confirmation. Delete first requires confirmation, then offers a persistent
session-scoped Undo notification and Cmd/Ctrl+Z. Dialog focus starts in the
title field and GPUI Component restores the invoking card or slot when the
dialog stack closes.

## M5 persistence and recovery surfaces

Persistence is deliberately quiet during normal planning. The toolbar keeps
calendar navigation primary and adds a secondary Export action. While a write
is in flight, the title shows `Saving…` and calendar mutations are temporarily
inert; the committed in-memory snapshot remains visible. A failed transaction
restores that snapshot and places a retryable message in the existing error
banner.

Opening a database is a distinct state rather than a blocking render path. The
window opens immediately with a calendar-shaped skeleton and a centered
`Opening timetable…` message, then restores the saved Day/Week mode and category
filter while anchoring the active range on today and resetting scroll. A fresh database contains the six default
categories and an intentional empty timetable, so the empty state teaches the
next action instead of presenting seeded sample work.

An unreadable, incompatible, or invalid database replaces the calendar with a
focused recovery panel. It explains the problem in user language, shows the
exact path, and offers Retry, Reveal data folder, and a confirmed Archive and
start fresh action. Archiving moves the original database and rollback journal
to a timestamped recovery folder before a new database is created; Cadence
never overwrites an unreadable file automatically. JSON export is versioned,
pretty-printed, and written through a temporary sibling file followed by a
rename so an interrupted export cannot leave a partial backup.

## M6 direct manipulation and history

Direct manipulation is an accelerated editing path layered on top of the M4
editor, not a second source of event rules. A card click selects, a double-click
opens inspection, dragging the body proposes a snapped date/time move, and
dragging a top or bottom handle proposes a snapped resize. The original card is
dimmed while a dashed ghost shows the proposed result, so the committed event
remains visually distinct until drop succeeds.

Move and resize math lives in `calendar::interaction` and preserves the domain
invariants: events stay within one civil day, moves preserve duration, and
resizes keep at least one snap interval. Near an edge of the scroll viewport,
the active surface auto-scrolls in the same direction as the pointer. Escape
cancels the manipulation and restores the original selection without touching
the repository.

History is session-scoped and bounded to 100 committed changes. Create, edit,
delete, move, and resize operations share one `EventChange` representation.
Undo/redo applies the inverse or forward repository operation first, then moves
the verified entry between the stacks; a failed write leaves both storage and
history unchanged. The toolbar keeps both controls next to the title, outside
the notification region, and disables each control when its stack is empty.

## M7 recurring schedules

The editor's `Repeats` control offers only the useful first-release rules:
Never, Daily, Weekdays, and Weekly with a Monday-first weekday toggle row. An
optional end date is inclusive. A schedule is stored as a series template plus
exceptions, not as a pre-expanded list of events. The repository expands only
the active Day/Week `DateRange`, and occurrence cards use a stable series/date
identity even when an exception moves its displayed date.

Editing or deleting a recurring card presents two explicit scopes: This event,
which writes a replacement or cancellation exception, and This and following,
which truncates the existing series and creates a successor when needed. A
series-start change revises the existing series; a middle-of-series change
splits it. Exceptions that still fall in the successor's rule are rehomed so a
series edit does not silently recreate or lose unrelated occurrence changes.

Recurrence schedules use Jiff civil dates and wall-clock times. They are not
converted to UTC for expansion, so a routine scheduled at 08:00 remains 08:00
across daylight-saving transitions in the configured IANA timezone. This is a
deliberate local-intent rule; timezone display and future notification behavior
remain separate concerns.
