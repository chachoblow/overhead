# SNTP KoD recovery: bounded back-off, fetches independent of SNTP replies
Status: active
Started: 2026-11-06. Scope: PLAN M6 standalone device; decisions 0080 (new), 0067, 0065, 0075, 0076, 0079

## Outcome
After any SNTP Kiss-o'-Death the runtime backs off per server and recovers
without a reboot; element refreshes follow the 0075 schedule whether or not
SNTP replies, dated by trusted holdover UTC. No KoD can leave the device at
`NO FRAME` until reboot.

## Done when
- 0080 accepted (done: user "go", 2026-11-06).
- Host tests for the KoD policy (RATE doubling/cap/decay, DENY/RSTR block and
  expiry, fallback order) and for the dating projection.
- Fault build on the board: injected RATE/DENY lines, back-off visible,
  queries resume, a refresh runs while no server answers but holdover is
  valid, NO FRAME only after holdover lapses and recovery after blocks end.
- Device app rebuilt, image gate passes, flashed; boot capture ok.
- `cargo check --workspace`, firmware host tests pass; README/HANDOFF/PLAN/PROGRESS updated.

## Boundaries
No change to app/render, 0075 timings or 0065 bounds. No new device-app log
lines beyond the KoD event. Toolchain unchanged.

## Human gates
- Decision 0080: answered 2026-11-06 ("go").
- Flash: no approval (0069); confirm port/MAC.

## Plan
- [x] `firmware/src/sntp_policy.rs` (pure, host-tested)
- [x] `utc_trust::projection_trusted` + tests
- [x] `fault_plan` KoD windows (`FAULT_KOD`) + `FAULT_KOD_BLOCK_MIN` + tests
- [x] Rewire network loop in `live_radar.rs`
- [x] Host tests, check, dummy release builds (live-fetch-faults, live-fetch-radar, device)
- [ ] Board fault run (~4.5 h), then reflash device app
- [ ] Docs, PROGRESS, handoff

## Discoveries and assumptions
- `utc_trust` is behind `sharp-radar`; `live-fetch-radar` implies it.

## Board run plan
Fault build, `REFRESH_MINUTES=120`, `FAULT_KOD="0:RATE:100-135,1:DENY:100-101,0:DENY:200-275,1:DENY:200-275"`,
`FAULT_KOD_BLOCK_MIN=30`, capture ~305 min to `firmware/target/kod-faults.log`.
Expected: A) t≈100 server0 RATE doubling 128…2048, server1 blocked to ~130;
refresh at ~120 with no reply that poll, dated by holdover (`FETCH_UTC age_s`);
B) t≈200 both DENY, re-DENY at ~230/260, refresh ~240 still in holdover,
`NO FRAME` ~254 when holdover lapses, recovery ~290.

## Verification
- 2026-11-06: `cargo check --workspace` ok; firmware host default 37,
  `element-fetch` 88, `live-utc-boot` 90, `sharp-boot-capture` 80; fmt ok;
  clippy (fault build) only pre-existing warnings; dummy release builds of
  `live-fetch-radar`, `device`, `live-fetch-faults` ok (RWX warning only).

## Open questions
