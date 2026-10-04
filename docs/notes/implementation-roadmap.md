# Implementation roadmap — planning context

_Historical planning notes from 2026-10-04. Descriptions of the code reflect
that session, not current state. See [PLAN](../PLAN.md) for current tasks and
[decision 0005](../decisions/0005-engine-first-implementation.md) for the
accepted roadmap rationale._

## Goal
Agree and record an engine-first implementation roadmap from the current
scaffolding to a standalone satellite-tracking dev-board prototype.

## Contract
- Success: agreed M1–M6 outcomes in docs/PLAN.md, with session-sized M1 tasks.
- This session is documentation only; no implementation or dependency changes.
- Preserve no_std core/render separation, 1-bit output, and decisions 0001–0004.
- Hardware flashing and toolchain changes require permission.
- Knob, enclosure, battery integration, RTC, and full-catalogue/Starlink work
  remain outside this roadmap's delivery scope.

## Assumptions
- [active] Fixed orbital fixtures and explicit time/location inputs allow
  useful calculation work to be tested independently of a UI.
- [active] Tunable values should be explicit; uncertain presentation behavior
  should stay separate rather than being hidden behind premature configuration.
- [active] Initial hardware availability and benchmark board need confirmation.
  Old-ESP32 timings do not establish the final S3 budget.

## Research
- Existing code is scaffolding: core/render stubs and one static simulator frame.
- Decisions 0001–0004 establish on-device SGP4, Wi-Fi/SNTP, curated groups, and a
  configured location. The simulator uses a checked-in OMM snapshot.
- No external API/dependency research performed in this planning session.
  M1 begins with that research and selection of independent reference cases.

## Structure
User accepted: verified calculations → catalogue/prediction/budget → real-data
radar → useful readouts/map → refined motion → standalone firmware.
Early compute and display checks run alongside the engine work.

Decision rationale is recorded in docs/decisions/0005-engine-first-implementation.md.
The canonical milestones, acceptance outcomes, and task status live in
docs/PLAN.md; this historical note is not a separate task list.
