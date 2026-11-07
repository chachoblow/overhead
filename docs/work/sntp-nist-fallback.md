# The NIST SNTP fallback renews UTC when Cloudflare is unavailable
Status: blocked: user choice on the trust-gate anchor rule (see Discoveries, Open questions)
Started: 2026-11-07. Scope: PLAN M6 standalone device (unattended operation);
decisions [0064](../decisions/0064-prototype-sntp-screen-and-leap-boundary.md),
[0066](../decisions/0066-prototype-live-sntp-stack-and-source.md),
[0080](../decisions/0080-sntp-kod-backoff-and-fetch-independence.md)

## Outcome
The reply screen accepts NTP version-3 server replies as well as version 4, so
`time.nist.gov` (which answers VN=3) renews UTC. With Cloudflare unavailable
for longer than the 1 h holdover, the board keeps tracking on NIST alone, and
the device app runs the change.

## Done when
- A new decision (0081) amends 0064's reply screen; 0064/0066 carry a pointer.
- `firmware/src/sntp_screen.rs` accepts VN 3 and 4 (mode 4), still rejects
  other versions/modes; host tests cover both.
- Board run of the fault build with Cloudflare `DENY`-injected for > 1 h:
  NIST `Observation`s renew trust, no `NO FRAME` in the window, and a refresh
  inside the window is dated by a NIST observation.
- Device app rebuilt, gated and flashed; boot capture shows radar up.

## Boundaries
- Requests stay VN=4. No change to the uncertainty bound, KoD policy, the
  100 ms target, holdover, leap rule, server list or order, or the
  non-smearing rule. No new device-app log lines.
- External requests: normal SNTP polling and the 0075 CelesTrak schedule only.

## Human gates
- VN=3 vs replace NIST: answered 2026-11-07: accept VN=3.
- Board access/flash: covered by 0069 (confirm port + MAC, gate image).

## Plan
- [x] Decision 0081 + pointers in 0064/0066.
- [x] Screen change + tests; firmware host tests (default, element-fetch,
      live-utc-boot, sharp-boot-capture); `cargo check --workspace`; commit.
- [ ] Fault build `REFRESH_MINUTES=120 FAULT_KOD="0:DENY:5-185"
      FAULT_KOD_BLOCK_MIN=30`; gate; flash; 3.5 h capture to
      `firmware/target/nist-fallback.log` (started 2026-11-07 ~16:28 UTC,
      `timeout 12600`). Built, gate passed (image 1,399,584 B, 17.50%,
      ELF sha256 73e7f056…), port /dev/ttyUSB0 MAC 80:45:6b:2a:45:08.
      If interrupted: check the log; rerun the capture only if it is short.
- [ ] Device app build, gate, flash, boot capture.
- [ ] README evidence, HANDOFF/PLAN/PROGRESS, delete this file, handoff.

## Discoveries and assumptions
- 2026-11-07 devbox query: `time.nist.gov` (132.163.96.4) replies LI 0,
  VN 3, mode 4, stratum 1, refid `NIST`, origin echoed; Cloudflare replies
  VN 4. Matches 0080's 39/44 `BadVersionOrMode`.
- A rejected NIST reply counts as a reply in the 0080 loop (no further
  server asked that poll), so the fix needs no policy change.
- Run 1 (fault build, started 16:29 UTC): VN=3 works: 61 NIST observations,
  0 rejected (uncertainty median 28 ms, p90 52 ms, max 99.9 ms); tracking
  held past the 1 h holdover on NIST alone. But at uptime 4,184 s NIST seq=66
  returned uncertainty 99,906 µs (< 100 ms budget); `utc_trust::observe`
  replaced the 23.9 ms anchor with it (`Resynced step_us=49120`), it lapsed
  ~4.7 s later → `NO FRAME` at 4,188 s; next reply (22.5 ms) `Accepted` at
  4,248 s (samples/selection discarded). Cause: `observe` always adopts an
  in-budget observation, even one with less remaining budget than the
  current anchor. Not NIST-specific; it applies to any slow reply. Changing it
  amends 0063/0065 (outside this item's boundaries) → asked the user.

## Verification
- Host tests (root, `--lib --locked`): default 37, element-fetch 88, live-utc-boot 91, sharp-boot-capture 81; `cargo check --workspace` ok; fmt ok.

## Open questions
- Trust gate: keep the anchor whose projected error bound is smaller (new
  observation replaces only if uncertainty_new ≤ uncertainty_old + drift ×
  age_old), via a new decision 0082, inside this item? Recommended: yes, then
  rerun the board capture.
