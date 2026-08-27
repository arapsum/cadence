# Timetable roadmap

## Product goal

Build a local-first desktop timetable that answers two questions quickly:

1. **What should I be doing today?**
2. **How is my week structured?**

The visual direction is the supplied reference: a seven-day Week viewport, a
fixed time gutter, color-coded event cards, date navigation, category filtering,
and a scrollable time grid. A focused Day plan sheet reuses the same event model
and time scale while giving one day enough width for detail and editing.

The first release should be a dependable personal planning tool, not a complete
Google Calendar replacement.

## Scope

### MVP

- Week workspace with a focused Day plan sheet
- Previous, next, and Today navigation
- A visible current-time indicator
- Create, inspect, edit, and delete an event
- Title, date, start time, end time, category, color, and optional notes
- Category filtering
- Local persistence and safe schema migration
- Keyboard navigation for the main workflow
- Light and dark themes
- Useful empty, invalid, and error states

### After MVP

- Drag an event to another time or day
- Resize an event to change its duration
- Repeating events (implemented in M7; retained here as the post-MVP feature
  that follows the core release gate)
- Search and agenda/list view
- Import/export and backup
- Reminders and system notifications
- Calendar sync

### Explicitly deferred

- Accounts, collaboration, or a server
- Google/Outlook/CalDAV sync
- Invitations and attendee management
- Arbitrary recurrence-rule editing
- Multiple time zones displayed at once
- Mobile UI

Deferring these prevents sync, identity, and recurrence edge cases from defining
the architecture before the basic timetable is useful.

## Technical direction

### Framework

