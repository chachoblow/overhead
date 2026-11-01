# M6 HTTPS element fetch — proposal

Status: **accepted by the user 2026-11-01 as [0071](decisions/0071-https-element-fetch-first-slice.md)** (all four decisions below: yes). Slices 1–3 are done; the on-board fetch ingested 20/20 records ([evidence](../firmware/README.md#https-element-fetch-on-the-board-flashed-one-boot-uart-capture)). Slice 2 found the MbedTLS wall clock needs a C toolchain, so the diagnostic runs without date checks under [0073](decisions/0073-diagnostic-fetch-without-certificate-dates.md). It plans the first slice of the M6 item "Wi-Fi credentials, HTTPS elements, SNTP, persistence, and refreshes" under [0002](decisions/0002-data-and-time-over-wifi.md) and [0003](decisions/0003-curated-catalogue.md).

## Why now

SNTP time is solved for the prototype ([0062–0067](decisions/0062-no-trusted-time-display-preflight.md)); elements are not. Every device image so far tracks checked-in OMMs, so live UTC does not make positions accurate. M6 is done when the prototype "boots with valid time/data", and the remaining M6 items (live multi-satellite cycling, stale-element handling, persistence, refresh) all need fetched, dated elements first.

## What the source requires (measured 2026-11-01 UTC from devbox)

- **Endpoint.** `https://celestrak.org/NORAD/elements/gp.php?GROUP=STATIONS&FORMAT=JSON` returned HTTP/1.1 200 with `Content-Length: 8441`: **20 OMM records**, epochs within the previous day. The same group as CSV was 3,253 bytes. One GET of each was made by hand; the bodies are scratch files, not committed.
- **TLS.** `celestrak.org` negotiated **TLS 1.2 only** (`ECDHE-RSA-AES256-GCM-SHA384`); a TLS 1.3-only handshake failed. The leaf is **RSA-2048**, `sha256WithRSAEncryption`, valid until **2026-12-11**. The chain is leaf → Sectigo Public Server Authentication CA DV R36 → Sectigo Public Server Authentication Root R46 (served cross-signed by USERTrust RSA Certification Authority).
- **Usage policy** ([CelesTrak GP documentation](https://celestrak.org/NORAD/documentation/gp-data-formats.php)): data is checked every 2 hours, so download at most once per update. A repeat download before new data can return **403** ("GP data has not updated since your last successful download"); enforcement started with the Active and Starlink groups on 2026-03-26. More than **50 HTTP errors (301/403/404) in 2 hours** sends the IP to the firewall. 403/404 should stop and report, never retry. 0002's ~8-hour refresh is well inside this.

## TLS stack: recommend `mbedtls-rs` (esp-rs)

| Option | Fit |
|---|---|
| **[`mbedtls-rs`](https://github.com/esp-rs/mbedtls-rs) 0.3.0** + `mbedtls-rs-sys` | **Recommended.** Maintained by esp-rs; TLS 1.2 and 1.3; RSA and ECDSA. Ships a **prebuilt MbedTLS library for `xtensa-esp32s3-none-elf`**, so no clang/cmake or toolchain change. Hardware hooks for SHA, AES and RSA modular exponentiation. Its ESP example uses the same published stack as ours (esp-hal ~1.2, esp-rtos 0.4, esp-radio 1.0.0-beta, embassy-net 0.9, esp-alloc 0.11), and it needs `embedded-io(-async)` 0.7 and `rand_core` 0.10, which are already in our lockfile. |
| `embedded-tls` 0.19 (drogue) | **Ruled out:** TLS 1.3 only, so it cannot connect to CelesTrak today. RSA also needs its `alloc` feature. |
| `reqwless` 0.14 HTTP client | Not needed. Its TLS backends are `embedded-tls` and the older unpublished `esp-mbedtls`, not `mbedtls-rs`. One fixed GET is simpler to write and host-test ourselves. |
| esp-idf (`esp-idf-svc`) | Ruled out: it replaces the bare-metal esp-hal/esp-rtos stack the existing images use. |
| Plain HTTP | Rejected: 0002 specifies HTTPS, and plain HTTP gives no integrity or authenticity for data that drives the display. |

What it would add: `mbedtls-rs` and `mbedtls-rs-sys`, including a **prebuilt C static library** (an opaque binary, like esp-radio's Wi-Fi blobs), the `tcp` feature on the existing `embassy-net`, and possibly a small libc shim (the upstream example links `tinyrlibc`; we need to check whether our build needs it). All of this would sit behind a new opt-in feature, so the default and existing images would not change. The upstream example pins esp-hal to a git revision, so compatibility with our exact `=1.2.2`/`1.0.0-beta.1` pins is **unverified** until we build it.

## Trust and time

- Verify the full chain and the hostname. Bundle **the root(s) only**, never the leaf, which rotates by 2026-12-11. Start with Sectigo Public Server Authentication Root R46 and USERTrust RSA Certification Authority. A root change becomes a reflash, and a failed verification should be reported as such.
- Certificate validity needs a wall clock. Fetch only **after** 0065's trust gate holds a trusted UTC, and feed MbedTLS's wall-clock hook from that screened UTC, not an unsynchronised RTC. The 0062 `NO FRAME` path covers the time before that.
- Entropy comes from esp-hal's TRNG with the radio running, as in the upstream example.

## HTTP and parsing

- One request per connection: an `HTTP/1.0` GET for a fixed path, with `Host` and an identifying `User-Agent`, and the connection closed afterwards. HTTP/1.0 avoids chunked responses.
- A **host-tested response parser** in `firmware/src/`: status line, headers, and a required `Content-Length` within a fixed cap (16 KiB proposed, about twice today's STATIONS body). Any other status, an oversized or missing length, or a truncated body becomes a typed failure. The body goes to the existing `overhead-core` `omm`/`catalogue` ingestion, so no new parser is needed and per-record diagnostics already exist.
- Keep the downloaded JSON only for the parse, then drop it. Persistence is a later step.

## Proposed slices

1. **Host-only.** Write the response parser and its tests: 200, 403 with policy text, 404, 301, a length over the cap, truncation, bad JSON, and partial-record failures through the catalogue. Optionally pin one STATIONS JSON snapshot as a firmware fixture (public data, recorded in the fixtures README).
2. **Build-only spike.** Add the feature and dependencies and build a target image with dummy credentials. Report image size, any new libc shim, linker warnings and the offline gate. If `mbedtls-rs` won't build against our pins, stop and report back rather than changing toolchains or bumping pins.
3. **Fetch diagnostic image `live-fetch`.** No display. Boot, join Wi-Fi, get trusted UTC through the existing SNTP path, then make **exactly one** HTTPS GET. Print the TLS result, HTTP status, byte count, the parsed record count and epoch range, handshake and transfer times, and the heap high-water. No retry: any failure stops for the rest of the boot. Each run costs one CelesTrak request, so keep runs to a few per 2 hours, well under the error limit.
4. **Later, each needing its own decision or approval:** compose fetched elements into the live-UTC radar, where BOOT cycling becomes visible on live data; persist elements and fetch time to flash; the 8-hour refresh with boot-time age check; stale-element and failure behavior (the third M6 item); and group choice beyond STATIONS.

Slices 1–3 together are about one or two sessions. Flashing the slice-3 image follows [0069](decisions/0069-flash-without-per-flash-approval.md) (confirm port, run the gate).

## Memory

The live-UTC image has a 96 KiB heap. The upstream `mbedtls-rs` ESP example allots **140 KiB**. MbedTLS's receive buffer must hold a full 16 KiB TLS record unless the server negotiates a smaller one, which we can't count on. The slice-3 diagnostic has no display, so it can size its heap separately and **measure** the peak. Combining fetch with the display image is a slice-4 question. The board's 8 MB PSRAM is unused and is a fallback, not part of this proposal.

## Risks and open questions

- An RSA-2048 TLS 1.2 handshake on the S3 may take seconds. That's fine for a diagnostic, but slice 4 must keep the display owner servicing COM while a fetch awaits (the 0067 channel pattern).
- CelesTrak may extend download enforcement to STATIONS or change TLS/certificates. A 403 must be reported, never retried.
- Wi-Fi association already causes a ~34 ms BOOT sampling gap (see HANDOFF); TLS work may add others. That's measured in slice 4, not here.
- Credentials still come from the user's local `firmware/.wifi.env` at build time. Provisioning stays out of scope.

## Decisions (accepted 2026-11-01)

1. **Accept `mbedtls-rs` 0.3** (with its prebuilt MbedTLS library and possibly `tinyrlibc`) behind an opt-in feature?
2. **STATIONS JSON as the first fetch target**, with the group mix left as later tuning under 0003?
3. **Scope slices 1–3 only** (no persistence, refresh or display composition), with one request per boot and no automatic retry?
4. **Pin a STATIONS snapshot as a host fixture?** I recommend yes, for parser tests on real bytes.
