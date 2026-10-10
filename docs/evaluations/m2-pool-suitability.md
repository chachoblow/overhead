# M2 pinned-pool source suitability

2026-10-10 UTC. Initial host-only preparation under
[0019](../decisions/0019-bounded-m2-evidence-preparation.md) and the
[bounded contract](m2-next-evidence.md). **Not the full host-preflight annex or a
capture-ready manifest.** No target harness, firmware image, hardware capture,
dependency/toolchain change, production defaults or capacity claim.

## Inputs and method

The [fixture README](../../tools/fixtures/m2-pool/README.md) owns source URLs,
retrieval times, hashes, identities/epochs, selection rules and regeneration.
The separate 16-object pool has eight LEOs, two HEOs, two GNSS and four GEOs.
Its fixed UTC is `2026-10-10T00:00:00Z`; signed element ages range from
−7.5018192 to +38.44587312 hours, satisfying the absolute 48-hour bound.
Historical controls and their UTC remain unchanged.

The [offline tool](../../tools/README.md#overhead-m2-pool-preflight) performs checked
OMM ingestion, core initialization, identity/order/epoch checks and rejection-free
catalogue construction. To establish actual path coverage, it serializes each
initialized `sgp4::Constants` with the already-enabled sgp4 2.4 Serde support and
inspects the selected method/resonance variant. It cross-checks resonance against
`initial_state()`. This is a version-bound host diagnostic, not a new core API or
source-label inference. Unknown representations fail closed; review it on sgp4
upgrades. All eight LEOs are near-Earth, both HEOs half-day resonant, both GNSS
nonresonant deep-space, and all four GEOs one-day resonant.

Searches use the contract's 24h window, 10° threshold, 60s detection, 5s crossing
tolerance, 200,000 per-satellite evaluations and 200,000 × population shared
allowance. The tool runs four Colorado growth searches and all 15 grid sites for
each ten-object density population. These **34 suitability searches are not 34
memory/work capture cases**. Ingestion follows prescribed population order;
search traversal is ascending NORAD ID. Every report/pass is compared to a direct
streaming search of the same model. No failures/rejections/incomplete results are
accepted or used to silently substitute another source object.

## Results

[m2-pool-suitability.json](m2-pool-suitability.json) publishes the initialized paths,
epoch ages, all 30 density candidates, growth counts, per-satellite work/pass
counts, selected indices and densest-case identity. All 34 searches completed.
Each population currently uses one raw-object JSON array/group; record counts
include all input records (there are no duplicates/rejections in this substep).
Subset bytes use the fixture's LF array/object separators. Exact group metadata,
input hashes for every final case and storage lifetimes still await full preflight.

| Candidate case | Population | Site (latitude°, longitude°, altitude km) | Input bytes | Stored passes | Evaluations |
|---:|---|---|---:|---:|---:|
| 03 | mixed 5 | 39.007, −104.883, 2.187 | 2,111 | 16 | 7,321 |
| 04 | mixed 9 | same Colorado site | 3,784 | 27 | 13,165 |
| 05 | mixed 12 | same Colorado site | 5,036 | 33 | 17,536 |
| 06 | mixed 16 | same Colorado site | 6,672 | 44 | 23,380 |
| 07 | LEO-heavy high | −60, 120, 0 | 4,212 | 68 | 14,946 |
| 08 | LEO-heavy low | 0, 0, 0 | 4,212 | 26 | 14,618 |
| 09 | deep-space-heavy high | −60, −120, 0 | 4,152 | 28 | 14,626 |
| 10 | deep-space-heavy low | 0, 0, 0 | 4,152 | 11 | 14,474 |

High/low sites follow the contract's pass-count and ascending-coordinate
tie-breaks, with the high site excluded when selecting low. **Case 07 is densest**
among 03–10 and is therefore the current base for cases 11/12 and the expanded RAM
variants. It is not a demonstrated maximum-heap or worst-compute case. No selected
population exceeds 16 satellites, 32 input records, four groups or 32 KiB JSON.

The snapshot's weather-derived LEOs are predominantly polar; results do not cover
all inclinations or all possible pass densities. Complete search means completion
of the configured sampling procedure, not detection of every physical pass.
Work agreement is same-model evidence, not an independent numerical reference.

## Reproduction and remaining preparation gate

Evidence generated with `cargo run --release --locked`, rustc
`1.99.0 (b940084d7 2026-09-28)`, Darwin arm64, on base revision
`471ef79aba5ea26ff8b02eedaca781f636efe1a0` plus this preparation change.
Cargo.lock SHA-256:
`58f907720e3e528a3881fb3c884150111e41e843be6ef309123af44150beaa47`.
No elapsed-time or allocation measurements are reported. Debug tests and release
regeneration match the published report. Commands and integrity tests live in the
[fixture README](../../tools/fixtures/m2-pool/README.md#offline-regeneration-and-checks).

Before manifest freeze/target implementation, still required:

- Preflight historical controls, partial/zero budgets, all ingestion variants and
  exact retained parse/core diagnostics, including whole-document failure.
- Observe collection capacities/allocation changes and explain each growth sample;
  review redundancy rather than assuming these sizes cross meaningful boundaries.
- Measure host input-copy/parse/catalogue/report overlap, dropped/retained RAM
  lifetimes, independent pipeline peaks and full release. Host fit is not S3 fit.
- Pin the 12 original TEME reference vectors and existing independent coordinate/
  observer checks with unchanged tolerances.
- Publish the full ≤20-case annex: exact group metadata, bytes/hashes, accepted
  identities/order, diagnostics, completion/work counts, storage lifetimes and
  numerical expectations. Only then extend/gate target harnesses and request flash
  approval. The fixed 64 KiB heap and separate stack/numerical gates remain binding.
