# 0064 — Prototype SNTP reply screen and leap-boundary holdover

Date: 2026-10-26
Status: accepted for the host preflight (user delegated the choice,
2026-10-26). Refines [0063](0063-bounded-utc-holdover-direction.md); no live
client, dependency, device build or flash.

## Decision

**Leap during holdover.** An anchor never carries tracking across a UTC month
end, the only place [RFC 5905 §7.3](https://www.rfc-editor.org/rfc/rfc5905.html#section-7.3)
leaps occur. Trust ends once projected UTC plus the whole 100 ms target reaches
the next month start (inclusive); an observation already inside that margin
cannot anchor, but does not revoke an older anchor. Only an observation after
the boundary recovers. This covers an outage spanning a leap without relying on
a missed LI announcement, at the cost of a brief `NO FRAME` at an offline month
end. It does not detect a smearing server (LI=0 proves nothing about smear).

**Reply screen** ([`sntp_screen`](../../firmware/src/sntp_screen.rs)), one
outstanding unicast request with a unique nonzero transmit nonce:

- Unmatched origin or short packet: ignored, request stays pending. A matched
  reply consumes it, so duplicates cannot match.
- VN≠4 or mode≠4, zero T2/T3, negative or >1 s root delay/dispersion,
  turnaround > RTT, receive before send, overflow: ignored (no renewal).
- Stratum 0: KoD — no time, no revocation; adapter honors rate/deny.
- LI=3 or stratum ≥16: unsynchronized — revoke. LI=1/2: revoke (0063).
- NTP era: reject if both era 0 and era 1 fall in the supported 1957–2100
  range (only 1957–1964 vs 2093–2100); no inference from build time or
  persisted elements.
- UTC at the client midpoint is the server midpoint (T2+T3)/2; uncertainty is
  `ceil((RTT−τ)/2) + ceil(root_delay/2) + root_dispersion + 2 µs + C`, all
  rounded up, where τ = T3−T2 and `C` is adapter-supplied capture evidence.
  Because the server interval lies inside the client interval, this needs no
  path-symmetry assumption.

Peer address/port matching, nonce generation and `C` belong to the future
adapter. Root fields are server assertions, so the bound inherits the 0063
benign-network trust model.

## Consequences

No drift bound `D`, holdover `H`, measured `C`, source selection (including
non-smearing policy), dependency or live integration follows. The ≤100 ms target
remains undemonstrated.
