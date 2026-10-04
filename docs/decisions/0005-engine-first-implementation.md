# 0005 — Build a verified calculation engine before the UI

Date: 2026-10-04
Status: accepted

## Decision
Build and test the headless satellite calculation foundation, catalogue
handling, and physical pass prediction before substantial display work.
Make tuning inputs explicit; keep presentation-specific behavior separate
from physical calculations.

## Why
- Deterministic fixtures and explicit time/location inputs give us a verified
  foundation that the UI can consume, and isolate calculation errors from
  projection or rendering errors.
- Catalogue groups, prediction thresholds, and operating limits can be tuned
  after seeing the UI without postponing the underlying capabilities.
- Display-first with synthetic data was considered. It offers early visual
  feedback, but is not a prerequisite for independently testable physics.
- Not everything is a parameter: projection semantics, selection behavior,
  and slow-down across zoom changes still need visual feedback. We defer
  those policies rather than trying to complete every possible engine feature.

## Consequences
- M1–M2 produce a useful headless tracker; M3 consumes real satellite data.
- Research and tests establish time/frame/unit conventions and reference
  tolerances; basic numerical output alone is not evidence of correctness.
- Configuration starts as development-facing inputs, not a commitment to
  device settings menus or arbitrary runtime reconfiguration.
- Catalogue requests remain subject to measured resource limits; SGP4 on-device
  and the curated catalogue decisions are unchanged.
- Hardware propagation benchmarking begins as soon as the path works, with
  prediction costs measured later. An old-board result is provisional until
  confirmed on the S3. Minimal display testing also stays early.
- Full firmware integration follows the simulator work. Physical and visual
  state stay separate so slowed markers cannot change true readouts.
- docs/PLAN.md owns the milestone sequence and task status.
