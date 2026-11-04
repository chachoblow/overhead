# Knob zooms the device radar continuously
Status: active
Started: 2026-11-03. Scope: PLAN M6 "Run shared engine/renderer on S3 + Sharp" (knob zoom remains); decisions [0060](../decisions/0060-defer-timing-to-physical-prototype.md), [0070](../decisions/0070-boot-input-capture-independent-of-render.md), [0076](../decisions/0076-device-app-build.md)

## Outcome
Turning the physical encoder on the device app zooms the radar continuously,
one zoom step per detent, without dropped or phantom steps during render/SPI.

## Done when
- UART diagnostic shows the wiring works and counts-per-detent is measured.
- Device app zooms on the board; user judges feel (0060 leaves this to them).
- Host tests for detent→zoom mapping; firmware builds pass the image gate.

## Boundaries
No toolchain change. BOOT cycling behaviour unchanged. Zoom presets only if the
knob proves infeasible (0060).

## Human gates
- Physical wiring: done 2026-11-03 by the user (A→IO4, C→GND, B→IO5, SW→IO6/GND, direct, internal pull-ups, no caps). Part unknown; looks like a 24-detent PEC11-style 5-pin encoder.
- Turning the knob for the diagnostic: user action.
- Zoom feel acceptance: user judgment.

## Plan
- [x] `knob-test` diagnostic: PCNT unit quadrature on IO4/IO5, glitch filter, IO6 switch; logs counts, A/B/SW levels over UART. Flash, user turns knob.
- [x] Measure counts per detent and direction. Zoom-per-detent mapping: ~1.095×/detent per HANDOFF (default).
- [ ] Wire PCNT into device runtime; map detents to app lens zoom.
- [ ] Board check + user feel pass.

## Discoveries and assumptions
- Port on devbox: /dev/ttyUSB0.
- Encoder: 4 PCNT counts per detent; detent rest is A=1,B=1, so detents = total/4.
  Clockwise (as the user turns it) counts negative. Default: clockwise = zoom in.
- Direct wiring (internal pull-ups, 1023-cycle PCNT filter) is clean: slow turns
  give exactly ±1 per edge; a fast back-and-forth spin ended at rest on a detent
  with total a multiple of 4 (−44), so no edges lost. No caps needed so far.
- Hardware counting means render/SPI stalls cannot drop steps (0070's concern)
  as long as the i16 counter is read before it overflows.

## Verification
- 2026-11-03 `knob-test` v1 release build, image gate pass, flashed /dev/ttyUSB0;
  2-min UART capture (`firmware/target/knob-test-v1.log`, ignored): 10 CW clicks → −40,
  10 CCW → 0, 3 presses → presses=3, fast spin → −44 at rest. Idle a=b=sw=1.

## Open questions
