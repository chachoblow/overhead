# Fetched elements stay current across a day
Status: verifying (capture running)
Started: 2026-11-01. Scope: PLAN M6 "network failure, missing/invalid time or data, and stale elements" + refresh part of "HTTPS elements … refreshes"; decisions [0075](../decisions/0075-fetched-element-refresh-and-staleness.md), 0074, 0071, 0002, 0004.

## Outcome
A powered `live-fetch-radar` board refreshes STATIONS on 0075's schedule from the user's home site, keeps tracking the previous catalogue when a refresh fails, and flags/drops old elements instead of presenting them as current.

## Done when
- Host tests (explicit time) cover schedule, failure classes/backoff, stale and expiry thresholds and notice lifecycle, and selection across a catalogue swap.
- One long on-board capture (diagnostic `REFRESH_MINUTES=120`) shows the boot fetch and at least one refresh with UTC, record count, heap and stack logged.
- User observes the radar on the panel after a refresh (home site).

## Boundaries
STATIONS only; no flash persistence; no location UI; no cadence work. CelesTrak: HTTP requests at most one per interval (≥2 h); never commit/print the site or credentials; credential/site-bearing images and raw logs stay local.

## Human gates
- 0075 decisions: answered 2026-11-01 (all recommendations accepted, home coordinates).
- Home coordinates in ignored `firmware/.site.env`: answered 2026-11-01 (file present, values numeric; never printed).
- Board left powered on `/dev/ttyUSB0` for ~4.5 h capture: needed.
- User panel observation after a refresh: needed.

## Plan
- [x] App `Freshness`, `load_fetched_catalogue`, selection kept across loads, expiry unload; tests.
- [x] Render `OLD DATA` wide notice and square panel line; test.
- [x] Firmware `element_refresh` schedule + `site` parsing; host tests.
- [x] Image: reusable TLS/buffers, schedule, outcome classes, fetch UTC, home site; dummy build, clippy (no new warnings), offline gate.
- [x] Decision 0075, `.gitignore`, README build recipe.
- [ ] Build with real `.wifi.env` + `.site.env`, `REFRESH_MINUTES=120`; gate; flash and capture 16,800 s in one command (boot + 2 refreshes). Built, gate passed, MAC `80:45:6b:2a:45:08` on `/dev/ttyUSB0` confirmed; capture started 2026-11-01 ~21:57 UTC to local `firmware/target/live-fetch-radar-v2-run1.log`.
- [ ] User observation after a refresh.
- [ ] Record evidence (firmware README), PLAN/HANDOFF/PROGRESS; delete this file.

## Discoveries and assumptions
- `Tls::new` allows one live instance and `ConstStaticCell::take` succeeds once, so the TLS context and buffers are created once and reused.
- The fetch is attempted only when an in-budget SNTP observation arrives (64 s polls), so due times have ~64 s granularity.
- Dummy build: image 1,338,608 B (16.08%), only the RWX warning; gate passed.

## Verification
- Boot fetch 2026-11-01 21:57:37 UTC: TLS 1.2, handshake 803 ms, 3 certificates dated OK, HTTP 200 (8,410 B body; read ended by ConnectionReset after full Content-Length), 20/20 accepted, total 2,172 ms, heap peak 111,116 B, stack written 7,184 of 21,384 B, display gap 455 ms; `FETCH_SCHEDULE outcome=Success next_in_s=7200`.
- Refresh 1, 2026-11-01 23:58:39 UTC (uptime 7,273 s; attempt due at 7,214 s, ran on the next in-budget SNTP poll): TLS 1.2, handshake 937 ms, dates OK, HTTP 200, 20/20 accepted (same epochs as boot), total 2,184 ms, catalogue swapped (`load=2`), next in 7,200 s. Heap high-water **136,808 B** (boot 111,116 B): old catalogue + persistent TLS context + new ingest; refresh 2 must show it plateaus, not grows. Display loop gap 449 ms (10 s stats; `DISPLAY_FETCH_GAP` repeated the boot max, fixed in source after flashing — not reset per fetch).
- Host prediction (pinned 2026-11-01 STATIONS fixture, home site, 60°): CSS complex in view 01:00–01:10 and 02:37–02:47 UTC 2026-11-02 (after refreshes at ~23:57 and ~01:57).
- `cargo check --workspace`, `cargo test --workspace` (325 passed), app/render `--no-default-features` check, fmt; firmware host `--lib --locked`: default 19, `element-fetch` 47, `live-utc-boot` 70, `sharp-boot-capture` 60 — all pass (uncommitted tree, 2026-11-01).

## Open questions
- None.

## Follow-ups
- The running image predates the `DISPLAY_FETCH_GAP` reset fix; any reflash picks it up.
