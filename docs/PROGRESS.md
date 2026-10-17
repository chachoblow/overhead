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

## 2026-10-10 UTC (M2 distinct-catalogue host heap slice)
- Added overhead-catalogue-benchmark with requested-heap instrumentation and eight pinned historical inputs; measures checked initialization and retained complete/partial/unsearched aggregation at common UTC. Usage in tools/README.md, provenance in tools/fixtures/README.md.
- Two release runs agree on all work/allocation counts; eight-entry catalogue retains 5,927 bytes, 24h results add 2,560, initialization peaks at 14,279 requested bytes excluding prepared input. Evidence/hashes and exclusions in docs/evaluations/catalogue-costs.md; not target peak RAM or capacity.
- Workspace check/tests (150 + 6 doctests), targeted strict Clippy, fmt, source comparison, and release runs pass. Clean staged snapshot checks/tests and evidence/link validation also pass. No hardware access or core/dependency/toolchain/firmware changes; existing fixtures unchanged.
- Next: bounded target catalogue/allocator and stack-high-water experiment, broader populations/pass density, independent target numeric checks, then operating policy. M2 remains open; no production defaults selected.

## 2026-10-10 UTC (M2 target catalogue-memory preparation)
- Added feature-gated overhead-s3-catalogue-memory: flash OMM, bounded internal esp-alloc LLFF heap, phase/requested/occupied accounting, release checks, and CPU0 written-stack probe/smoke check. Scope in decision 0017; method/commands in firmware/CATALOGUE_MEMORY.md.
- Added same-model host manifest and strict memory-capture validation; default kernel workloads remain allocation-free. Optional target dependencies/firmware lockfile changed; core, host workspace, fixtures, and toolchain unchanged.
- Workspace check/tests (150 + 6 doctests), default/feature firmware tests (6/10), strict host/target Clippy, formatting, 11 Python tests, release host manifest parsing, both target builds, and probe disassembly review pass; existing RWX warning remains.
- No port opened/reset/flash and no target memory results. Next: approved first capture/repeat, broader samples and independent target numeric checks; no capacity/cadence/scheduling defaults selected.

## 2026-10-10 UTC (M2 first S3 catalogue-memory captures)
- With explicit approval, flashed the unchanged prepared catalogue-memory binary and captured two runs of eight cases × three samples. Raw logs, host manifest, summary and hashes live under docs/evaluations/s3-catalogue-memory*.
- All work/heap counters and stack marks agree: eight-entry initialization peaks at 16,024 occupied bytes; 24h catalogue/results retain 8,088, prediction ~14.53s. Observed written-stack depth is 5,824 bytes, not maximum reserved stack or whole-device peak RAM.
- Workspace check/tests (150 + 6 doctests), ten firmware host tests, 11 Python tests, release build/manifest, capture validation and evidence/source/input cross-checks pass. No code/dependency/toolchain changes during capture; board left with catalogue-memory (three samples), no monitor.
- Both boots emit a multiple-DROM mapping diagnostic despite complete runs; retain as unresolved alongside the existing linker warning. Broader populations/density/input-memory/stack evidence and independent target numeric checks precede operating policy; M2 remains open.

## 2026-10-10 UTC (clean-session verification and packaging)
- Verified the staged source in a clean directory without ignored artifacts: workspace check/tests (150 + 6 doctests), firmware tests (6/10), 11 Python tests, strict host/target Clippy, formatting, and both target builds pass.
- Regenerated release host manifest matches the saved one; source/evidence hashes, both captured matrices, summary medians/counters, and documentation links revalidate without local measurement artifacts. Handoff points to fresh-session commands.
- Extended Git's byte-preservation rule to the new UART logs so line-ending conversion cannot invalidate recorded hashes. No hardware access or measured-source changes; M2 scope and unresolved boot diagnostic unchanged.

## 2026-10-10 UTC (M2 offline DROM investigation and next-evidence proposal)
- Reproduced the multiple-DROM cause from the hash-verified saved ELF: a 32-byte NOBITS alignment gap is skipped by espflash. A disposable section-type counterfactual yields one DROM segment; no source fix or corrected target boot claimed. Evidence in docs/evaluations/s3-drom-diagnostic.md.
- Bootloader source and image addresses explain why the captured layout maps both DROM sections through page alignment, not general safety. Original captures/costs remain unchanged; RWX warning remains.
- Proposed at most 20 targeted memory/work cases plus independent numeric and stronger stack gates in docs/evaluations/m2-next-evidence.md; not an accepted contract or operating policy. Next: source remedy/image gate, then refine/freeze evidence scope.
- Workspace check, diff checks, saved ELF hash, regenerated image checksum/hash and both boot inventories validate; counterfactual preserves original address contents except descriptor ELF SHA. No hardware, application source, dependency or toolchain changes; no fresh target build/tests.

## 2026-10-10 UTC (DROM investigation clean-snapshot verification)
- Verified the staged source without ignored artifacts: workspace check/default/all-feature tests, firmware host tests (6/10), 11 Python tests, formatting, strict host/catalogue-target Clippy and both target release builds pass; existing RWX warning remains.
- Regenerated catalogue manifest matches; checked-in source/evidence hashes, both saved capture matrices and documentation links validate. No hardware access, source/dependency/toolchain changes, or claim of a corrected target boot.
- Restored fresh-session commands in HANDOFF and documented rebuilding for structural investigation when the historical ELF is unavailable. Historical binary hashes are not promised for new build paths; scratch files are not prerequisites.

