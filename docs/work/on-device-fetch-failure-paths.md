# Fetched-element failure paths behave as 0075 says on the board
Status: blocked: user choice — second ~2h15 run (2 more GETs) to show a load clearing OLD DATA, or accept host-test coverage for it
Started: 2026-11-02. Scope: PLAN M6 "Handle network failure, missing/invalid
time or data, and stale elements"; decisions [0075](../decisions/0075-fetched-element-refresh-and-staleness.md),
[0074](../decisions/0074-live-utc-radar-on-fetched-elements.md), [0069](../decisions/0069-flash-without-per-flash-approval.md).

## Outcome
One continuous board run of a fault-injecting build of `live-fetch-radar`
shows, from UART evidence, that each 0075 failure class happens on real
network/TLS paths and is handled as decided: a failed boot fetch keeps
`NO DATA` and backs off; DNS, TCP and mid-handshake failures back off
5/15 min; a certificate-verification failure waits a full interval; the
previous catalogue keeps tracking through all of them; `OLD DATA` is raised
and later cleared; the catalogue is unloaded (`NO DATA`) on expiry and a
later success reloads it; and a Wi-Fi drop reconnects and recovers without
a reboot.

## Done when
- Host tests for the fault-plan parser and freshness override pass, plus the
  existing firmware host suites and `cargo check/test --workspace`.
- Default `live-fetch-radar` build is unchanged in behavior (the plan,
  overrides and wrong root exist only with the new feature; the default
  passes `Fault::None` and no Wi-Fi drop); fault build passes clippy (no new warnings) and
  the offline image gate.
- A board capture shows, per attempt, the injected fault, the stage and
  0075 class reported, and the `next_in_s` matching 0075; `LIVE_FETCH stale=`
  transitions and unload/reload lines; Wi-Fi disconnect/reconnect and SNTP
  recovery; no panic; heap flat.
- Evidence and limits are in firmware/README; PLAN/HANDOFF/PROGRESS updated;
  board left on the default 8-hour home-site build.

## Boundaries
- No change to 0075's schedule, thresholds or classes; overrides exist only
  in the opt-in fault build.
- CelesTrak budget: at most two HTTP GETs (both expected 200), ≥2 h apart;
  injected TLS failures are handshakes only, with correct SNI, no HTTP.
  DNS faults query `.invalid`; TCP faults target TEST-NET-1 (192.0.2.1).
- Post-GET failures (HTTP status, truncation, parse) and the repeated-
  handshake rule are not injected (they would cost a download and an
  extra 2 h each); they stay host-tested.
- Site and credentials never printed or committed; logs/images stay local.

## Human gates
- None blocking. Optional: user glances at the panel during the OLD DATA /
  NO DATA windows (visual acceptance of those notices on hardware).

## Plan
- [x] `fault_plan` module (host-tested): parse `FAULT_PLAN` (comma list per
  attempt: ok, dns, tcp, handshake, verify; exhausted = ok), and minute
  overrides for stale and expire-after-first-load.
- [x] `live-fetch-faults` feature in `live_fetch_radar_main.rs`: inject at
  the real stage; schedule Wi-Fi drop; freshness override applied on first
  load relative to its newest epoch; extra log lines.
