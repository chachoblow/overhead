# 0074 — Live-UTC radar on freshly fetched elements, with checked certificate dates

Date: 2026-11-01
Status: accepted (user, 2026-11-01: certificate dates checked in Rust, 160 KiB
internal heap, no markers plus `NO DATA` on failure, persistence/refresh/stale
handling out of scope). Resolves [0073](0073-diagnostic-fetch-without-certificate-dates.md)'s
condition for fetched data reaching the radar. Builds on
[0071](0071-https-element-fetch-first-slice.md) and [0067](0067-opt-in-live-utc-iss-radar-scope.md).
Done 2026-11-01: two on-board fetches with dates checked and a user-observed
pass with visible BOOT cycling ([evidence](../../firmware/README.md#live-utc-radar-on-fetched-stations-flashed-uart-captures)).

## Decision
1. **Image.** A separate opt-in `live-fetch-radar` image composes 0071's one
   STATIONS request per boot with the live-UTC radar and 0068/0070 BOOT
   cycling. The network task screens SNTP as before and fetches once, after the
   first observation within 0065's 100 ms budget; a finished catalogue goes to
   the display owner over a channel. Any failure is final for the boot. Existing
   `live-fetch` and `live-utc-*` images are unchanged.
2. **Certificate dates.** MbedTLS still verifies chain, signatures and hostname
   against the two bundled roots. The socket is wrapped so the server's first
   8 KiB are recorded; after a successful TLS 1.2 handshake, `firmware/src/tls_dates.rs`
   extracts every certificate from the clear-text Certificate message and
   requires each to be within `notBefore..=notAfter` at the screened UTC,
   **before** the GET is sent. The authenticated handshake binds those bytes
   to what MbedTLS verified. It fails closed: no Certificate message (for
   example TLS 1.3, or a truncated capture) is a failure. The bundled roots are
   not served and so are not checked on the device; a host test requires them
   to remain valid beyond 2037.
3. **No fixture fallback.** The app may hold no catalogue
   (`RadarApp::awaiting_catalogue`, `load_catalogue`); with a valid frame and
   an empty load the renderer shows `NO DATA` (wide notice and square panel)
   instead of a horizon claim. Until and unless the fetch succeeds, no
   markers are shown. STATIONS has the explicit Station role (0045).
4. **Memory.** Internal heap 160 KiB (no PSRAM). The large network buffers are
   const-initialised statics.
5. **View.** The historical fixed site `LIVE_UTC_ISS_OBSERVER` is kept (no
   recentering); the wide view starts at 60° vertical FOV so co-orbiting
   objects can share the view. Both are diagnostic choices, not product defaults.

## Why
Installing a C toolchain for MbedTLS's wall-clock hook (0073's alternative)
is a machine change; the clear-text TLS 1.2 Certificate message gives the same
dates with host-tested Rust and no new dependency. Showing old fixture elements
on a live display would present stale positions as current.

## Consequences
A CelesTrak move to TLS 1.3 makes this image fail closed until certificate
access changes. Root validity becomes a host-test maintenance item. Each boot
still costs one CelesTrak request. Persistence, refresh, stale-element policy
and frame cadence remain open; frame cost at 60° is near the 50 ms grid.
Evidence and limits: [firmware README](../../firmware/README.md#live-utc-radar-on-fetched-stations-flashed-uart-captures).
