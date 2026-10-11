# 0020 — M2 capture manifest freeze

Date: 2026-10-11
Status: accepted

## Decision
Freeze `m2-bounded-20-v1`: all 20 existing candidate cases, their exact host annex,
input lifetimes/retained diagnostic representations, and the independent numerical
selection under the [frozen preparation annex](../evaluations/m2-frozen-manifest.md).
This completes 0019's preparation gate; target implementation/capture comes next.

## Why
The candidate populations, density sites, collection transitions, error/partial
outcomes and RAM-input overlap already pass host preflight. No unanswered question
justifies adding/replacing workload cases. The remaining independent references
are now pinned from original published TEME outputs and all existing coordinate/
observer fixtures, with unchanged tolerances and one allocation-free host/target
checker. The target representation review preserves what the host actually measured
rather than silently substituting compact diagnostics or dropping metadata.

## Consequences
- The machine-readable manifest pins exact source/input/annex/checker hashes and
  portable expected outcomes; host requested bytes/capacities remain evidence,
  not target occupancy expectations. Changes require reviewed renewed preflight.
- Preserve fixed 64 KiB heap, original cohorts/UTCs, ceilings, capture/reset/sample
  rules and separate numerical/stack gates from 0019. No new benchmark dimension,
  production limit/default, allocator strategy or scheduling architecture.
- Prepared portable numerical code is not an expanded capture harness or evidence
  of target accuracy. No target images were built or hardware accessed for freeze.
- Next implement/capture the frozen suite, gate exact normal images and ask before
  flashing. Then settle the initial operating budget and separately verify bounded
  scheduling latency. No more matrix without a specific unanswered decision.
