# Changelog

All notable Cadence releases are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and releases use
[Semantic Versioning](https://semver.org/).

## [Unreleased]

No unreleased changes yet.

## [0.1.9] - 2026-08-31

### Changed

- Render the Week timetable on a true half-hour grid: full-hour boundaries stay
  prominent, while subtle half-hour dividers improve time placement without
  adding visual noise.
- Start Week with today in the leftmost column and keep the following six days
  visible. Previous and Next now advance the rolling window by exactly seven
  days, preserving the selected day’s relative position.

### Fixed

- Align event cards with the half-hour grid so event start times and durations
  render at their correct positions.

## [0.1.8] - 2026-08-28

### Fixed

- Make Linux tray shutdown deterministic while preserving orderly application
  lifecycle and local data safety.

### Changed

- Align release validation, tagged-binary identity, and documented package
  verification with the reproducible release contract.
- Add regression coverage for audited editor, viewport, persistence, recovery,
  recurrence, appearance, and startup behavior.


## [0.1.7] - 2026-08-27

### Added

- Make desktop reminders actionable: notification text now includes the event
  category, date, and start time, and opening a reminder restores Cadence to
  the exact Day plan and event.
- Add Vim-style Week viewport navigation: `h` and `l` slide the seven-day
  window by one day, while `j` and `k` scroll the time grid by one hour.
- Keep Cadence running when its window is minimized and expose a Linux
  StatusNotifier tray menu for restoring or quitting the application.

### Fixed

- Remove internal occurrence identifiers from user-visible notification bodies.
- Preserve reminder delivery while the main window is minimized.

## [0.1.6] - 2026-08-27

### Added

- Keep Week as a seven-day viewport while allowing continuous horizontal
  scrolling through a buffered timeline; the date range follows the viewport
  instead of staying fixed to one calendar week.
- Split Settings into Themes and Typography pages with searchable catalogs,
  light/dark/system controls, and a Termy-inspired hierarchy.
- Preview a theme or font globally on hover or keyboard focus; click, Enter, or
  Space commits the choice, while leaving or closing Settings restores the
  committed appearance.

### Fixed

- Keep the selected week aligned after surface resize and preserve the rolling
  window during persistence rollback.

## [0.1.5] - 2026-08-27

### Changed

- Make category colours follow the active theme across event cards, indicators,
  filters, summaries, and category settings while preserving saved category
  assignments.
- Derive category surfaces, borders, and indicators from semantic theme colours
  with contrast safeguards for readable text and controls.

## [0.1.4] - 2026-08-26

### Added

- Open application preferences in a dedicated, single-instance Settings window
  that stays synchronized with the main calendar and closes with it.

### Changed

- Reduce the shared main and Settings title toolbar height for a more compact
  workspace.

## [0.1.3] - 2026-08-26

### Changed

- Make Week the sole persistent calendar workspace and open an interactive Day
  plan sheet when a weekday header is selected.

### Fixed

- Ensure the Week workspace and Day plan sheet surfaces fill their available
  height.

## [0.1.2] - 2026-08-24

### Added

- Select multiple visible events and delete them as one undoable operation;
  recurring selections cancel only their chosen dates.
- Start or toggle bulk event selection with Cmd/Ctrl+Left Click.

### Changed

- Make weekly recurrence date-aware by defaulting to the event's weekday (for
  example, “Weekly on Monday”) while retaining custom weekday selections.

## [0.1.1] - 2026-08-23

### Fixed

- Give the week overview more space than the day plan in the initial split-view
  layout.

## [0.1.0] - 2026-08-22

### Added

- Day and Week timetable views with independent scrolling and category filters.
- Event creation, editing, inspection, duplication, deletion, drag-and-resize,
  conflict validation, and bounded session undo/redo.
- Daily, weekday, and weekly recurring events with end dates and scoped edits.
- Local SQLite persistence with schema migrations, recovery for unreadable
  databases, JSON backup export, and data-folder reveal.
- Category management, appearance preferences, GPUI themes, and font controls.
- A live Now / Next sidebar summary, Agenda sheet, per-event reminders, and
  opt-in desktop notifications while Cadence is running.
- Native client decorations, a Cadence application icon, About dialog, version
  output, desktop metadata, and an Ubuntu x86_64 Debian package.

### Notes

- The first release supports Ubuntu 26.04 LTS on x86_64 in a Wayland session.
- X11, macOS, Windows, mobile platforms, and in-app updates are not supported.
