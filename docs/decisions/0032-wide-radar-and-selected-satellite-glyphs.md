# 0032 — Wide radar and selected-satellite glyphs

Date: 2026-10-15
Status: accepted (design direction; viewport/arrows implemented as an opt-in comparison under [0033](0033-wide-radar-comparison-contract.md), selected-marker labels under [0034](0034-selected-marker-identification.md); glyphs remain unimplemented)

## Decision
Pursue a wide borderless radar with off-screen indicators, a compact identifier
beside the selected satellite, and a narrow right-aligned strip of category and
physical-data glyphs. [DESIGN](../DESIGN.md#selected-satellite-presentation) and
its retained mockup are the fresh-session starting point, not a pixel-exact or
fully specified implementation contract. [0036](0036-shared-glyph-capsule.md)
subsequently adds the shared rounded enclosure; glyph implementation remains open.

This amends [0028's square allocation](0028-borderless-square-viewport.md) as the
future presentation direction and [0029's normal readout direction](0029-normal-and-debug-presentation-direction.md),
including its exclusion of NORAD ID. The default simulator retains 0028 and the
original diagnostic panel. 0033 refines the initially open spatial behavior below
for an opt-in real-data comparison; the full normal/debug presentation remains
unimplemented.

## Why
The user prefers spatial context in the radar itself rather than exact population
counts or a separate population diagram. Edge indicators can show that more
satellites are available at wider zoom. Bare IDs provide compact, unambiguous
identification when most catalogue names are unfamiliar or long. A small set of
familiar-name exceptions preserves recognizable objects such as ISS.

A narrow strip of graphical instruments provides selected-satellite context with
less labelling than a numeric panel. The user favors an angular filled elevation
glyph and a vertical spring-like period coil, with further refinement of the
other encodings. Preserve breathing room rather than adding data to fill space.

## Consequences
- Expand the radar into the area left of the glyph strip and recenter the shared
  view's aim point within that allocation. The mock's 352×240 radar and 48-pixel
  strip are provisional. Do not stretch Earth or independently recenter its
  outline: the WGS-84 Earth and satellites still share the perspective camera.
  Rectangular aspect/FOV semantics must be specified before implementation;
  this does not silently change camera height or select a new zoom default.
- Preserve true elevation > 0 eligibility. Distinct outward-pointing indicators
  just inside the perimeter show direction toward eligible off-screen objects,
  not their physical position on the edge, distance, or travel direction. They
  become normal markers when their true projections enter the view. Exact
  countability is not required. Crowding/grouping, near-plane/behind-camera
  cases, and selection leaving the view remain open; do not project invalid
  depth as a misleading reversed indicator. No below-horizon indicators.
- Use bare NORAD ID beside the selected marker, allowing at least the five- and
  six-digit examples already in local data without hard-coding a five-digit
  limit. Curated familiar-name exceptions are keyed by ID, starting with
  `25544 -> ISS`; additional exceptions are not selected. Keep full catalogue
  names and objective counts/readouts available in diagnostics.
- Put the category pictogram at the top of the right-hand strip, followed by
  elevation, altitude, speed and period. Prefer minimally labelled, 1-bit shapes
  to precise routine numbers. DESIGN owns the visual brief; category mappings,
  multiple/unknown membership treatment, non-elevation scales and clipping/
  saturation behavior are still to be designed. Group provenance alone does not
  establish operational purpose or status. Existing data hooks: app state has
  true altitude/elevation; propagation supplies TEME velocity (currently discarded
  by the app), whose magnitude is not ground or observer-relative speed; retained
  mean motion can supply an element-derived period. Group indices are retained,
  but named group metadata needs loader plumbing or a curated mapping. None of
  these establishes a visual normalization range or production classification.
- Remove normal-view in-view/above-horizon counts and FOV text, in addition to
  0029's branding/replay/diagnostic removals. Retain objective diagnostics in a
  separate presentation; debug access is not yet chosen. Empty-sky and
  incomplete-data/tracking treatment is parked, not waived: absence of markers
  must not falsely imply a complete empty sky.
- Favor animated transitions between selected satellites' glyph values as a
  later presentation refinement. Intermediate shapes are not physical samples;
  do not mutate true state or interpolate any exact readout as if it were true.
  Timing, interruption, and category transitions remain open. This is separate
  from deferred satellite-marker slowdown.
- Stay in M3 and scope a bounded real-data comparison before coding. No new
  selection ranking, panning/camera movement, fetching, population/capacity limit,
  cadence/scheduler, firmware work or frozen-M2 evidence renewal is approved.
  Upcoming-pass lists are not desired. A selected-pass start/end/peak graphic
  remains deferred; existing forward pass search does not supply a peak/profile
  or recover the earlier start of an already-in-progress pass.
