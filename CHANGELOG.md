# Changelog

All notable Cadence releases are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and releases use
[Semantic Versioning](https://semver.org/).

## [Unreleased]

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
