# Detection-interval evaluation — 2026-10-09

## Scope and reproduction
Same-model detection/integration evidence for M2, **not independent pass-time
accuracy, a completeness proof, a frequency-of-misses estimate, or an operating
budget**. No default interval is selected. Physical contracts remain 0012–0015.

Run the fixed offline [host experiment](../../tools/README.md#overhead-evaluate-intervals):

```sh
cargo run --release -p overhead-tools --bin overhead-evaluate-intervals > /tmp/overhead-intervals.json
```

Inputs/provenance: [fixture README](../../tools/fixtures/README.md).
The recorded run used macOS arm64, rustc 1.99.0 (b940084d7), workspace Cargo.lock;
two release executions produced byte-identical JSON on that host. Numeric output
is not a cross-platform bitwise contract. Tests check structure and bounds.

## Method
- Four historical orbits: ISS LEO, resonant Molniya HEO, GPS GNSS, inclined GEO.
  Each uses Colorado, Sydney, and equator 80°E observers, a 10° threshold,
  and epoch −12h to +12h (including negative propagation times).
- Three constructed 600-second stress windows: a roughly 4-second ISS excursion,
  a roughly 4-second GEO gap, and an above-peak no-pass control. Find an interior
  local extremum on a 60-second grid, narrow its neighborhood to 2 ms, and use
  the mean elevation at extremum ±2 seconds as threshold. The no-pass control
  uses peak elevation +1e-7 rad; this is not exact tangency. Inputs/thresholds
  are emitted in JSON. These cases demonstrate limits, not occurrence rates.
- Sweep detection intervals 1, 5, 10, 30, 60, 120 seconds, separately at fixed
  250 ms and 5 s crossing tolerances. Shift start by floor(interval × q/4) whole
  seconds for q=0..3, deduplicating; keep end fixed and compare the exact shortened
  window. The 1-second interval has only phase zero; others have four phases.
  Total: 15 cases × 42 searches = 630 searches, not exhaustive phase coverage.
- Reference: direct orbital elevation samples, no search kernel or refinement.
  Ordinary cases compare 500 ms and 1 s grids; stress cases compare 10 ms and
  20 ms grids. Both grids share samples/model. Crossing structure must agree
  before evaluating candidates; agreement cannot rule out still-shorter events.
- Associate crossings only by direction and mutually unique bracket overlap.
  Report unmatched reference/candidate crossings and ambiguous associations
  separately. Missing both boundaries of an internal excursion/gap is counted
  separately; window-clipped events have no inferred duration or lost pair.
  Equality follows 0014. Duration bounds come from reference brackets.
- All candidate searches must complete under a deliberately ample 200,000-call
  guard. Errors/incomplete work fail the experiment rather than masquerading as
  no-pass. Counts below are evaluations, not wall-time, memory, or scheduling.

## Observed results
The 12 ordinary cases contain 39 reference crossings. All sampled candidate
intervals/phases/tolerances matched them; no unmatched or ambiguous crossings.
GEO coverage includes all-below, all-above, and a window-clipped crossing case.
The shortest ordinary excursion was ISS/Colorado: 104–105 seconds. Even the
120-second grid happened to detect it in these four phases; this does **not**
establish reliable detection for intervals shorter than the sample spacing.

Both reference grid pairs agreed for every case. All detected candidate brackets
met their configured tolerance. Tightening tolerance did not alter detected
crossing counts in this suite.

| Interval | Ordinary evaluations/search, 250 ms tolerance | At 5 s tolerance | Phases missing each constructed short event (of phases tested) |
|---|---:|---:|---:|
| 1 s | 86,401–86,429 | 86,401 | 0/1 |
| 5 s | 17,281–17,351 | 17,281 | 1/4 |
| 10 s | 8,641–8,725 | 8,641–8,655 | 2/4 |
| 30 s | 2,881–2,979 | 2,881–2,923 | 3/4 |
| 60 s | 1,441–1,553 | 1,441–1,497 | 3/4 |
| 120 s | 721–847 | 721–791 | 3/4 |

The short-event miss counts are identical at both tolerances, but are **not
probabilities**. The deliberately centered stress cases share sampling alignment.
ISS duration is bracketed at 3.99–4.01 s near a 62.052024° threshold; the GEO gap
at 4.00–4.02 s near 4.008137°. Missed ISS excursions produce no pass record;
missed GEO gaps merge two above-threshold intervals into one window-spanning
record. Detected short events remain present even with 5-second tolerance.
The above-peak control produced no passes at any setting.

## Interpretation and next work
This confirms why detection spacing and crossing tolerance must remain separate:
refining detected transitions cannot recover an excursion/gap with no regular
sample on its opposite side. Finer sampling costs substantially more evaluations.
There is no demonstrated product need or device-cost evidence here to choose a
production interval, accepted miss rate, catalogue size, or numeric allowance.

Next measure propagation and prediction costs by orbit class, then on confirmed
ESP32-S3 hardware, and choose supported capacity/cadence, scheduling and explicit
over-budget behavior. Retain these sweeps as regression evidence. Broader epochs,
observers, natural near-10° grazes, and phase coverage would strengthen selection
of an interval. Independent resonant/GEO/negative-time geometry and crossing-time
references remain follow-ups; the existing Skyfield point samples do not make
this experiment an independent timing oracle or validate real-world orbit error.
