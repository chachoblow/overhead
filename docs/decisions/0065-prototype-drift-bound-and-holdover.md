# 0065 — Prototype drift bound and holdover cap

Date: 2026-10-26
Status: accepted by the user, 2026-10-26. Refines
[0063](0063-bounded-utc-holdover-direction.md) and
[0064](0064-prototype-sntp-screen-and-leap-boundary.md).

## Decision

For the first prototype, the UTC trust gate uses:

- **`D = 20 ppm`**: the S3 monotonic-timer frequency-error bound in the budget
  `U0 + ceil(age × D) ≤ 100 ms`.
- **`H = 3,600 s`**: an absolute holdover cap; trust is absent at age `H`.

The gate continues to end trust at whichever comes first: budget, `H`, a UTC
month boundary (0064), or a fault. With a typical `U0 ≈ 30 ms` the budget ends
trust after ~3,500 s; `H` binds only for unusually accurate syncs.
`UtcTrustPreflight::prototype()` encodes these values.

## Why

A 3-hour UART capture measured the board's timer at ≈+1.2 ppm versus UTC
([evidence](../../firmware/README.md#s3-clock-drift-capture-flashed-3-hour-uart-run)),
resolved only to a few ppm because the host reference wandered. 20 ppm adds
margin for temperature, voltage, aging and that reference error, and is twice
Espressif's ±10 ppm crystal recommendation. An hour comfortably exceeds a
typical outdoor session between Wi-Fi resyncs while keeping the ≤100 ms target.

## Consequences

One board, room temperature, three hours: `D` is a margin-bearing prototype
choice, not a verified bound. Revisit after cold/hot or multi-board evidence, or
if a live adapter shows typical `U0` well away from ~30 ms. No live SNTP client,
dependency, source selection or production policy follows from this decision.
