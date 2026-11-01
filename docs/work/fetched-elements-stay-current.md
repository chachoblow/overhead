# Fetched elements stay current across a day
Status: active
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
- Home coordinates in ignored `firmware/.site.env`: needed (user writes `SITE_LAT_DEG=`, `SITE_LON_DEG=`, optional `SITE_ALT_M=`).
- Board left powered on `/dev/ttyUSB0` for ~4.5 h capture: needed.
- User panel observation after a refresh: needed.

## Plan
- [x] App `Freshness`, `load_fetched_catalogue`, selection kept across loads, expiry unload; tests.
- [x] Render `OLD DATA` wide notice and square panel line; test.
- [x] Firmware `element_refresh` schedule + `site` parsing; host tests.
- [x] Image: reusable TLS/buffers, schedule, outcome classes, fetch UTC, home site; dummy build, clippy (no new warnings), offline gate.
- [x] Decision 0075, `.gitignore`, README build recipe.
- [ ] Build with real `.wifi.env` + `.site.env`, `REFRESH_MINUTES=120`; gate; flash and capture ~4.5 h in one command (boot + 2 refreshes).
- [ ] User observation after a refresh.
- [ ] Record evidence (firmware README), PLAN/HANDOFF/PROGRESS; delete this file.

## Discoveries and assumptions
- `Tls::new` allows one live instance and `ConstStaticCell::take` succeeds once, so the TLS context and buffers are created once and reused.
- The fetch is attempted only when an in-budget SNTP observation arrives (64 s polls), so due times have ~64 s granularity.
- Dummy build: image 1,338,608 B (16.08%), only the RWX warning; gate passed.

## Verification
- `cargo check --workspace`, `cargo test --workspace` (325 passed), app/render `--no-default-features` check, fmt; firmware host `--lib --locked`: default 19, `element-fetch` 47, `live-utc-boot` 70, `sharp-boot-capture` 60 — all pass (uncommitted tree, 2026-11-01).

## Open questions
- None.
