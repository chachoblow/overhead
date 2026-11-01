# 0070 — Capture BOOT input independently of render/SPI

Date: 2026-11-01
Status: accepted (user, 2026-11-01); implemented as the opt-in
`sharp-boot-capture` diagnostic; physical tap session passed 2026-11-01
([capture](../../firmware/README.md#interrupt-sampled-boot-capture-diagnostic-flashed-user-tap-session)).
Adopted in the `live-utc-boot` image on TIMG1 via the shared
`firmware/src/boot_capture.rs` (2026-11-01); its physical tap session passed
the same day: 44 presses, 44 single-press events
([capture](../../firmware/README.md#opt-in-boot-selection-on-the-live-utc-diagnostic-interrupt-capture-user-tap-session)).

## Problem

The BOOT diagnostics read GPIO0 only between blocking render and SPI work. The
[sampling-gap review](../evaluations/boot-sampling-gaps.md) shows that the
blind windows, not [0068](0068-boot-selection-gesture.md)'s provisional 20 ms
filter, set the shortest press that is sure to register: about 25 ms when idle,
about 77 ms around a VCOM transfer, and about 90 ms in the modelled live-UTC
loop. Presses are missed or merged; extra selections never occur. Blind windows
will grow with more objects, Wi-Fi and higher frame rates. The knob intended
under [0060](0060-defer-timing-to-physical-prototype.md) produces short encoder
pulses, so it would drop steps under the same polling.

## Proposal

Separate noticing input from acting on it:

1. **Capture:** a periodic hardware-timer interrupt samples GPIO0 every 5 ms and
   feeds the existing `CompletedPress` filter. This runs regardless of render,
   SPI or executor state. It needs a timer not already used by esp-rtos or
   Wi-Fi.
2. **Hand-off:** the interrupt only counts completed presses into a small
   critical-section-protected counter. It never touches the app, renderer or
   panel.
3. **Act:** the display loop drains the counter at its existing boundary and
   applies 0068 unchanged: one `select_next()` per completed press, no hold
   repeat, and presses completed without a valid frame are consumed, not
   queued. Several presses drained at once each advance the selection.

`button_input` stays GPIO-independent and host-tested. Only the sampling source
moves. Polarity, the pull-up and the 20 ms interval remain provisional.

### Alternative considered: edge interrupt

Timestamping each GPIO edge gives finer resolution but fires repeatedly on
contact bounce, needs a bounded edge queue with overflow policy, and replaces
the tested sample-based filter. Revisit it for the knob if 5 ms sampling proves
too coarse for encoder steps.

## Expected result

At 5 ms sampling the review's idle guarantee applies everywhere: a clean press
of at least 25 ms always registers, including during render/SPI. Interrupt
latency and jitter are unmeasured, so treat this as a target, not a bound.
Visible response still waits for the next rendered frame. This proposal does
not shorten render or transfer time, which stays with 0060's deferred timing
work.

## Verification if accepted

- Host tests: the interrupt-side counter and the drain-side 0068 behavior,
  including several presses drained at once and presses without a valid frame.
  These reuse the gap schedules in `button_input` tests.
- Check the current esp-hal (workspace pin 1.2.2) timer-interrupt and
  critical-section APIs before writing code; do not guess them.
- A separate opt-in diagnostic image, offline-gated, logs completed presses
  per drain. Then a short physical session where the user makes deliberate
  presses, quick taps and double taps, including immediately after a
  selection. A pass shows no missed or merged taps that the polling image would
  have lost. This does not measure exact press durations.

## Consequences if accepted

- Interrupt code and shared state are added to firmware; app, render and core
  crates stay unchanged and platform-agnostic.
- One hardware timer is reserved for input.
- The knob can later reuse the same capture path.
