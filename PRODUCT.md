<!-- product-spec-version: 1 -->

# Cadence

## Platform

adaptive

## Users

Cadence is for people who plan their own days and want a calm, local-first
overview of how their week is structured.

## Primary job

Answer “what should I be doing now?” and “how is my week structured?” quickly
with a timetable that stays readable as events overlap.

## Product shape

Cadence is a native desktop app built in Rust with GPUI. It presents a
seven-column week view, a focused day view later, and a shared event model with
categories, colors, notes, and local wall-clock times.

## Experience principles

- Keep the time grid spatially honest: event position and duration are the
  source of truth.
- Keep overlaps readable and individually targetable.
- Prefer a quiet, information-dense canvas to dashboard decoration.
- Make navigation, filtering, scrolling, and theme changes reversible and
  obvious.
- Treat empty, invalid, and repository failures as intentional UI states.

## Current scope

Milestone 2 is a read-only seeded week view. It includes week navigation,
Today, a single category filter, sticky day headers, a fixed time gutter,
half-hour grid lines, overlap lanes, event selection/tooltips, current-day
highlighting, a live current-time line, and light/dark theme support.

Event creation, editing, deletion, drag/resize, day view, persistence, and
recurrence are later milestones.
