# Overhead — Design

A handheld, realtime satellite radar for a set location, inspired by Nathan
Matsuda's flight-tracking interfaces. A rewrite of “Satellite notifier,” which
used an LED matrix and a position API.

## Hardware
- ESP32-S3-DevKitC-1-N8R8: 8 MB flash, 8 MB PSRAM.
- Adafruit 2.7" Sharp Memory Display (#4694): 400×240, 1-bit; target 20 Hz.
  Handle the panel/ribbon gently during bring-up.

## Interface intent
A location-centered, wide borderless orbital view with a narrow right-hand
strip of selected-satellite glyphs. [0032](decisions/0032-wide-radar-and-selected-satellite-glyphs.md)
records the overall direction, not a completed interface. The simulator retains
[0028's 240×240 square](decisions/0028-borderless-square-viewport.md) and diagnostic
panel by default. [0033's opt-in comparison](decisions/0033-wide-radar-comparison-contract.md)
implements a 352×240 comparison radar with vertical FOV and per-satellite
off-screen arrows. The user approved this layout and overlaps-allowed treatment.
[0038](decisions/0038-labelled-borderless-glyph-direction.md) now selects a
borderless labelled glyph stack, circular data glyphs and a padded vertical
separator. The retained mocks below are the current visual direction, not an
implementation contract. The executable now implements
[0041's static circular elevation](decisions/0041-circular-elevation-only-contract.md),
a bitmap `Elev` label and padded separator, replacing
[0037's visually accepted elevation capsule](decisions/0037-static-elevation-glyph-contract.md).
[0040's white-backed bottom-left notices](decisions/0040-white-backed-bottom-left-notice-contract.md)
remain unchanged and visually accepted. The user also reviewed and accepted the
new executable elevation slice; the other glyphs and full stack are unimplemented.

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
The implemented contract is [0026](decisions/0026-fixed-camera-perspective-slice.md),
amended by [0027's higher camera](decisions/0027-higher-fixed-camera-overview.md),
0028's default square clipping, and 0033's opt-in rectangular comparison.
See [simulator usage](../sim/README.md#controls-and-view)
for current settings, readout units and comparison modes.

User feedback favors zoom-magnified motion and a fuller satellite overview with
smaller Earth at wide FOV. Preserve breathing room rather than auto-fitting or
adding satellites to fill it; the square choice removes the ring, not that
preference. These choices do not select production counts or capacity.

Only satellites above the local geometric horizon are eligible for markers,
then clipped to the current view—even at whole-Earth zoom. This is the observer's
sky, not a global satellite tracker. Zooming out can reveal additional eligible
objects from the catalogue; catalogue size is not the visible marker count.
Physical passes use a separate configurable
elevation threshold, independent of zoom
([pass semantics](decisions/0012-physical-pass-semantics.md)).

### Selected-satellite presentation

The current accepted visual direction is [0038](decisions/0038-labelled-borderless-glyph-direction.md):
symbol-led with short field labels, no enclosing glyph borders, and no menus or
pages. Normal/debug separation from
[0029](decisions/0029-normal-and-debug-presentation-direction.md) remains agreed;
0032 amends which information belongs in the normal view, with glyph treatment
subsequently revised by 0038.

![Labelled borderless glyph stack with a navigation cross and radar separator](mockups/0038-labelled-glyph-radar.png)

![The same illustrative layout with the selected station body-and-wings glyph](mockups/0038-labelled-glyph-station.png)

**Reference only:** two native 400×240, strictly 1-bit composites retained on
2026-10-17. The first preserves the final reviewed layout; the second substitutes
the selected station-B icon without changing anything else. Both use the wide
weather/GPS replay radar at `2026-10-10T00:00:00Z`, observer
`39.007 -104.883 2.187`, 20° vertical FOV
([reproduction inputs](../sim/README.md#wide-radar-comparison)). Radar pixels are
unchanged apart from conversion to pure black/white. Pillow replaced the strip
with illustrative glyphs: 45° elevation cutout, mid-sized altitude ring, half
speed arc and a two-turn spiral. Neither category nor glyph values describe the
selected `32711`; the station variant is not a claim that it is a station.
These are not executable output, golden tests or physically consistent scenes.

The reviewed layout uses 24×24 glyph footprints in the existing 48px strip,
short centered `Cat`, `Elev`, `Alt`, `Spd`, `Per` labels, and equal **10px** blank
space above the top label and below the period's last ink. Label tops are at
`y=10,57,104,151,198`; glyph footprints start at `x=364`,
`y=19,68,115,162,209`. The 1px separator is at `x=352`, `y=12..227`, leaving
12px top/bottom margins and space on each side in the reviewed scene. These are
mock observations for the next comparison, not a final geometry/clearance contract.
The tiny Menlo labels are provisional rasterizations; use a crisp renderer bitmap
font before final spacing review. No temporary assets or scripts are needed to
recover this direction; the retained PNGs and text are sufficient.

- **Radar:** use the space left of the narrow glyph strip, with a centered aim
  point and shared Earth/satellite projection. The mock uses 352×240 plus a
  48-pixel strip; these dimensions are provisional and now used by the opt-in
  comparison. Keep breathing room and no perimeter border. Earth is not stretched
  to fill the rectangle.
- **Outside-view context:** small hollow, outward-pointing edge indicators for
  above-horizon satellites outside the viewport. Wider zoom brings them into
  view as actual markers. Use one arrow per eligible off-screen satellite;
  overlaps are allowed for the first comparison, without suppression, merging
  or displacement. Convey that there is more to explore, not a guaranteed
  countable tally. No separate population-dot summary, normal-view counts, or
  FOV text.
- **Identification:** bare NORAD ID beside the selected marker, with the sole
  familiar-name exception `25544 -> ISS`. Implemented in the opt-in wide view
  under [0034](decisions/0034-selected-marker-identification.md): right-side text,
  left-side fallback, vertical clamping and transparent overlaps allowed. The
  complete ID stays inside the radar, with the marker rather than at the top of
  the strip. [0035](decisions/0035-label-zoom-visibility.md) hides the text at ≥70°
  vertical FOV and restores it at ≤60°, retaining visibility between thresholds.
  The user reviewed and accepted this behavior for now; thresholds remain
  configurable comparison defaults. The zoom gate retains the selected diamond;
  0040's notice overlay may locally cover it. No fade or dithering. Full names
  remain diagnostic information.
- **Glyph strip, top to bottom:** category pictogram, elevation, altitude,
  orbital speed, orbital period. Keep it near the right edge, without enclosing
  borders; one padded vertical line separates it from the radar. Center short
  labels above each glyph. No routine numeric values. Navigation uses a thin
  square-ended cross; station uses a small outlined body with straight horizontal
  wings (selected option B). Weather-cloud and unknown-question-mark icons remain
  proposals, not selected categories or a finalized taxonomy.
- **Elevation:** a 1px circle outline with an angular cutout opening upward
  from the rightward horizontal radius; outline the remaining circle and both
  radial edges, not the isolated wedge. Keep the radial strokes single-pixel,
  avoiding the doubled edges seen in the initial mock. True observer elevation
  supplies the input. [0041](decisions/0041-circular-elevation-only-contract.md)
  now implements the bounded static circular cutout, endpoint/raster rules,
  label and separator, replacing the filled-square executable.
- **Altitude:** fixed outer circle with an outlined inner circle that shrinks
  as true WGS-84 altitude rises. **Speed:** an increasing circle-outline arc,
  absent at zero and complete at maximum; the mock grows clockwise from the top,
  not as a heading. **Period:** a spiral, replacing the coil; more turns mean a
  shorter period. Scales, speed reference frame and period semantics remain open.
  The earlier logarithmic diamond/bar altitude preview does not establish a
  mapping for the concentric circles.
- **Transitions:** favor glyphs animating toward the new selected satellite's
  values. Intermediate shapes are presentation only, never new physical samples.
  This has not been implemented or given timing/interruption rules.

### Circular elevation-only contract

[0041](decisions/0041-circular-elevation-only-contract.md) now implements the bounded
comparison: just `Elev`, a circular cutout and the separator, with all other
slots blank. The elevation stays in its future stack slot rather than
moving into the category position. The user approved a 23px circle inside the
24×24 footprint for a single-pixel hub, a crisp `FONT_6X10` label, immediate
true-state updates, and glyph/label clearing without selection while retaining
the separator. Implementation is complete and the user accepted the fresh
executable output. The retained references below are not that output;
[simulator usage](../sim/README.md#static-elevation-glyph) owns reproduction.

![Elevation-only contract reference with illustrative 45-degree reading](mockups/0041-elevation-only-proposal.png)

![Native strips and 4x raster details for 0, 15, 45, 75, 90 degrees and no selection](mockups/0041-elevation-raster-proposal.png)

**Contract references only:** retained and approved on 2026-10-17, both strictly
1-bit. The first is a native 400×240 composite preserving every pixel of the radar portion (`x<352`)
of the retained 0038 navigation mock above. Its 45° glyph is illustrative and
is **not** a measurement of selected `32711`. The second is a 672×416 contact
sheet: six native 48×240 strips, angle/empty headings outside those strips, and
4× nearest-neighbor label/glyph details below. `EMPTY` shows only the no-selection
strip, not a complete empty-sky radar or an ingestion warning.
The 0° case is a defensive/raster endpoint, not an above-horizon selected sample.

Pillow composed these previews using 0041's explicit integer circle/line rules;
label pixels came from the existing embedded-graphics 0.8.2 `FONT_6X10` ASCII
bitmap, not a system-font approximation. No package was added. Circle endpoints
are quantized to its outline pixels; intermediate examples need not have an
exactly 15°/75° raster slope. These approved contract references are neither
simulator output, golden tests, independent physical evidence nor executable
visual acceptance. The retained images, 0041's raster specification and the
existing font asset suffice to recover the references without the disposable
generation script. Notice behavior remains 0040's accepted executable behavior,
not a new mock treatment.

### Bottom-left notice reference

[0039](decisions/0039-bottom-left-condition-driven-notices.md) selects bottom-left
notices for the revised stack, replacing the earlier unapproved radar-corner
suggestion. Notices follow their underlying conditions rather than expiring:
`NO MARKERS`, `NO FRAME`, `TRACK ERROR` and `INGEST WARN` keep their current
meanings. Healthy selected views have no notice ink. The decision owns lifecycle
details and the distinction between renderer states and simulator error handling.
[0040](decisions/0040-white-backed-bottom-left-notice-contract.md) now implements
these notices independently of the full glyph stack. A small borderless white
backing keeps the text readable over crossing graphics. It may visually cover
markers (including the selected diamond), IDs, arrows or Earth within its bounds;
this does not change eligibility, counts, physical state or selection. Neither
text nor backing remains when no notice applies. 0040 originally preserved the
right-hand elevation capsule; 0041 now changes only that strip, leaving notices
unchanged. The user confirmed review and acceptance of the executable notice
snapshots; the composites below remain historical layout references, not that
executable evidence.

![Selected marker and illustrative warnings at bottom-left](mockups/0039-bottom-left-selected.png)

![No in-view markers with illustrative warnings at bottom-left](mockups/0039-bottom-left-empty.png)

**Reference only:** native 400×240, strictly 1-bit composites retained on
2026-10-17 after the user chose bottom-left comparison B over top-left A.
Backgrounds are fresh headless weather/GPS replays at the same UTC/site as the
0038 references, at 20° and 0.1° vertical FOV respectively
([reproduction inputs](../sim/README.md#wide-radar-comparison)). Both source
frames are healthy: the warning strings were added for layout review, not
reported by the replay. The selected strip reuses 0038's illustrative navigation
and data glyphs, replacing provisional labels with the existing renderer's
`FONT_6X10` bitmap. Glyphs still do not describe the selected satellite.

Notice ink starts at `x=12`, with 13px line spacing and the last line's ink at
`y=216..222`; the two-line example starts at `y=203`, the three-line example at
`y=190`. These observations informed 0040's subsequent geometry contract.
The retained mocks use transparent text, not the implemented white backing.
The strip retains 10px outer padding and the separator; clearing labels/glyphs
without selection while retaining that
separator is still a proposal for the full stack, not a newly accepted lifecycle.

The [synthetic overlap stress mock](mockups/0039-bottom-left-overlap.png) adds a
marker, ID and arrow near the notices to expose possible collisions. Those added
objects are not physical samples. Bottom-left placement alone cannot guarantee
legible warnings; 0040 subsequently resolves this with the approved local white
backing. No displacement or change to arrow geometry/selection rules follows.
No temporary assets or scripts are required to recover this direction.

### Glyph capsule reference

**Historical direction:** 0038 replaces the capsule and earlier glyph shapes
for future work. The images here remain provenance, not competing current briefs.
The [original 0032 reference](mockups/0032-glyph-radar.png) contains four independent
400×240 1-bit schematics prepared on 2026-10-15: weather/navigation/ISS/weather
examples with invented positions and illustrative values, not measured states.

![Earlier wide replay with illustrative glyphs inside the shared capsule](mockups/0036-glyph-capsule.png)

**Reference only:** retained 400×240, strictly 1-bit composite from the 2026-10-16–17
visual session. The radar is the current wide weather/GPS replay at
`2026-10-10T00:00:00Z`, observer `39.007 -104.883 2.187`, 20° vertical FOV
([reproduction inputs](../sim/README.md#wide-radar-comparison)); its pixels are
unchanged apart from converting the light background to white. Pillow added the
illustrative navigation/elevation/altitude/speed/period stack and shared capsule
in the reserved 48-pixel strip. Glyph values are not measurements of the selected
satellite. The mock uses 6-pixel-tall side openings; its gap length, padding and
radius did not establish an implementation contract. The initial approval covered
only the wrapper treatment; 0037 below subsequently bounds the first implementation.
At the time, this image superseded the earlier reference's unboxed treatment,
not its data semantics. It is neither executable output nor a golden test.
No temporary assets or generation scripts are needed to continue.

The user subsequently approved [0037's elevation-only slice](decisions/0037-static-elevation-glyph-contract.md):
a shortened capsule with separate space below for explicit notices, immediate
true-elevation updates, and no capsule or glyph without selection. This temporary
layout was not the full-stack mock's final geometry. The slice was implemented
and the user accepted its rendered snapshot set. 0040 subsequently relocated
only its notices to bottom-left; 0041 now replaces the capsule and filled glyph
with labelled circular elevation and the separator. Other glyphs remain absent.
Reproduce the current state through the
[simulator's wide view](../sim/README.md#static-elevation-glyph).

### Open questions and boundaries

0033 settles the first comparison's vertical FOV/aspect, no-arrow invalid-depth
handling, overlaps-allowed arrow policy and unchanged visible-only selection.
The user approved the current wide layout and allowed arrow overlaps; retain
these rather than adding collision avoidance. 0034 settles the first marker-adjacent
label's placement and overlaps-allowed policy. User review of the crowded overview
led to 0035's configurable clean hide/show refinement, now user-reviewed and
accepted for the current comparison. Define glyph ranges/mappings and unknown/
ambiguous category handling explicitly; don't turn illustrative mock values into
defaults. 0038 replaces 0036's wrapper and the earlier glyph visual brief.
[0041](decisions/0041-circular-elevation-only-contract.md) now bounds and implements
only the unanimated elevation label/cutout and separator, superseding 0037;
warning placement/compositing remain owned by 0040. The full stack still needs
exact raster/font geometry, glyph mappings, lifecycle and transition rules
before coding. Elevation-only no-selection clearing is implemented, but the
full-stack lifecycle has not yet been specified.

Circular radar with an info brick, surrounding noise fields and unlabelled
visible-population graphs were explored, not selected as replacements. For any
future field experiment, the user's interest is a data-driven visual fingerprint
rather than readable text placed over texture. No field mapping, time-history
policy or implementation is approved; keep such experiments separate from the
accepted wide/glyph direction.

Refined empty-view explanations and incomplete-data/tracking notices are parked
for focused design, not removed as a correctness requirement. The comparison
uses minimal explicit notices; final debug access remains open. The revised
full-height stack occupied the former strip warning area.
[0039](decisions/0039-bottom-left-condition-driven-notices.md) selects bottom-left
condition-driven notices, with retained comparisons [above](#bottom-left-notice-reference);
[0040](decisions/0040-white-backed-bottom-left-notice-contract.md) now implements
their geometry and white-backed compositing. A normal view must not conceal
failed/omitted tracking as a complete empty sky, including when some valid markers
remain.

The selected-satellite start/end/peak pass graphic is interesting but deferred;
upcoming-pass lists are not desired. Local time, orbit-data age, range and bearing
are not part of this starting layout. Camera movement/panning, marker slowdown,
new automatic selection policy, fetching and production population/cadence
choices are outside this slice. No radio information or battery indicator.

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
