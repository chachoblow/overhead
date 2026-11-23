# Simulator

400×240, 1-bit real-data radar replay. The SDL window uses the LcdWhite theme
at 2×; rendering and clockless application state live in `overhead-render` and
`overhead-app`, not in SDL. Requires SDL2: the existing Homebrew/linker setup on macOS, or `libsdl2-dev`
on Linux. On a headless host, use `--snapshot`; the window needs a display.

## Run

From the workspace root:

```sh
cargo run -p overhead-sim -- --demo
cargo run -p overhead-sim -- --demo --fov-deg 100
cargo run -p overhead-sim -- --help
```

`--demo` explicitly selects the pinned historical 16-object pool, UTC
`2026-10-10T00:00:00Z`, and observer `39.007 -104.883 2.187`. It uses paths in
this build's checkout. It is not live tracking or a production catalogue.
At the default 20° lens setting its four above-horizon objects are outside the
viewport at the start time; zoom out or use `--fov-deg 100` to reveal them.
Without arguments the program prints an input error rather than guessing time.

For another local catalogue, time or observer:

```sh
cargo run -p overhead-sim -- sim/examples/weather-gps.json \
  2026-10-10T00:00:00Z 39.007 -104.883 2.187 \
  --fov-deg 100 --radar-size 240
```

The five positional arguments are manifest, UTC, latitude degrees, east
longitude degrees, and WGS-84 ellipsoidal height km. UTC must end in `Z`, in
1957–2100, without leap seconds. Replay advances at 1× from that explicit time
using monotonic elapsed time, not the computer's wall clock. No live fetching,
cache, refresh, local-time conversion or freshness cutoff is implemented.

