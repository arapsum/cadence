# Cadence user guide

Cadence is a local-first timetable. It helps you turn a day into clear time
blocks while keeping the week visible for context. Your timetable stays on the
machine where you create it.

## Install and launch

Download the `cadence_<version>_amd64.deb` asset from a GitHub release and
install it with:

```sh
sudo apt install ./cadence_<version>_amd64.deb
```

Launch **Cadence** from the application menu or run `cadence`. The supported
release environment is Ubuntu 26.04 LTS on x86_64 with a Wayland session.

## Plan your week

- **Week** is the main calendar, so you can compare seven days and spot the
  shape of the week at a glance. Scroll horizontally to slide the seven-day
  viewport through adjacent dates; the range label follows the window instead
  of stopping at one fixed calendar week.
- Click a weekday header, or focus it and press Enter/Space, to open that
  date's focused **Day plan** in a sheet. The sheet keeps the full time grid,
  including event creation, editing, drag-and-resize, and scrolling.
- Close the Day plan with its close button, Escape, or a click outside the
  sheet. The selected date remains highlighted in Week.
- Use the date controls or **Today** to navigate.
- Use the category menu to focus on one type of work.
- Select an empty time slot or choose **New event** to open the editor.
- Choose **Select events** from the overflow menu, or hold Cmd/Ctrl while
  clicking an event card, to enter bulk-selection mode. Click event cards, or
  focus a card and press Enter/Space, to select them.
  Use **Select all**, **Delete selected**, or Escape to finish the operation.

An event can be moved by dragging its body and resized from its start or end.
Adjacent events are allowed, but overlapping events are rejected before they are
saved.

The sidebar shows what is happening now and what comes next. Open **Agenda** to
scan every event in the selected calendar range.

## Recurring events

Choose Daily, Weekdays, or Weekly in the event editor. A new weekly routine
defaults to the weekday of its event date and shows that choice explicitly—for
example, “Weekly on Monday.” Change the date before customizing the weekday
buttons and the weekly day follows it automatically. Once you select custom
weekday buttons, those days are preserved while you adjust the date, so a
multi-day routine can be scheduled deliberately. Weekly routines also support
an inclusive end date. When editing or deleting a recurring event, choose
whether the change applies to one occurrence or this occurrence and all
following occurrences.

In bulk-selection mode, deleting recurring events cancels only the selected
occurrence dates. The rest of each recurring series remains scheduled, and the
complete batch can be restored with one Undo.

Choose a reminder in the editor, then enable desktop notifications in
**Settings**. Reminders are delivered only while Cadence is running and remain
subject to the operating system's notification permission.

## Organize and recover

Create and edit categories in **Settings**. Category assignments are preserved
when you change themes, while their event surfaces, borders, indicators, and
filter dots adapt to the active palette. **Themes** and **Typography** are
separate settings pages. Hover or keyboard-focus a theme or font to preview it
across the application; click it or press Enter/Space to commit. Leaving the
option or closing Settings restores the last committed appearance. The pages
also provide light, dark, or system mode and font-size controls.

Undo and redo apply to committed changes in the current session. **Export**
creates a versioned, human-readable JSON backup. If a database cannot be opened,
Cadence keeps the original file untouched and offers recovery actions; archive
the old database only after confirming that you want to start fresh.

## Keyboard shortcuts

| Action | Shortcut |
| --- | --- |
| Open a day plan | Click a weekday header, or Enter/Space when focused |
| Previous or next period | Alt+Left / Alt+Right |
| Go to today | Cmd/Ctrl+T |
| New event | Cmd/Ctrl+N |
| Start or toggle event selection | Cmd/Ctrl+Left Click |
| Select all visible events (selection mode) | Cmd/Ctrl+A |
| Delete selected events (selection mode) | Delete / Backspace |
| Close Day plan or cancel event selection | Escape |
| Undo | Cmd/Ctrl+Z |
| Redo | Cmd/Ctrl+Shift+Z or Ctrl+Y |

## Data location and limitations

Cadence stores its database at `$CADENCE_DATA_DIR/cadence.sqlite3` when that
variable is set. Otherwise it uses `$XDG_DATA_HOME/cadence/cadence.sqlite3`,
falling back to `$HOME/.local/share/cadence/cadence.sqlite3`.

The first release does not include cloud synchronization, an in-app updater,
X11 support, or mobile/Windows/macOS builds. Keep a recent JSON backup before
replacing or removing the package; package removal retains the local data
directory.