## 2026-10-10 UTC (M2 source DROM remedy and offline image gate)
- Added pinned local firmware/rodata.x remedy and ELF/image byte-verification gate; decision 0018 and firmware/IMAGE_GATE.md document rationale, pre-flash checks and reproduction.
- Both firmware binaries, their 64 KiB stress builds and four alignment fixtures pass; a source-generated NOBITS negative control reproduces two DROM segments and is rejected. Report: docs/evaluations/s3-image-gate-report.json; historical artifacts/costs unchanged.
- Workspace check/tests, firmware host tests (6/10), default/feature strict host Clippy, formatting, 23 Python tests and four full target builds pass. Historical ELF/image also rejected as expected.
- No hardware/dependency/registry/toolchain changes. Corrected normal-build target boot/capture requires approval; separate RWX warning remains. Bounded evidence proposal still unaccepted; M2 limits/cadence/scheduling remain open.

## 2026-10-10 UTC (image-gate clean-snapshot verification)
- Exported staged source to a fresh directory without ignored artifacts; workspace check/default/all-feature tests, firmware host tests (6/10), default/feature strict host Clippy, formatting and 23 Python tests pass.
- Rebuilt both firmware binaries and both 64 KiB stress variants; all eight positive image cases and the source-generated NOBITS negative control pass without historical ELFs. Existing RWX warning remains.
- Fresh-session docs link both image regression and host checks. No hardware access, dependency/toolchain change, or corrected target boot claim; historical report remains unchanged.

## 2026-10-10 UTC (approved corrected catalogue-memory boot verification)
- Fresh normal three-sample build passes the exact ELF/image gate; explicit approval obtained before flash and two reset-separated captures. Both boots map one DROM segment without the historical diagnostic; all boot segment addresses/sizes match the gated image.
- Both eight-case matrices validate (48 samples); work/heap/stack records match the historical experiment, excluding timings. Raw logs and artifact report linked from docs/evaluations/s3-drom-diagnostic.md; original captures unchanged, new UART logs byte-preserved in Git.
- Workspace check, 23 Python tests, locked target build, regenerated host manifest, input-byte/hash and compiled stack-probe checks pass. No application source/dependency/toolchain changes; separate RWX warning remains.
- Board now holds the corrected catalogue-memory build; no monitor left running. M2 remains open: refine/freeze the bounded evidence proposal before broader measurements, supported limits or scheduling decisions.

## 2026-10-10 UTC (corrected-boot fresh-context verification)
- Exported staged source without ignored artifacts: workspace check/default/all-feature tests, firmware host tests (6/10), strict default/feature firmware host Clippy, formatting and 23 Python tests pass.
- Rebuilt both normal firmware binaries and both 64 KiB stress variants; all eight positive image cases and the NOBITS negative control pass. Existing RWX warning remains; no hardware accessed during this verification.
- Fresh snapshot validates saved capture hashes/matrices/boot inventories, measured code/input hashes, regenerated host manifest and Markdown file links. HANDOFF links offline commands; historical ignored binaries are not prerequisites.

## 2026-10-10 UTC (bounded M2 evidence preparation scope)
- Accepted 0019 and refined `evaluations/m2-next-evidence.md`: ≤20 cases, explicit input/population caps, deterministic density selection, ingestion outcomes and separate numeric/stack gates.
- User chose a separately pinned real-orbit cohort while preserving historical controls; no new snapshot or exact capture manifest is claimed. Host fixture preparation/preflight and manifest freeze precede target-harness work.
- Updated PLAN/HANDOFF and the firmware guide's next step. Docs-only: workspace check, whitespace and changed-doc local file links pass; no code/dependency/toolchain/hardware change.

## 2026-10-10 UTC (preparation-contract fresh-context verification)
- Exported staged source without ignored artifacts; with cached dependencies and installed toolchain/SDL2, offline workspace check/default/all-feature tests, no_std core check, firmware host tests/strict Clippy, formatting and 23 Python tests pass.
- Rebuilt normal/stress firmware images; all eight positive image-gate cases and the NOBITS negative control pass. Regenerated host catalogue manifest matches saved evidence; Markdown file links resolve.
- HANDOFF now records reproducible verification and links the offline commands. No hardware accessed; fixture preparation and exact expanded-manifest freeze remain next.

## 2026-10-10 UTC (pinned M2 pool and initial source suitability)
- Archived four CelesTrak response bodies/provenance in `tools/fixtures/m2-pool/`; deterministic offline extraction preserves 16 distinct source objects verbatim, with all epochs within 48h of the new cohort UTC. Historical fixtures unchanged.
- Added `overhead-m2-pool-preflight` and published `evaluations/m2-pool-suitability.{md,json}`: initialized paths and all 34 growth/density searches pass; case 07 is densest at 68 stored passes. No full preflight/manifest freeze or target fit claimed.
- Workspace check/default/all-feature tests, no_std core check, strict all-target/all-feature Clippy, fmt, five Python integrity tests, exact offline regeneration and changed-doc links/whitespace pass. No dependencies/toolchain/firmware/hardware changed.
- Full diagnostic/error/partial/RAM and collection/input-memory preflight plus independent numerical references remain next, before the ≤20-case manifest freeze and target implementation.