- Use [GPUI](https://gpui.rs/) for the application shell, rendering, focus,
  actions, and key bindings.
- Use [GPUI Component](https://longbridge.github.io/gpui-component/) for standard
  controls and visual tokens: `Root`, `Button`, `Select`/`Combobox`, `Input`,
  `DatePicker`/`Calendar`, `Dialog` or `Sheet`, `Popover`, `Menu`, `Notification`,
  `Tooltip`, and scrolling primitives.
- Build the time grid, event layout, current-time line, selection states, and
  direct manipulation as timetable-specific components. A generic `Table` or
  `DataTable` is not a good foundation for overlapping, duration-based events.
- Use the component library theme through `ActiveTheme`; define timetable
  semantic tokens on top of it rather than scattering literal colors.

GPUI Component keeps most basic elements stateless while application views own
state. Follow that model: keep calendar math and data outside render functions,
and let the GPUI view entities coordinate interaction state.

### Dependency policy

GPUI and GPUI Component move together and their documentation currently shows
both published-crate and Git dependency setups. Milestone 0 must prove a matching
pair on the target platform, then pin exact versions or Git revisions. Do not
track an unpinned branch after the spike.

### Current workspace layout

The implementation now uses three crates with one-way dependencies:

```text
crates/
  core/
    src/                    GPUI-free domain, calendar, editor, and store
    tests/                  calendar math, layout, persistence, repository
  ui/
    src/                    GPUI views, state, dialogs, and components
    assets/themes/          bundled GPUI Component themes
  desktop/
    src/main.rs             native startup, platform, and window setup
```

```text
cadence-core  <-  cadence-ui  <-  cadence-desktop
 (no GPUI)       (GPUI views)     (native startup)
```

The core crate must not depend on GPUI. Keeping the domain, calendar geometry,
editor rules, and persistence behind that boundary makes them cheap to test and
lets the desktop shell evolve independently.

### Initial domain model

Use stable opaque IDs from the beginning.

```text
Event
  id
  title
  date
  start_time
  end_time
  category_id
  notes?
  created_at
  updated_at

Category
  id
  name
  color_token
  is_visible

Settings
  week_starts_on
  clock_format       (12h or 24h)
  time_zone          (IANA identifier)
  snap_minutes       (default 15)
  day_start/day_end  (display preferences, not data limits)
```

MVP events are contained within one local calendar day and `end_time >
start_time`. Store wall-clock intent plus the configured time zone rather than
silently assuming UTC. Cross-midnight events and recurrence are added only with
explicit normalization rules and tests.

### Calendar geometry

Create one pure layout engine shared by day and week views:

- Convert time to a vertical offset from midnight.
- Convert duration to height with a minimum visible event height.
- Use a configurable pixels-per-minute scale.
- Partition events by day.
- Detect intersecting intervals for validation; assign overlap lanes only as a
  rendering fallback for legacy conflicting records.
- Expand an event only when doing so does not collide with a later lane.
- Clip only for rendering; preserve the real event times.

Render a full 24-hour model and scroll to the useful range on entry. A preference
may visually emphasize normal waking hours, but events outside them must remain
reachable.

## Milestones

### M0 — Framework and platform spike

**Outcome:** a pinned, reproducible GPUI toolchain that opens a window on the
primary development platform.

**Status (2026-08-20): complete.** The tested dependency graph contains GPUI at
`2b37a3ed5ec75a54f67936630548da03d411d2e8` and GPUI Component at
`49229371e095bfd2ca77d336b5972b16956f0a87`. The smoke screen and exact manual
checklist are documented in `README.md`. Do not start M1 until that checklist has
been exercised on the baseline platform.

Tasks:

- Change the empty library skeleton into a binary plus testable library.
- Add a compatible, pinned GPUI + GPUI Component dependency pair.
- Initialize `gpui_component`, wrap the top-level view in `Root`, and load icons.
- Render a header using a library button, select, popover, and theme token.
- Confirm text input, focus, scrolling, and light/dark theme behavior.
- Record required native packages and the run/test commands in `README.md`.

Done when:

- A fresh checkout builds and opens the same smoke-test window.
- `cargo fmt`, `cargo clippy`, and `cargo test` pass.
- Dependency revisions and platform prerequisites are documented.

**Decision gate:** continue with GPUI only if text input, focus, scroll behavior,
and the supported target platform are acceptable. Resolve this before building
the timetable grid.

### M1 — Domain and deterministic calendar math

**Outcome:** UI-independent types can represent and query a valid timetable.

**Status (2026-08-20): complete.**

Tasks:

- Add `Event`, `Category`, `Settings`, and stable ID types.
- Add event validation and user-facing validation errors.
- Implement day boundaries, week boundaries, date navigation, time formatting,
  time-to-offset conversion, and 15-minute snapping.
- Define a repository trait and an in-memory implementation.
- Seed a realistic sample week covering short, adjacent, and recurring events.

Done when:

- Unit tests cover Sunday/Monday week starts, month/year boundaries, leap days,
  invalid durations, and snap rounding.
- The same query returns the data required by either a day or week view.
- No core domain or layout type imports GPUI.

### M2 — Read-only week view

**Outcome:** the reference design is recognizable and useful with seeded data.

**Status (2026-08-20): implementation complete; manual Wayland visual pass
pending.**

Tasks:

- Build the toolbar: title, category filter, Today, date range, previous/next.
- Build a sticky day header and time gutter.
- Render a seven-day viewport and hourly/half-hour grid lines.
- Render color-coded event cards with title, category, and time.
- Add vertical scrolling and scroll initially near the current/first event time.
- Add horizontal scrolling through adjacent dates while keeping seven columns
  visible.
- Add a current-day treatment and a live current-time line.
- Enforce one event per time interval in repository mutations and retain the
  pure layout engine's overlap lanes for legacy-data presentation.

Implementation notes:

- `crates/core/src/calendar/layout.rs` contains the GPUI-free end-exclusive overlap algorithm
  with minimum occupancy for short events.
- `crates/core/src/calendar/state.rs` owns selected date, category filter, and selection
  transitions.
- `crates/ui/src/app/week.rs` renders one tracked scroll plane with fixed header/gutter
  overlays and a responsive minimum column width; `grid.rs` and `event_card.rs`
  keep the body surfaces independently evolvable.

Done when:

- Adjacent events do not overlap visually.
- Legacy simultaneous events remain individually clickable and readable while
  new and edited events are rejected with an actionable conflict message.
- Week navigation works across month and year boundaries.
- Resizing the window preserves usable headers, gutter, and columns.
- No-event days and an entirely empty week have intentional empty states.

### M3 — Week workspace, day-plan sheet, and shared navigation

**Status (2026-08-20): implementation complete; manual Wayland visual pass
pending.**

**Outcome:** the user can open a focused daily plan without leaving the weekly
overview or losing context.

Tasks:

- Keep Week as the only persistent calendar workspace and open a Day plan from
  a focused weekday header. Alt+Left/Right and Cmd/Ctrl+T remain the shared
  navigation actions.
- Reuse the grid, event card, layout, and current-time components through the
  shared `surface` renderer; the Day sheet is one full-width column and the
  Week surface remains a seven-column viewport.
- Give day cards room for notes/category details where height permits. Seeded
  deep-work events now carry notes, which are shown in sufficiently tall Day
  cards and remain available in tooltips.
- Preserve the selected date, filter, and approximate scroll position when
  opening and closing the sheet. Day and Week use the same minute-based
  vertical scroll coordinate.
- Make Today select the current date and reveal the current-time line.

Implementation notes:

- `CalendarState` keeps the selected date while the UI derives a one-day
  `DateRange` for the sheet and a rolling seven-day window for Week.
- `layout_events` accepts any supported date range, so Day and Week share the
  same overlap lanes and event geometry.
- `crates/ui/src/app/day.rs` and `crates/ui/src/app/week.rs` are thin surface
  adapters over `crates/ui/src/app/surface.rs`; the sheet and root action
  handling share the same state.

Done when:

- Clicking or focusing a Week header opens the selected day; closing the sheet
  returns to the same weekly context.
- Previous/next moves the selected Week window by one week; horizontal scrolling
  moves it continuously by day.
- Both views produce identical geometry for the same start/end times.
- The primary flow is usable at the minimum supported window size.

### M4 — Event inspection and editing

**Status (2026-08-20): implementation complete; manual editor journey pending.**

**Outcome:** the timetable is no longer read-only. Events can be created from a
slot or the toolbar, inspected before mutation, edited in place, duplicated as
a new draft, deleted intentionally, and restored during the current session.

Tasks:

- Click an empty time slot to create an event with date/time prefilled.
- Click an event to inspect it; expose Edit, Duplicate, and Delete.
- Build one editor form for create and edit using library inputs, selects,
  date/time controls, dialog/sheet, and notifications.
- Validate title, duration, and date/time before committing.
- Define app actions for new, save, cancel, delete, and view switching.
- Add keyboard flow: arrow/tab focus, Enter to open, Escape to close, and a
  documented shortcut for New Event.

Done when:

- Create, edit, duplicate, and delete update both views immediately.
- Cancel never changes stored data.
- Destructive deletion requires an intentional action and offers undo during
  the current session.
- Focus returns to the invoking slot/event when an editor closes.
- All validation errors appear next to the relevant field.

Implementation notes:

- `crates/core/src/editor.rs` owns the UI-independent form draft, default-time rules,
  snap-aware time options, date adapters, and field-level validation.
- `crates/ui/src/app/editor/` owns the GPUI Component dialog, editor subscriptions,
  inspector actions, repository mutations, and transient undo state. Create and
  edit share the same form entity; duplicate starts a fresh create draft with a
  new identifier only when it is saved.
- The toolbar's New event button and Cmd/Ctrl+N use the selected date. An empty
  grid hour and Enter on a focused event/slot use the clicked date and hour.
  Today uses the next snapped local time, while another date starts at the
  configured display day start.
- Save validates title, category, and end-after-start before touching the
  repository. Cancel and dialog close require confirmation when the draft is
  dirty. Delete uses a confirmation dialog and keeps the latest deleted event in
  memory for the Undo notification and Cmd/Ctrl+Z. Time options begin at the
  configured display-day start, and the end-time options begin at the first
  valid slot after the selected start time.
- Dialog focus is initially placed in the title field; GPUI Component's dialog
  focus restoration returns focus to the event card or empty slot that opened
  the surface.

### M5 — Durable local persistence

**Status (2026-08-21): implementation complete; manual restart, export, and
recovery pass pending.**

**Outcome:** real data survives restarts and schema evolution.

Tasks:

- Implement the repository with SQLite and numbered migrations.
- Keep database work away from render functions; surface loading/saving state.
- Use transactions for multi-row changes and atomic writes.
- Decide and document the platform-specific data path.
- Add export to a human-readable backup format before adding import.
- Provide clear recovery behavior for a corrupt or incompatible database.

Done when:

- Events, categories, settings, and filters survive restart.
- Migration tests upgrade a fixture from every supported schema version.
- Simulated failed writes preserve the last valid data.
- The user can locate and export their data from the UI.

Implementation notes:

- `crates/core/src/store/sqlite.rs` owns the numbered `PRAGMA user_version` migrations,
  foreign-key/integrity checks, canonical civil date/time encoding, and the
  repository contract implementation. A fresh database creates the six
  categories but no sample events.
- `crates/core/src/store/worker.rs` owns one SQLite connection on a dedicated worker
  thread. The GPUI view receives asynchronous load/write results and keeps the
  last committed in-memory snapshot when a transaction fails.
- On Linux the database is `$CADENCE_DATA_DIR/cadence.sqlite3` when the
  override is set, otherwise `$XDG_DATA_HOME/cadence/cadence.sqlite3`, falling
  back to `$HOME/.local/share/cadence/cadence.sqlite3`.
- The toolbar's Export action writes a versioned, pretty-printed JSON backup
  through the native save dialog. Recovery presents Retry, Reveal data folder,
  and an explicitly confirmed Archive and start fresh action; unreadable files
  are moved into a timestamped recovery directory before a new database is
  created.
- Startup restores the Day/Week mode and category filter, then anchors the
  calendar on today with a fresh scroll position. Scroll, selection, dialogs,
  and undo state remain transient.

**MVP release gate:** M0–M5 are complete, the acceptance journey below passes,
and no known issue can silently lose or shift an event.

### M6 — Direct manipulation

**Status (2026-08-21): implementation complete; manual manipulation pass
pending.**

**Outcome:** reshaping a day is fast while keyboard/form editing remains a full
fallback.

Tasks:

- Add selection, hover, drag preview, drop target, and resize handles.
- Drag vertically to change time and horizontally to change day in Week mode.
- Snap by the configured interval; show the proposed time during manipulation.
- Auto-scroll near viewport edges.
- Commit only on drop; Escape cancels and restores the original event.
- Add undo/redo commands for create, edit, move, resize, and delete.

Implementation notes:

- `crates/ui/src/app/interaction.rs` owns transient drag payloads, snapped proposals,
  resize edges, and viewport-edge auto-scroll calculations.
- `crates/core/src/calendar/interaction.rs` keeps move and resize proposal math free of
  GPUI, including duration preservation, date changes, clamping, and minimum
  snap-sized durations.
- Event cards distinguish click, double-click, move, and resize gestures. A
  dashed preview and dimmed original card make an in-progress mutation clear;
  Escape cancels it before persistence.
- `EventHistory` stores up to 100 committed changes in memory. Undo and redo
  only advance their stacks after the corresponding repository write succeeds;
  failed writes restore the prior snapshot and view state.
- Undo and redo controls sit beside the timetable title so notifications do not
  obscure repeated history actions. Cmd/Ctrl+Z undoes and Cmd/Ctrl+Shift+Z (or
  Ctrl+Y) redoes the latest committed change.

Done when:

- Drag and resize cannot produce invalid or negative durations.
- A cancelled/failed operation leaves storage unchanged.
- Click, double-click, drag, and resize gestures do not conflict.
- All pointer operations have a keyboard/form alternative.
- Undo and redo preserve the bounded session history and remain disabled when
  no corresponding change is available.

### M7 — Repeating events

**Status (2026-08-21): implementation complete; manual recurrence journey and
timezone pass pending.**

**Outcome:** routines can be scheduled once without making the MVP data model
fragile.

Tasks:

- Start with Daily, Weekdays, Weekly on selected days, and an optional end date.
- Store a recurrence series plus exceptions; do not copy rows indefinitely.
- Expand occurrences only for the visible/query range.
- Support editing/deleting This event and This and following events.
- Define daylight-saving behavior before implementation.

Implementation notes:

- `crates/core/src/domain/recurrence.rs` models Daily, Weekdays, and Weekly-on-selected-days
  rules as civil-date series. Occurrences are expanded only inside the visible
  `DateRange`, so a long-running routine never becomes one row per future date.
- Durable storage keeps one `recurrence_series` row per schedule and one
  `recurrence_exceptions` row per cancelled or modified original occurrence.
  SQLite schema version 3 and versioned JSON backups include both collections;
  deleting a series cascades its exceptions.
- A recurring occurrence keeps a stable `(series_id, original_date)` identity.
  Editing or deleting opens a scope choice for **This event** or **This and
  following**. The latter truncates the predecessor and creates a successor
  series while rehoming exceptions that still belong to the successor.
- Recurrence uses civil dates and wall-clock `Time` values. It never converts a
  schedule to UTC, so daylight-saving transitions preserve the user's local
  clock time; the configured IANA timezone remains display/runtime context.
- Recurrence mutations are captured as full repository snapshots in the
  session undo/redo history, keeping series splits, exceptions, and standalone
  events atomic from the user's perspective.

Done when:

- Tests cover daylight-saving transitions, leap days, month/year boundaries,
  deleted occurrences, persistence round trips, and range-bounded expansion.
- A long-running series does not materially slow week navigation.
- Editing one occurrence never unexpectedly rewrites the whole series.

### M8 — Daily guidance and polish

**Outcome:** the app actively helps the user follow the plan, rather than merely
storing it.

**Status (2026-08-22): implementation complete; scaling, localization, and
desktop-permission behavior remain release smoke-test coverage.**

Tasks:

- Add a compact “Now / Next” summary to Day view.
- Add optional desktop notifications and per-event reminder offsets.
- Add category management and accessible color choices.
- Add search or an agenda list for fast event discovery.
- Polish typography, density, truncation/tooltips, empty states, and animation.
- Verify high-DPI behavior, 12h/24h clocks, localization, and dark mode.

Implementation notes:

- The sidebar derives a live Now / Next summary from the current clock and the
  visible schedule. The application clock refreshes it without a restart.
- Event forms provide fixed reminder offsets, while Settings keeps delivery
  opt-in. Due reminders use GPUI system notifications only while Cadence is
  running and still respect operating-system permissions.
- The sidebar and Settings provide create, edit, delete, visibility, and
  replacement flows for categories. Cards, agenda entries, labels, and
  tooltips always expose the category name alongside its colour.
- An Agenda sheet lists the events in the current range. Appearance settings
  persist the mode, bundled GPUI theme, font family, and font size.
- Shared semantic tokens, responsive workspace breakpoints, empty states,
  tooltip text, and independent Day/Week scrolling provide the polish baseline.

Done when:

- Now/Next updates when an event begins or ends without restarting.
- Notifications are opt-in, testable, and respect operating-system permission.
- Color is never the only indication of category or state.
- Common actions remain visible at 125%, 150%, and 200% display scaling.

### M9 — Packaging and release

**Outcome:** a versioned build another person can install and trust.

**Implementation status:** Complete for the first Ubuntu 26.04 LTS x86_64
Wayland target. The repository now contains a reproducible `.deb` builder,
metadata/content validators, a draft-release workflow, and the release/user
documentation. Automated Ubuntu 26.04 container checks cover install, upgrade,
uninstall, and data retention; a Wayland smoke test of the actual draft artifact
and final publication remain deliberate human gates for each tagged release.

Tasks:

- Add CI for formatting, linting, tests, and release builds.
- Package for each actually supported operating system.
- Add app metadata, icons, version display, licenses, and release notes.
- Verify clean install, upgrade, backup, uninstall, and data retention behavior.
- Write a short manual and list known limitations.

Implementation notes:

- `scripts/package-linux.sh` builds a deterministic `cadence_<version>_amd64.deb`
  with desktop/AppStream metadata, the scalable icon, runtime dependencies, and
  SHA-256 checksums.
- `scripts/validate-linux-package.sh` validates package contents, desktop and
  AppStream metadata, executable permissions, and `cadence --version`.
- `scripts/verify-linux-install.sh` uses a clean Ubuntu 26.04 container to
  install a synthetic prior package, upgrade to the candidate, remove it, and
  verify retained user data.
- CI checks formatting, strict linting, tests, and release packaging. Pushing a
  `v*.*.*` tag verifies the changelog/version, attests the artifacts, and opens
  a draft GitHub release with checksums.

Done when:

- A release artifact installs on a clean supported machine.
- Upgrading preserves an existing timetable.
- The release procedure can be repeated from a tag.

### M10 — Rolling Week and appearance previews

**Status (2026-08-27): implementation complete; visual Wayland verification
remains part of the v0.1.6 release gate.**

**Outcome:** Week behaves as a continuous seven-day viewport, and appearance
choices are easy to compare before committing them.

Tasks:

- Keep a 21-day query/render buffer around the seven visible dates and rebase it
  by one week near either edge without a visible jump.
- Preserve the logical date window across resize, refresh, and persistence
  rollback.
- Separate Settings into Themes and Typography pages with searchable catalogs
  and a Termy-inspired hierarchy.
- Apply theme and font candidates globally on hover or keyboard focus; commit on
  click/Enter/Space and restore the committed appearance when a preview ends.

Implementation notes:

- `state::viewport` derives the logical Week start from the horizontal scroll
  offset and compensates the offset whenever the buffered range is rebased.
- `appearance::preview` owns the shared reversible candidate state, while
  `appearance::themes` and `appearance::typography` render the two settings
  pages independently.

Done when:

- The range label follows the seven dates under the viewport while the Week
  header and body remain aligned.
- Settings previews are visible in the main and Settings windows without
  writing until the user commits a choice.
- Strict formatting, lint, tests, package metadata, and release-version checks
  pass for v0.1.6.

## Acceptance journey for the MVP

A release candidate should pass this uninterrupted scenario:

1. Launch into the current week and navigate to Today.
2. Create “Morning routine” for today from 06:00 to 06:30.
3. Attempt a second overlapping event, see the conflict message, then create
   an adjacent event successfully.
4. Filter to one category and clear the filter.
5. Open the selected weekday's Day plan sheet and edit the second event's time.
6. Close the sheet, scroll the Week viewport into the adjacent date range, and
   navigate back using only the keyboard.
7. Delete the first event, undo it, then restart the app.
8. Confirm both final events, settings, selected mode, and correct local times.
9. Export a backup and verify that it contains both events.

## Testing strategy

- **Unit tests:** validation, date ranges, snapping, formatting, schedule
  conflicts, legacy conflict flags, overlap lanes, recurrence expansion, and
  migrations.
- **Property tests:** generated event intervals never yield negative sizes,
  overlapping rectangles never occupy the same lane, and week navigation
  round-trips.
- **Repository contract tests:** run the same create/update/delete/query suite
  against memory and SQLite stores.
- **View/action tests:** dispatch GPUI actions and verify state transitions where
  the framework permits deterministic tests.
- **Visual checks:** maintain fixtures for empty, dense, overlapping, current-day,
  light, and dark calendars at a few standard window sizes.
- **Manual checks:** focus order, keyboard-only editing, screen scaling, locale,
  daylight-saving boundaries, and window resizing.

## Risks and early mitigations

| Risk | Mitigation |
| --- | --- |
| GPUI and component version drift | Prove compatibility in M0 and pin exact revisions. |
| Calendar grid is forced into a generic table | Build a custom grid over pure layout geometry. |
| Date/time bugs move real plans | Centralize time rules and test boundaries before UI editing. |
| Recurrence overwhelms the MVP | Ship single events first; add series + exceptions in M7. |
| Rendering dense schedules becomes slow | Keep rendering cheap, measure first, virtualize time rows only if needed. |
| UI state leaks into persistence | Persist domain/settings only; keep hover, drag, focus, and dialog state transient. |
| Pointer-first interaction harms accessibility | Define GPUI actions and focus behavior before drag-and-drop. |
| A failed write loses data | Transactions, migration fixtures, explicit errors, and exportable backups. |

## Recommended implementation order inside each milestone

Use a thin vertical slice each time:

1. Pure model/state change
2. Unit test
3. Minimal rendering
4. Input/action handling
5. Empty/error/accessibility states
6. Visual polish

This keeps every milestone demonstrable and avoids a long “infrastructure” phase
with no usable timetable.
