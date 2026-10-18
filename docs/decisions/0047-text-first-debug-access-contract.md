# 0047 — Text-first simulator debug access

Date: 2026-10-18
Status: accepted (M3 contract; implemented and user visually accepted)

## Decision

Give the accepted opt-in wide radar a separate, **radar-free, text-first debug
presentation** for inspecting tracking and data health. In interactive wide replay,
`v` toggles between the existing wide normal view and debug. `--debug` starts an
interactive `--wide` replay in debug; it is not an implicit replacement for
`--wide` and does not change the default square/diagnostic comparison. The user
approved no-radar debug, `v` access and an explicit debug-start option after
[0029's normal/debug direction](0029-normal-and-debug-presentation-direction.md).
The user subsequently authorized implementation of the approved contract/layout.
After implementation, the user reviewed the fresh executable output and said it
looks great, closing the separate visual-acceptance gate.

## Why

The user wants debug primarily for tracking/data health, logging and troubleshooting,
not for explaining marker geometry. A radar on the debug page would consume scarce
1-bit text space without serving that purpose. Keeping the underlying wide app
view unchanged avoids changing visible counts or selection merely by inspecting
them; the existing square radar remains a separate diagnostic comparison, not
the new debug mode. Detailed logs belong on the host, not in an invented
on-device scrolling console.

## Consequences

### Purpose and content

Debug is an objective status page, not a second radar or a scrolling log console.
The first bounded page should expose:

- Replay UTC (or `NO FRAME`), current vertical lens FOV, and local-input/no-fetch
  context. UTC is the supplied/replayed time, not the host wall clock.
- Accepted catalogue, above-horizon, in-view and failed-object counts. These are
  different quantities; an empty viewport does not imply an empty or complete sky.
- Ingestion warning presence, and tracking failure count even when successful
  markers exist. For tracking failures, identify the first failed NORAD ID and
  its current error if it fits; explicitly direct the user to the host log for
  truncated detail and other failed IDs. Do not turn a clipped message into a
  complete-looking cause. Ingestion diagnostic details remain in the host report.
- Selected NORAD ID and full available name, plus true WGS-84 altitude, observer
  range/elevation, instantaneous TEME speed, nominal element-set period, and
  signed epoch offset. If there is no selected visible object, say so and omit
  selected-object values. Mark unavailable individual values rather than reuse
  a previous reading. Epoch offset is **not** a freshness or validity score;
  period and TEME speed are not pass duration or ground speed. Category is a
  display role, not a tracking-health or operational-status assertion.

The initial access contract deferred row positions, numeric precision and
abbreviations to a separate preimplementation renderer review. The user has
now approved the [layout/overflow reference](../mockups/0047-debug-layout-proposal.md),
closing that gate before implementation. It prioritizes status and failure visibility over a long
selected name. Avoid silent truncation, stale values or false completeness.
Host stderr retains the complete ingestion report and per-object failure causes
when the failure set changes; this contract adds
no log history, persistence, fetching or on-screen log viewer.

### Access and state

- `v` toggles only in interactive `--wide` replay; it does not change the square
  baseline or other viewport comparison modes. `--debug` requires `--wide` and
  selects the initial interactive presentation. The CLI must reject `--debug`
  without `--wide`, duplicate flags, and `--debug` with `--snapshot` rather than
  silently ignoring them. Existing `--wide --snapshot` stays a static normal
  image, preserving its current reproduction and comparison behavior. A future
  debug snapshot option, if needed, requires its own contract.
- Changing presentation never alters app view/projection, FOV, selection,
  replay UTC, catalogue, counts, true samples or current failures. While debug
  is open, keep normal zoom and Tab inspection inputs active, updating the
  underlying app and the displayed objective values. Returning to normal
  draws the current true scene; never replay a queued animation or show a stale
  glyph/notice. A fresh static wide frame on return is acceptable; no new
  transition continuity policy is approved by this contract.
- Existing wide `NO MARKERS`/`NO FRAME`, `TRACK ERROR`, `INGEST WARN` notices
  remain condition-driven in normal view, including alongside valid markers.
  Debug displays the same underlying conditions without dismissing or masking
  them in normal mode. The simulator still exits on fatal load/clock errors;
  this does not introduce a persistent `NO FRAME` host state.

### Accepted renderer layout

The user reviewed six native-size 1-bit illustrative pages and approved the
[layout/overflow reference](../mockups/0047-debug-layout-proposal.md): fixed
status-first rows, the existing 6×10 bitmap font, two name rows, explicit
shortening/substitution notices and host-stderr guidance, numeric formatting,
field-local unavailability and unsampled-count presentation. The linked reference
owns exact row positions, text bounds and formatting rules; the retained images
remain illustrative, not physical samples or executable output.

This closed the required preimplementation layout review without changing the
access/state contract. The user subsequently authorized implementation, which is
complete. They subsequently accepted the fresh executable output separately from
the illustrative layout approval.

### Boundaries and verification for implementation

Keep [0033's wide geometry/visible-only selection](0033-wide-radar-comparison-contract.md),
[0040's notices](0040-white-backed-bottom-left-notice-contract.md), and
[0046's presentation-only transitions](0046-selected-data-glyph-transition-contract.md).
Do not repurpose the square radar/diagnostic panel as this new debug page, switch
viewport geometry, add new selection/ranking, expose inferred freshness, add
logging I/O to app/render, change core/firmware or refresh frozen M2 evidence.
No normal-view default change, production/device control or target 20 Hz claim.

Implementation uses `sim/src/config.rs` and `sim/src/main.rs` for access and
`render/src/debug.rs` for an independent page; existing app APIs supply counts,
samples and ordered per-object errors. No app/core or logging behavior changed.
Both toggle directions discard presentation history, so returning starts at the
current static endpoints rather than resuming hidden transitions.

Workspace check/tests, app/render no-default-feature checks, formatting and
strict all-target app/render/sim/tools Clippy passed. Tests cover option conflicts,
`v` cycling, identical app state/geometry/failures across toggles, Tab/zoom/replay
while debugging, selected/empty/unsampled/mixed-failure cases, numeric and text
boundaries, complete host causes/names and stale-state clearing. Pre/post
weather/GPS square-default and wide-normal PNGs plus stderr are byte-identical
at 0.1°, 20° and 100°. These are host checks, not physical-display or target
performance validation. No SDL window was launched during implementation.

The user subsequently reviewed and accepted the fresh executable debug output.
This closes 0047's visual gate, not physical-display validation or target operating
budgets. No code changes or new Rust check/test run accompanied that confirmation.
[Simulator usage and review](../../sim/README.md#debug-access) own current controls
and reproduction; the retained mock pages remain illustrative, not executable evidence.