## 2026-10-10 UTC (pinned-pool fresh-context verification)
- Exported staged source without ignored artifacts; offline workspace check/default/all-feature tests, no_std core check, strict all-target/all-feature Clippy and formatting pass using cached dependencies and the installed toolchain/SDL2.
- Offline pool hash/raw-byte regeneration, all 34 published host searches and exact report comparison pass; firmware default/feature host tests/Clippy and 28 Python tests pass. No target images rebuilt or hardware accessed.
- Clarified fresh-checkout prerequisites/commands, preserved raw-response CRLF through scoped Git attributes, ignored host-tool Python caches, and refreshed HANDOFF. Full preflight/manifest freeze remains next; no new product or preparation-contract decision.

## 2026-10-10 UTC (candidate M2 host ingestion/work/requested-memory preflight)
- Added `overhead-m2-preflight` and deterministic `tools/fixtures/m2-cases/` input archive; all 20 candidate cases pass retained diagnostic/error/partial outcomes and full-report/pass consistency checks. Historical fixtures/pool unchanged.
- Published `evaluations/m2-host-preflight.{md,json}` with capacities/per-record allocation changes, independently counted allocate-copy-free/input overlap, dropped/retained RAM endpoints, repeatability and full requested-memory release. No backend/S3-fit, timing, stack or numerical claim.
- Offline workspace check/default/all-feature tests, no_std core check, strict Clippy, fmt, eight pool/case Python tests and exact debug/release/input regeneration pass. No dependency, core API, toolchain, firmware or hardware changes.
- Numerical references/identical host-target check selection and final contract review/manifest freeze remain next; no target implementation or new policy decision.

## 2026-10-10 UTC (candidate-host fresh-context verification)
- Exported staged source without ignored artifacts; offline workspace check/default/all-feature tests, no_std core, strict Clippy and formatting pass with the installed toolchain/cached dependencies and SDL2.
- Firmware default/feature host tests/Clippy, all 31 Python tests, candidate/pool input integrity checks, and exact candidate debug/release plus pool release reports pass. No hardware accessed or target images rebuilt.
- Added fresh-context startup/check commands and refreshed HANDOFF. Numerical preparation/final manifest freeze remains next; no new policy or dependency/toolchain change.

## 2026-10-11 UTC (M2 numerical preparation and manifest freeze)
- Pinned 12 original CelesTrak TEME vectors and all existing independent coordinate/observer rows in `tools/fixtures/m2-numerical/`; one allocation-free firmware-library checker passes all 75 selected rows on host with unchanged tolerances.
- Completed target metadata/diagnostic representation review and froze all 20 unchanged cases as `m2-bounded-20-v1` under decision 0020; `evaluations/m2-frozen-manifest.md` owns the annex and next implementation block. No new matrix or production policy.
- Workspace check/default/all-feature tests, no_std core, strict Clippy/fmt, firmware host feature combinations, 39 Python tests and exact preflight/numerical regeneration pass; clean-source export verifies offline checks and reports without ignored artifacts.
- No target images built or hardware accessed; no dependency/toolchain/core API changes. Next: implement/capture the frozen suite, then choose operating limits/cadence/over-budget behavior and separately verify scheduling latency.

## 2026-10-11 UTC (frozen-manifest fresh-session verification)
- Exported the Git index without ignored artifacts; offline workspace check/default/all-feature tests, no_std core, strict Clippy/fmt, all firmware host feature combinations and 39 Python tests pass.
- Exact input/numerical/manifest regeneration and debug/release host preflight/numerical reports agree; original reference bytes survive Git staging unchanged. Existing toolchain/dependency caches and SDL2 remain documented prerequisites.
- Refreshed HANDOFF verification; preparation scope, frozen manifest and next target-implementation block are unchanged. No target images built or hardware accessed.

## 2026-10-11 UTC (frozen M2 harness implementation, offline only)
- Added lossless static case tables, the owned 20-case pipeline, timed/diagnostic examples, independent pipeline heap peaks and per-phase two-pattern stack probes; pinned preparation inputs/core/dependencies/toolchain remain unchanged.
- Added strict capture/set validation and normal-image archival/gating with source hashes, section/symbol inventories and frame review; `firmware/M2_CAPTURE.md` owns commands, timeout derivation and limits. No full private collection-capacity or safe-stack claim.
- Workspace/firmware host gates, 48 Python tests, exact preflight/freeze regeneration, clean-source smoke checks, both exact normal image gates and existing linker regression pass. Images remain local ignored artifacts, not target evidence.
- No flash, reset or monitor access. Next: obtain approval for two flashes/four reset-separated captures; operating-budget and scheduling decisions remain open.

