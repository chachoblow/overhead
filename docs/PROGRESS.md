# Overhead â Progress

Append-only. Newest entry at the bottom. 3â5 lines per session.

## 2026-10-04
- Set up the Pi coding agent for this project: web search, permission gate.
- Scaffolded AGENTS.md, docs/, and Pi prompt templates.

## 2026-10-04 (later)
- Filled in AGENTS.md (crates, verified commands); M0 complete.
- Resolved all DESIGN.md open questions; wrote decisions 0001–0004
  (on-device SGP4, Wi-Fi data + SNTP, curated catalogue, hardcoded location).
- Corrected HANDOFF.md to match actual code state (stub crates, static sim).
- Added .gitignore; untracked ~2100 build artifacts; pushed as 414e80d.

## 2026-10-04 (implementation roadmap)
- Agreed an engine-first M1–M6 roadmap; M1 is current with small research/test/implementation tasks.
- Recorded decision 0005: verified headless calculations before UI, explicit tuning inputs, separate presentation policies.
- Kept compute benchmarks and minimal display testing early; full firmware integration remains M6.
- Updated DESIGN, PLAN, HANDOFF, and planning context; no code changes or build/test runs.

## 2026-10-04 (handoff housekeeping)
- Reconciled HANDOFF with commits 7be5c0d and a39aa98: M1 research, OMM ingestion/validation, and timestamped propagation are implemented.
- Recorded the next task: Earth-fixed/geodetic transforms with reference and boundary tests, followed by observer geometry and the headless runner.
- Reviewed PLAN; its first three M1 checkboxes were already correct. No new decisions or code changes.
- Documentation-only session; builds, tests, and simulator were not rerun.

## 2026-10-04 (M1 Earth-relative coordinates)
- Implemented no_std TEME→ECEF position rotation and WGS-84 geodetic inverse/forward conversions with explicit errors and bounded iteration.
- Added independent ERFA/pymap3d fixtures, a pinned offline generator, and 12 tests including 594 round trips and geometric/time boundaries.
- Recorded decision 0007's frame/boundary clarifications; checked off M1 transforms. Observer-relative range/azimuth/elevation is next.
- Workspace check/tests (27 tests), no-default-feature core check, all-feature tests, strict Clippy, formatting, and fixture reproducibility pass; no simulator/hardware run.

## 2026-10-04 (historical notes relocation)
- Moved the three historical QRSPI notes from `.pi/qrspi/` into `docs/notes/` and removed the empty workflow directories.
- Added historical labels and links to current plans/decisions; removed the obsolete instruction to create workflow unit files.
- Updated HANDOFF; milestone status and accepted decisions are unchanged. Documentation links and diff whitespace checked; no code changes or build/test runs.

## 2026-10-04 (M1 observer geometry)
- Implemented no_std ECEF→observer range/azimuth/elevation, with explicit vertical, coincident-position, pole, and invalid-input behavior (decision 0008).
- Added 27 independent pymap3d geometry references, 12 Skyfield ISS pipeline references, a pinned offline generator, and 11 new tests.
- Workspace check/tests (38 tests), no-default-feature core check, all-feature tests, strict Clippy, formatting, and byte-for-byte fixture regeneration pass.
- Checked off M1 observer measurements; the reproducible headless runner is next. No simulator launch, hardware flashing, or toolchain changes.

## 2026-10-05 (M1 headless runner)
- Added host-only `overhead-track` for explicit single-object OMM, UTC, and WGS-84 observer inputs; reports the full pipeline with units and clear errors.
- Added seven tests, including real-executable comparisons against all 12 existing Skyfield cases; documented reproducible T0/T1/T2 commands.
- Workspace check/tests (45 tests), no-default-feature core check, all-feature tests, strict Clippy, and formatting pass; no new external crates or core changes.
- M1 complete; refine M2 catalogue work next and confirm hardware for early benchmarks. No simulator launch, flashing, or toolchain changes.

## 2026-10-05 (codebase walkthrough and pre-M2 hardening plan)
- Walked through ingestion, propagation, coordinate/observer geometry, CLI, tests, and the static simulator; no implementation changes.
- Agreed to address time-validation consistency, timestamp-bound conversion, and modest frame-type protection before M2; exact API choices remain open.
- Workspace check, 45 tests, standalone default core check, and the T0 CLI example passed during the walkthrough; no numerical CLI bug was demonstrated.
- Updated PLAN/HANDOFF; verified an ISS fixture change was formatting-only and restored its original bytes at the user's request. No simulator or hardware run.

## 2026-10-06 (pre-M2 core API hardening)
- Centralized UTC validation (1957–2100, no explicit leap seconds); a new boundary test reproduced unsupported requested-time acceptance before the fix.
- Added read-only timestamp-bound TEME state and distinct TEME/ECEF position types; migrated CLI/reference pipelines and recorded decision 0009.
- Workspace/default and all-feature tests pass: 50 tests plus 3 compile-fail doctests; standalone no_std core check, strict Clippy, formatting, and diff checks pass.
- Pre-M2 complete; catalogue configuration and merge/conflict policy are next. No dependencies, fixtures, physical models, simulator launch, flashing, or toolchain changes.
