# Cadence

Cadence is a local-first desktop timetable for guiding a day and understanding a
week. It is being built in Rust with [GPUI](https://gpui.rs/) and
[GPUI Component](https://longbridge.github.io/gpui-component/).

The current application is the Milestone 3 read-only timetable. It renders a
seeded seven-day week view and a focused day view with category filtering,
mode-aware navigation, sticky headers, overlap-aware event cards, vertical and
horizontal scrolling, selection/tooltips, current-time treatment, seeded notes,
and light/dark theme switching.

## Current platform baseline

The initial supported development environment is:

- Ubuntu 26.04 LTS, x86_64
- Wayland
- Stable Rust 1.97.1 or newer
- GPUI Component pinned in `Cargo.toml`; the complete GPUI graph pinned by
  `Cargo.lock`

Only the Wayland backend is enabled right now. Enabling and testing the `x11`
feature is a separate compatibility decision; do not assume an X11 session is
supported by this milestone.

## Prerequisites

Install Rust through [rustup](https://rustup.rs/) and select the current stable
toolchain:

```sh
rustup default stable
```

On Ubuntu/Debian, these packages cover the compiler and the native libraries
needed by the current GPUI Wayland build:

```sh
sudo apt update
sudo apt install \
  build-essential clang cmake git pkg-config \
  libfontconfig-dev libglib2.0-dev libssl-dev libvulkan1 \
  libwayland-dev libx11-xcb-dev libxkbcommon-x11-dev libzstd-dev
```

A working Vulkan driver and Wayland compositor are runtime requirements. GPUI is
pre-1.0 and changes quickly. GPUI Component currently declares GPUI from an
unqualified Git source internally, so Cadence uses that same source to avoid two
incompatible GPUI copies. Reproducibility comes from the committed `Cargo.lock`;
use Cargo's `--locked` flag in automated checks.

## Build and run

From the repository root:

```sh
cargo run --locked
```

The first build downloads Git dependencies and can take several minutes. Later
builds reuse Cargo's cache.

## Quality checks

Run the same checks expected before completing a milestone:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings \
  -W clippy::pedantic -W clippy::nursery -W rust-2018-idioms
cargo test --locked --all-targets
```

## Milestone 3 manual check

After `cargo run`, verify all of the following:

1. The window opens at a usable size and can be moved and resized.
2. Minimize, maximize/restore, and close behave correctly.
3. The category select opens, shows category dots, accepts one category, and
   filters the visible cards.
4. The segmented control switches between Week and Day; Cmd/Ctrl+1 and
   Cmd/Ctrl+2 perform the same actions.
5. Selecting a day header or event opens that date in Day mode; returning to
   Week highlights the same date and preserves the category filter.
6. Today, previous, and next move by one day in Day mode and one week in Week
   mode; Alt+Left/Right and Cmd/Ctrl+T perform the keyboard actions.
7. The day header and time gutter remain fixed while the grid scrolls.
8. Adjacent events do not collide, and the seeded Wednesday overlap remains
   individually clickable.
9. Event hover/focus reveals the full title, category, time, and notes; a tall
   Day card also shows the seeded note text.
10. The current day tint and green current-time line appear when today is in the
   displayed week.
11. Reducing the window below the seven-column minimum makes the Week grid
   horizontally scrollable while the toolbar remains usable.
12. The theme button switches the entire window between light and dark colors.

Milestone 3 passes only when the automated checks and this manual check both pass
on the baseline platform. Record visual regressions before starting event
editing.

## Project documents

- [Roadmap](ROADMAP.md)
- [Product contract](PRODUCT.md)
- [Design record](DESIGN.md)
