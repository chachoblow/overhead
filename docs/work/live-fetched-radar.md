# Live-UTC radar tracking freshly fetched STATIONS elements
Status: active
Started: 2026-11-01. Scope: PLAN M6 "Wi-Fi credentials, HTTPS elements…" (proposal slice 4);
decisions [0071](../decisions/0071-https-element-fetch-first-slice.md),
[0073](../decisions/0073-diagnostic-fetch-without-certificate-dates.md), 0067, 0068, 0070.

## Outcome
A separate opt-in Sharp image boots, obtains screened UTC, makes one HTTPS
STATIONS fetch with certificate validity dates checked against trusted UTC, and
tracks the fetched elements on the live-UTC radar with BOOT cycling. The display
owner keeps servicing the panel while the fetch is in flight. With no usable
data it shows no markers and a `NO DATA` notice, never fixture elements.

## Done when
- Host tests: app awaiting/loaded catalogue; renderer `NO DATA`; TLS record →
  Certificate message → X.509 validity extraction and date check on the real
  CelesTrak chain (expired / not-yet-valid / truncated / no-certificate cases).
- Firmware host suites and `cargo check/test --workspace` pass.
- Image builds and passes the offline gate.
- One-boot UART capture: date check result, fetch/catalogue summary, heap peak
  within heap, max display-loop gap during the fetch, BOOT capture stats.
- User panel observation in a predicted pass window: live markers move; BOOT
  cycles if ≥2 in view.

## Boundaries
No persistence, refresh or stale-element policy; STATIONS only; one request per
boot, no retry, a few boots per 2 h. No toolchain/machine change. Existing images
(`live-fetch`, `live-utc-*`) and frozen M2 evidence unchanged. No knob.

## Human gates
- Cert dates / heap / failure notice / scope: asked 2026-11-01, answered: app-level
  date check (fallback: ask for C toolchain), internal heap ~160 KiB first,
  no markers + `NO DATA`, persistence/refresh/stale out.
- Panel observation: at the end.

## Plan
- [ ] app: `RadarApp::awaiting_catalogue` + `load_catalogue` (empty state); tests
- [ ] render: `NO DATA` for an empty catalogue (wide notice, square panel); tests
- [ ] firmware `tls_dates`: TLS 1.2 handshake capture parser + DER validity; chain fixtures; tests
- [ ] firmware image `live-fetch-radar` (feature, bin): network task SNTP→fetch→ingest→channel;
      recording stream adapter; display loop gap metric; BOOT capture
- [ ] build, clippy, gate; flash; one-boot capture
- [ ] predict pass windows; user observation
- [ ] decision 0074, README evidence, PLAN/HANDOFF/PROGRESS; delete this file

## Discoveries and assumptions
- mbedtls-rs 0.3 exposes no peer-certificate accessor; TLS 1.2 sends the
  Certificate message in clear, so a recording adapter around the socket captures
  exactly the bytes MbedTLS verified (Finished authenticates the transcript).
  Fails closed if no Certificate message is seen (e.g. a TLS 1.3 server).
- Served chain (2026-11-01): leaf celestrak.org (2026-05-27..2026-12-11), DV R36
  (2021..2036), R46 cross-signed by USERTrust (2021..2038); ~5 KB DER. Every served
  certificate must be within its validity at trusted UTC.
- `Catalogue` is documented nonempty (0011), so emptiness lives in the app, not core.
- Default: fixed observer `LIVE_UTC_ISS_OBSERVER` (unchanged site), wide view, wider
  FOV than the ISS image's 5° so several objects can be in view (reversible).
- Fetch UTC comes from the network task's own screened observation (as `live-fetch`).

## Verification

## Open questions
- Observer site: keep the historical equatorial-Pacific site, or a user location later?
