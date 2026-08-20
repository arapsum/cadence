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

## Validation

Automated validation currently passes:

- cargo fmt --all -- --check
- cargo clippy --locked --all-targets --all-features -- -D warnings
- cargo test --locked --all-targets

The remaining release check is a Wayland visual pass at the baseline and
minimum window sizes, including horizontal scrolling and the seeded overlap.
