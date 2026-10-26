# 0063 — Prototype UTC trust, bounded holdover and validated resync

Date: 2026-10-26
Status: accepted prototype directions and UTC-error target; source checks, drift/holdover and source refined by 0064–0066

## Decision

After a newly accepted, externally validated UTC observation, a Wi-Fi disconnect
or an unanswered resync alone does **not** immediately invalidate tracking.
Continue from the accepted UTC–monotonic anchor only for an explicitly bounded
holdover; reconnecting without a new accepted observation does not extend it.
Aim for **≤100 ms total UTC error while tracking**, including uncertainty at
the accepted sync and drift during holdover. This is a desired accuracy budget,
not measured S3 clock performance or a verified device guarantee; choose the
actual holdover only after an error bound can support it. At the agreed expiry
boundary, revoke time trust and use [0062's existing `NO FRAME`
transition](0062-no-trusted-time-display-preflight.md). Cold boot has no
holdover: cached elements or a saved timestamp cannot establish UTC after a
power cycle. If a **new source-validated** UTC observation differs from the
old monotonic-derived UTC by more than the desired 100 ms budget, discard the
old anchor and schedule a fresh frame immediately at the new time. Do not keep
sampling from the old anchor, reuse a stale tracking image as a new frame, or
require a second corroborating response solely because of the jump. As in 0062,
if fresh time is already available before a loss-frame send, an intermediate
`NO FRAME` transfer is not required. This is not permission to accept an
unvalidated or malformed SNTP response. These choices refine [0002's
Wi-Fi/SNTP direction](0002-data-and-time-over-wifi.md) without changing its
sync-on-each-connection requirement.

For the **first prototype**, a screened *unicast SNTP* reply may supply that
observation. The client must associate it with its own request, reject invalid
server/time status and evaluate uncertainty before passing a UTC–uptime pair to
the preflight; a connected Wi-Fi interface or received packet alone is not a
sync. This is an operational, benign-network trust model, **not cryptographic
authentication**. An on-path attacker can still alter ordinary NTP packets
([RFC 8633 §4](https://www.rfc-editor.org/rfc/rfc8633.html#section-4)).
A matched reply reporting synchronized time but too uncertain to meet the
accuracy target **does not renew** the anchor; the previously accepted anchor
may continue only until its original bounded expiry. This differs from an
explicitly unsynchronized matched response, which revokes trust immediately.
For the prototype, a matched reply announcing a pending leap insertion or
deletion (LI=1/2) also **revokes trust immediately** and cannot establish a new
anchor: the linear holdover model cannot carry the ≤100 ms target through a
leap. This does not handle an outage spanning a leap without a matched
announcement; that case remains open. LI=0 does not prove a non-smearing source.
Source checks and error estimation must be selected and tested before any
implementation claims the desired ≤100 ms budget.

## Why

A connection failure is not itself evidence that the monotonic clock has
stopped, and the device is intended to track between connections. Conversely,
without a battery-backed RTC, cold boot cannot safely recover UTC from storage,
and unlimited holdover would turn unmeasured drift into unbounded position error.
The roughly eight-hour *element-fetch* cadence is not a time-quality bound.

## Boundaries

No numerical holdover, measured drift, validated 100 ms guarantee, SNTP
response acceptance *implementation*, cryptographic security claim, production
timer, device loop or hardware behavior is accepted here. Exact request/reply
checks, uncertainty estimation and drift/holdover questions remain in the
[UTC trust/loss proposal](../utc-trust-loss-proposal.md). An observation
explicitly judged invalid is different from no response; its handling is still
specified by the existing rejected-time preflight and the eventual source
contract. A separately authorized [host-only gate](../../firmware/src/utc_trust.rs)
uses explicit test inputs; it establishes no measured time quality, source
validation or device holdover; [0065](0065-prototype-drift-bound-and-holdover.md)
later sets prototype drift/holdover values. No device build follows from this direction.
