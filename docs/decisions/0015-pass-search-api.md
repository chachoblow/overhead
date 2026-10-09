# 0015 — Allocation-free pass-search API

Date: 2026-10-08
Status: accepted

## Decision
Implement the first search kernel in shared core against a caller-supplied
UTC-to-elevation callback. Stream pass records to a caller-supplied sink and
return a search report; do not require allocation, catalogue storage, or an
orbital adapter. Preserve [0012](0012-physical-pass-semantics.md),
[0013](0013-pass-detection-resolution.md), and
[0014](0014-pass-search-baseline.md).

### Inputs and validation
- Construct a read-only configuration from start UTC, positive look-ahead,
  finite minimum elevation in [-90°, 90°], positive detection interval, and
  positive crossing tolerance. Validate start and checked end against the
  existing UTC contract; use integer-duration arithmetic.
- Keep the accepted 10° / 24-hour / 5-second convenience values. Detection
  remains explicit, with no complete `Default` configuration.
- Detection interval and tolerance are independent. An interval larger than
  the window samples both endpoints; a bracket already within tolerance needs
  no refinement. There is no arbitrary numeric maximum or new duration filter.
- Supply a per-satellite evaluation allowance and a mutable shared total
  budget. Zero allowance is valid and returns an explicitly unsearched report.

### Results and interruptions
- Emit each finished pass and any open pass when the search stops. Start states
  distinguish in-progress, equality-boundary start, and observed upward crossing.
  End states distinguish downward crossing, above threshold beyond a completed
  window, equality at the window boundary, and interruption.
- Crossing brackets retain both endpoints and whether tolerance was met. If
  refinement fails or exhausts its budget, preserve the narrowed bracket already
  established, even when it is still wider than tolerance. Mark the report
  incomplete; do not discard the detected transition.
- Reports distinguish complete, interrupted, and unsearched work, retaining
  evaluation counts, sampling progress, and stop details. Error details identify
  the UTC and sampling/refinement phase. Failed/invalid evaluations count;
  budget-denied calls do not run. Reject nonfinite/out-of-range elevations.
- No-upcoming-pass requires a complete report with no observed upward crossings,
  subject to detection limits. An in-progress or boundary-start pass is not an
  upcoming arrival. Partial records never establish catalogue-wide completeness.

## Why
A callback kernel isolates numerical search from propagation and makes synthetic
edge cases directly testable. Streaming bounds internal storage without imposing
an arbitrary pass capacity. A shared budget composes across satellites without
requiring core to allocate or own the catalogue. Keeping coarse brackets avoids
losing established events when finer timing cannot be obtained.

## Consequences
Public Rust types and the executable test matrix belong in
[`core/src/passes.rs`](../../core/src/passes.rs) and
[`core/tests/passes.rs`](../../core/tests/passes.rs). Cover validation, exact grid
endpoints, ordinary/short/window-spanning passes, equality runs and touches,
detection misses, refinement accuracy, failures, budget accounting, and
repeatability. Evaluate detection intervals separately from tolerance.

The initial kernel is synchronous, not a scheduler. Orbital adapters, catalogue
aggregation/reporting, representative orbital interval evaluation, and measured
operating budgets follow separately. No dependency, hardware, UI, or freshness
policy changes are needed.