- [x] Wrong-root PEM for the verify fault (public root not in CelesTrak's chain).
- [x] Host checks; dummy builds of default and fault images; clippy; gate.
- [x] Commit checkpoint (4b12d9c). Build with real env, gate, flash + capture in one
  command (background), run ~4.5 h. Running since 2026-11-02 18:15 UTC:
  `firmware/target/live-fetch-faults-run.log` (ignored, local), `timeout 17400`.
- [x] Make the `LIVE_FETCH stale=` log check skip frames with no UTC (see
  discoveries); clippy both features unchanged, fmt clean. The running board
  has the pre-fix image (4b12d9c).
- [ ] Read capture, check every done-when line; restore default 8 h build.
- [ ] Promote evidence, delete this file, handoff.

Planned run (REFRESH_MINUTES=120, FAULT_PLAN=tcp,ok,dns,handshake,verify,ok,
STALE 90 min, expire 210 min after first load, Wi-Fi drop at 60 min for 10 min):
attempt 1 tcp at boot → +5 min; 2 ok (~5 min) loads; Wi-Fi drop ~1 h;
OLD DATA ~1h35; 3 dns ~2h05 → +5 min; 4 handshake ~2h10 → +15 min;
5 verify ~2h25 → +2 h; NO DATA (expiry) ~3h35; 6 ok ~4h25 clears both.

## Discoveries and assumptions
- `RadarApp::with_freshness` is a by-value builder, so the display task can
  set the override after the first load; no app change.
- Newest STATIONS epoch was ~9 h old at one earlier fetch, so a fixed expiry
  threshold would fire unpredictably; expiry override = first load's epoch
  age + 210 min.

- Each accepted SNTP renewal invalidates the app frame (`utc_trust.rs`
  observe, 0062), and the display loop checked `data_stale()` between that and
  the next render, so the run logs a spurious `stale=false frame_utc=None` then
  `stale=true` on each renewal. Render always follows `update`, so the panel
  notice is unaffected; read staleness from frames with UTC.

## Verification
- Run so far: MAC 80:45:6b:2a:45:08 on /dev/ttyUSB0. Attempt 1 tcp → Transport,
  next 300 s; attempt 2 ok 20/20 loaded 18:21:41Z, next 7200 s; newest epoch
  ~7.9 h old (expire_after_s=41180). Wi-Fi drop 3600 s, off 600 s, reconnected
  at 4200 s without reboot; frames continued, heap ~72.9–73.4 KB.
  `stale=true` at 19:51:41Z (= fetch + 90 min). Attempt 3 dns → Transport,
  300 s; attempt 4 handshake → `Io(ConnectionReset)` verify=0x0 → Handshake,
  900 s; attempt 5 verify → MbedTLS -0x2700 verify=0x8 (not trusted) →
  Rejected, 7200 s, no GET. All `FAULT_RESULT match=true`.
  Expired 21:51:41Z (= load + 210 min): catalogue=0, live heap 73,612 → 51,456 B.
  Attempt 6 22:44:54Z: HTTP 200, 20/20, but newest epoch still
  2026-11-02T10:25:21Z (unchanged since load 1), ~12.3 h old > 41,180 s
  threshold → unloaded at once (`loaded load=2 catalogue=0`). Correct
  epoch-based behaviour, but `OLD DATA` clearing on a load and reload after
  expiry are NOT shown. Other: SNTP 243 Cloudflare + 1 NIST observation,
  242 accepted, 1 NoRenewal; first accept after reconnect 11 s later; one
  NO FRAME (boot); no panic; heap peak 135,320 B of 163,840; stack 6,112 of
  21,144 B; display gap per fetch 452/18/112/243/451 ms; BOOT max gap 33 ms at
  boot (known), ≤14 ms after.
- 2026-11-02 (pre-flash, dummy env): `cargo check --workspace` ok; `cargo test
  --workspace` 325 passed; firmware host default 19, `element-fetch` 53 (+6
  `fault_plan`), `live-utc-boot` 70, `sharp-boot-capture` 60; fmt clean;
  clippy default and fault builds: same 6 warnings as unchanged baseline;
  fault image 1,350,768 B (16.23%), RWX warning only, offline gate exit 0.

## Open questions
- Asked 2026-11-02 ~22:50 UTC: second run (fault build, no faults,
  FAULT_STALE_MIN=30, no expiry override, REFRESH_MINUTES=120; flash no
  earlier than 00:45Z so the boot GET is ≥2 h after 22:44Z; ~2h15; 2 GETs)
  to show OLD DATA raised then cleared by a successful load, vs closing with
  host-test coverage. Recommendation: close now.
