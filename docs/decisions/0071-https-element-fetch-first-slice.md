# 0071 — First HTTPS element fetch: mbedtls-rs, STATIONS, one request per boot

Date: 2026-11-01
Status: accepted (user, 2026-11-01). Slice 1 (host parser, ingestion and
fixture) implemented 2026-11-01; slices 2–3 not started. Research, measurements
and alternatives are in the [element-fetch proposal](../element-fetch-proposal.md).

## Decision

Start [0002](0002-data-and-time-over-wifi.md)'s element fetch with the bounded
slices 1–3 of the proposal:

1. **TLS:** use [`mbedtls-rs`](https://github.com/esp-rs/mbedtls-rs) 0.3 with
   `mbedtls-rs-sys`'s prebuilt MbedTLS library for `xtensa-esp32s3-none-elf`,
   behind a new opt-in firmware feature. Also accepted: the `tcp` feature on the
   existing `embassy-net`, and a small libc shim such as `tinyrlibc` if the
   build turns out to need one. Default and existing images stay unchanged. If
   it does not build against the current esp-hal/esp-radio/esp-rtos pins, stop
   and report back; do not change toolchains or bump pins without asking.
2. **Source:** CelesTrak `GROUP=STATIONS&FORMAT=JSON` as the first and only
   fetch target. The group mix stays later tuning under
   [0003](0003-curated-catalogue.md).
3. **Request:** one `HTTP/1.0` GET per boot, written in-repo, with a host-tested
   response parser that requires `Content-Length` within a 16 KiB cap. The body
   goes through the existing `overhead-core` `omm`/`catalogue` ingestion. Any
   TLS, HTTP or parse failure stops fetching until reboot: **no automatic
   retry**, since CelesTrak firewalls IPs after 50 errors in 2 hours.
4. **Trust:** verify the full chain and hostname against bundled roots only
   (Sectigo Public Server Authentication Root R46 and USERTrust RSA Certification
   Authority), never the leaf. Fetch only after
   [0065](0065-prototype-drift-bound-and-holdover.md)'s gate holds trusted UTC,
   and give MbedTLS that screened UTC as its wall clock.
5. **Evidence image:** the fetch diagnostic `live-fetch` has no display. It
   reports the TLS result, HTTP status, bytes, parsed records, epoch range,
   timings and heap high-water. Development runs stay to a few per 2 hours.
6. **Fixture:** pin one STATIONS JSON download as a firmware host fixture, with
   its provenance in the fixtures README.

## Why

CelesTrak serves only TLS 1.2 with an RSA-2048 certificate (measured
2026-11-01 UTC), which rules out the TLS 1.3-only `embedded-tls`. `mbedtls-rs`
is maintained by esp-rs, needs no toolchain change for the S3, has hardware
hooks, and its ESP example uses the same published stack as ours. A
display-free image measures handshake time and memory before anything competes
with the display owner.

## Boundaries

This does not decide persistence, refresh cadence, stale-element behavior,
composing fetched elements into the radar, credentials provisioning, or the use
of PSRAM. Each needs its own decision. It does not change
[0067](0067-opt-in-live-utc-iss-radar-scope.md)'s fixture-only live-UTC image or
any frozen M2 evidence.