## 2026-10-11 UTC (approved frozen M2 S3 captures)
- Flashed the two exact normal images with approval; both timed boots and both diagnostic boots pass: 120 timed samples, 75 unchanged numerical rows per diagnostic boot, 252 phase-stack observations. No failed/partial capture attempt or firmware/dependency/toolchain change.
- Work/heap/collection counters agree across boots/images with full release. Peak requested/backend occupancy is 31,993/32,124 B; slowest aggregation ~25.58 s; deepest observed phase-stack mark 9,248 B is not a safe allocation. `evaluations/s3-m2.md` owns interpretation and limits.
- Preserved raw logs/build/reset/hash provenance in `evaluations/s3-m2/`; added reproducible standard-library reporting and tests. Workspace checks/tests, 52 Python tests, frozen gates and exact table regeneration pass; all boot segments match gated images without multiple-DROM diagnostics.
- Board now holds the diagnostic image, idle with monitors closed. Next: initial operating-budget decision and separate bounded-scheduling verification; whole-device RAM/safe stack and RWX remain open.

## 2026-10-11 UTC (fresh-session verification before commit)
- Exported the exact Git index without ignored files; workspace check/default/all-feature tests, no_std core, strict Clippy/fmt, all firmware host feature combinations and 52 Python tests pass offline.
- Frozen sources/fixtures, exact host preflight/numerics and all derived capture tables reproduce; staged raw-log/sidecar hashes retain original bytes. Both normal M2 images and the complete historical image-gate regression build/pass from the fresh export.
- Updated fresh-session entry points/prerequisites in HANDOFF and harness docs. No hardware access, toolchain/dependency change or new operating-policy decision; remaining M2 scope is unchanged.

## 2026-10-11 UTC (radar-first iteration on the verified engine)
- Accepted 0021: begin the minimal real-data M3 simulator slice while M2 budgets/scheduling remain open; preserve engine correctness, prediction code/tests and frozen evidence.
- Radar quality takes priority over prediction; catalogue size and information-area contents/proportions need visual iteration. The proposed small-catalogue operating package was not accepted.
- Updated PLAN/DESIGN and cross-linked amendments to 0003/0005; HANDOFF now points to refining the M3 slice. Live retrieval remains M6; no scheduler or numeric production limits selected.
- Documentation only; no code/dependency/toolchain or hardware changes. Build/test suites were not rerun; documentation whitespace and local-link checks pass.

## 2026-10-11 UTC (radar-first fresh-session verification before commit)
- Exported staged tracked sources without ignored build/capture artifacts; offline workspace check/default/all-feature tests, no_std core, strict Clippy/fmt, simulator build and local catalogue smoke test pass.
- All 52 Python tests, frozen manifest/source checks and exact capture-table regeneration pass. No simulator window opened, target images rebuilt, hardware accessed or tools/dependencies changed.
- Refreshed HANDOFF with current static-simulator/renderer-stub state, startup links and verification. Next session refines M3 under 0021; M2 limits/scheduling remain open.

## 2026-10-12 UTC (minimal real-data ground-map simulator)
- User chose a map-like ground-track radar; implemented shared no_std + alloc app state, orthographic projection, 1-bit rendering and explicit 1× replay under 0022. Core/prediction/firmware sources and frozen data are unchanged.
- Added zoom, basic identification, typed per-object failures, local catalogue examples and deterministic PNG snapshots; inspected 16-object and 105-object replays at two layouts/zoom scales. No product capacity or cadence selected.
- Workspace check/default and all-feature tests, no-default-feature core/app/render checks, strict Clippy/fmt and diff checks pass; 21 app/render/simulator tests cover state, geometry, rendering and real CLI behavior.
- SDL dummy loop ran for two seconds; no interactive input review, hardware access, firmware build or toolchain change. Next: user feedback on moving radar/density/layout; M2 budgets/scheduling remain open.

## 2026-10-12 UTC (M3 fresh-session verification before commit)
- Exported staged tracked sources to a new directory with an empty build target; offline locked workspace check/default/all-feature tests, core/app/render no-default-feature checks and strict Clippy/fmt pass.
- Built the simulator there; documented snapshots reproduce 4/22/6 visible markers, demo works from an unrelated directory, and SDL dummy-loop smoke passes. Cached dependencies/toolchains and Homebrew SDL2 remain prerequisites; no interactive review or hardware access.
- Found the current-tree frozen Python gate rejects the new workspace lockfile; confirmed this is the only pinned-source mismatch and external locked packages are unchanged. Accepted 0023: historical evidence uses full revision ab267ae587585d1a0fa9983e49bad49537055c0d, without weakening guards or renewing the freeze.
- From that pinned Git archive, all 52 Python tests, pool/case/numerical/source/manifest/table gates, four-log validation and exact derived-table regeneration pass. Documented separate current/historical verification; no target images rebuilt or evidence changed.

## 2026-10-12 UTC (perspective-camera intent; implementation deferred)
- User liked the busier weather/GPS first pass and clarified that markers should represent actual 3D satellites viewed by a north-up camera above the observer; zoom moves the camera at fixed field of view, rather than scaling ground tracks.
- Accepted direction 0024, amending only 0022's projection/zoom intent; retain true observer-horizon eligibility, physical readouts, explicit failures and the existing shared architecture. Numeric camera parameters remain open.
- Updated DESIGN/PLAN and fresh-session HANDOFF; current code/tests/commands still use the orthographic ground map. User explicitly requested documentation only, with implementation refinement deferred to a fresh session.
- No code, dependency, fixture, hardware or toolchain changes; build/test suites not rerun. Documentation whitespace and local file links checked.

