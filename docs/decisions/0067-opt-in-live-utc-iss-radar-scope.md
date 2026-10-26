# 0067 — Opt-in live-UTC ISS radar candidate scope

Date: 2026-10-26
Status: accepted fixture-only diagnostic scope; display-owner implementation
is an experiment, not a standalone-device contract.

## Decision

For the first live-UTC radar integration, use the checked-in single-ISS elements
without network element fetching. Join [0066's screened SNTP adapter](0066-prototype-live-sntp-stack-and-source.md),
[0065's prototype trust gate](0065-prototype-drift-bound-and-holdover.md),
and the existing cached wide render/selective Sharp sender in a **separate
opt-in image**. The display task alone owns the app, gate, framebuffer and SPI;
network tasks pass screened outcomes over a bounded channel so Wi-Fi awaits do
not themselves hold the display owner. Use [0062's `NO FRAME`](0062-no-trusted-time-display-preflight.md)
on cold boot and loss of trust, and the existing *diagnostic* frame/VCOM/full
refresh timings. On a matched KoD, stop querying both servers until reboot in
this experiment rather than bypassing rate/deny through fallback. A failed SPI
transaction halts the diagnostic with panel state unknown, not a recovery claim.

The user approved fixture-only scope before implementation and separately
approved the first exact-gated flash. [Firmware usage and evidence](../../firmware/README.md#live-utc-iss-radar-candidate-flashed-bounded-uart-capture)
record the implementation, capture and limits.

## Why and boundaries

This isolates clock-to-display composition from HTTPS, persistence and element
freshness, while giving a separate display owner a chance to service COM when
network operations await. A channel protects app/SPI ownership and retains
screened events; the gate evaluates each observation at processing uptime so
a delayed delivery cannot renew an expired anchor.

The fixture is historical: live UTC does not make its ISS position accurate.
The physical review reported an empty-sky notice, **not** an ISS marker or
motion. This scope neither proves physical COM edges/blocked-loop service nor
chooses a production cadence, source-age rule, credentials provisioning,
controls, data fetch, SPI-error recovery or device architecture beyond this
image. Those need separate evidence and decisions before standalone-device
claims. The earlier historical replay and `live-sntp` source diagnostic remain
separate images; no frozen M2 evidence is changed.
