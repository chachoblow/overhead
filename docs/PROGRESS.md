# Overhead — Progress

Append-only. Newest entry at the bottom. 3–5 lines per session.

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

## 2026-10-06 (M1 review and validation fixes)
- Reviewed M1 against the product direction; added failing regressions for successful NaN propagation and silently ignored contradictory OMM metadata.
- Added finite-output errors and reusable checked OMM deserialization; migrated the CLI and recorded decision 0010. Serde is now an optional direct dependency, with no new library/version.
- Workspace/default and all-feature tests pass: 58 tests plus 3 compile-fail doctests; standalone core checks with/without OMM, strict Clippy, formatting, and diff checks pass.
- M2 remains next; catalogue policy and target measurements are still open. No fixture/model changes, simulator launch, hardware flashing, or toolchain changes.

## 2026-10-06 (documentation cleanup)
- Shortened handoff, roadmap, design, decision records, and usage/reference docs; corrected the stale crate map in AGENTS.md.
- Removed redundant historical notes; retained decision rationale, numerical contracts, fixture provenance, and this session history.
- Added documentation ownership guidance and a short-handoff target to limit repetition. Milestone scope and technical choices are unchanged.
- Local Markdown links and diff whitespace checked; documentation only, no code/fixture changes or build/test rerun.

## 2026-10-07 (M2 local catalogue)
- Settled catalogue merge, conflict, invalid-record, provenance, and nonempty acceptance policies in decision 0011.
- Added opt-in no_std + alloc catalogue assembly, an offline manifest-driven tool/example, and 20 policy/CLI tests; M2 catalogue slice complete.
- Workspace check/tests (78 + 3 compile-fail doctests), all-feature tests, core feature checks, strict Clippy, fmt, and the offline example passed; no new crate/version or fixture changes.
- Next: refine physical pass prediction and measure operating budgets. No simulator launch, firmware/hardware run, flashing, or toolchain changes.

## 2026-10-08 (M2 pass semantics)
- Accepted decision 0012: physical passes, configurable 10°/24-hour/5-second defaults, in-progress/window-limited results, and explicit incomplete searches; peak prediction deferred.
- Clarified above-horizon radar eligibility independently of the pass threshold and zoom; updated DESIGN and checked off product semantics in PLAN.
- Prepared HANDOFF for a new context focused on search strategy, API, and tests, including short-pass detection and boundary cases; implementation and cost measurements remain open.
- Documentation links and diff whitespace checked; no code changes, builds/tests, simulator/hardware runs, flashing, or toolchain changes.

## 2026-10-08 (M2 detection limits)
- Accepted decision 0013: explicit numerical detection limits may miss brief threshold excursions/gaps; retain detected short passes rather than imposing a duration filter.
- Kept radar eligibility and fast-transit presentation independent of prediction; linked the clarification from 0012 and PLAN and refreshed HANDOFF.
- Search algorithm, API, tests, detection resolution, and work budgets remain open; the discussed 10-second limit is not an accepted default.
- Documentation links and diff whitespace checked; no code changes, build/test runs, simulator/hardware runs, flashing, dependencies, or toolchain changes.

## 2026-10-08 (M2 pass-search baseline)
- Accepted fixed-step sampling plus crossing bisection, with no detection default.
- Settled threshold/window boundary behavior and work-exhaustion/partial-result policy in 0014; linked from 0012/0013 and updated PLAN/HANDOFF.
- Kept prediction independent of radar zoom; API, validation, tests, and numeric budgets remain open.
- Documentation links and diff whitespace checked; no implementation, build/test runs, dependencies, or hardware changes.

## 2026-10-08 (M2 synthetic pass-search kernel)
- Accepted API decision 0015; added allocation-free callback search, checked configuration, streamed pass records, and explicit local/shared evaluation budgets.
- Added 30 synthetic tests plus API doctests, including equality boundaries, detection misses, nanosecond refinement, and interruption at every evaluation point.
- Workspace check/default and all-feature tests pass (108 tests + 5 doctests); core feature checks, strict Clippy, fmt, and rustdoc checks pass.
- Next: orbital integration, catalogue reporting, representative interval evaluation, and measured budgets. No dependencies, fixture changes, toolchain changes, or hardware operations.

## 2026-10-08 (M2 orbital pass integration)
- Added allocation-free `search_satellite` in core/src/passes.rs, composing bound propagation/rotation/observer geometry while preserving typed failures, budgets, and partial records.
- Added 10 integration tests: existing Skyfield elevations, ISS/deep-space brackets against same-model fine scans, negative times, boundaries, failures, and budget interruptions; no independent pass-time oracle or detection default is claimed.
- Workspace check/default and all-feature tests pass (118 tests + 6 doctests); standalone core feature checks, strict Clippy, fmt, and rustdoc checks pass.
- Next: catalogue aggregation/reporting, then representative detection intervals and measured budgets. No dependency, fixture, toolchain, simulator, or hardware changes.

## 2026-10-08 (M2 catalogue pass reporting)
- Added alloc-enabled core catalogue aggregation and decision 0016 for uncertain earliest-arrival ordering, preserving every satellite's partial/unsearched report and retained brackets.
- Added `overhead-passes` with explicit observer/time/detection/allowance inputs and incomplete-result exit code 2; reused the extracted local catalogue loader and documented usage in tools/README.md.
- Added 17 tests; workspace check/default and all-feature tests pass (135 tests + 6 doctests), as do standalone core feature checks, strict Clippy, fmt, and rustdoc checks.
- Next: representative detection intervals and measured operating budgets. No dependency, fixture, toolchain, simulator, or hardware changes.

