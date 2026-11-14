# Device inputs a gift unit can use, and M6 closed
Status: blocked: user taps the knob during a refresh (TLS stall), deferred by the user 2026-11-14
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
- Wiring: done 2026-11-14 (four-leg tactile button: wires on diagonally
  opposite legs; same-side legs are always connected).
- Tap session: selection/info done 2026-11-14. Knob taps during a refresh
  deferred by the user (asleep at the 07:03 UTC refresh). The board stays on
  the 2 h session image, so refreshes come every ~2 h from 05:02:44 UTC
  (07:03, 09:03, 11:03, 13:03, …; check `FETCH_SCHEDULE`/`LIVE_FETCH loaded`
  times in a fresh capture before telling the user a time). To resume: start
  a host-timestamped capture a few minutes before the next refresh
  (`stty -F /dev/ttyUSB0 115200 raw -echo -hupcl`, `cat`, no reset), user
  taps the knob ~1/s across it.

## Plan
- [x] `boot_capture`: optional second (info) input sampled in the same ISR,
      own `PressLatch`, `take_info_presses()`.
- [x] `live_radar`: selection on GPIO7, knob switch into the capture after
      the boot hold, info toggle from drained presses. `SELECT` and
      `INFO_PAGE` are event-log lines (app prints them) with `loop_gap_us`
      and `fetching`, so a press drained after a TLS stall is visible.
- [x] Host test for the toggle helper; checks and tests.
- [x] Device build, gate, flashed once (boot healthy, settings kept).
- [x] Session image: `OTA_CHANNEL=beta REFRESH_MINUTES=120 OVERHEAD_BUILD=15
      OVERHEAD_VERSION=0.1.15-inputs`, `device,dev-settings`; built and gated
      (ELF sha256 prefix 3d61d4de). Flash only when the user is ready: the
      stored copy is >2 h old, so it fetches right after boot. Flashed
      05:02 UTC; its boot fetch (05:02:44) ran before the user could press.
- [x] Selection and info page on the board (below).
- [ ] Knob taps during the next 2 h refresh (~07:03 UTC; a sooner fetch
      would break CelesTrak's 2 h rule); capture around it, look for
      `INFO_PAGE … loop_gap_us≫5 ms fetching=true`.
- [ ] Ask user: wire IO7, then flash + tap session with capture
      (`target/inputs-session.log`); afterwards reflash a default-interval
      build (`OVERHEAD_BUILD=15`, no `REFRESH_MINUTES`).
- [x] Decision 0088 (accepted, from the user's 2026-11-13 answers), DESIGN
      controls, 0068/0070 pointers, README OTA-idle wording.
- [ ] After the refresh capture (`firmware/target/inputs-refresh.log`,
      host-timestamped, started after the session capture): README evidence
      section `selection-button-and-captured-knob-switch-0088` (0088 links to
      it) + wiring line, PLAN M6 boxes ticked, HANDOFF board wiring/limits;
      reflash an 8 h build (`OVERHEAD_BUILD=15`); delete this file; handoff.

## Discoveries and assumptions
- Selection presses were already interrupt-captured (0070); only the info
  page was polled once per display loop, which is why a press during a TLS
  stall could be missed.
- IO7 is free on the DevKitC-1 N8R8 (not a strap, flash/PSRAM or USB pin).
- Default: GPIO0 selection is dropped from the device app, not kept as a
  second selector.
- A dev build with `OVERHEAD_BUILD=0` is offered release 15 on `beta` at the
  next refresh and would reinstall it, so board builds here use
  `OVERHEAD_BUILD=15` (`Offer::Current`); release 16 still installs over it.
- First flash with `timeout 90` stopped before the write; flashes need the
  documented 600 s window (re-flashed fine).

## Verification
- `cargo +stable test --manifest-path firmware/Cargo.toml --lib --locked` →
  38 passed (incl. `info_presses_toggle_per_press_and_a_boot_hold_never_counts`).
- `cargo check --workspace` ok; `cargo test --workspace` → 361 passed.
- First device build: gate pass; flashed 2026-11-14 04:44 UTC: `OTA boot
  build=0 … state=Valid channel=beta`, `STORE loaded`, Wi-Fi 2 s, first frame,
  no panic (`firmware/target/inputs-boot.log`).
- Session 2026-11-14 05:02 UTC (`firmware/target/inputs-session.log`):
  `OTA boot build=15 version=0.1.15-inputs … Valid channel=beta`, fetch
  05:02:44, 5 in view; IO7 presses: 11 `SELECT drained=1 applied=1`, cycling
  20443→22286→22803→28415→20443…, user saw the selection change; knob:
  `INFO_PAGE` shown=true then false.

## Open questions