[Manifest format, acceptance and diagnostics](../tools/README.md#overhead-catalogue)
are shared with the catalogue tool. Group paths resolve relative to the manifest.
The complete accepted provenance report and ingestion warnings go to stderr.
Missing fetch metadata remains unknown; it is not inferred from filenames.

## Controls and view

- Wheel/up arrow: zoom in. Down arrow: zoom out. Each step scales magnification
  by 1.2×, clamped to the lens range; the camera does not move.
- Tab: select the next in-view NORAD ID, wrapping; with no in-view markers,
  leave a retained wide-view selection alone. In the wide view, a successfully
  tracked above-horizon satellite stays selected even after zoom or motion takes
  it off-screen. On horizon/tracking loss, fall back to the first in-view ID or
  clear selection. Square/circular comparisons remain visible-only. This is
  inspection, not a new ranking.
- `v`: toggle radar/text-first debug in interactive `--wide` replay only. Replay,
  zoom and Tab stay active in debug; returning shows the current normal frame.
  Holding `v` does not repeatedly toggle.
- Escape/window close: quit.
- `--fov-deg`: full vertical angle, 0.1°–100°; default 20°. Also the horizontal
  angle in square/circular modes; wide mode derives horizontal coverage from aspect.
  The former `--radius-km` option is rejected with migration guidance.
- `--radar-size`: left-hand radar allocation, 180–240 pixels; default 240.
  The viewport stays vertically centered. Smaller values give more panel width.
- `--viewport`: `square` (default, no outer border), `circle` (circular clipping
  and ring), or `borderless-circle` (circular clipping, no ring).
  See [comparison](#viewport-shape-comparison).
- `--wide`: the device's full-width 400×240 radar
  ([0089](../docs/decisions/0089-full-width-radar-revision.md)): left pills,
  octagon markers, below-horizon context diamonds, pulsing selected ring,
  off-screen arrows, scale ruler and bottom-left notices. Incompatible with
  `--radar-size` and `--viewport`. See [wide view](#wide-view-0089).
- `--debug`: start interactive `--wide` replay on the radar-free tracking/data-health
  page. Requires `--wide`; incompatible with `--snapshot`. See [debug access](#debug-access).

The [perspective contract](../docs/decisions/0026-fixed-camera-perspective-slice.md)
owns lens semantics, with square clipping now selected by
[0028](../docs/decisions/0028-borderless-square-viewport.md).
[0027](../docs/decisions/0027-higher-fixed-camera-overview.md)
raises the fixed camera to 75,000 km along geodetic local up from the observer,
looking back at that observer, north up and east right. The central cross is the observer aim point, small crosses are
actual 3D satellite positions, and the diamond is the selected satellite.

The default viewport is a borderless square. Earth's outline is the WGS-84
ellipsoid through the same perspective camera. Satellites can appear outside
its silhouette. The circular comparison mode draws a separate outer ring.
At terrestrial sites Earth spans roughly 45% of the viewport width at the default
20°, 14% at 60°, and 6.6% at 100° (about 15 pixels at the default layout).
At tight zoom the limb can leave the view entirely.
The opt-in wide comparison now draws coastline strokes; square/circular modes have
none. There is no filled occlusion mask. `FOV ... DEG` is a lens angle,
**not surface, map or slant distance**.

Only satellites above the true local geometric horizon are eligible for markers,
selection, arrows and counts; `--wide` also draws the group's below-horizon
objects the camera can see as light context diamonds (0089). The camera also clips forward depth < 1 km (including
behind-camera objects) and marker centers outside the selected viewport. Lens zoom
cannot put an object behind the camera or bring one in front. Camera configurations
with a nonfinite Earth view or Earth crossing the near plane fail explicitly;
these concern extreme observer heights, not ordinary terrestrial sites.

`VIEW` is drawn marker count, `SKY` is above-horizon count, and `CAT` is accepted
catalogue count. They need not agree. Overlapping markers are not spread apart.
Altitude, slant range and elevation use true physical samples. `EPOCH +... H`
is elapsed hours since the selected elements' epoch; a negative value means a
future epoch, not an error. It is not a freshness assessment. Names are sanitized
for the font; `+` at the end of a clipped line indicates truncation.

Failed objects lose their old markers, increment `ERR`, and are identified with
causes in stderr when the failure set changes. `INGEST WARN` means some source
records were omitted: see stderr. An empty view with failures is not presented
as a complete empty sky. Fatal configuration/load/clock/output errors exit 1;
success/help exits 0. A rendered snapshot containing per-object failures still
exits 0, with its nonzero `failed` count reported explicitly.

The default panel described here is still the diagnostic presentation; the
wide view is described [below](#wide-view-0089). The strip, glyph and
marker-label options of 0034–0046 were retired by 0089.

The synchronous loop targets 20 Hz on the host, without a catch-up queue. This
is a visual test loop, **not a selected device cadence or bounded scheduler**.
No pass countdown, motion slow-down or hardware access is included.

## Debug access

Under [0047](../docs/decisions/0047-text-first-debug-access-contract.md), interactive
wide replay has a separate radar-free page for tracking/data health. Start there:

```sh
cargo run -p overhead-sim -- sim/examples/weather-gps.json \
  2026-10-10T00:00:00Z 39.007 -104.883 2.187 --wide --debug
```

Press `v` to return to radar, and again to inspect debug. Tab and wheel/up/down
continue changing the underlying wide view while debug is open. Replay never
pauses or switches to wall-clock UTC; toggling does not change geometry, counts,
selection or physical samples. Returning discards no history
and draws current static endpoints; subsequent normal-view selection changes
animate as before. Normal notices remain condition-driven and cannot be dismissed
by visiting debug. Square and circular comparisons ignore `v`.

The fixed text page shows replay UTC, vertical FOV, separate catalogue/sky/view/
failure counts, ingestion status, the first failed NORAD ID/cause, and selected
identity plus true altitude, range, elevation, TEME speed, nominal period and
signed epoch offset. Epoch offset is not freshness, period is not pass duration,
and speed is not ground speed. No selection omits selected values; unavailable
individual fields are explicit. Unsampled frame counts show `--`, not zero;
`NO FRAME` is renderer-tested, while fatal host load/clock errors still exit.

Names wrap across two rows; omissions and unsupported-character replacements
explicitly point to the original host report. A bounded first cause likewise
points to complete causes/other failed IDs in stderr. The accepted catalogue and
ingestion report is printed at load, and full tracking causes when the failure
set changes. There is no on-screen log history or fetching. Exact geometry,
precision and overflow behavior belong to the [approved layout reference](../docs/mockups/0047-debug-layout-proposal.md).

For executable review, use the command above to inspect selected readings, Tab,
zoom, and repeated `v` round trips, including zooming to an empty viewport and
back. To review selected-plus-failure or all-failed pages, first create the
disposable inputs in the [notice review recipe](#notice-overlay-review), then run:

```sh
cargo run -p overhead-sim -- "$review_dir/combined.json" \
  2026-10-04T12:43:41.833056Z 0.000017409 -85.180357315 0 --wide --debug
# Repeat with "$review_dir/failed.json" for no successful selection.
```

Those failures/omissions are synthetic controls, not reported catalogue incidents.
The user accepted the fresh executable output separately from layout approval
and passing implementation tests. Tests invoke input/presentation code without
an SDL window; neither those tests nor user visual acceptance establish
physical-display legibility or target performance.
`--debug` without `--wide`, repeated flags, and `--debug --snapshot` are rejected.
There is no debug snapshot option: existing square and `--wide --snapshot` outputs
remain static normal images.

## Deterministic snapshots and density comparison

```sh
cargo run -p overhead-sim -- --demo --snapshot /tmp/overhead-pool.png
cargo run -p overhead-sim -- sim/examples/weather-gps.json \
  2026-10-10T00:00:00Z 39.007 -104.883 2.187 \
  --snapshot /tmp/overhead-weather-gps.png
cargo run -p overhead-sim -- sim/examples/weather-gps.json \
  2026-10-10T00:00:00Z 39.007 -104.883 2.187 \
  --fov-deg 100 --snapshot /tmp/overhead-wide.png
cargo run -p overhead-sim -- sim/examples/weather-gps.json \
  2026-10-10T00:00:00Z 39.007 -104.883 2.187 \
  --fov-deg 5 --radar-size 200 --snapshot /tmp/overhead-local.png
```

Snapshots use exactly the supplied start UTC, write a 400×240 PNG, print counts
and exit without opening SDL. A later explicit UTC gives another physical frame;
there is no hidden accelerated clock. At the example time/site:

| Catalogue | Above horizon | Visible at 20° | Visible at 100° | Visible at 5° |
|---|---:|---:|---:|---:|
| Historical pool (16) | 4 | 0 | 4 | 0 |
| Weather/GPS (105) | 22 | 6 | 22 | 4 |

Weather/GPS has zero markers at the tightest 0.1°. All these examples have zero
calculation failures. Counts are checked against an independent physical-look-angle
projection, not copied from the old ground map. These are reproducible observations,
not capacity or density targets. Visual feedback and open panel questions live
in [DESIGN](../docs/DESIGN.md#interface-intent), not in these example populations.

Both manifests reference existing immutable snapshots; they do not copy or
modify orbital data. `catalogue.json` uses the selected 16-object pool;
`weather-gps.json` uses the archived 73-record weather and 32-record GPS groups.
[Acquisition, exact bytes, selection and provenance](../tools/fixtures/m2-pool/README.md)
remain there. These archives include historical/inactive objects and are not a
chosen production group set. Source URLs are provenance only, never fetched.

## M4 executable coastline comparison (appearance accepted)

`--wide` draws coastline behind markers, limb, pills and condition-driven
notices, sphere-shaded beyond the 1,000 km sky ring (0089). Since [0058](../docs/decisions/0058-prepared-wide-coastline-slice.md) it
uses the accepted quadratic representation, prepared once for the fixed observer
(a full-screen `PREPARING MAP` frame precedes the first interactive frame; stderr
reports host preparation time) and reused across motion and zoom; a cached raster
of the current view is redrawn only when the zoom changes ([0059](../docs/decisions/0059-current-view-coastline-raster-cache.md)). The square/circular
baseline remains coast-free. Example **headless** executable reviews (outputs are 1-bit):

```sh
cargo run -p overhead-sim -- sim/examples/weather-gps.json \
  2026-10-10T00:00:00Z 37.7749 -122.4194 0 \
  --wide --fov-deg 0.1 --snapshot /tmp/overhead-coast-sf-tight.png
cargo run -p overhead-sim -- sim/examples/weather-gps.json \
  2026-10-10T00:00:00Z 39.007 -104.883 2.187 \
  --wide --fov-deg 20 --snapshot /tmp/overhead-coast-co.png
```

[0054](../docs/decisions/0054-first-executable-coastline-comparison.md) owns the
original comparison scope and open no-omission/target-cost gates. The user reviewed the
[fresh executable scenes](../docs/mockups/0061-first-executable-coastline-review.md)
and tried the simulator, accepting their appearance; neither that acceptance nor
the earlier offline mocks proves physical-display legibility or target cost.
The reviewed scenes show the earlier 0054 renderer; the quadratic candidate's
appearance was accepted separately under 0056.
A `COAST ERROR` notice reports a genuinely invalid coast-only calculation/asset,
failed preparation or a layer prepared for a different observer;
display write errors still fail the whole frame.

## Viewport shape comparison

The user selected borderless square under [0028](../docs/decisions/0028-borderless-square-viewport.md),
keeping the working 240×240 allocation and provisional 20° default. Circular
modes remain available for comparison; panel contents are not settled.

```sh
for mode in circle borderless-circle square; do
  cargo run -p overhead-sim -- sim/examples/weather-gps.json \
    2026-10-10T00:00:00Z 39.007 -104.883 2.187 \
    --fov-deg 20 --radar-size 240 --viewport "$mode" \
    --snapshot "/tmp/overhead-$mode.png"
done
```

All three retain the same camera, lens scale, Earth outline, observer aim point,
N label and panel. Both circular modes have identical marker eligibility;
only the outer ring differs. Square mode keeps the same inset and horizontal/
vertical extent, admitting the corners rather than rescaling the view. Its
**diagonal** FOV is consequently wider. Glyphs and the Earth outline are clipped
to the chosen shape; counts and selection use that same viewport. Physical
samples, true horizon eligibility and failures are unchanged. The option also
works in the interactive replay; it is not a runtime shape-switch control.

At the pinned weather/GPS time/site, all modes show six markers at 20°; the two
borderless images are identical because no marker occupies an extra corner.
For a visible corner comparison, repeat at 40°: circular modes show 17 markers,
square shows 18. All have SKY 22 and zero failures. This does not select 40° as
the default or change the production population.

## Wide view (0089)
<a id="wide-radar-comparison"></a>

`--wide` is the device's view. Compare at the same explicit UTC, observer and
**vertical** FOV; `sim/examples/visual.json` is the device's pinned `visual`
group (2026-11-03):

```sh
cargo run -p overhead-sim -- sim/examples/visual.json \
  2026-11-03T02:00:00Z 40 -105 1.6 --wide --fov-deg 20 --snapshot /tmp/overhead-wide.png
cargo run -p overhead-sim -- --demo --wide --snapshot /tmp/overhead-demo-wide.png
```

The radar fills 400×240 with the aim point at (200, 120) and drawable
half-extents 197×117 pixels. Horizontal FOV is
`2 * atan(tan(vertical_fov / 2) * 197 / 117)`, not a second zoom; Earth and
satellites keep the square baseline's pixel magnification.

- **Markers:** 7 px octagon with a 1 px cleared halo for each in-view
  above-horizon satellite; the group's below-horizon objects as 5 px hollow
  diamonds with halo, skipped where the Earth hides them. Those are context
  only: never selected, counted or given arrows.
- **Selected:** its octagon on top and a ring pulsing outward over 1.4 s,
  counted from when that satellite was selected (`RadarApp::selected_since`).
- **Selected orbit:** one period centred on now, solid, broken where the
  Earth hides it (the inertial orbit turned to the Earth's orientation at the
  frame, so it passes through the selected marker).
- **Trails:** four straight steps behind each unselected in-view
  above-horizon marker, `clamp(60 s × tan(fov/2) / tan 10°, 1 s, 900 s)` per
  step (1 min at 20°, ~3.3 min at the device's 60°), cut where the Earth hides
  a past point. Both come from `overhead_app::motion::Motion`, kept beside the
  app; the replay first runs 30 min of 10 s frames before the start so trails
  are full at the first frame (a device grows them as it runs).
- **Pills** (top left): NORAD ID, name (9 characters), EL, AZ, ALT, RNG, SPD,
  PER and the visibility word, all true state; `NO SEL` / `SKY` / `VIEW` when
  nothing is selected; none without a frame.
- **Earth:** coastline full tone inside the 1,000 km sky ring, then sphere
  shading to 12.5% at the limb, applied once per lens to the cached raster;
  solid WGS-84 outline.
- **Scale ruler** (bottom right): km per the aim point's ground scale; hidden
  without a frame.

One filled outward arrow ([0061](../docs/decisions/0061-filled-arrows-id-backing-none-in-view.md),
with a 1 px cleared halo) is drawn for each successful, above-horizon satellite
whose finite projection is outside the view and whose camera depth is at least
1 km. Arrows follow the projection direction, inset to fit the complete glyph.
They are not velocities, physical edge positions, or distance encodings.
**Overlaps are allowed: no arrows are suppressed, merged, or spread apart.**
Near-plane/behind-camera objects receive no arrow.

Under [0049](../docs/decisions/0049-stable-wide-selection.md), the wide view
retains a successfully tracked selected ID above the horizon even if zoom or
motion takes it off-screen; its pills stay current. Initial acquisition and
genuine loss choose the first in-view NORAD ID; Tab cycles only in-view markers.
A standalone headless snapshot starts without selection history and never
selects an off-screen object. The snapshot report adds `indicators=N`, the
number of arrows submitted (zero in baseline modes).

Under [0040](../docs/decisions/0040-white-backed-bottom-left-notice-contract.md),
complete notice lines are bottom-left at `x=12`, ending at baseline `y=222`, with
13px spacing, on a borderless white backing drawn last. It may cover markers,
arrows or Earth in that area without changing eligibility, state or counts.

Notices are condition-driven, not timed pop-ups:
- `NONE IN VIEW` ([0061](../docs/decisions/0061-filled-arrows-id-backing-none-in-view.md); formerly `SKY OUTSIDE VIEW`): no in-view markers but at least one valid off-screen arrow.
  This reports tracked sky outside this viewport, not a promise that zoom can
  reveal every tracked object.
- `NONE ABOVE HORIZON`: no in-view markers or arrows, and a nonempty accepted
  catalogue has no above-horizon samples, tracking failures or ingestion warning.
  This describes only the loaded catalogue at this UTC, not the complete sky.
- `NO MARKERS`: remaining valid-time zero-visible cases, including empty input,
  failed/incomplete tracking, or above-horizon objects without valid arrows.
  Any bottom status clears as soon as a marker enters view.
- `TRACK ERROR`: at least one current calculation failed, including when other
  markers remain valid. It clears on a frame with no tracking failures.
- `INGEST WARN`: the catalogue load produced diagnostics, such as omitted records.
  It persists for the current run; a new clean load clears it. No live reload exists.
- `NO FRAME`: the renderer has no valid-time frame; this replaces `NO MARKERS`.
  The current simulator exits on invalid-time errors rather than waiting in that
  state. Renderer tests exercise it directly.

Active lines appear in status/tracking/ingestion order. No active notice means
neither text nor backing. Redrawing restores scene pixels as notices disappear or
the block shrinks; there is no timeout, dismissal control or permanently reserved
box. The maximum three-line backing occupies `x=10..121`, `y=187..226` (the
18-character horizon status sets the widest case).
Debug exposes the same conditions separately without replacing these comparison
notices. Counts appear only as the no-selection pills.

At the pinned weather/GPS time/site above, all frames have SKY 22 and zero failures:

| Vertical FOV | Square markers | Wide markers | Wide arrows |
|---|---:|---:|---:|
| 0.1° | 0 | 0 | 22 |
| 5° | 4 | 4 | 18 |
| 20° | 6 | 8 | 14 |
| 40° | 18 | 18 | 4 |
| 100° | 22 | 22 | 0 |

The demo at 20° has zero markers and four arrows; at 100° it has four markers
and no arrows (unchanged by 0089's wider radar at these sites). These are
historical replay observations, not capacity targets.
Repeat with UTC `2026-10-10T00:00:30Z` for another physical frame. CLI tests cover
both catalogues, both times, these zooms, independent visibility/arrow counts,
and deterministic snapshots with SDL unavailable. Tight-view images show some
overlapping arrows; no collision policy has been added to make them countable.

### Notice overlay review

To regenerate the selected-plus-warnings, empty-plus-warnings and crossing-marker
examples from a fresh checkout, run this from the workspace root. Python 3's
standard library creates disposable inputs outside the repository; it does not
modify fixtures or fetch data. The ISS orbit is real, but the duplicate overflow
orbit and malformed record are **synthetic fault controls**, not evidence of a
source-catalogue incident. The crossing observer is a deliberate visual test site.

```sh
review_dir="$(mktemp -d "${TMPDIR:-/tmp}/overhead-notice-review.XXXXXX")"
python3 - "$review_dir" <<'PY'
import json
import sys
from pathlib import Path

out = Path(sys.argv[1])
records = json.loads(Path("core/tests/fixtures/iss-25544.json").read_text())
bad = dict(records[0], NORAD_CAT_ID=99999, MEAN_MOTION=1e300)
for name, data in {
    "clean": records,
    "ingest": records + [{"bad": "record"}],
    "combined": records + [bad, {"bad": "record"}],
    "failed": [bad, {"bad": "record"}],
}.items():
    (out / f"{name}-data.json").write_text(json.dumps(data))
    (out / f"{name}.json").write_text(json.dumps({"groups": [
        {"name": "synthetic-notice-review", "path": f"{name}-data.json"}
    ]}))
PY
cargo build -p overhead-sim
for case in combined failed; do
  target/debug/overhead-sim "$review_dir/$case.json" \
    2026-10-04T12:43:41.833056Z 0.000017409 -85.180357315 0 \
    --wide --snapshot "$review_dir/$case.png"
done
for case in clean ingest; do
  target/debug/overhead-sim "$review_dir/$case.json" \
    2026-10-04T12:43:41.833056Z 0.480017409 -84.480357315 0 \
    --wide --fov-deg 0.1 --snapshot "$review_dir/crossing-$case.png"
done
printf 'Review images: %s\n' "$review_dir"
```

`combined.png` has one valid selected ISS marker, one tracking failure and an
ingestion warning. `failed.png` has no valid markers and all three notice lines
(`NO MARKERS`, `TRACK ERROR`, `INGEST WARN`).
Compare `crossing-clean.png` with `crossing-ingest.png`: the overlay can cover the
ISS marker while its pills and the `visible=1` report remain.
Clean loading restores that scene; this is not a live ingestion-warning timeout.
`NO FRAME` and same-app redraw/recovery are covered by renderer tests, not these CLI images.
These are executable snapshots.

### Retired strip and label sections
<a id="static-elevation-glyph"></a><a id="static-altitude-glyph"></a><a id="static-speed-glyph"></a><a id="static-period-glyph"></a><a id="static-category-glyph"></a><a id="visibility-glyph"></a><a id="label-zoom-visibility"></a><a id="identification-review"></a>
The glyph strip (0038–0046, 0078's pictogram), the marker-adjacent ID (0034)
and its zoom visibility (0035) were retired by
[0089](../docs/decisions/0089-full-width-radar-revision.md). Their usage and
review recipes are in this README at revision `4ac3778`.

### Visibility pill

Under [0078](../docs/decisions/0078-visibility-glyph.md)'s rules, shown as a
pill word since 0089: `VISIBLE` when the selected object is sunlit while the
observer's Sun is below −6°, `SHADOW` when it is in Earth's shadow, and
`DAYLIGHT` when the observer's sky is too bright (Sun at or above −6°).

Review recipe: the ISS from the pinned `visual` fixture, observed from its own
subpoint at three moments of 2026-11-03 (single-object review group; the sites
are review inputs, not device settings):

```sh
review_dir=$(mktemp -d)
python3 -c "import json,sys; d=json.load(open('firmware/fixtures/celestrak-visual-20261103.json')); json.dump([r for r in d if r['NORAD_CAT_ID']==25544], open(sys.argv[1],'w'))" "$review_dir/iss.json"
printf '{"groups":[{"name":"iss","path":"%s"}]}' "$review_dir/iss.json" > "$review_dir/catalogue.json"
for case in "daylight 2026-11-03T00:05:00Z -44.7576 -121.6808" \
            "visible 2026-11-03T00:16:00Z -16.3560 -86.5176" \
            "eclipsed 2026-11-03T00:21:00Z -1.2043 -75.3781"; do
  set -- $case
  cargo run -q -p overhead-sim -- "$review_dir/catalogue.json" "$2" "$3" "$4" 0 \
    --wide --fov-deg 20 --snapshot "$review_dir/visibility-$1.png"
done
```

Each report reads `catalogue=1 sky=1 visible=1`. Shadow edges use a cylinder
(no penumbra), so a state can change a few seconds early or late at the edge.

## Verify

```sh
cargo check --workspace
cargo test -p overhead-app -p overhead-render -p overhead-sim
```

Tests cover actual-position perspective, altitude/depth effects, fixed-camera
magnification, near-plane/circular/square boundaries, poles/dateline, independent
ellipsoid tangency, true-state traceability, horizon counts, failed-frame
replacement, selection/zoom, clipped outline pixels, explicit clock/config,
and real CLI PNG snapshots with SDL unavailable. Wide tests cover rectangular
aspect and pixel scale, per-object arrows, marker entry and notices in empty and
partially successful frames, plus 0089: the selected octagon's cleared halo and
pulse ring (restarting per selection), below-horizon diamonds (Earth occlusion,
never selected or counted), pill contents and drawing, the scale ruler, the
shading table and its bake into the coast raster. CLI tests find each expected
pill value (EL, ALT, SPD, PER, visibility, derived outside the app) in real
snapshots, including low/navigation/GEO orbits and warning states. Debug tests
cover fixed cells, text boundaries, field-local availability and stale-ink
clearing; simulator unit tests cover wide-only nonrepeating `v` toggles with
unchanged app truth, Tab/zoom and replay while debugging, and preserved notices.

The [2026-10-12 verification record](../docs/PROGRESS.md#2026-10-12-utc-square-default-fresh-session-verification-before-commit)
contains the prior fresh-source build, broader check suite, headless counts,
reviewed-image comparison and SDL dummy-loop result. Those historical passes
are not verification of later changes or an agent interactive input/hardware test.

Existing Rust tool/dependency caches and Homebrew SDL2 remain prerequisites;
no ignored build artifacts or disposable panel mockups are needed. `--demo`
uses the checkout's paths when built there.

Frozen M2 evidence is a separate historical verification scope: use its
[pinned revision workflow](../docs/evaluations/s3-m2/README.md#offline-validation-and-table-regeneration),
not the current workspace lockfile. No target-capacity, whole-device RAM,
safe-stack or interactive usability claim follows from simulator checks.
