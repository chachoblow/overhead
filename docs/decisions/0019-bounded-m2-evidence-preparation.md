# 0019 — Bounded M2 evidence preparation

Date: 2026-10-10
Status: accepted

## Decision
Prepare at most 20 expanded catalogue memory/work cases under the
[bounded preparation contract](../evaluations/m2-next-evidence.md), retaining the
historical controls and adding a separately pinned real-orbit acquisition set.
Freeze the exact fixture/host-preflight manifest before extending the target harness.

## Why
The existing eight-object catalogue has only two LEOs and narrow input/diagnostic
coverage. The upstream verification set mixes epochs and intentional error/decay
cases; padding a larger catalogue with those records, repeated identities or
mutated epochs would not establish representative distinct-catalogue costs.
A separate near-common-epoch pool can broaden coverage without rewriting historical
controls or confusing cost inputs with independent numerical references.

Bound population, density, ingestion and input-lifetime variations rather than
forming a Cartesian product. Host-selected sites and observed allocation changes
must justify the cases before target work. An accepted preparation scope is not
a frozen capture manifest: exact input identities/bytes, source suitability,
diagnostics and expected work still need evidence.

## Consequences
- Preserve 0017's existing eight-case experiment and evidence. The new suite keeps
  its fixed 64 KiB internal heap; at most 16 accepted satellites, 4 groups,
  32 records and 32 KiB aggregate input JSON per case are preparation ceilings,
  not implemented or supported device limits.
- Keep historical and new cohort UTCs explicit. Pin source bytes/provenance;
  subsequent builds/tests run offline. Acquisition does not introduce live
  fetching or active-catalogue refresh into M2.
- Require independent target numerical gates, separate two-pattern per-phase
  stack observations, and strict expected-success/failure/release validation.
  Host fit and stack watermarks do not establish target capacity or stack safety.
- Unsuitable fixtures, redundant cases requiring replacement, or failed preflight
  require explicit review before manifest freeze; do not silently widen the scope,
  relax numerical tolerances or enlarge the heap.
- No target implementation/capture is claimed. Normal-image gating under 0018
  and explicit flash approval remain required. No toolchain/dependency change,
  PSRAM, scheduling architecture, production cadence or capacity is selected.
