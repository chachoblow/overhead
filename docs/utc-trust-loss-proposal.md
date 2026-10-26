# M6 UTC trust and loss — proposal for review

Status: **largely resolved; kept as the reasoning record.** Accepted outcomes:
[0062](decisions/0062-no-trusted-time-display-preflight.md) no-time display,
[0063](decisions/0063-bounded-utc-holdover-direction.md) holdover direction and
≤100 ms target, [0064](decisions/0064-prototype-sntp-screen-and-leap-boundary.md)
reply screen and leap boundary, [0065](decisions/0065-prototype-drift-bound-and-holdover.md)
`D = 20 ppm`/`H = 3,600 s`, [0066](decisions/0066-prototype-live-sntp-stack-and-source.md)
stack and source. Where the sections below say "proposed", "candidate" or
"unselected", those decisions supersede them. Still open: running the trust
gate and radar together on the device (live device loop).

## Existing boundaries

[0002](decisions/0002-data-and-time-over-wifi.md) requires SNTP on each Wi-Fi
connection and no battery-backed RTC; cached elements alone cannot enable
tracking after cold boot. [0039](decisions/0039-bottom-left-condition-driven-notices.md)
defines `NO FRAME` as the absence of a valid-time frame. [0062](decisions/0062-no-trusted-time-display-preflight.md)
requires old tracking to be invalidated on time loss and a *completed full send*
to confirm replacement pixels. `core::validate_utc_time` checks only the
1957–2100 era and explicit leap-second representation. Neither that check nor
the historical fixture timestamp authenticates a live time source or bounds
clock error. The 8-hour *element-fetch* interval from 0002 is not a UTC
holdover allowance.

## Proposed trust gate

