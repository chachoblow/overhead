# 0066 — Prototype live SNTP stack and time source

Date: 2026-10-26
Status: accepted by the user, 2026-10-26. Builds on
[0063](0063-bounded-utc-holdover-direction.md)–[0065](0065-prototype-drift-bound-and-holdover.md).

## Decision

**Stack.** Live network code uses `esp-radio 1.0.0-beta.1` (Wi-Fi, WPA/WPA2 only),
`esp-rtos 0.4.0`, `embassy-executor 0.10`, `embassy-net 0.9` (DHCP, DNS, raw UDP)
and `embassy-time 0.5`, compatible with the pinned `esp-hal 1.2.2`. They are
optional, enabled only by opt-in features; existing images and pinned versions
are unchanged. No `sntpc`: the adapter sends its own request and passes the
whole reply, peer endpoint and bracketing uptime captures to
[`sntp_screen`](../../firmware/src/sntp_screen.rs).

**Source.** `time.cloudflare.com` first, `time.nist.gov` as fallback only when
the first yields no reply. Both state they do not smear leap seconds; never add
a smearing source (e.g. Google Public NTP). Ordinary unicast SNTP; 0063's
benign-network model applies.

**Credentials.** Build-time `SSID`/`PASSWORD` from the ignored
`firmware/.wifi.env`. They are embedded in the local ELF; never commit or share
such an ELF. Provisioning for the handheld device is a separate future choice.

## Why

The approach was proven on target: a one-hour live run
([evidence](../../firmware/README.md#live-sntp-source-evidence-flashed-1-hour-uart-run))
accepted every reply with initial uncertainty 21–58 ms (median 28 ms), and every
consecutive pair agreed within its combined bound. That makes the ≤100 ms target
plausible with 0065's 20 ppm / 3,600 s, in this home-network setting.

## Consequences

`esp-radio` is a pre-release with closed Espressif Wi-Fi drivers; revisit on
upgrade. WPA3-only networks are unsupported by this version. The diagnostic is
not the device app: no trust gate on target, radar integration, element fetch,
persistence, reconnection policy or production cadence follows.
