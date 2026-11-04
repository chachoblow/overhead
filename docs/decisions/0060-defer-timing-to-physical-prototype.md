# 0060 — Accept coastline zoom cost for now; defer timing work to the physical prototype

Date: 2026-10-24
Status: accepted direction

## Decision

- The [measured](../evaluations/m4-coastline-prepared-target.md) prepared/cached
  coastline is accepted for now, including zoom steps of up to ~21 ms on the S3.
  The user will judge zoom feel on the physical device rather than set a
  millisecond ceiling in advance.
- Further frame-timing work — including the non-coast frame cost (18–20 ms
  ordinarily, 137 ms in the selected ISS scene) — is deferred until a working
  physical prototype exists, then handled with real-device measurements.
- **Control intent:** a button cycles through satellites and a knob zooms
  continuously. The user is open to adjusting (for example zoom presets) if the
  knob proves infeasible on the device.

## Why

Milliseconds are not a meaningful acceptance unit before the device can be held
and used. A 21 ms step is under half a 20 Hz frame, on only the frame where the
zoom changes. Optimizing in isolation risked more pre-testing without product
feedback.

## Consequences

- Continuous knob zoom re-rasters the coastline on every frame while turning;
  whether that fits the 50 ms frame is a prototype-stage question.
- This amends the [DESIGN controls](../DESIGN.md#controls) intent. The first
  firmware prototype may still start with simpler BOOT-button control; that is
  an implementation step, not a change of intent. 0004's compile-time location
  is unchanged.
- No performance ceiling is relaxed or adopted; the provisional screens remain
  measurement references, not shipping gates.

## Note — 2026-11-03: knob implemented and accepted

The knob is feasible; presets are not needed. A 24-detent PEC11-style encoder
(part number unknown) is wired directly with internal pull-ups and no
capacitors: A→IO4, C→GND, B→IO5, switch→IO6 (unused). The PCNT peripheral
decodes quadrature in hardware (4 counts per detent, max glitch filter), so
render/SPI stalls cannot drop steps. One detent is half a 1.2× zoom step
(√1.2 ≈ 1.095×, `RadarApp::zoom_detents`); clockwise zooms in. The user judged
the feel good on the board. [Evidence](../../firmware/README.md#rotary-encoder-wiring-check-knob-test).
