# 0089 — Full-width radar revision (radar lab Favourite C)

Date: 2026-11-22
Status: accepted (user, 2026-11-22: replace the current wide view; new M8,
done before finishing M7; release 16 waits for it and carries 0088 too;
pills as on the board). Not yet implemented on `main`; tracked as
[PLAN M8](../PLAN.md#m8--radar-revision).

## Problem

The accepted wide view (a 352×240 radar beside a 48 px glyph strip, ID label
beside the selected diamond) came out of contract-by-contract slices. The
user iterated a different look in the browser radar lab (orphan branch
`radar-lab`, README "Favourite C") and tried it on the Sharp panel through a
throwaway branch (`radar-fav-c-board-test`, `4901556`, image `0.1.15-favc2`).
On the panel they changed two things (hollow diamond below the horizon, no
crosshair on the selected satellite) and kept the rest. That branch took
shortcuts (feature-switched constants, state hidden in the period cache, no
tests), so it is not something to merge.

## Decision

The wide view becomes the following, replacing the strip design. Square and
circular comparison viewports and the debug page are unchanged.

- **Layout:** the radar fills all 400×240 px. No glyph strip, no separator,
  no centre cross, no "N".
- **Pills:** split pills stacked on the left, sized to content, light fill
  with a dark label side. Selected: NORAD ID, name, then EL, AZ; ALT, RNG;
  SPD, PER; visibility word (`VISIBLE`, `SHADOW`, `DAYLIGHT`), in those
  groups. No selection: `NO SEL`, `SKY` (above-horizon count), `VIEW`
  (in-view count). This supersedes, for the wide view, 0032's "no counts in
  the normal view", 0050's "range in debug only" and 0078's pictogram (the
  word replaces it). Readouts stay true state (DESIGN).
- **Markers above the horizon:** 7 px octagon (rounded filled diamond) with a
  1 px cleared halo; replaces 0061's unselected marker. No heading.
- **The rest of the group below the horizon:** shown as a 5 px hollow diamond
  with a 1 px cleared halo, hidden when the Earth is between it and the
  camera. Never selectable, no arrows, not counted. This changes the view
  from "only what is above you" to "the whole group, with what is above you
  stressed" (DESIGN's observer-sky rule is amended accordingly); selection,
  arrows, counts and notices still consider only above-horizon objects.
- **Selected satellite:** its own octagon with halo, drawn on top, plus a
  pulsing ring (1.4 s cycle) and its full orbit (one period centred on now)
  as a solid line broken where the Earth hides it. No crosshair, no ID label
  (0034/0035 retired for the wide view); 0049's stable selection is kept.
- **Motion cue:** short past-line trail behind each unselected above-horizon
  marker, four steps whose spacing scales with the lens.
- **Earth:** coastline at full tone inside the ring where a 1,000 km
  satellite is above the horizon; a quick drop to 70% just past it, then
  sphere shading down to 12.5% at the limb (8×8 ordered dither, a function of
  screen distance only because the camera is straight above the observer).
  The outline stays solid. No night dither.
- **Overlays:** off-screen arrows (0061, filled) gain a 1 px cleared halo;
  a scale ruler in km (one line, end caps, four divisions, labels) sits
  bottom right on a rounded light backing; 0039/0040/0048 notices keep their
  bottom-left place and wording.

## Budget and contracts

- No propagation is added for markers or trails: `RadarApp` already samples
  every object under 0077's stagger; trails come from stored past samples.
- The selected orbit costs about 160 extra propagations per selection, spread
  over frames and refreshed slowly; it must not break 0077's frame budget or
  0088's input latency. Measured on the device before acceptance.
- Trail history and orbit points are presentation state owned outside the
  frame renderer (M5's "keep presentation state separate"); a frame without
  that state draws no trails or orbit, never stale ones.
- The coastline raster cache (0059) grows to the full width.

## Rejected

Recorded in the `radar-lab` README ("Ideas tried and set aside"); notably the
crosshair on the selected satellite (tried twice on the panel), solid or ring
below-horizon markers, night dither under the octagons, and pass-progress
pills (0077 defers pass prediction).
