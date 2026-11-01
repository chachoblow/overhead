# BOOT sampling gaps (host-only review)

Date: 2026-11-01 UTC. Scope: [0068](../decisions/0068-boot-selection-gesture.md)'s
provisional 20 ms completed-press filter as used by the two BOOT diagnostic
images. No target trial, image change or new timing measurement; figures below
reuse earlier captures and are **estimates** for these loops, not bounds.

## How the loops sample

Both loops read GPIO0 once per iteration, then block in render/SPI. Nothing
latches a press while they block.

- `sharp_boot_cycle_main.rs`: about one sample every 5 ms while idle. A
  ~52 ms retained full transfer (measured 51.7 ms on the natural-speed audit)
  repeats about every 0.55 s for VCOM. After each completed press, an uncached
  render plus a full transfer block for roughly 190 ms. That figure comes from
  the ISS replay's ~141 ms render; this 16-object scene was never measured.
- `live_utc_main.rs` (`live-utc-boot`): a sample before each 50 ms-grid frame,
  then 5 ms awaits. One partial frame measured ~31.6 ms total, so gaps are about
  37 ms. Full data frames (~82 ms) and retained refreshes (~52 ms) widen them.
  Wi-Fi and executor stalls are unmeasured.

## What the filter guarantees under sparse samples

`CompletedPress` qualifies a level change after **two samples at least 20 ms
apart** that both read the new level. The sample spacing S, not the 20 ms
interval, therefore sets the minimum press. Host tests in
`firmware/src/button_input.rs` pin these results for clean presses (bounce
included in one test):

| Schedule (model) | Always registers | Can be missed |
|---|---|---|
| Regular S = 5 ms (boot-cycle idle) | P ≥ 25 ms | P < 25 ms |
| Regular S ≈ 37 / 57 / 87 ms | P ≥ 2S (≈ 73 / 113 / 174 ms) | P just under 2S |
| 5 ms with one 52.4 ms gap (boot-cycle VCOM) | P ≥ gap + 20 + 5 ≈ 77 ms | press entirely inside the gap |
| Live-UTC partial-frame cycle (0, 36.6, 41.6, 46.6 ms of each 50 ms) | P ≥ 90 ms | 60 ms |

Failures are **misses or merges**, never extra selections. A release hidden
inside a gap merges two taps into one event. Sparse sampling and contact chatter
never emitted more than one event per press in any modelled phase.

## Consequences

- Deliberate presses on the boot-cycle image are fine, which matches the
  user-confirmed captures. Quick taps can be lost about 10% of the time, when
  they land in the VCOM transfer. A second tap within about 0.2 s of a
  selection redraw is likely lost or merged.
- In the live-UTC loop, any press shorter than about two frame periods is
  best-effort. That limit worsens as render cost grows.
- The worst-case delay from release to a visible change is roughly: remaining
  blocking work + the 20–25 ms release qualification + the next render/send.
  On the boot-cycle image this is about 0.27 s.

## Recommendation

Do not tune the 20 ms value or the 5 ms wait to fix this; the sample gaps
dominate both. For the standalone device, capture BOOT
independently of render/SPI, as proposed in
[0070](../decisions/0070-boot-input-capture-independent-of-render.md)
(timer-sampled, keeping 0068 semantics).

A physical validation of the current polling loop is **not** recommended. Press
durations have no ground truth without instrumentation, so a test could only
re-observe the qualitative misses modelled here. Revisit a bounded protocol
once the input-capture approach is decided.
