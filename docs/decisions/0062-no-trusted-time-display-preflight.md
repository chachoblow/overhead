# 0062 — No-trusted-time display preflight

Date: 2026-10-26
Status: accepted behavior; host-only preflight, not a device policy

## Decision

On cold boot and after loss of previously trusted UTC, discard app tracking and
selection without manufacturing a timestamp. Replace the old radar image with
the existing `NO FRAME` view as soon as a full transfer of the newly rendered
pixels can complete. Do not treat a failed or partial transfer as confirmation
that the panel has changed. A new externally trusted UTC sync permits fresh
sampling immediately only after the shared supported-year/leap-second check;
invalid input instead clears tracking and requests `NO FRAME`. If recovery
arrives before a loss frame is sent, no intermediate `NO FRAME` transfer is
required. This representation check does not establish SNTP provenance or time
quality. [0039](0039-bottom-left-condition-driven-notices.md)
owns the notice meaning; [0002](0002-data-and-time-over-wifi.md) owns the
future SNTP source, not this preflight.

## Host boundary

`RadarApp::invalidate_frame` removes physical samples/time/selection while
preserving catalogue, observer and view. The opt-in firmware
`TimeLossPreflight` gives a cold-boot or time-loss `RenderNoFrame` action priority
over diagnostic retained refresh and idle COM. Polling that action invalidates
even a pre-populated app on cold boot and re-invalidates it on retries, before
the caller draws and sends a **full** frame to acknowledge the action. Once confirmed,
the same diagnostic timing helpers can independently request idle COM and full
refreshes of those warning pixels without new orbit sampling. Failed commands
receive no credit and require a full resync. A regression in polled monotonic
uptime (including one earlier than the last completed transfer), or UTC/deadline
arithmetic overflow, invalidates the frame anchor and clears old app samples
before another action; it cannot silently resume on a later plausible uptime.
Recovery anchors the next due frame to the supplied checked UTC, never to a
missed deadline or a historical fixture offset.

No live time-quality validation, Wi-Fi/SNTP, production refresh interval,
blocked-loop or SPI-error recovery, safe COM-edge guarantee, selected production
cadence, target image, flash, or new notice wording is approved here. The
historical `sharp-natural` source still calls `DiagnosticSchedule::next`, not
`TimeLossPreflight`; its fixture time is **not** trusted device UTC. The approved
flashed image was not rebuilt or replaced by these host changes. This preflight
is not integrated into a production S3 loop.

## Verification scope

Host app tests check explicit invalidation and recovery without losing observer,
catalogue, view or group categories. The firmware preflight tests cold-boot
invalidation without an explicit loss call and re-invalidation before retry. The checked sync tests invalid years,
explicit leap seconds, fractional microseconds and recovery after rejection.
Clock tests cover regression between due frames, regression relative to a
completed command, overflow between frames/at a deadline, and resync. Opt-in
firmware host tests compose the
app, renderer, selective sender and timing preflight: cold boot, valid frame,
time loss, failed `NO FRAME` transfer and retry, independent COM/full-data
reminders while UTC is absent, and new-UTC recovery. These prove command-side
rules and framebuffer intent only, not physical panel timing or service.
[firmware usage](../../firmware/README.md#utcframe-scheduling-preflight-host-only)
records the test command and limitations.
