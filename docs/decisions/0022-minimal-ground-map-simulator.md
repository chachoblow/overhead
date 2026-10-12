# 0022 — Minimal ground-map simulator

Date: 2026-10-12
Status: accepted

## Decision
Use a map-like ground-track view, not an azimuth/elevation sky chart, for the
first real-data radar. Start with a north-up, observer-centered spherical
orthographic projection; keep all physical state in the verified engine and
all map/zoom/selection state in a shared, clockless `overhead-app` crate.

## Why
The user chose geographic ground tracks after comparing them with a sky view.
Geographic zoom and future coastlines fit the product intent. Orthographic
projection gives a recognizable globe when zooming beyond Earth, with simple
local-to-global behavior. Its radial compression near the limb is accepted for
this initial visual iteration; it is not a surface-distance map. Projection and
layout remain subject to visual feedback under [0021](0021-radar-first-iteration-on-verified-engine.md).

## Consequences
- Map WGS-84 geodetic latitude/longitude onto a display sphere of radius
  6,378.137 km using [Snyder's orthographic equations 20-3/20-4](https://neacsu.net/geodesy/snyder/5-azimuthal/sect_20/).
  Target altitude does not move its ground marker. The supplied observer longitude
  defines the limiting north/east basis at poles. Reject invalid coordinates;
  clip the far hemisphere instead of folding it into the near hemisphere.
- Marker eligibility is true geometric elevation strictly greater than zero,
  independent of zoom. Then apply hemisphere and circular viewport clipping,
  including glyph pixels. View radius is projected map km, not ground-arc or
  slant range. Zoom beyond the globe adds margin, not below-horizon satellites.
- Use checked catalogue ingestion and the timestamp-bound physical pipeline.
  Recompute every accepted object's result at the caller's explicit UTC. Failed
  samples replace old markers, retain typed errors and contribute to displayed
  error counts. Invalid UTC invalidates the shared frame; it cannot retime old
  positions. Readouts never use projected or slowed state.
- `overhead-app` is `no_std` + alloc, without I/O or a scheduler. The 1-bit renderer
  consumes it without owning a clock or framebuffer. SDL owns only host input,
  time, window and file adapters, reusing the existing checked manifest loader.
  Core remains default allocation-free. No new third-party library is introduced.
- The simulator requires explicit catalogue/time/observer or an explicit historical
  `--demo`. Replay advances 1× from UTC using monotonic elapsed time; snapshots use
  the exact start UTC without SDL. No implicit current time or freshness claim.
- Initial identification retains the selected visible NORAD ID, falls back to
  the first visible ID, and offers Tab cycling. This is an inspection baseline,
  not final automatic ranking, dwell or empty-sky policy.
- Radius/layout knobs, a synchronous 20 Hz host-loop target and the two example
  catalogue populations are simulator choices only. They do not accept operating
  limits, prediction cadences, bounded scheduling, target RAM or safe stack.
  M2 stays open; no prediction integration, coastlines, slowed motion, live data
  retrieval or firmware work is required by this slice.

[Simulator usage and replay examples](../../sim/README.md) own commands and data
provenance links. Code/tests own API details and numeric boundary behavior.
