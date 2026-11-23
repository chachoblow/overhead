# 0091 — Setup idle time counts requests, not links

Date: 2026-11-23
Status: accepted (user, 2026-11-23: proposed in the full code review's
firmware pass, accepted as written).
Amends [0087](0087-ios-setup-app-over-ble.md)'s "left … after 10 min with no
client when settings exist" (`firmware/src/setup_mode.rs`). The setup
window's reasons, pairing and protocol are unchanged.

## Decision
- **Idle means no answered request.** With settings stored, the window
  restarts on them once 10 min (`SETUP_IDLE_S`) pass since it opened or since
  a request was last answered, whether or not a phone is connected.
  Connecting or dropping a link does not reset the clock. A request being
  handled (a scan or a test join) is never cut short: the clock is checked
  only between requests, and the answer resets it.
- **Unpaired links are dropped.** A link that has not paired within 60 s
  (`PAIR_TIMEOUT_S`) of connecting, or of its last passkey being shown, is
  disconnected and the device advertises again. This applies with or without
  stored settings.
- The rules are `setup_policy::Idle` and host-tested; `setup_mode` waits on
  the next BLE event or the next expiry, whichever comes first.

## Why
Under 0087 the idle clock ran only between connections, and a session had no
time limit. A configured device that fell into setup (most likely a
`JoinTimeout` after a power cut, with the router slower to return than the
device) stayed there for as long as any phone held a link: the recipient's
phone with the setup app left open, or any nearby BLE scanner that connects
and never pairs. The radar did not come back until the link dropped, the
stranding 0087's idle rule set out to prevent. An unpaired link also kept
other phones from connecting, since the device serves one link at a time.

Only an answered request shows someone is setting the device up: requests
need a passkey-paired link, so a stranger cannot reset the clock. Letting a
link (connected or dropped) reset it would let a phone that reconnects every
minute hold the window forever.

Rejected: a session time limit regardless of activity (could cut off a
recipient mid-setup); keeping the clock paused while paired (a paired phone
left idle still strands the device).

## Consequences
- A recipient who pairs and then leaves the app untouched for 10 min on a
  configured device sees it restart into the radar; the app reconnects only
  if setup opens again. Unconfigured devices still wait indefinitely.
- A user who mistypes the passkey has 60 s from the latest passkey to retry.
- Not yet exercised on the board: an iPhone left connected without pairing,
  and a paired idle phone on a configured device, would show both rules.
