# 0068 — BOOT selection gesture for the wide radar

Date: 2026-10-30
Status: accepted bounded M6 prototype behavior; input timing remains provisional

## Decision

BOOT requests one `RadarApp::select_next()` per **completed debounced press**,
without repeat while held. It cycles only successful **in-view** wide-view markers
in ascending NORAD ID order, wrapping as [0049](0049-stable-wide-selection.md)
already specifies; an off-screen arrow is never a cycle target. A completed
press without a valid app frame is consumed, not queued for recovery. Knob zoom
remains separate.

## Why

Sharing simulator Tab/0049 selection avoids conflicting manual and automatic
selection rules or accidentally selecting an off-screen object. Requiring both
qualified press and release prevents hold-repeat; discarding no-frame events
avoids a delayed selection that the user did not see at the time of pressing.
Retaining a valid off-screen selection when no marker is in view is still 0049's
rule, not a new BOOT behavior.

## Consequences

A GPIO-independent input filter and selection composition are host-tested in
[`button_input`](../../firmware/src/button_input.rs). Two separate opt-in S3
experiments are documented in the [firmware README](../../firmware/README.md#historical-multi-marker-boot-cycle-diagnostic-flashed-short-uart-capture):
active-low BOOT/GPIO0 with pull-up and **provisional** 20 ms press/release
qualification; a fixed historical multi-marker scene recorded two advancing
selected IDs on UART, both visually confirmed by the user. This does not adopt
a production debounce/poll cadence, guarantee short presses during blocking
render/SPI, measure latency/COM edges, select a live catalogue, or authorize
further hardware access. Standalone device integration and knob policy remain
open under [0060](0060-defer-timing-to-physical-prototype.md).
