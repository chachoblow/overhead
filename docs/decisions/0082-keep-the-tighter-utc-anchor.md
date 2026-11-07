# 0082 — The UTC trust gate keeps the tighter anchor

Date: 2026-11-07
Status: accepted (user, 2026-11-07: fix inside the 0081 work item).
Amends [0063](0063-bounded-utc-holdover-direction.md)'s renewal rule (and
its 2026-11-03 small-resync note) and how [0080](0080-sntp-kod-backoff-and-fetch-independence.md)
picks the observation that dates fetches. [0065](0065-prototype-drift-bound-and-holdover.md)'s
bounds are unchanged.

## Decision
The error bound of an observation at time `now` is its uncertainty plus
drift over its age (20 ppm, rounded up, as 0065). When a new in-budget
observation arrives while an anchor is still trusted:

- if the anchor's bound at `now` is **strictly smaller** than the new
  observation's, the anchor is kept: `NoRenewal`, and the anchor keeps its own
  expiry;
- otherwise (equal or tighter) the new observation renews as before: a step
  within 100 ms is `Resynced`, a larger one `Accepted` with samples cleared.

The runtime's `latest` observation, which dates element fetches (0080), is
chosen by the same rule (`UtcTrustPreflight::tighter`). Unchanged: no anchor or
an expired one is replaced by any in-budget observation; LI/stratum
revocation, holdover cap and leap boundary.

## Why
On 2026-11-07 (Cloudflare held off, NIST only;
[evidence](../../firmware/README.md#nist-fallback-renews-utc-0081)) one NIST
reply carried 99,906 µs uncertainty, just under the 100 ms budget. The gate
adopted it over a 23.9 ms anchor, it lapsed about 4.7 s later, and the board
showed `NO FRAME` for 60 s until the next reply, clearing samples and
selection. Both observations were valid, so taking the one with the smaller
bound gives a bound that is at least as good for longer. This applies to any
server whose reply is delayed and may have contributed to the unobserved
2026-11-06 `NO FRAME` (unproven).

Rejected: a fixed acceptance margin (for example, uncertainty ≤ 50 ms), which
would discard replies the bound admits and still shorten trust when a 49 ms
reply replaces a 20 ms anchor; intersecting the two intervals, which is tighter
but would need a new anchor representation and evidence for it.

## Consequences
A worse reply that also disagrees with the anchor by more than 100 ms is now
ignored rather than adopted as a jump, as long as the anchor stays trusted.
Under 0063's benign-network model both are valid bounds, so this does not
weaken the guarantee. A server that is consistently worse only renews once
the anchor's bound has grown past its own.