The host-testable gate sits **upstream** of `TimeLossPreflight`. It receives
an externally screened UTC observation paired with a monotonic uptime sample,
plus explicit source-loss/quality events; it never obtains time from the host clock,
fixture, flash timestamp or catalogue. A mere Wi-Fi connection, request sent,
SNTP timeout, or calendar-valid UTC value is not proof of a new sync. The future
adapter must establish response provenance and an error/latency bound for its
UTC–uptime pair. Ordinary SNTP is not cryptographic authentication: the first prototype assumes
a benign network and cannot claim protection from on-path false-time injection.
Exact protocol/client checks and error estimation require separate review
before choosing an implementation. [RFC 4330 §§4–5](https://www.rfc-editor.org/rfc/rfc4330.html)
describes unicast request/reply association, server-mode/stratum, unsynchronized
leap indicator and timestamp checks; [RFC 8633 §§3.2, 4](https://www.rfc-editor.org/rfc/rfc8633.html)
warns that a single source can be wrong and ordinary NTP packets are unprotected
against an on-path attacker. Passing format/replay checks alone cannot prove
the ≤100 ms target: the adapter also needs a justified uncertainty bound for
the source, round trip, timestamp capture and local clock. Public servers
provide no universal error guarantee, and [RFC 8633 §3.7.1](https://www.rfc-editor.org/rfc/rfc8633.html#section-3.7.1)
notes leap smear can offset time from UTC by as much as a second; choose source
behavior accordingly before claiming 100 ms. Leap-indicator handling near a
leap event remains to be reviewed against the engine's explicit-leap-second
rejection; a server's status is not a `NaiveDateTime`. The gate advances its
anchor linearly and cannot represent a leap insertion or deletion during
holdover.

- **Cold boot:** trust starts absent even with persisted elements; no orbital
  frame. Only a freshly accepted time observation can establish the first UTC
  anchor. Reject unsupported year or explicit leap second before acceptance.
- **Tracking:** derive UTC from that accepted pair and monotonic elapsed time;
  resync replaces the pair, never shifts already-rendered frames retroactively.
  [0063](decisions/0063-bounded-utc-holdover-direction.md) accepts an immediate
  fresh-frame reanchor for a source-validated jump even if it exceeds 100 ms;
  no stale tracking frame or old anchor may be reused. It also selects bounded
  holdover after a Wi-Fi disconnect or unanswered resync, not immediate time
  loss. Its duration remains unselected. Reconnecting without a new accepted
  response does not renew it.
- **Loss:** on holdover expiry, source-reported invalid time, backwards uptime,
  or arithmetic impossibility, revoke trust before the next draw. Ask the
  existing preflight to discard app tracking and show `NO FRAME`; no frame or
  retained tracking refresh can outrank that transition. Repeated failures
  cannot reinstate trust. Keep the full-transfer/failed-send rules from 0062.
- **Recovery:** only a new accepted observation reanchors time; if it arrives
  before `NO FRAME` was sent, 0062 allows going directly to a fresh frame.
  Do not silently reuse an old timestamp after a reset or a rejection. Element
  freshness and propagation errors remain separately reported conditions, not
  substitutes for time quality.

The gate must evaluate expiry at **every poll before** allowing a frame or a
retained tracking refresh, including when no frame is due. Use checked monotonic
arithmetic; an expiry boundary is inclusive (at age `H`, trust is absent), and
its result must not depend on the order of a later redraw/transfer. `H > 0` is
an explicit configuration for tests, **not** a chosen device duration. Avoid
turning this proposal into an implicit production default.

## Candidate source-evidence contract (history; see 0064)

**Adapter review (historical; see 0066):** `firmware/Cargo.toml`
then had `esp-hal` for S3 timing but no wireless, UDP, SNTP or ESP-IDF
client. There is therefore no existing client API whose raw packet fields or
capture points we can certify. A future adapter must expose the raw response,
peer address/port and monotonic send/receive captures, or demonstrate equivalent
evidence before it can call the gate's `observe` method. This is an interface
requirement to review, not a dependency recommendation.

For the prototype's benign-network model, propose one outstanding **unicast**
request at a time. A future adapter records monotonic send/receive timestamps
around the packet and a unique request transmit value; it does not need a
preexisting UTC clock to create that value. Accept only a reply from the queried
endpoint with the expected origin value, supported NTP version and server mode,
stratum 1–15, no-warning leap status (LI = 0; LI=1/2/3 revoke under 0063), valid nonzero server receive/
transmit timestamps and coherent server timestamp order. Reject duplicates,
malformed packets and impossible delay/arithmetic; treat stratum 0 as a server
refusal/KoD, **not** a UTC sample, and honor KoD rate/deny behavior. The adapter
must handle the 2036 NTP timestamp-era rollover deliberately rather than
assuming an untrusted cold-boot wall clock disambiguates it. The 32-bit NTP
seconds field wraps every 2³² seconds (~136 years); the supported 1957–2100
calendar range alone does **not** always select one era (e.g., the same seconds
can represent a date in 1957 or 2093). A conservative candidate is to reject
an observation if more than one unfolded era lands in the supported range,
not infer an era from persisted elements or build time. This does not defend
against a false but otherwise unambiguous date on a hostile network. These are proposed
screens, not authentication or proof that a server has correct UTC
([RFC 4330 §5](https://www.rfc-editor.org/rfc/rfc4330.html#section-5),
[RFC 5905 §§7.4, 8](https://www.rfc-editor.org/rfc/rfc5905.html)).

Proposed event distinction: an unrelated, unmatched, duplicate, malformed or
missing reply cannot establish/renew trust; it is **not** evidence that a
previously accepted clock stopped working, so the old anchor ages toward expiry.
A *matched* server reply explicitly declaring itself unsynchronized (LI=3), or
an accepted-source UTC value failing the engine representation check, revokes
trust immediately. Under [0063](decisions/0063-bounded-utc-holdover-direction.md), a well-formed
matched reply reporting synchronized time but insufficient accuracy **does not
renew or revoke** an otherwise still bounded old anchor. At its original
expiry, trust ends as usual; it is never prolonged by this reply. KoD means no new time, not
proof of incorrect old time. No packet is accepted simply because Wi-Fi is up.

The adapter must pair UTC with the **midpoint of monotonic send and receive
captures**, not the end of a slow SNTP call. With a valid reply, its server
transmit timestamp estimates UTC at that midpoint: even without assuming
symmetric outbound/inbound transit, server transmission occurred between the
two client captures, so half the *full* elapsed RTT bounds this part of
midpoint error, assuming a correctly ticking monotonic clock and a bounded
server timestamp. Server turnaround and capture/quantization error still need
to be accounted for; this is not the usual four-timestamp NTP offset formula
with an unsynchronized client clock. A *candidate* conservative
initial-uncertainty estimate in microseconds is

`U0 = ceil(RTT/2) + ceil(max(root_delay, 0)/2) + root_dispersion + C`

where `C` must bound send/receive timestamp capture error, conversions and
source precision/other unaccounted uncertainty; all operations are checked and
rounded *up*. This uses the full RTT instead of claiming symmetric network
paths; root fields are server-reported, not certified. Server turnaround,
server/reference clock quality, leap-smear policy and client stack behavior
must be reviewed before treating this as a defensible total UTC-error bound.
An observation with `U0 >= 100_000 µs` cannot establish the chosen target
*with positive holdover*. A merely syntactically valid packet is insufficient.
RFC 5905's root-distance calculation also includes peer dispersion/jitter;
the simpler candidate above is **not** a substitute for reviewing the actual
client algorithm and measurements ([RFC 5905 §A.5.5.2](https://www.rfc-editor.org/rfc/rfc5905.html#appendix-A.5.5.2)).

Let `D` be an evidenced **upper bound** on monotonic frequency error in ppm
under relevant temperature/operating conditions, not an average from a short
run. The host-test gate requires
`U0 + ceil(age_us * D / 1_000_000) <= 100_000 µs` while tracking and revoke at
an explicit `H > 0` even if the computed error has not exceeded the target;
trust is absent at age `H`. Thus `H` must be chosen so the inequality holds
*strictly before* expiry. For illustration only, `U0 = 20 ms` and `D = 20 ppm`
would leave 80 ms of drift budget and allow **at most 4,000 s** by this model;
these numbers are not S3 measurements, an SNTP result or a device setting.
Without defensible `U0` and `D`, no numerical holdover follows; the prototype values are now set by [0065](decisions/0065-prototype-drift-bound-and-holdover.md).

**Clock-path review, not a bound:** the pinned `esp-hal 1.2.2` source used by
the firmware documents `Instant::now()` as a 1 µs-resolution timer that stops
in sleep; its ESP32-S3 SYSTIMER tick-frequency branch scales XTAL to 16 MHz
(`xtal_clk_frequency() * 10 / 25`). The [S3 hardware design
checklist](https://docs.espressif.com/projects/esp-hardware-design-guidelines/en/latest/esp32s3/schematic-checklist.html)
recommends a selected 40 MHz crystal accurate to ±10 ppm. That recommendation
is **not** a measured or verified tolerance for this DevKit, across temperature,
voltage, aging and operating modes. No S3 `D`, deep-sleep holdover or duration
can be inferred solely from these references. A future measurement/bound must
use the actual clock path/configuration and an independent time reference;
network RTT alone is not oscillator characterization.

### Source-review blockers (leap and era now resolved by 0064)

- **Leap policy:** [RFC 5905 §7.3](https://www.rfc-editor.org/rfc/rfc5905.html#section-7.3)
  defines LI=1/2 as pending insertion/deletion and LI=3 as unsynchronized.
  The current gate cannot advance linearly through a leap and still promise
  ≤100 ms UTC error, so [0063](decisions/0063-bounded-utc-holdover-direction.md)
  now revokes on a matched LI=1/2 for the prototype. That does **not** cover
  an outage spanning a leap without a matched announcement. A separately
  justified leap calendar/exclusion or a different time-scale policy is needed
  before an uninterrupted UTC-error guarantee across leap events. A smearing
  server may advertise LI=0, so LI=0 cannot prove non-smearing
  ([RFC 8633 §3.7.1](https://www.rfc-editor.org/rfc/rfc8633.html#section-3.7.1)).
- **Era and source policy:** review unique-era rejection above and select an
  explicitly known non-smearing source or limit the claim to conditions where
  smear has been ruled out. Neither packet checks nor ordinary unicast SNTP
  prove server correctness ([RFC 8633 §§3.2, 4](https://www.rfc-editor.org/rfc/rfc8633.html)).
- **Timing evidence:** identify actual capture points and server timestamp
  quality, RTT and root-field conversion bounds, local clock drift and
  quantization. Reject any overflow/saturation rather than manufacture a
  small `U0`. The root fields are assertions by the server, not independent
  measurements. No source/oscillator evidence or `H` is established here.

## Host acceptance checklist

The abstract gate tests the time/uncertainty/lifecycle portions below with a
fake monotonic counter; packet-screening and physical evidence still require a
separate adapter review. No networking is exercised:

1. Persisted elements and a valid-looking fixture timestamp cannot enable a
   cold-boot frame. Invalid years and leap seconds cannot establish trust.
2. One accepted UTC–uptime pair enables a fresh frame; uptime progression uses
   elapsed time and missed frame deadlines do not replay old timestamps.
3. Disconnect or failed resync permits frames at age `H−1`, not age `H`;
   reconnect without an accepted observation cannot extend the deadline.
4. Expiry between frame deadlines invalidates app selection/samples and requests
   `NO FRAME` ahead of retained full refresh; a failed full send keeps that
   replacement pending while COM credit follows existing diagnostic rules.
5. Backwards uptime or overflow fail closed; only a new accepted observation
   recovers. A representation-invalid or explicitly invalid-time observation
   must not preserve an older anchor; timeout/no response is not such an event.
6. A large *source-validated* resync jump replaces the old anchor and makes a
   new frame due immediately; it cannot preserve an old sample. Check boundary
   behavior after cold restart and when recovery precedes a pending loss-frame
   transfer; separate element-age failure must not be misreported as UTC loss.
7. Inject request/reply metadata without a network: unmatched, duplicate,
   unsynchronized, KoD, bad timestamps and oversized/overflowing uncertainty
   never establish a fresh anchor. Test the agreed distinction between no
   renewal and revocation while a previous holdover is still valid.
8. Test uncertainty arithmetic at the 100 ms boundary and `H−1`/`H`, and
   account for an in-flight response whose midpoint predates the current poll:
   it may renew only if its **actual age** and uncertainty still fit the bound.
   A jump discards stale app samples before its new-frame attempt; a failed
   transfer does not certify new pixels on the panel.

Host tests would establish state/command intent, not actual SNTP correctness,
clock stability, target scheduling, SPI success, or physical VCOM service.

## Remaining evidence: capture bound, drift and stack (proposal)

**Capture bound `C`.** If the adapter reads uptime *before* handing the request
to the stack and *after* the stack returns the reply, those captures bracket
the actual radio send/receive. Queueing, Wi-Fi retries and asymmetric stack
latency then only lengthen the measured RTT, which the 0064 bound already
counts; they need no separate characterization. `C` reduces to 1 µs floor
resolution plus local drift over the RTT
([`bracketing_capture_bound_us`](../firmware/src/sntp_screen.rs)). This relies
on the server's T2/T3 being correct within its reported root fields, the 0063
trust assumption.

**Budget sensitivity (illustrative, not measured).** A 50 ms Wi-Fi RTT with
~4 ms turnaround, ~10 ms root delay and ~2 ms dispersion gives `U0 ≈ 30 ms`,
leaving ~70 ms for holdover drift: about 7,000 s at 10 ppm or 1,400 s at
50 ppm. `D` dominates `H`; SNTP quality is secondary.

**Proposed drift capture (needs separate build/flash approval).** A separate
opt-in diagnostic image prints `Instant::now()` once per second over the
existing UART for several hours; the host logs each line's arrival with its
NTP-disciplined clock and fits a line. Millisecond USB/host jitter over a
multi-hour baseline resolves well under 1 ppm, with no wireless dependency.
It yields one board's room-temperature rate, **not** a bound across
temperature, voltage or aging. `D` would be that rate plus an explicit,
separately agreed margin, and the host clock's own offset must be logged.

**Drift result (2026-10-26).** The capture above ran for ~3 h: the S3 timer
measured **≈+1.2 ppm** versus UTC, resolved only to a few ppm because the
host reference wandered (see [firmware README](../firmware/README.md#s3-clock-drift-capture-flashed-3-hour-uart-run)).
One board, room temperature, three hours: an observation, not a bound. A
[0065](decisions/0065-prototype-drift-bound-and-holdover.md) sets the prototype
`D = 20 ppm` and `H = 3,600 s`, with margin for temperature, aging and reference
error.

**Stack (dependency not yet proposed for adoption).** The current pinned
`esp-hal 1.2.2` is compatible with `esp-radio 1.0.0-beta.1` (requires
`esp-hal ~1.2.0`, `esp-rtos`, MSRV 1.95; the installed `esp` toolchain is
1.97). A UDP socket via `smoltcp`/`embassy-net` permits bracketing captures
around send/receive and peer address/port checks. It is a pre-release, adds
an RTOS, a heap and a binary radio blob, and is a material change. Explain and
approve it separately before adding; it is not required for the drift capture.

**Source policy candidate.** Use operators documented as non-smearing and
traceable to UTC (e.g. NIST ITS; Cloudflare states it does not smear), never
mixed with a smearing source; avoid Google Public NTP, which smears. Verify
each operator's current statement before adoption; this is a candidate, not
a selection.

## Accuracy illustration, not holdover evidence

At the [checked-in ISS fixture](../core/tests/fixtures/README.md)'s element
epoch `2026-10-04T12:43:41.833056Z`, use `overhead-track` with a fixed observer
at the reported subsatellite point (lat `0.000017409°`, lon `−85.180357315°`,
height `0 km`). The fixture places ISS nearly overhead (~425 km away). Re-run
with the same OMM/observer and UTC offsets of +0.1, +1 and +10 seconds, then
compute the angle between each resulting azimuth/elevation unit vector and
the epoch vector. The host model gives approximately **0.099°, 0.993°, and
9.840°** of line-of-sight motion, respectively; at +1 second the elevation is
~89.007° instead of 90°. This is one near-zenith geometry at an element epoch,
not a worst-case bound, display-pixel displacement, SGP4 accuracy claim, SNTP
error measurement or reason to choose `H`. A tighter-zoom, close-pass device
may make one second noticeable; the user selected **≤100 ms total UTC error**
as a desired product target under 0063, not as a verified measurement.

## Decisions needed before source-adapter implementation

1. **Holdover (resolved for the prototype):** [0065](decisions/0065-prototype-drift-bound-and-holdover.md)
   sets `D = 20 ppm`, `H = 3,600 s`. Live `U0` values still need observing to
   demonstrate the ≤100 ms target end to end.
2. **Source and capture evidence:** packet screens and uncertainty arithmetic
   are settled for the host preflight by [0064](decisions/0064-prototype-sntp-screen-and-leap-boundary.md).
   Still needed: a non-smearing UTC source policy, a UDP/wireless stack whose
   peer address/port and monotonic send/receive capture points support a
   bounded `C` (explain any dependency first), and nonce generation.

Resolved by [0066](decisions/0066-prototype-live-sntp-stack-and-source.md): source
policy and stack, with a one-hour live run in which every reply was accepted and
every consistency check stayed in bound. Next: integrate the trust gate and
radar in a device loop (separate step). No production refresh cadence, text, fetch-staleness
rule, target build or flash follows from this host preflight.