## 2026-10-12 UTC (fixed-camera lens-zoom decision)
- Reconsidered the initial camera-travel intuition; accepted 0025's fixed camera with variable FOV, superseding only 0024's moving-camera/fixed-FOV choice. Actual 3D perspective and physical-state contracts remain unchanged.
- Lens zoom favors magnifying the observer-centered scene without crossing satellite altitudes. Fixed camera distance, FOV defaults/range, clipping, Earth depiction and CLI/readout semantics remain open; no numeric proposal was accepted.
- Added the supersession note to 0024 and updated DESIGN/PLAN/HANDOFF. Current code/tests/CLI still implement the orthographic ground map; no behind-camera indicators or implementation changes included.
- Documentation only, approved after preview; whitespace and local file links checked. No build/test rerun, dependency/fixture/toolchain changes or hardware access.

## 2026-10-12 UTC (lens-zoom handoff fresh-source verification before commit)
- Exported staged tracked files to a clean directory with an empty build target; offline locked workspace check/default/all-feature tests, core/app/render no-default-feature checks, strict Clippy and formatting pass.
- Built and ran the current simulator from that export: snapshots reproduce 4/22/6 visible markers from an unrelated working directory with SDL unavailable; SDL dummy-loop smoke runs for two seconds. No ignored binaries needed; cached Rust tools/dependencies and Homebrew SDL2 remain prerequisites.
- Refreshed HANDOFF verification and run prerequisites; 0025 and open camera parameters remain the next-session direction. Results cover the existing ground map, not perspective/lens zoom; no interactive review, hardware access, firmware build or frozen-evidence rerun/change.

## 2026-10-12 UTC (fixed-camera perspective implementation)
- Accepted 0026 after user review: 50,000 km above observer, 20° default/0.1°–100° lens zoom, 1 km near plane, shared WGS-84 Earth outline and actual ECEF satellite projection. Replaced map-radius CLI/readout semantics without changing physical tracking or horizon eligibility.
- Updated shared app/renderer and simulator tests with independent ellipsoid tangency and physical-look-angle references. Inspected weather/GPS snapshots at 20°/100°/5° (4/22/4 markers, SKY 22); the unchanged small demo is empty at default zoom and shows its four eligible objects at 100°.
- Working-tree workspace check/default/all-feature tests, core/app/render no-default checks, strict Clippy/fmt, deterministic CLI snapshots and two-second SDL dummy-loop smoke pass. No fresh-export verification or interactive input/motion review; visual density/layout feedback is next in M3.
- Updated decision/status/usage/handoff docs. No dependencies, core/firmware sources, frozen fixtures/evidence or toolchains changed; no hardware access. M2 operating budgets/scheduling remain open.

## 2026-10-12 UTC (higher fixed camera after perspective feedback)
- User strongly preferred the perspective view, specifically zoom-magnified screen motion and the fuller overview, and chose a 75,000 km fixed camera for a smaller Earth. Accepted 0027, superseding only 0026's height; lens range/controls and physical-state contracts stay unchanged.
- Updated the camera constant, CLI help and geometry/render/snapshot regressions. Weather/GPS now shows 6/22/4 markers at 20°/100°/5° with unchanged SKY 22; inspected regenerated PNGs, including the roughly 15-pixel Earth at widest zoom.
- Workspace check/default/all-feature tests, core/app/render no-default checks, strict Clippy/fmt, deterministic snapshots and two-second SDL dummy-loop smoke pass at the new height. No dependencies, engine/firmware/fixture changes, hardware access or frozen-evidence renewal.
- Recorded the user feedback in DESIGN/PLAN and refreshed usage/handoff. Higher-camera and panel-layout review remain next in M3; no fresh-export verification or agent interactive input review.

## 2026-10-12 UTC (higher-camera framing approved)
- User prefers the 75,000 km view and likes the empty outer band as framing; retain the camera and margin rather than filling it through auto-fit or more objects.
- Recorded this review in 0027/DESIGN/PLAN and current usage/handoff; radar/information-area proportions remain the next visual question in M3, with no production population chosen.
- Documentation only; no code or hardware changes and no test rerun. Prior implementation verification remains applicable; documentation whitespace and local links checked.

## 2026-10-12 UTC (perspective fresh-session verification before commit)
- Exported staged tracked sources to a clean directory with an empty build target; offline locked workspace check/default/all-feature tests, core/app/render no-default checks, strict Clippy/fmt and simulator build pass.
- From an unrelated directory with SDL unavailable, fresh executable snapshots reproduce pool 0/4 markers at 20°/100° and weather/GPS 6/22/4/0 at 20°/100°/5°/0.1°. SDL dummy-loop smoke runs for two seconds; cached Rust tools/dependencies and Homebrew SDL2 remain prerequisites, with no ignored build artifacts needed.
- Refreshed HANDOFF and simulator verification notes; user explicitly deferred radar/panel review to the next session. The 75,000 km camera and empty outer framing are accepted; no further UI changes made.
- No new dependencies, fixtures, toolchains, hardware access, firmware build or frozen-evidence rerun/change. Documentation whitespace/local links checked; M2 budgets/scheduling remain open.

