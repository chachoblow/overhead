# Overhead — Design

A handheld, realtime satellite radar for a set location, inspired by Nathan
Matsuda's flight-tracking interfaces. A rewrite of “Satellite notifier,” which
used an LED matrix and a position API.

## Hardware
- ESP32-S3-DevKitC-1-N8R8: 8 MB flash, 8 MB PSRAM.
- Adafruit 2.7" Sharp Memory Display (#4694): 400×240, 1-bit; target 20 Hz.
  Handle the panel/ribbon gently during bring-up.

## Interface intent
A location-centered borderless square orbital view with a secondary information
area. The user prefers the 240×240 view allocation; the remaining 160×240
right-hand area is still open for exploration, not a settled set of readouts.
[0028](decisions/0028-borderless-square-viewport.md) records the shape choice.
Zoom ranges from local to wider than Earth. Motion should feel fluid and lively;
fast markers have a minimum on-screen traversal time. **Readouts always use
true positions, never visually slowed markers.**

The visual brief is **alive and active, but not busy**. User-supplied Are.na
references span purpose-built audio instruments, aviation/GPS readouts, compact
technical diagrams and monochrome/dot graphics. These guide exploration, not a
requirement to add dashboard density or decorative activity indicators.

A responsive, full but legible radar is the priority, not a particular satellite
count. Curate and tune density on the small 1-bit display: tens may work better
than hundreds. Pass prediction remains useful but secondary to radar quality
([0021](decisions/0021-radar-first-iteration-on-verified-engine.md)).

The radar is a north-up perspective view of actual 3D satellite positions,
from a fixed camera above the configured observer looking down toward that
location. Lens zoom narrows or widens the field of view without moving the camera.
Earth and satellites share the perspective; altitude affects marker placement,
and satellites can appear outside Earth's silhouette rather than being flattened
onto its surface. Objects outside the view or behind the camera are clipped;
zoom alone cannot move an object behind the camera.
[0024](decisions/0024-perspective-orbital-view-direction.md) records the 3D direction,
with lens zoom selected in [0025](decisions/0025-fixed-camera-lens-zoom.md).
The simulator implements [0026's perspective contract](decisions/0026-fixed-camera-perspective-slice.md)
with [0027's higher fixed camera](decisions/0027-higher-fixed-camera-overview.md)
and a WGS-84 Earth outline through that same camera, replacing the ground map.
The viewport is now borderless square under 0028, preserving the same lens
scale while admitting corners. Circular modes remain simulator comparisons.
User feedback favors the faster apparent motion when zoomed in and the fuller
satellite overview when zoomed out. The higher fixed viewpoint makes Earth
smaller at the widest lens setting; zoom itself still never moves the camera.
The user likes breathing room in the overview, not space that must be filled;
the later borderless-square choice removes the drawn ring, not that preference.
The provisional default remains 20° with room to zoom in and out. These visual
choices do not select production counts or capacity.

Only satellites above the local geometric horizon are eligible for markers,
then clipped to the current view—even at whole-Earth zoom. This is the observer's
sky, not a global satellite tracker. Zooming out can reveal additional eligible
objects from the catalogue; catalogue size is not the visible marker count.
Physical passes use a separate configurable
elevation threshold, independent of zoom
([pass semantics](decisions/0012-physical-pass-semantics.md)).

The normal information area should reflect the radar, with no menus or pages.
[0029](decisions/0029-normal-and-debug-presentation-direction.md) separates this
from objective diagnostics: omit branding, replay/date/time labels, catalogue/
error counts, NORAD ID and diagnostic footer from the normal view. Debug access
and normal-view failure communication remain open. This split is not yet
implemented; neither reviewed panel mockup is an accepted visual specification.

Keep view/sky information, but redesign its presentation. Other candidate
contents, not a mandatory checklist:
- Local time and orbit-data age.
- Selected satellite: name, altitude, elevation, travel direction. Selection
  behavior remains open; range was questioned as low-value, not finally ruled out.
- Next pass and countdown to its start, especially when the local sky is empty,
  if useful without compromising the radar. A graphical rise/peak/set profile
  is also a candidate, not an approved layout or prediction-integration task.
- Lens-zoom readout: the current simulator shows the full horizontal/vertical
  field of view in degrees, not a map radius or physical distance.

No pass lists, radio information, or battery indicator. Layout, selection, and
projection policies should evolve with simulator feedback.

## Controls
- Simulator: continuous zoom via scroll wheel and up/down arrows.
- Initial device: BOOT button cycles through roughly four zoom presets.
- Knob and optional auto-zoom demo mode come later; neither blocks the prototype.

## Product choices
[On-device SGP4](decisions/0001-on-device-sgp4.md),
[Wi-Fi elements/SNTP](decisions/0002-data-and-time-over-wifi.md),
[curated catalogue](decisions/0003-curated-catalogue.md)
(scale tuned through [0021](decisions/0021-radar-first-iteration-on-verified-engine.md)), and
[configured location](decisions/0004-hardcoded-location.md) are settled;
follow those records for rationale and constraints. Implementation order and
deferred scope belong to [PLAN.md](PLAN.md).