## 2026-10-09 (M2 detection-interval evaluation)
- Added offline `overhead-evaluate-intervals`: 15 historical/constructed orbital cases, 630 interval/phase/tolerance searches, same-model reference-grid comparisons, and evaluation counts; results in docs/evaluations/detection-intervals.md.
- Added pinned Vallado HEO/GNSS/GEO input records and six tests. Ordinary sampled cases agree; constructed ~4s excursions/gaps expose misses that tighter crossing tolerance cannot recover. No independent timing-accuracy or completeness claim.
- Workspace check/default/all-feature tests pass (141 tests + 6 doctests); core feature checks, strict Clippy, fmt, rustdoc, and repeatable release output verified.
- Next: measured operating budgets. No production detection default, numeric allowance, dependency/toolchain changes, simulator launch, or hardware operations; existing ISS fixture unchanged.

## 2026-10-09 (M2 host cost baseline)
- Added `overhead-benchmark`: 62 propagation/geometry/prediction and mixed scaling workloads, calibrated wall-time samples, explicit work accounting, and four tests; shared historical input loading without changing fixtures.
- Recorded two release runs and host/build context in docs/evaluations/host-costs.md: M1 Pro 24h/60s searches ~0.7–0.9ms per fixture, ~48.6ms for 64 repeated mixed slots; not device or catalogue-capacity evidence.
- Workspace check/default/all-feature tests pass (145 tests + 6 doctests); core feature checks, strict Clippy, fmt, rustdoc, and diff checks pass. Work counts agree across runs; timings vary.
- Next: confirm hardware, measure S3 costs and real catalogue memory/aggregation, then choose operating policy. No defaults, dependencies, toolchains, simulator, or hardware changed.

## 2026-10-09 (M2 first S3 cost baseline)
- With user approval, installed ESP tooling and flashed an isolated no_std benchmark on the S3-DevKitC-1/WROOM-1: revision v0.2, 8 MB flash, PSRAM unconfirmed/unused. UART works directly; dock path did not enumerate. Display untouched.
- Added firmware/ with generated original fixture inputs, bounded serial capture, and five offline tests; target dependencies/lock/toolchain isolated from the unchanged host workspace. Usage in firmware/README.md.
- Recorded 14 workloads × 5 samples in docs/evaluations/s3-costs.md: 24h/60s searches ~1.18–2.14s per fixture, four-slot prediction ~6.54s, full tracking ~4.77ms per four-slot tick. All work counts match host; no capacity/accuracy/worst-case claim.
- Workspace check/default/all-feature tests, core feature checks, host Clippy/fmt, new tests, and target check/build/flash/capture pass; embedded linker retains an RWX warning. Next: broaden ages/orbits, measure catalogue/peak memory, then choose scheduling and operating limits; no production defaults selected.

## 2026-10-09 (M2 expanded S3 harness and 16-slot capture)
- Added separately runnable age/search/16–64-slot suites in firmware/, host expected-work manifests, and strict V2 capture validation; all 122 workloads complete/repeat on the host. Core, fixtures, dependencies, and toolchain unchanged.
- With explicit approval, flashed/captured only scaling-16: three samples each, all host counts match. Tracking averages 19.19ms per tick; 24h prediction median 27.34s. Evidence/build hashes in docs/evaluations/s3-scaling-16.md; not catalogue capacity or a controlled same-build comparison with the old baseline.
- Workspace check/tests (145 + 6 doctests), six firmware host tests, seven Python tests, strict firmware-host Clippy, fmt/diff checks, host manifest round trips, target builds, and approved flash/capture pass; existing RWX linker warning remains.
- Next: remaining target suites/V2 baseline, broader orbit samples, numeric checks, and distinct-catalogue/peak-memory measurements before operating policy. All seven age grids remain in flash; no peak-memory measurement or production default chosen.

## 2026-10-10 UTC (M2 remaining S3 target sweeps)
- With explicit approval, flashed/captured ten V2 suites: baseline, 64-slot scaling, and all prepared age/search sweeps. All 120 workloads × three samples match host counts; all 122 prepared workloads now have target evidence including the earlier 16-slot run.
- Recorded raw captures, expected manifests, timing CSV, and artifact hashes in docs/evaluations/s3-expanded.md: 64-slot tracking averages 76.69ms/tick, 24h prediction 109.37s; HEO search rises from 2.35s near epoch to ~21s at ±30 days despite equal evaluation counts.
- Workspace check/tests (145 + 6 doctests), six firmware host tests, seven Python tests, strict firmware-host Clippy, firmware fmt, ten release builds/flashes/captures, and offline revalidation pass. Clean staged snapshot checks and evidence validation pass without local measurement artifacts; CSV line endings normalized and its checksum updated. Known RWX linker warning remains; core, harness, fixtures, dependencies, and toolchain unchanged.
- M2 stays open: distinct-catalogue/aggregation/peak-memory measurements, broader orbit samples, and target numeric checks precede operating policy. Board left with search-gnss (three samples); no monitor running, display untouched, no production default selected.
