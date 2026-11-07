# 0080 — SNTP Kiss-o'-Death backs off per server; fetches no longer wait for replies

Date: 2026-11-06
Status: accepted (user, 2026-11-06: per-server RATE doubling to ~1 h, 24 h
DENY/RSTR block, fetch on uptime with trusted holdover UTC, fault-build
evidence, no new logging).
Replaces [0067](0067-opt-in-live-utc-iss-radar-scope.md)'s "on a matched KoD,
stop querying both servers until reboot" for the shared runtime of
[0076](0076-device-app-build.md) (`firmware/src/live_radar.rs`).

## Decision
1. **KoD is per server and bounded** (RFC 4330 §8, RFC 5905 §7.4). Each
   server has its own poll interval, starting at the 64 s poll.
   - `RATE`, and any kiss code other than `DENY`/`RSTR`: that server's
     interval doubles, capped at 3,600 s, and it is not asked again until
     the interval has passed. Each matched reply from it halves the interval
     again, down to 64 s, so a server that just rate-limited us is not
     immediately polled at full rate.
   - `DENY`/`RSTR`: that server is not asked for 24 hours.
   - No KoD stops queries until reboot.
2. **Fallback after a KoD is allowed.** Within one poll, a server that is
   not yet due, gave no reply or sent a KoD is followed by the next one that
   is due (Cloudflare, then NIST). The two are independent operators, so
   asking the other one does not bypass the first one's rate or deny.
3. **Fetches run on uptime.** The 0075 schedule is checked on every poll,
   whether or not any server answered. A due fetch is dated by the newest
   in-budget observation, projected to now under 0065's prototype bounds
   (`utc_trust::projection_trusted`: 20 ppm, 1 h holdover, 100 ms total,
   no month boundary crossed). If no observation is still trusted, the fetch
   waits for the next one. The stored copy (0079) is still dated only by a
   fresh observation.
4. **Logging.** The KoD event line names the server, code and next query
   time; the device app prints no other new line (0076).

## Why
The 0067 rule was an experiment-scope shortcut: one KoD parked the network
loop forever, and because fetches ran from that loop the device stopped
refreshing and, once holdover lapsed, showed `NO FRAME` until a reboot
(lead for the 2026-11-06 unobserved `NO FRAME`; unproven). Rejected:
resetting a server's interval to 64 s on its first reply (invites an
endless RATE cycle); treating unknown kiss codes as ignorable (RFC 5905
leaves them to the client; backing off is the safe reading); dating fetches
by the uptime clock alone (the fetch time sets `OLD DATA`/expiry, so it must
come from trusted UTC).

## Consequences
With both servers denied, the device keeps tracking until 0065's holdover
lapses, then shows `NO FRAME` until a block ends (at most 24 h) or a reboot.
Back-off state is not persisted: a reboot starts every server at 64 s, as
before. Evidence and the fault-build switches (`FAULT_KOD`,
`FAULT_KOD_BLOCK_MIN`) are in the [firmware README](../../firmware/README.md#sntp-kod-recovery-0080).
