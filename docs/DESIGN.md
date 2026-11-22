# Overhead — Design

A handheld, realtime satellite radar for a set location, inspired by Nathan
Matsuda's flight-tracking interfaces. A handful will be given as gifts: each
recipient sets Wi-Fi and location from an iPhone app over Bluetooth, and the
device updates itself over the air from signed releases
([0083–0087](decisions/0083-flash-layout-for-updatable-devices.md)). A rewrite of “Satellite notifier,” which
used an LED matrix and a position API.

## Hardware
- ESP32-S3-DevKitC-1-N8R8: 8 MB flash, 8 MB PSRAM.
- Adafruit 2.7" Sharp Memory Display (#4694): 400×240, 1-bit; target 20 Hz.
  Handle the panel/ribbon gently during bring-up.

## Interface intent
A location-centred, borderless orbital view filling the whole 400×240 panel,
with the selected satellite's true-state readouts in pills on the left
([0089](decisions/0089-full-width-radar-revision.md), from the radar lab and
accepted on the panel 2026-11-22; it replaced the 352×240 radar and glyph strip
of [0032](decisions/0032-wide-radar-and-selected-satellite-glyphs.md)/[0038](decisions/0038-labelled-borderless-glyph-direction.md)).
The simulator retains [0028's 240×240 square](decisions/0028-borderless-square-viewport.md)
and diagnostic panel by default; `--wide` is the device view. Vertical FOV and
per-satellite off-screen arrows follow [0033](decisions/0033-wide-radar-comparison-contract.md);
notices follow [0040](decisions/0040-white-backed-bottom-left-notice-contract.md)/[0048](decisions/0048-refined-empty-view-notices.md);
debug access [0047](decisions/0047-text-first-debug-access-contract.md).

Zoom ranges from local to wider than Earth. Motion should feel fluid and lively;
minimum on-screen traversal time for fast markers is planned, not implemented.
**Readouts always use true physical state, never visually slowed markers.**

The visual brief is **alive and active, but not busy**. User-supplied Are.na
references span purpose-built audio instruments, aviation/GPS readouts, compact
technical diagrams and monochrome/dot graphics. These guide exploration, not a
requirement to add dashboard density or decorative activity indicators.

A responsive, full but legible radar is the priority, not a particular satellite
count. Curate and tune density on the small 1-bit display: tens may work better
than hundreds. Pass prediction remains useful but secondary to radar quality
([0021](decisions/0021-radar-first-iteration-on-verified-engine.md)); on the device it is
not planned ([0077 note](decisions/0077-m2-operating-budget-visual-group.md#note--2026-11-10-device-pass-prediction-not-planned)).

The radar is a north-up perspective view of actual 3D satellite positions,
from a fixed camera above the configured observer looking down toward that
location. Lens zoom narrows or widens the field of view without moving the camera.
This iteration may reuse prepared coastline geography for a fixed observer during
use; the user would like to keep globe scrolling/panning open for a future
iteration, not require it now. [0057](decisions/0057-fixed-observer-coastline-preparation-scope.md)
scopes that direction without selecting a location UI; [0058](decisions/0058-prepared-wide-coastline-slice.md)
implements a startup-prepared wide-view layer in the simulator, with
[0059](decisions/0059-current-view-coastline-raster-cache.md)'s current-view raster
cache; neither is a device adoption. Earth and satellites share the perspective; altitude affects
marker placement,
and satellites can appear outside Earth's silhouette rather than being flattened
onto its surface. Objects outside the view or behind the camera are clipped;
zoom alone cannot move an object behind the camera.
[0024](decisions/0024-perspective-orbital-view-direction.md) records the 3D direction,
with lens zoom selected in [0025](decisions/0025-fixed-camera-lens-zoom.md).
The implemented contract is [0026](decisions/0026-fixed-camera-perspective-slice.md),
amended by [0027's higher camera](decisions/0027-higher-fixed-camera-overview.md),
0028's default square clipping, and 0033's opt-in rectangular comparison.
See [simulator usage](../sim/README.md#controls-and-view)
for current settings, readout units and comparison modes.

User feedback favors zoom-magnified motion and a fuller satellite overview with
smaller Earth at wide FOV. Preserve breathing room rather than auto-fitting or
adding satellites to fill it; the square choice removes the ring, not that
preference. These choices do not select production counts or capacity.
[0051](decisions/0051-angular-coastline-appearance-direction.md) records a
reviewed **angular 1-bit coastline appearance direction** from 5° through 100°,
including local detail and the tiny wide-end cue, with evidence-gated exploration
down to 0.1°. [0052](decisions/0052-natural-earth-coastline-candidate.md) accepts
Natural Earth 1:110m v4.1.0 as a conditional source candidate only. The
[offline comparisons](mockups/0057-coastline-sampling-comparison.md) are not
executable coastline output. [0054](decisions/0054-first-executable-coastline-comparison.md)
now implements a bounded wide-only host comparison using the user's accepted
40 km orientation look and a checked source export. The user accepted its
fresh executable images and simulator appearance, including the tight-zoom
comparison. This is not a certified no-omission geographic renderer, target-cost
result or production asset. Square/circular modes remain unchanged.

Only satellites above the local geometric horizon are eligible for markers,
selection, arrows and counts, then clipped to the current view—even at
whole-Earth zoom. It stays the observer's sky, not a global satellite tracker;
since [0089](decisions/0089-full-width-radar-revision.md) the rest of the
loaded group is shown as context (light hollow diamonds where the camera can
see them), never selectable or counted. Catalogue size is not the visible
marker count.
Physical passes use a separate configurable
elevation threshold, independent of zoom
([pass semantics](decisions/0012-physical-pass-semantics.md)).

### Wide view ([0089](decisions/0089-full-width-radar-revision.md))
<a id="selected-satellite-presentation"></a>

- **Radar:** the full 400×240 panel, centred aim point, shared Earth/satellite
  projection, no border, centre cross or "N". Earth is not stretched.
- **Markers:** 7 px octagon (rounded filled diamond) with a 1 px cleared halo
  above the horizon; the group's below-horizon members as a 5 px hollow
  diamond with halo, hidden behind the Earth.
- **Selected:** its own octagon on top, plus a pulsing ring that restarts with
  each new selection; no crosshair or ID label. Selection continuity follows
  [0049](decisions/0049-stable-wide-selection.md).
- **Pills (left, sized to content):** NORAD ID and name; EL, AZ; ALT, RNG;
  SPD, PER; visibility word (`VISIBLE`, `SHADOW`, `DAYLIGHT`, from
  [0078](decisions/0078-visibility-glyph.md)'s rules). No selection: `NO SEL`,
  `SKY`, `VIEW` counts. Nothing without a frame.
- **Earth:** coastline full inside the ring where a 1,000 km satellite is above
  the horizon, then sphere shading to 12.5% at the limb; solid outline.
- **Overlays:** haloed filled arrows ([0061](decisions/0061-filled-arrows-id-backing-none-in-view.md));
  km scale ruler bottom right on a light rounded backing.
- **Planned ([PLAN M8](PLAN.md#m8--radar-revision)):** short past-line trails
  behind unselected markers and the selected satellite's full orbit line.

The earlier glyph strip (0038–0046, 0078's pictogram) and marker-adjacent ID
(0034/0035) are retired; their decisions keep the history.

### Retained mock references
<a id="circular-elevation-only-contract"></a><a id="static-altitude-contract"></a><a id="static-speed-contract"></a><a id="static-period-contract"></a><a id="static-category-contract"></a><a id="bottom-left-notice-reference"></a><a id="glyph-capsule-reference"></a>
Provenance for the approved illustrative mocks of
[elevation](mockups/glyph-and-notice-references.md#circular-elevation-only-contract),
[altitude](mockups/glyph-and-notice-references.md#static-altitude-contract),
[speed](mockups/glyph-and-notice-references.md#static-speed-contract),
[period](mockups/glyph-and-notice-references.md#static-period-contract),
[category](mockups/glyph-and-notice-references.md#static-category-contract),
[bottom-left notices](mockups/glyph-and-notice-references.md#bottom-left-notice-reference)
and the historical [0036 glyph capsule](mockups/glyph-and-notice-references.md#glyph-capsule-reference)
lives in [mockups/glyph-and-notice-references.md](mockups/glyph-and-notice-references.md).
They are contract references for the retired strip (0089), not executable
output, golden tests or physical evidence.

### Open questions and boundaries

Settled for the current wide view: 0089's layout; 0033's vertical FOV/aspect,
no-arrow invalid-depth handling and overlaps-allowed arrows (no collision
avoidance); 0047 debug access; warning placement/compositing owned by 0040.

Circular radar with an info brick, surrounding noise fields and unlabelled
visible-population graphs were explored, not selected as replacements. For any
future field experiment, the user's interest is a data-driven visual fingerprint
rather than readable text placed over texture. No field mapping, time-history
policy or implementation is approved; keep such experiments separate from the
accepted wide/glyph direction.

[0049](decisions/0049-stable-wide-selection.md) keeps the selected ID and current
true-state readouts (now the pills) when a successfully tracked above-horizon object moves outside
the viewport through zoom or motion. Tab chooses an in-view marker;
horizon/tracking loss ends the selection and falls back to the first in-view
marker. No off-screen diamond, marker-adjacent label or specially marked arrow
is approved. Square diagnostics retain visible-only selection.

[0039](decisions/0039-bottom-left-condition-driven-notices.md) selects bottom-left
notices that follow their underlying conditions rather than expiring:
`NO MARKERS`, `NO FRAME`, `TRACK ERROR` and `INGEST WARN`. Healthy selected views
have no notice ink. [0040](decisions/0040-white-backed-bottom-left-notice-contract.md)
gives them a small borderless white backing that may visually cover markers
(including the selected diamond), IDs, arrows or Earth within its bounds without
changing eligibility, counts, physical state or selection.
[0048](decisions/0048-refined-empty-view-notices.md) refines the wide empty view:
`NONE IN VIEW` (0061's wording; originally `SKY OUTSIDE VIEW`) when there are
eligible arrows, `NONE ABOVE HORIZON` only for a clean nonempty loaded catalogue
with nothing above the horizon, and conservative `NO MARKERS` otherwise.
Independent tracking/ingestion warnings remain, and 0047's debug access does not
replace normal notices. A normal view must not conceal failed/omitted tracking
as a complete empty sky, including when some valid markers remain.

The selected-satellite start/end/peak pass graphic is interesting but deferred;
upcoming-pass lists are not desired. [0050](decisions/0050-keep-range-in-debug-and-defer-display-prediction.md)
kept range in debug (0089 now shows it in a pill)
and defers display prediction, including a countdown study, for now; the existing
physical pass-search engine remains intact. Local time, orbit-data age and bearing
are not part of this starting layout. Camera movement/panning, marker slowdown,
any ranking beyond 0049's agreed selection continuity/fallback, fetching and
production population/cadence choices are outside this slice. No radio
information or battery indicator.

## Controls
- Simulator: continuous zoom via scroll wheel and up/down arrows.
- Intended device ([0060](decisions/0060-defer-timing-to-physical-prototype.md)):
  a button cycles through satellites and a knob zooms continuously; zoom presets
  remain a fallback if the knob proves infeasible.
- Device inputs ([0088](decisions/0088-selection-button-and-captured-knob-switch.md)):
  a separate button cycles in-view satellites once per completed debounced
  press ([0068](decisions/0068-boot-selection-gesture.md)); the knob's push
  switch holds for setup/factory reset at boot and toggles the info page
  ([0087](decisions/0087-ios-setup-app-over-ble.md)). Presses during a fetch
  stall apply late rather than being lost. BOOT is not a device control.
  Optional auto-zoom demo mode comes later.

## Product choices
[On-device SGP4](decisions/0001-on-device-sgp4.md),
[Wi-Fi elements/SNTP](decisions/0002-data-and-time-over-wifi.md),
[curated catalogue](decisions/0003-curated-catalogue.md)
(scale tuned through [0021](decisions/0021-radar-first-iteration-on-verified-engine.md)),
[recipient-set location and Wi-Fi](decisions/0084-settings-store.md) through the
[iOS setup app](decisions/0087-ios-setup-app-over-ble.md), and
[signed over-the-air updates](decisions/0085-signed-updates-and-release-keys.md)
with [rollback](decisions/0086-bootloader-rollback-and-health-check.md) are settled;
follow those records for rationale and constraints. Implementation order and
deferred scope belong to [PLAN.md](PLAN.md).
