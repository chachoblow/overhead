# 0090 — SNTP falls back to the next server after an unusable reply

Date: 2026-11-23
Status: accepted (user, 2026-11-23: proposed in the full code review's
UTC-trust pass, accepted as written).
Amends [0080](0080-sntp-kod-backoff-and-fetch-independence.md) item 2 for
the shared runtime (`firmware/src/live_radar.rs`). [0063](0063-bounded-utc-holdover-direction.md)'s
revocation rules and [0065](0065-prototype-drift-bound-and-holdover.md)'s
bounds are unchanged.

## Decision
Within one poll, the servers (Cloudflare, then NIST) are asked in order
until one gives a **usable** reply: matched, screened into an observation
and under the 100 ms budget. Every other answer falls through to the next
due server:

- no reply, or a KoD (as 0080 item 2);
- **new:** a matched reply that gives no usable observation, whether it
  reports itself unsynchronized (stratum 16 or LI=3), announces a leap
  (LI=1/2), fails the screen (zero timestamps, root delay or dispersion
  out of range, impossible timing) or is over the budget.

A matched unusable reply still counts as a reply for that server's KoD
back-off (its interval halves towards 64 s), and an unsynchronized or
leap-pending reply still revokes trust (0063) before the next server is
asked. A usable reply from the next server can then restore it in the same
poll. The rule lives in `sntp_policy::Policy::answer` and is host-tested.

## Why
Under 0080 a matched reply ended the poll whatever it said. While
Cloudflare kept answering with something unusable, NIST was never asked:
the device could not renew UTC and, once holdover lapsed (or at once, for
an unsynchronized reply), showed `NO FRAME`, the single-server stall 0080
set out to remove. The two servers are independent operators, so a bad
answer from one is no reason not to ask the other; 0080 already accepted
this for KoD and silence.

Leap safety does not depend on which server is believed: holdover never
crosses a UTC month boundary (0064), so a leap one server announced and
the other did not cannot add error to a trusted anchor.

Rejected: retrying the same server within the poll (it is likely to
answer the same way and would bypass its own rate); counting an unusable
reply as no reply for back-off (a server that answers is not silent, and
0080 halves the interval on any matched reply).

## Consequences
A poll may now ask NIST more often: whenever Cloudflare's reply is
unusable, not only when it is silent or kissing. NIST is still asked at
most once per poll and only when due, so its own KoD back-off is honoured.
Not yet exercised on the board; a fault-build switch that makes one
server's replies unusable would show it.
