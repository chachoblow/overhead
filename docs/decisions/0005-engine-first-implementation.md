# 0005 — Build a verified calculation engine before the UI

Date: 2026-10-04
Status: accepted

## Decision
Build headless calculations, catalogue handling, and physical pass prediction
before substantial display work. Keep tuning inputs explicit and presentation
state separate from physical calculations. [PLAN](../PLAN.md) owns task status.

## Why
Deterministic fixtures and explicit time/location inputs isolate calculation
errors from rendering errors. Display-first synthetic data offers early visual
feedback but is not needed to verify physics. Projection, selection, and
slow-down policies still require visual feedback; do not prebuild every policy.

## Consequences
- Establish independent references, conventions, and tolerances before UI use.
- Configuration starts development-facing, not as device menus or a commitment
  to arbitrary runtime reconfiguration.
- Benchmark propagation early, then prediction; another ESP32's timings remain
  provisional until confirmed on the S3. Validate minimal display output early.
- Full firmware follows simulator work. Slowed markers cannot change readouts.
