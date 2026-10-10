# M2 next evidence — bounded proposal

Proposed 2026-10-10 UTC; **not an accepted measurement contract, implemented suite,
or production policy**. Refine before implementation. Existing decisions 0011–0017
remain authoritative; [PLAN](../PLAN.md) owns milestone status.

The goal is to decide an initial supported catalogue and prediction operating
budget, not benchmark an unbounded set of combinations. Keep hardware unchanged:
internal RAM, no PSRAM/display/network, installed toolchain, explicit flash approval.
First prepare the [DROM remedy/image gate](s3-drom-diagnostic.md); do not broaden
captures while treating the mapping diagnostic as unexplained or silently waived.

## Proposed ceiling: 20 memory/work cases, not a Cartesian product

Retain the existing search settings (10°, 60s detection, 5s tolerance, common UTC)
as experimental controls, not defaults. New orbital fixtures require pinned source
provenance, distinct identities, near-common epochs, and successful host preflight;
do not rename or epoch-shift repeated orbits to manufacture catalogue capacity.

| Cases | Variation | Decision informed |
|---:|---|---|
| 2 | Existing eight-object reference, 1h and 24h, flash inputs | Bridge to prior costs and short/long lookahead |
| 4 | Mixed distinct populations of 5, 9, 12 and 16; 24h | Vec/tree growth and supported-size envelope |
| 4 | LEO-heavy and deep-space-heavy populations, each at two explicit sites; at most 16 objects, 24h | Pass/result density versus propagation-path cost |
| 2 | Densest selected case, shared allowances 1,000 and zero | Partial/unsearched reporting and retained storage |
| 4 | Equivalent duplicates across groups; newest-epoch conflict with survivors; invalid orbital records with survivors; malformed document rejection | Input/diagnostic/group-growth costs and load-failure semantics |
| 4 | Reference and densest case, RAM input dropped after initialization versus retained through aggregation | Input lifetime contribution to simultaneous heap peak |

Select the two density sites on host from a small declared grid; publish candidates,
selection criterion, measured pass counts and chosen coordinates before target
capture. Do not assume labels like “LEO-heavy” guarantee high output density.
Freeze exact population sizes and input record/group/byte caps in the preparation
contract. Synthetic conflict/invalid variants must be labeled error cases, never
independent orbital evidence. Malformed-document rejection is an expected outcome
only in that named case; panic/OOM or unexpected work remains an invalid run.

Instrument actual capacity/allocation changes on host rather than assuming each
chosen size crosses a particular implementation boundary. If a proposed sample
adds no distinct evidence, replace it before freezing the manifest; do not grow
past 20 cases by adding every combination. Keep the existing eight-case capture
as historical evidence, not extra repetitions in every expanded run.

RAM cases must measure input allocation/copy before parsing and its overlap with
catalogue/results, not allocate the input before resetting peak counters. Report
phase timings and pipeline-wide peak separately. Retain diagnostics through the
measured endpoint and verify complete destruction returns to baseline. Use host
preflight to avoid knowingly overflowing the fixed target heap, but never treat
host fit as proof of target fit or silently enlarge the heap to make cases pass.

## Correctness and stack gates — separate from timed cases

- Port a bounded set of independent reference checks to target: published SGP4
  vectors spanning near-Earth/resonant/nonresonant paths, plus existing independent
  coordinate and observer fixtures. Record provenance and tolerances before target
  execution. Host/target work-count agreement is still only same-model evidence.
  Reuse existing numerical contracts where applicable; don't loosen tolerances
  merely to make the S3 pass. Pass-detection evidence remains a separate gate.
- Add per-phase written-stack observations and two paint patterns in separate
  diagnostic runs. Inspect compiled frame reservations/call paths, especially
  parsing and prediction, to identify unwritten-frame blind spots. Unresolved
  indirect calls/interrupt paths must be documented; a watermark is never a safe
  stack allocation or a proven worst-case bound.
- Extend the strict manifest/capture validation to exact identities, provenance,
  expected diagnostics/failures, work counts, input lifetimes, and heap release.
  Use one warm-up plus three samples and two reset-separated captures once approved.
  Derive timeout from preflight rather than automatically retaining 900 seconds.

## Stop rule and decisions after capture

No additional matrix unless a specific unanswered decision justifies it. Write an
operating-budget decision separating input bytes/records/groups from accepted
satellites and stored passes/diagnostics. Account for fixed RAM reservations and
future display/application headroom; these experiments do not measure whole-device
peak RAM or network/refresh overlap.

Use the measured short/long prediction costs and detection-interval evidence to
choose initial lookahead, accuracy settings, recomputation cadence and explicit
partial/over-budget behavior. Then scope the smallest scheduling experiment that
can meet display responsiveness: the current synchronous search cannot simply be
called in a 20 Hz loop. If resumable work is chosen, verify bounded slice latency
and unchanged results before declaring M2 scheduling complete. Do not select an
executor, second core, allocator policy, or cache architecture by implication.
