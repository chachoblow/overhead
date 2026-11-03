# Device app: the live radar as the standalone firmware build
Status: active
Started: 2026-11-02. Scope: PLAN M6 "Run shared engine/renderer on S3 + Sharp"
(app part; knob is a later item); decisions 0074, 0075, 0068/0070, 0062–0067.

## Outcome
A named firmware build that is the device application, not a diagnostic: it
boots straight into the live radar at the home site with screened UTC and
fetched STATIONS elements, refreshes on 0075's schedule, cycles with BOOT, and
keeps a short event log instead of diagnostic telemetry. The `live-fetch-radar`
and `live-fetch-faults` diagnostics keep their behaviour and share the same
runtime code, so the app is not a fork.

## Done when
- New decision (0076) records what the app keeps and drops, accepted by the user.
- `cargo check --workspace`, `cargo test --workspace`, firmware host suites
  (default, `element-fetch`, `live-utc-boot`, `sharp-boot-capture`) pass; app,
  diagnostic and fault builds compile with dummy env; clippy adds no warnings.
- App image passes the offline gate, is flashed, and a UART capture shows boot →
  UTC → fetch 20/20 → frames, with only the agreed event lines.
- User sees the radar on the panel (pass or empty-sky notice) and BOOT cycling.
- (Optional, per answer) a ≥2 h run with `REFRESH_MINUTES=120` shows a refresh.

## Boundaries
- No crate rename or change of the default binary (later decision if wanted).
- No knob, persistence, location source, timing policy or catalogue change.
- No behaviour change in the runtime beyond logging; 0075 schedule unchanged.
- One CelesTrak request per boot/refresh; no extra resets beyond the capture.
- Site and credentials stay in ignored env files; never print or commit them.

## Human gates
- App behaviour: asked/answered 2026-11-02: short event log (boot, Wi-Fi,
  fetch outcome + next time, load/OLD DATA/expiry, panics; nothing per frame or
  press); halt on panic as now (no auto-reboot: a crash loop would cost a
  request per boot); yes to a ~2–3 h `REFRESH_MINUTES=120` run; leave the board
  on the app with the 8 h refresh afterwards.
- Panel observation + BOOT presses: at the end, after the capture.

## Plan
- [x] Extract runtime from `firmware/src/live_fetch_radar_main.rs` into a
      target-only lib module (`live_radar`), with a verbosity switch; diagnostic
      main becomes thin and keeps every existing line in verbose mode.
- [x] Add `device` feature + `overhead-s3-device` bin (thin main, event log).
- [x] Host tests/checks, dummy builds of all three, clippy.
- [x] Draft 0076; checkpoint commit.
- [ ] Real build, gate, flash, capture (confirm `/dev/ttyUSB0` + MAC).
- [ ] User panel check; optional refresh run.
- [ ] Promote docs (README section, PLAN, HANDOFF, PROGRESS), delete this file.

## Discoveries and assumptions
- Diagnostic main is ~1,000 lines; ~44 println lines, fault hooks via cfg.
- Blocking UART prints (~10 ms per long line at 115200) sit in the display
  loop; fewer lines in the app also shorten loop stalls (not a timing claim).

## Verification
- Refactor checkpoint: `cargo check --workspace` ok; `cargo test --workspace`
  325 passed; firmware host default 19, `element-fetch` 53, `live-utc-boot` 70,
  `sharp-boot-capture` 60; fmt clean. Dummy release builds of `device`,
  `live-fetch-radar`, `live-fetch-faults` ok (RWX warning only);
  `device,live-fetch-faults` fails with the intended compile_error. Clippy: same
  6 warnings as before (the 4 static-deref ones moved from bin to lib).

## Open questions
