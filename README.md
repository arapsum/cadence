# Cadence

Cadence is a local-first desktop timetable for planning a day and understanding
a week. Built with Rust, [GPUI](https://gpui.rs/), and [GPUI
Component](https://longbridge.github.io/gpui-component/), it keeps the calendar
as a quiet, spatially honest time grid rather than a dashboard.

> **Project status:** Milestone 9 is implemented for the first supported
> release target. Cadence stores timetable data in a local SQLite database,
> supports snap-aware event dragging and resizing, repeating schedules with
> scoped edits, bounded session undo/redo, non-destructive recovery for
> unreadable databases, and a versioned Ubuntu x86_64 `.deb` release artifact.

## What it does

- Switch between a seven-day Week view and a focused Day view.
- Navigate by day or week, jump to today, and filter by category.
- Keep one event per time interval across every category, with a fixed time
  gutter, sticky day headers, current-time treatment, and horizontal or
  vertical scrolling where needed.
- Create events from the toolbar or an empty time slot; inspect, edit,
  duplicate, delete, move, and resize them; undo or redo committed changes
  during the current session.
- Validate event titles, categories, time ranges, and schedule conflicts before
  changing the timetable. Adjacent end/start boundaries remain valid.
- Use pointer and keyboard interactions with light and dark themes.
- Drag event bodies across time and days, resize their start or end, preview
  snapped changes, and cancel an in-progress manipulation with Escape.
- Schedule Daily, Weekdays, or Weekly routines on selected weekdays with an
  optional inclusive end date; edit or delete one occurrence or this and all
  following occurrences without expanding the series into copied rows.
- Store events, categories, settings, and calendar preferences locally with
  transactional writes and numbered schema migrations, including recurring
  series and per-occurrence exceptions.
- Export a human-readable JSON backup or reveal the data folder from the
  toolbar.

## Requirements

The supported development baseline is Ubuntu 26.04 LTS on x86_64, running a
Wayland session with stable Rust 1.97.1 or newer. Cadence currently enables only
the Wayland backend; X11 support has not been enabled or tested.

Install Rust with [rustup](https://rustup.rs/):

```sh
rustup default stable
```

On Ubuntu or Debian, install the compiler toolchain and native libraries used by
the current GPUI Wayland build:

```sh
sudo apt update
sudo apt install \
  build-essential clang cmake git pkg-config \
  libfontconfig-dev libglib2.0-dev libssl-dev libvulkan1 \
  libwayland-dev libx11-xcb-dev libxkbcommon-x11-dev libzstd-dev
```

A working Vulkan driver and Wayland compositor are runtime requirements. The
GPUI dependency graph is locked in `Cargo.lock`; use Cargo's `--locked` flag for
reproducible builds and automated checks.

## Run

From the repository root:

```sh
cargo run --locked
```

## Install a release

Download the Ubuntu x86_64 `.deb` from a draft or published GitHub release and
install it with:

```sh
sudo apt install ./cadence_<version>_amd64.deb
```

The release target is Ubuntu 26.04 LTS on Wayland. See the [user guide](docs/USER_GUIDE.md)
for the day-to-day workflow and [release procedure](docs/RELEASING.md) for
package verification.

The first build downloads Git dependencies and can take several minutes. Later
builds reuse Cargo's cache.

## Workspace structure

Cadence is split into three crates with one-way dependencies:

- `crates/core` — GPUI-free domain types, calendar geometry, editor rules, and
  local persistence. Its integration tests exercise the application data model
  without opening a window.
- `crates/ui` — GPUI views, application state, interactions, dialogs, themes,
  and reusable timetable components.
- `crates/desktop` — the native binary named `cadence`; it owns platform
  startup, app identity, window options, and mounts the UI crate.

The default workspace member is `cadence-desktop`, so `cargo run --locked`
continues to launch the desktop application from the repository root.

## Keyboard shortcuts

| Action | Shortcut |
| --- | --- |
| Show Day | Cmd/Ctrl+1 |
| Show Week | Cmd/Ctrl+2 |
| Previous or next period | Alt+Left / Alt+Right |
| Go to today | Cmd/Ctrl+T |
| Create an event | Cmd/Ctrl+N |
| Undo the latest change | Cmd/Ctrl+Z |
| Redo the latest change | Cmd/Ctrl+Shift+Z or Ctrl+Y |

In Day mode, previous and next move one day; in Week mode, they move one week.

## Local data

On Linux, Cadence stores its database at:

```text
$CADENCE_DATA_DIR/cadence.sqlite3
```

when `CADENCE_DATA_DIR` is set. Otherwise it uses
`$XDG_DATA_HOME/cadence/cadence.sqlite3`, falling back to
`$HOME/.local/share/cadence/cadence.sqlite3`. The first run creates the six
default categories and no sample events. The Export action produces a
versioned, pretty-printed JSON backup without changing the database.

If the database cannot be opened or migrated, Cadence keeps the original file
untouched and shows a recovery screen. Retry, reveal the data folder, or
explicitly confirm Archive and start fresh; the latter moves the database and
rollback journal into a timestamped `cadence-recovery-*` folder before creating
a new one.

## Validate

Run these checks before completing a milestone or submitting a change:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --locked --all-targets --all-features -- -D warnings \
  -W clippy::pedantic -W clippy::nursery -W rust-2018-idioms
cargo test --workspace --locked --all-targets --all-features

scripts/package-linux.sh
scripts/validate-linux-package.sh target/dist/cadence_<version>_amd64.deb
```

## Manual verification

After running the application, verify the following on the supported baseline:

1. The window can be moved, resized, minimized, maximized/restored, and closed.
2. The category filter, Day/Week control, navigation, and theme control work;
   the selected date and filter survive a mode change.
3. The fixed header and time gutter remain aligned while the grid scrolls.
   Narrow windows retain usable cards through horizontal Week scrolling.
4. Adjacent events do not collide; conflicting legacy records are visibly
   flagged, and event hover/focus exposes their complete details.
5. The current-day tint and current-time line appear when today is displayed.
6. New event, empty-slot creation, event inspection, editing, duplication,
   deletion, dragging, resizing, recurring scope edits, undo, and redo update
   both Day and Week immediately.
7. Invalid titles, categories, and time ranges show field-level errors without
   changing stored data; cancelling a dirty form asks for confirmation.
8. Keyboard focus begins in the editor title, follows the form in order, and
   returns to the invoking card or slot when the dialog closes.
9. Restart the app and confirm events, categories, settings, mode, and filter
   survive while the selected date returns to today and the scroll position is
   fresh.
10. Create a Daily or Weekly routine, cancel one occurrence, edit This and
    following, and verify the unaffected predecessor/exception history.
11. Export a JSON backup and verify its version, categories, preferences,
    events, recurring series, and exceptions; test recovery with a copy of an
    unreadable database.

## Project documents

- [Roadmap](ROADMAP.md) — scope, milestones, and technical direction.
- [Product contract](PRODUCT.md) — product intent and experience principles.
- [Design record](DESIGN.md) — surface, geometry, and editor decisions.
- [Changelog](CHANGELOG.md) — release history and supported target.
- [User guide](docs/USER_GUIDE.md) — install, workflow, shortcuts, and data.
- [Release procedure](docs/RELEASING.md) — repeatable tag and package process.