## 2026-10-12 UTC (viewport sizing and borderless square)
- Compared 240/200-pixel allocations and 20°/30°/40°/100° views; user retained 240×240 and provisional 20°. Added shared circular/borderless-circular/square clipping with explicit simulator comparison options, then selected borderless square as default under 0028.
- App/render/CLI tests cover lens-scale preservation, corner clipping for markers/Earth, counts/selection, unchanged physical state, deterministic snapshots and implicit-default square wiring. Weather/GPS stays 6/22/4/0 at 20°/100°/5°/0.1°; at 40° square shows 18 versus circular 17, with SKY 22 and no failures.
- Working-checkout workspace check/tests, app/render no-default checks, targeted strict Clippy and fmt pass; the default 20° PNG matches the reviewed square image byte-for-byte. No new fresh-export or agent interactive verification, dependencies, hardware access, toolchain/firmware changes or frozen-evidence renewal.
- Updated decision/design/plan/usage/handoff; next is right-panel exploration, not finalizing its existing contents. M3 continues and M2 product budgets/scheduling remain open.

## 2026-10-12 UTC (panel direction, visual iteration deferred)
- Reviewed current readouts and agreed normal/debug separation plus removal of branding/routine diagnostic fields from the normal view (0029). Retain redesigned view/sky information; range, other fields, controls and failure communication remain open.
- User supplied Are.na inspiration and the brief “alive and active, but not busy.” Produced disposable normal/debug and instrument-style mockups under `/tmp/overhead-panel-mockups/`; neither normal-panel design was accepted. Pass hills/carets were explicitly illustrative, not predictions.
- User deferred further visual iteration to another session. Recorded direction and boundaries in DESIGN/PLAN/HANDOFF; no panel split, debug toggle, graphical readouts or prediction integration implemented. M3 remains open.
- Design/docs only after the previously verified square-default change; no further application code, test rerun, dependencies, hardware/toolchain work or frozen-evidence changes.

## 2026-10-12 UTC (square-default fresh-session verification before commit)
- Exported the staged Git source tree into a fresh directory with an empty build target. Offline locked workspace check/default/all-feature tests, core/app/render no-default checks, strict workspace Clippy/fmt and simulator build pass.
- Fresh executable runs from an unrelated directory with SDL unavailable: pool 0/4 at 20°/100°, weather/GPS square 6/18/22/4/0 at 20°/40°/100°/5°/0.1°, circular modes 17 at 40°, all with zero failures. Default 20° PNG matches the reviewed square image; two-second SDL dummy-loop smoke passes.
- Updated HANDOFF and simulator verification notes. Existing Rust caches and Homebrew SDL2 are prerequisites; no ignored builds or disposable mockups are required. Normal/debug split remains direction only; panel design is deferred.
- No new application code, dependencies, hardware access, firmware/toolchain changes or frozen-evidence renewal; no agent interactive review. Documentation whitespace/local links checked.

## 2026-10-13 UTC (session orientation and automatic wrap-up)
- Added `/next` for a bounded, read-only next-task recommendation; expanded `/handoff` to verify, preserve fresh-session context, then commit and push without routine approval (0030).
- AGENTS routes equivalent natural-language requests to these prompts. Ambiguous scope, unresolved checks, unsafe Git recovery and unclear push destinations require a pause; unrelated work is preserved.
- Refreshed HANDOFF while retaining the unapproved-panel boundary and M3 next step. PLAN unchanged: no milestone work completed or product decisions altered.
- Docs/prompts only: whitespace, local paths/links and instruction consistency reviewed; no Rust checks rerun, hardware/toolchain changes or frozen-evidence renewal.

## 2026-10-13 UTC (documentation consolidation)
- Shortened the default reading path: grouped completed PLAN work, separated resume context from DESIGN intent, and linked simulator verification to dated history. M3 panel exploration and M2 budgets/scheduling remain open; no product scope changed.
- Corrected stale firmware/fixture status and separated current Python checks from pinned M2 reproduction. Accepted decisions, frozen document bytes, fixture data and evidence remain unchanged.
- Added the cleanup prompt and AGENTS routing, including a safeguard for manifest-hashed documentation; guarded completed bootstrap setup against accidental reuse. User confirmed inclusion of the pre-existing prompt/routing in the wrap-up commit.
- Docs-only verification: final diff, whitespace, local links/anchors/paths and instruction consistency checked; command contracts reviewed statically. No Rust/runtime checks, hardware/toolchain changes or evidence regeneration; older records and external sources were not exhaustively revalidated.

## 2026-10-13 UTC (retire completed bootstrap prompt)
- Deleted `.pi/prompts/bootstrap.md` at the user's request; setup is complete and `/resume`/`/next` cover ongoing orientation. Historical bootstrap mentions are retained.
- Updated HANDOFF for the next session; PLAN, DESIGN and accepted decisions remain unchanged. Next product work is still scoping M3 panel exploration with the user.
- Checked final diff, whitespace, handoff local links/anchors/paths, remaining bootstrap references and instruction consistency. No Rust checks rerun: no code/build inputs changed; no runtime/hardware checks, toolchain changes or frozen-evidence renewal.

## 2026-10-13 UTC (consolidate orientation prompts)
- Deleted `.pi/prompts/resume.md` at the user's request; `/next` remains unchanged and covers orientation plus a bounded task recommendation (0031).
- Updated HANDOFF and recorded the choice in `docs/decisions/0031-consolidate-orientation-prompts.md`; historical prompt mentions are retained. PLAN/DESIGN unchanged; next product work remains scoping M3 panel design with the user.
- Checked final diff, whitespace, changed-document local links/anchors/paths, remaining resume references and instruction consistency. No Rust checks rerun because no code/build inputs changed; no runtime/hardware checks, toolchain changes or frozen-evidence renewal.

