# Device inputs a gift unit can use, and M6 closed
Status: active
Started: 2026-11-13. Scope: PLAN M6 (two open boxes); decisions 0068, 0070, 0087, 0060

## Outcome
The device app's controls are the ones a gift unit exposes: a separate
momentary button cycles satellites (instead of the unreachable BOOT/GPIO0
strap) and the knob's push switch keeps setup hold, factory reset and the
info page. Both are sampled by the 5 ms timer interrupt, so a press during a
TLS stall (~0.5 s) is applied late rather than lost. M6's two open boxes are
ticked with evidence, and the accepted input limits are recorded.

## Done when
- Host tests cover the info-press handling (each drained press toggles; a
  setup hold at boot never counts); `cargo check --workspace`, workspace
  tests and firmware host tests pass.
- Device build gated (`image_gate.py`), flashed to the user's board with
  settings kept.
- User tap session on the board: the new button cycles satellites; a knob
  press toggles the info page, including one pressed during a fetch (TLS
  stall) that shows up when the display resumes. UART capture of the run.
- New decision (0088) records the input mapping and late-apply rule; DESIGN,
  PLAN (M6 boxes ticked), firmware README wiring/evidence, HANDOFF updated.

## Boundaries
- No change to setup hold/factory reset timing, knob zoom, BLE, OTA.
- Diagnostics (`sharp_boot_capture_main`, `live_utc_main`) keep GPIO0; they
  are historical evidence images.
- Host-only failure paths (post-GET, repeated handshake, reload after
  expiry, >100 ms SNTP jump) stay documented limits, not proven on board.
- No release built or published here; release 16 is the other-iPhone item.
- No toolchain or bootloader changes.

## Human gates
- Selection control: asked 2026-11-13 / answered: separate button; BOOT not
  reachable on gift units, which will have a button and a knob.
- Fix in this item: answered 2026-11-13: yes.
- Wiring: user wires a momentary button IO7 → GND on the board (needed
  before the tap session).
- Tap session: user presses buttons and watches the panel.

## Plan
- [ ] `boot_capture`: optional second (info) input sampled in the same ISR,
      own `PressLatch`, `take_info_presses()`.
- [ ] `live_radar`: selection on GPIO7, knob switch into the capture after
      the boot hold, info toggle from drained presses; rename diag lines.
- [ ] Host test for the toggle helper; checks and tests.
- [ ] Device build (`dev-settings`, `OTA_CHANNEL=beta`), gate, commit.
- [ ] Ask user: wire IO7, then flash + tap session with capture.
- [ ] Decision 0088, DESIGN, PLAN, README, HANDOFF; delete this file.

## Discoveries and assumptions
- Selection presses were already interrupt-captured (0070); only the info
  page was polled once per display loop, which is why a press during a TLS
  stall could be missed.
- IO7 is free on the DevKitC-1 N8R8 (not a strap, flash/PSRAM or USB pin).
- Default: GPIO0 selection is dropped from the device app, not kept as a
  second selector.

## Verification

## Open questions
