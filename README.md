# Cadence

Cadence is a local-first desktop timetable for guiding a day and understanding a
week. It is being built in Rust with [GPUI](https://gpui.rs/) and
[GPUI Component](https://longbridge.github.io/gpui-component/).

The current application is the Milestone 0 framework smoke test. It deliberately
exercises the window chrome, bundled icons, text input and focus, select overlay,
popover dismissal, vertical scrolling, and light/dark theme switching before the
timetable domain is introduced.

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
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
```

## Milestone 0 manual check

After `cargo run`, verify all of the following:

1. The window opens at a usable size and can be moved and resized.
2. Minimize, maximize/restore, and close behave correctly.
3. The bundled Sun/Moon icon renders on the theme button.
4. The Day/Week select opens, accepts a selection, and closes.
5. The popover opens, receives interaction, and dismisses when clicking outside.
6. The event-title input accepts typing, selection, and keyboard focus traversal.
7. The theme button switches the entire window between light and dark colors.
8. Reducing the window height makes the Scroll checkpoint reachable by scrolling.

Milestone 0 passes only when the automated checks and this manual check both pass
on the baseline platform. Record regressions before starting timetable domain
work.

## Project documents

- [Roadmap](ROADMAP.md)
