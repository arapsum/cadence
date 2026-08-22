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

## Plan a day or week

- Use **Day** for a focused, vertically scrollable plan.
- Use **Week** to compare seven days and spot the shape of the week.
- Use the date controls or **Today** to navigate.
- Use the category menu to focus on one type of work.
- Select an empty time slot or choose **New event** to open the editor.

An event can be moved by dragging its body and resized from its start or end.
Adjacent events are allowed, but overlapping events are rejected before they are
saved.

## Recurring events

Choose Daily, Weekdays, or Weekly in the event editor. Weekly routines can use
selected weekdays and an inclusive end date. When editing or deleting a
recurring event, choose whether the change applies to one occurrence or this
occurrence and all following occurrences.

## Organize and recover

Create and edit categories in **Settings**. Appearance settings include light,
dark, or system mode, GPUI themes, font family, and font size.

Undo and redo apply to committed changes in the current session. **Export**
creates a versioned, human-readable JSON backup. If a database cannot be opened,
Cadence keeps the original file untouched and offers recovery actions; archive
the old database only after confirming that you want to start fresh.

## Keyboard shortcuts

| Action | Shortcut |
| --- | --- |
| Show Day | Cmd/Ctrl+1 |
| Show Week | Cmd/Ctrl+2 |
| Previous or next period | Alt+Left / Alt+Right |
| Go to today | Cmd/Ctrl+T |
| New event | Cmd/Ctrl+N |
| Undo | Cmd/Ctrl+Z |
| Redo | Cmd/Ctrl+Shift+Z or Ctrl+Y |

## Data location and limitations

Cadence stores its database at `$CADENCE_DATA_DIR/cadence.sqlite3` when that
variable is set. Otherwise it uses `$XDG_DATA_HOME/cadence/cadence.sqlite3`,
falling back to `$HOME/.local/share/cadence/cadence.sqlite3`.

The first release does not include cloud synchronization, notifications, an
in-app updater, X11 support, or mobile/Windows/macOS builds. Keep a recent JSON
backup before replacing or removing the package.
