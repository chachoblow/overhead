# 0001 — Compute positions on-device with SGP4

Date: 2026-10-04
Status: accepted

## Decision
Satellite positions are computed on the device with SGP4 from orbital
elements (CelesTrak, OMM format), fetched a few times a day. No position API.

## Why
- Animation needs positions many times per second; APIs are rate-limited and
  can't deliver that.
- The slow-down feature must predict pass entry/exit before they happen,
  which requires propagation, not polling.
- Elements stay usable for days, so the device works offline.
- OMM over TLE: catalog numbers are outgrowing the TLE format.

## Consequences
- Commits us to running SGP4 on the ESP32-S3, which has single-precision FPU
  only; double precision is software-emulated. The planned benchmark (on the
  old ESP32) sets how many satellites we can track and how often — it sizes
  the design, it doesn't reopen this decision.
- Needs a fetch path (Wi-Fi) and a reasonably accurate clock on the device.
- Rules out the predecessor's position-API approach.