## 2026-10-15 UTC (wide radar and selected-satellite glyph direction)
- Accepted 0032's future presentation direction: wide borderless radar, off-screen indicators, marker-adjacent bare ID/ISS exception, and right-hand category/elevation/altitude/speed/vertical-coil period glyphs; normal counts/FOV removed from the brief, not the executable.
- Retained one final schematic in `docs/mockups/0032-glyph-radar.png`; DESIGN and 0032 preserve the starting point and open contracts without requiring chat, `/tmp` scripts or the iteration history. Updated PLAN/HANDOFF and clarified current simulator usage.
- Docs/static-image changes only; square radar and diagnostic panel remain implemented. Reviewed source/tests and generated synthetic 1-bit mockups, not real-data/target validation; no Rust checks rerun because code/build inputs did not change.
- Verified final/staged whitespace, changed-document local links/anchors/paths, retained PNG dimensions/mode and instruction consistency. No interactive/hardware checks, toolchain changes or frozen-evidence renewal; M2 budgets and M3 implementation remain open.

## 2026-10-15 UTC (wide-radar comparison and visual approval)
- Implemented opt-in `--wide` under 0033 in shared app/render code and the simulator: 352×240, vertical FOV preserving pixel scale, one arrow per eligible off-screen satellite with overlaps allowed, no invalid-depth arrows, and explicit warning notices; square diagnostics remain default.
- User approved the wide layout/arrows and deferred selected-marker identification to next session. Updated DESIGN/PLAN/HANDOFF and simulator usage; label placement/collisions, glyph mappings/transitions and final normal/debug access remain open.
- Wrap-up reran fmt, workspace check and full workspace tests successfully; headless CLI regressions cover two real catalogues/times with independent geometry. Earlier snapshots show weather/GPS at 20° with 8 markers/14 arrows and at 100° with 22/0; selected PNGs visually inspected. An earlier concurrent rebuild caused CLI spawn ENOENT; serial reruns pass.
- No new dependencies, engine/fixture/lockfile changes, agent interactive/hardware validation, toolchain/firmware work or frozen-evidence renewal. M2 budgets/scheduling remain open; no device-performance claim.

## 2026-10-16 UTC (selected-marker identification and zoom visibility)
- Implemented wide-only NORAD ID / ISS labels under [0034](decisions/0034-selected-marker-identification.md): bounded placement, transparent overlaps and full-width IDs; square diagnostics, arrows, selection and warnings unchanged.
- User review of crowding led to [0035](decisions/0035-label-zoom-visibility.md): configurable 70° hide / 60° restore hysteresis in clockless app state, selected diamond retained. Dither mockups stayed exploratory; updated HANDOFF/PLAN/DESIGN and simulator reproduction/tuning commands.
- Wrap-up reran fmt, workspace check/tests, app/render no-default-feature checks and strict all-target app/render/sim Clippy successfully; fixed two new-test iterator lint warnings. Checked local Markdown links/anchors and final/staged whitespace.
- Earlier headless snapshots/pixel comparisons covered edges, ISS, six-digit IDs and zoom overrides; baseline unchanged and hiding removes only label ink. No agent interactive/hardware/performance validation, new dependencies, core/fixture/lockfile changes or frozen-M2 renewal; new zoom-behavior visual review remains pending.

## 2026-10-17 UTC (glyph wrapper direction and label-review acceptance)
- User confirmed acceptance of existing 70°/60° label behavior; recorded [0036](decisions/0036-shared-glyph-capsule.md)'s shared rounded capsule, 1px outline and one central break per long side. Glyph mappings/transitions remain unimplemented and unscoped.
- Retained `docs/mockups/0036-glyph-capsule.png` with DESIGN provenance; updated PLAN/HANDOFF and decision cross-references. Circular/info-brick, generative-field and real-data population-graph mocks stayed exploratory, not a replacement layout or approved history policy.
- Earlier session headless replays and a temporary exporter supported mock comparisons; `cargo check --workspace` passed. Wrap-up checked Markdown links/anchors, image mode/dimensions/radar provenance, instruction consistency and final/staged whitespace; no Rust checks/tests rerun for the docs/image-only diff.
- No application code, dependencies, lockfiles, source fixtures, firmware/toolchain or frozen-M2 changes; no agent interactive, physical-display or performance validation. Next: agree a bounded first glyph contract, not implement the entire stack by inference.

## 2026-10-17 UTC (static elevation glyph and visual acceptance)
- Implemented [0037](decisions/0037-static-elevation-glyph-contract.md) in `render/src/{glyphs,lib}.rs`: true-elevation fill, shortened capsule, independent warning area and absent-selection clearing. User reviewed and accepted the snapshot output; radar/arrows, selection, label hysteresis and square diagnostics remain unchanged.
- Added renderer pixel/lifecycle coverage and independent real-data glyph checks in `sim/tests/snapshot.rs`; updated simulator usage and current docs. Renderer directly uses already-locked `libm`; no new package/version, app/core runtime, fixture, firmware or frozen-M2 changes.
- Wrap-up reran workspace check/tests, fmt, app/render no-default-feature checks and strict all-target app/render/sim Clippy successfully; checked local Markdown links/anchors, instruction consistency and final/staged whitespace. Earlier headless comparisons at four zooms preserved square PNG bytes, wide radar pixels and physical/count reports.
- No interactive, physical-display or performance validation. Next: agree another bounded glyph contract (altitude recommended, not approved); other mappings, full-stack layout and transitions remain open. No new slice started or production policy selected.

