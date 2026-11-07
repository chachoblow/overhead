# 0081 — The SNTP screen accepts version-3 server replies

Date: 2026-11-07
Status: accepted (user, 2026-11-07: accept VN=3 rather than replace NIST).
Amends [0064](0064-prototype-sntp-screen-and-leap-boundary.md)'s reply
screen; keeps [0066](0066-prototype-live-sntp-stack-and-source.md)'s sources.

## Decision
A matched server reply (mode 4) passes the version check when VN is **3 or
4**. Every other VN, and every mode other than 4, is still
`Ignored(BadVersionOrMode)`. Requests stay VN=4. All other 0064 rules (KoD,
LI/stratum revocation, root ranges, era, uncertainty bound) apply unchanged
to both versions.

## Why
`time.nist.gov` answers a VN=4 request with VN=3 (devbox query 2026-11-07:
132.163.96.4, LI 0, VN 3, mode 4, stratum 1, origin echoed). Under 0064
this rejected most NIST replies (39 of 44 in the 0080 run), so the 0066
fallback seldom renewed and Cloudflare was effectively the only source; an
outage past the 1 h holdover would show `NO FRAME`.

The NTPv3 (RFC 1305) and v4 (RFC 5905) headers have the same layout and
meaning for every field the screen reads (LI, mode, stratum, root
delay/dispersion as unsigned 16.16, reference ID, origin/receive/transmit
timestamps), and RFC 4330 SNTP clients interoperate with v3 servers, so the
bound and the trust model are unaffected.

Rejected: replacing NIST with another server (needs a new non-smearing,
independently operated source to be found and checked; gains nothing over
NIST, which states it does not smear); accepting any VN (VN 1–2 headers
differ, and 0 or 5–7 are not defined replies).

## Consequences
NIST becomes a working fallback. Board evidence (Cloudflare held off by
the 0080 fault build for longer than the holdover) is in the
[firmware README](../../firmware/README.md#nist-fallback-renews-utc-0081).
