# 0075 — Fetched elements refresh, age out visibly and track the home site

Date: 2026-11-01
Status: accepted (user, 2026-11-01: refresh/backoff split, stale and expiry
thresholds, no persistence yet, home coordinates from an ignored file).
Builds on [0002](0002-data-and-time-over-wifi.md), [0071](0071-https-element-fetch-first-slice.md)
and [0074](0074-live-utc-radar-on-fetched-elements.md); an interim site source
under [0004](0004-hardcoded-location.md). Replaces 0071/0074's "one request per
boot, any failure final" for the `live-fetch-radar` image only.

## Decision
1. **Refresh.** The image fetches STATIONS on boot (after the first SNTP
   observation within 100 ms) and then one interval after each success:
   8 hours by default (0002), never less than 2 hours (CelesTrak's update
   cycle). A build-time `REFRESH_MINUTES` (≥120) shortens it for diagnostics.
   Schedule logic is host-tested in `firmware/src/element_refresh.rs`.
2. **Failures, split by whether the GET was sent.**
   - Before it (DNS, TCP, TLS setup, a handshake that did not fail
     certificate verification): no HTTP error can be counted against the
     address, so retry after 5, 15, 30, then every 60 minutes.
   - After it (any HTTP status other than 200, head, truncation, parse or
     ingest failure, or a timeout once the GET was written), or on
     certificate verification or date failure: wait one full interval. A 200
     has already counted as a download (a repeat before CelesTrak updates gets
     403), and error statuses count toward the 50-errors-in-2-hours firewall.
   - Two handshake failures in a row (for example a TLS 1.3-only server this
     image cannot date-check) also wait one interval.
3. **Keep, flag, then drop old data.** A failed refresh keeps tracking the
   previous catalogue. The app's `Freshness` policy (`Freshness::DEVICE`)
   raises a condition-driven `OLD DATA` notice (0039/0040) once the frame UTC
   is more than 24 hours past the last successful fetch (three missed 8-hour
   refreshes). Once the newest element epoch is more than 7 days old the
   catalogue is unloaded and the radar shows `NO DATA` until a new fetch.
   Thresholds are exact-boundary inclusive (not stale at exactly 24 h).
4. **Selection across a refresh.** Loading a new catalogue keeps the selected
   NORAD ID if the new catalogue still tracks it under 0049; otherwise
   selection falls back to first acquisition.
5. **No persistence yet.** Without an RTC a reboot loses trusted time and
   shows `NO FRAME` anyway, so stored elements would mainly save one request
   per boot. Flash persistence (0002's consequence) stays a later item.
6. **Home site.** The observer comes from build-time `SITE_LAT_DEG`,
   `SITE_LON_DEG` and optional `SITE_ALT_M` (decimal degrees, metres) in the
   ignored `firmware/.site.env`, validated in `firmware/src/site.rs`; a bad
   value panics at boot before any request. The coordinates are personal
   data: never commit, print or record them. Images built with them stay
   local, like credential-bearing ones.

## Why
CelesTrak's limits make retries asymmetric: a request that never reached
HTTP costs nothing against them, while one that did either succeeded (so
asking again is pointless) or will fail the same way. Tracking on old elements
stays useful for days, but the user should see when data has stopped
refreshing and never see week-old elements presented as current.

## Consequences
Retries before the GET can still be 24 per day during a long outage, all
below HTTP. The 2-hour floor and the after-request rule bound HTTP requests to
one per interval. The `OLD DATA`/`NO DATA` thresholds are app policy and apply
only to apps given a `Freshness`; the simulator is unchanged.