## 2026-10-17 UTC (labelled borderless glyph direction)
- Accepted [0038](decisions/0038-labelled-borderless-glyph-direction.md): labelled borderless circular-data-glyph stack, padded separator, equal 10px outer padding, thin navigation cross and simple station body/wings (B); 0037 remains the executable contract.
- Retained navigation/station references in `docs/mockups/0038-labelled-glyph-*.png`; updated DESIGN/PLAN/HANDOFF and earlier decision cross-references. Synthetic glyph states are not readings or classifications of the replay's selected satellite.
- Wrap-up checked local links/paths/anchors, instruction consistency, final/staged whitespace, 1-bit PNG dimensions/padding, unchanged radar pixels and station-only substitution. Rust checks were not rerun because only docs/static images changed; no interactive, hardware, toolchain or frozen-M2 work.
- Next recommendation: review explicit warning placement displaced by the full-height stack before bounding implementation. Scales, speed/period semantics, category rules, bitmap geometry and lifecycle remain open; no new implementation slice started.

## 2026-10-17 UTC (bottom-left condition-driven notice overlay)
- Accepted [0039](decisions/0039-bottom-left-condition-driven-notices.md)'s bottom-left placement/lifecycle after mock review, then [0040](decisions/0040-white-backed-bottom-left-notice-contract.md)'s small white backing that may cover crossing graphics. Retained illustrative mocks; executable visual acceptance remains pending.
- Implemented notice-only relocation in `render/src/lib.rs`, with opaque bounds, recovery, real selected-marker/ID overlap and CLI warning tests in `sim/tests/snapshot.rs`; capsule, square diagnostics, physical state and selection remain unchanged. Updated current docs and added a self-contained [snapshot review recipe](../sim/README.md#notice-overlay-review).
- Wrap-up reran workspace check/tests, fmt, app/render no-default-feature checks and strict all-target app/render/sim Clippy successfully; verified the recipe reproduces all four earlier warning/crossing PNGs byte-for-byte, local links/anchors, retained image formats and whitespace. Earlier healthy square/wide snapshots were unchanged before/after.
- No dependencies, core/app runtime, fixtures, lockfiles, firmware/toolchain or frozen-M2 changes; no interactive, physical-display or performance validation. Next: user review of executable notices, then one bounded glyph contract; full-stack mappings/lifecycle remain open.

## 2026-10-17 UTC (circular elevation glyph and executable acceptance)
- Accepted and implemented [0041](decisions/0041-circular-elevation-only-contract.md) in `render/src/{glyphs,lib}.rs`: circular cutout, `Elev` label and retained separator without selection; user accepted executable output and confirmed prior notice acceptance. Other glyphs remain absent.
- Added exact raster/lifecycle and independent real-data coverage in renderer tests and `sim/tests/snapshot.rs`; retained illustrative references and updated simulator usage, DESIGN/PLAN/HANDOFF and decision cross-references. Earlier 12-case comparisons preserved square PNG bytes, wide radar/notices and count/diagnostic reports.
- Wrap-up freshly reran workspace check/tests, fmt, app/render no-default-feature checks and strict all-target app/render/sim Clippy successfully; checked local links/anchors, retained 1-bit images and final/staged whitespace. No dependencies, core/app runtime, fixtures, lockfiles, firmware/toolchain or frozen-M2 changes; no interactive/display/performance validation.
- Next recommendation: agree a bounded static concentric-circle altitude contract, not implement it yet. Its scale/raster/lifecycle remain open; earlier diamond/bar mapping is not approved for circles. No new implementation slice started.

## 2026-10-17 UTC (static altitude glyph and executable acceptance)
- Accepted and implemented [0042](decisions/0042-static-altitude-glyph-contract.md) in `render/src/{glyphs,lib}.rs`: fixed compressed true-altitude scale, concentric outlines, `Alt` label and immediate two-field lifecycle. User separately accepted the fresh executable output; elevation, notices, radar and square diagnostics remain unchanged.
- Added exact mapping/raster/lifecycle and independent real-data coverage in renderer tests and `sim/tests/snapshot.rs`; retained illustrative references and updated simulator recipes, DESIGN/PLAN/HANDOFF and decision cross-references. Earlier 12-case comparisons preserved square PNG bytes, wide pixels outside altitude and count/diagnostic reports.
- Wrap-up freshly reran workspace check/tests, fmt, app/render no-default-feature checks and strict all-target app/render/sim Clippy successfully; checked local links/anchors, retained 1-bit images and final/staged whitespace. No dependencies, core/app runtime, fixtures, lockfiles, firmware/toolchain or frozen-M2 changes; no interactive/display/performance validation.
- Next recommendation: propose a bounded static speed-arc contract, settling reference frame/input, scale/raster and lifecycle before coding. No speed metric or next implementation is authorized; category, period and full-stack semantics remain open.
