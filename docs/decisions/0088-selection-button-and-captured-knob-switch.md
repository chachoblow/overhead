# 0088 — Selection button on IO7; knob switch captured like it

Date: 2026-11-14
Status: accepted (user, 2026-11-13: separate button; the fix belongs in the
M6 closeout). Implemented in the device runtime (`firmware/src/live_radar.rs`,
`boot_capture.rs`, `button_input.rs`); selection and info page shown on the
board 2026-11-14
([evidence](../../firmware/README.md#selection-button-and-captured-knob-switch-0088)).

## Problem

The device app cycled satellites with BOOT (GPIO0), a DevKitC strap button.
Gift units will not expose BOOT without taking them apart; they will have a
button and a knob. The knob's push switch (IO6) was read once per display
loop, so a press during a fetch's TLS stall (~450–485 ms with no loop) could
be missed; selection presses were already interrupt-captured under
[0070](0070-boot-input-capture-independent-of-render.md).

## Decision

- **Inputs:** a separate momentary button, active low on **IO7** with the
  internal pull-up, cycles satellites under
  [0068](0068-boot-selection-gesture.md)'s rules. The knob switch keeps
  [0087](0087-ios-setup-app-over-ble.md)'s roles (setup hold and factory reset
  at boot, a short press toggles the info page); the knob zooms under
  [0060](0060-defer-timing-to-physical-prototype.md). The device app no
  longer reads GPIO0; the historical BOOT diagnostics still do.
- **Capture:** both inputs are sampled by 0070's 5 ms TIMG1 interrupt into
  separate latches and drained at the display loop boundary. A press during a
  stall that leaves a valid frame is applied when the loop resumes (late, not
  lost); 0068's no-queue rule still applies only to presses drained without a
  valid frame. Each drained info press toggles the page, so late presses end
  in the state the user pressed for. A setup hold still down when sampling
  starts never counts.
- **Log:** `SELECT` and `INFO_PAGE` are event-log lines with `loop_gap_us`
  and `fetching`, so a press applied after a stall is visible.

## Why

DESIGN already intended "a button cycles satellites, a knob zooms"; giving
the knob push a second, timed gesture (long press for the info page) was the
rejected alternative: no new hardware, but a slower info page and a gesture
recipients must learn. IO7 is not a strap, flash/PSRAM or USB pin on the
ESP32-S3-DevKitC-1-N8R8.

## Consequences

Every unit needs the button wired IO7→GND (a four-leg tactile button across
diagonally opposite legs; same-side legs are always connected). Release 15
and earlier still select with BOOT; a board on them ignores IO7 until it
installs a release with this change. The ~34–41 ms sampling gap during Wi-Fi
association (0070) applies to both inputs; with 20 ms qualification a press
shorter than about 60 ms can still be missed then.
