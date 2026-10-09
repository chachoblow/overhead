# Host propagation/prediction costs — 2026-10-09

## Scope and reproduction
M2 host kernel measurements, **not ESP32-S3 timings, supported catalogue size,
worst-case execution bounds, memory budgets, or operating defaults**. Detection
accuracy remains a separate [evaluation](detection-intervals.md).

Usage and timing/CLI contracts: [overhead-benchmark](../../tools/README.md#overhead-benchmark).
Inputs: [historical fixture provenance](../../tools/fixtures/README.md).
Raw results: [run 1](host-costs-run-1.json), [run 2](host-costs-run-2.json).
These are measured artifacts, not expected timing fixtures or CI performance gates.

Recorded context:
- Apple M1 Pro, aarch64-apple-darwin; macOS 26.6.2 (25G83).
- rustc 1.99.0 (b940084d7 2026-09-28), LLVM 23.1.1;
  cargo 1.99.0 (5f94df478 2026-08-27).
- Base revision `9c87b3e` plus this session's benchmark changes; unchanged
  workspace Cargo.lock. Default Cargo release profile, debug assertions off.
- Workspace `.cargo/config.toml` adds `-L /opt/homebrew/lib`; no environment
  RUSTFLAGS/CARGO_ENCODED_RUSTFLAGS/CARGO_BUILD_TARGET overrides were present.
- Two sequential invocations on 2026-10-09 UTC; no CPU pinning, frequency control,
  or background-load isolation. Builds/tests were not run concurrently with them.

```sh
cargo build --release -p overhead-tools --bin overhead-benchmark
# Run twice, saving each output separately:
target/release/overhead-benchmark --samples 5 --min-sample-ms 20 \
  --label 'Apple M1 Pro; macOS 26.6.2 (25G83); rustc 1.99.0 b940084d7; default release; base 9c87b3e + benchmark changes' \
  > /tmp/overhead-benchmark.json
```

## Workload
One historical orbit per class; observer Colorado (39.007°, −104.883°, 2.187 km
WGS-84 height), pass threshold 10°. Every fixture starts at its own epoch −12h;
these different historical epochs do **not** form a common-UTC catalogue.

- Eight tracking rows: `state_at` alone and `state_at` → ECEF → look angles for
  each class, at 1,441 precomputed times spaced 60 seconds through epoch +12h.
  This includes negative/positive propagation times, not only the epoch fast path.
- Forty-eight prediction rows: each class × 1h/24h windows × 5/30/60s detection
  spacing × 250ms/5s crossing tolerance. Configurations are prepared outside timing;
  each invocation creates fresh budgets and consumes/counts streamed pass records.
- Six scaling rows: 4/16/64 repeated fixture slots, equal shares of the four
  classes, for full tracking and 24h prediction at 60s detection / 5s tolerance.
  Tracking iterates times then slots; prediction iterates slots synchronously.
  Repeated references do not model a larger working set or catalogue allocations.

The harness warms up/calibrates each row, measures five batches, and normalizes
wall time by invocations per batch. Work counts matched across both runs. All
searches completed under the experimental guard; no incomplete work was timed as
success. Kernel and harness overhead are included, not subtracted. Geometry here
means ECEF rotation plus observer look angles, **not** satellite geodetic altitude,
a complete display update, or a full tracker application.

## Observed results
Below are the ranges between the **two run medians**, not confidence intervals,
sample extrema, or worst-case latency. Raw batch samples remain in the JSON.

### Tracking (nanoseconds per state evaluation)

| Class | `state_at` | Propagation + ECEF + look angles |
|---|---:|---:|
| ISS LEO | 264–283 | 454–456 |
| Resonant HEO | 435–460 | 600–643 |
| GEO | 330–334 | 496–498 |
| GNSS | 282–285 | 458–459 |

Divide a row's median ns/invocation by `state_evaluations` to reproduce these.
Each number is an average over its timestamp grid, not a bound on one call.

### 24-hour prediction (milliseconds per single-fixture search, 5s tolerance)

| Class | 5s detection | 30s detection | 60s detection | Evaluations at 60s |
|---|---:|---:|---:|---:|
| ISS LEO | 7.31–7.69 | 1.34–1.38 | 0.711–0.756 | 1,497 |
| Resonant HEO | 10.36–10.56 | 1.74–1.76 | 0.888–0.896 | 1,449 |
| GEO | 8.78–9.08 | 1.46–1.47 | 0.740–0.740 | 1,441 |
| GNSS | 8.13–8.15 | 1.37–1.37 | 0.701–0.706 | 1,449 |

The 5s grid uses 17,281 evaluations per 24h search at 5s tolerance, with no
bisection required. Detected records are 7/1/0/1 for LEO/HEO/GEO/GNSS in this
observer/window at all three intervals. This is not event-completeness evidence.
The 1h windows and tighter tolerance rows are retained in the raw artifacts.

### Mixed repeated-fixture scaling

| Slots | Full tracking, µs per tick (amortized) | 24h search batch, ms | Search evaluations |
|---|---:|---:|---:|
| 4 | 2.00–2.01 | 3.03–3.06 | 5,836 |
| 16 | 7.80–7.88 | 12.12–12.28 | 23,344 |
| 64 | 30.86–31.01 | 48.62–48.63 | 93,376 |

Tracking per tick = median ns/invocation ÷ 1,441 ÷ 1,000; search batch ms = median
ns/invocation ÷ 1,000,000. These cases show approximately linear kernel work.
They do not establish a 64-satellite product capacity or a safe display cadence.
Noise is visible: run 1's HEO full-tracking batches span about 0.89–2.19ms per
invocation despite a 0.93ms median. Five batches cannot establish tail latency.

## Implications and next work
The host baseline distinguishes orbit-class cost and makes the much greater cost
of dense prediction explicit. It does not justify choosing a coarser detection
interval: [short-event misses](detection-intervals.md) are independent of crossing
refinement, and this benchmark does not quantify acceptable miss frequency.

Confirm available ESP32 board/display and permission before any flashing or
toolchain changes. Port equivalent workloads to the S3, recording board/CPU clock,
compiler/profile/features, memory placement, math implementation, and timer method.
Include positive/negative and older element ages: a ±12h window does not bound
resonant propagation costs farther from epoch. Broaden orbit samples and measure
real distinct-catalogue storage/aggregation, initialization, and peak memory.

Then choose catalogue limits, tracking and prediction cadence, time-sliced work,
and explicit over-budget/partial-result behavior with headroom for display and
network activity. The synchronous kernel's evaluation allowances alone are not
scheduling or wall-time bounds. M2 remains open until those choices are justified
by target-device measurements; no firmware was created or hardware operated here.
