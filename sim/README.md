# Simulator

400×240, 1-bit real-data radar replay. The SDL window uses the LcdWhite theme
at 2×; rendering and clockless application state live in `overhead-render` and
`overhead-app`, not in SDL. Requires the existing Homebrew SDL2/linker setup.

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
- Tab: select the next visible NORAD ID, wrapping. Selection stays while visible;
  otherwise the first visible ID is used. This is inspection, not final ranking.
- `v`: toggle radar/text-first debug in interactive `--wide` replay only. Replay,
  zoom and Tab stay active in debug; returning shows current normal state without
  queued glyph motion. Holding `v` does not repeatedly toggle.
- Escape/window close: quit.
- `--fov-deg`: full vertical angle, 0.1°–100°; default 20°. Also the horizontal
  angle in square/circular modes; wide mode derives horizontal coverage from aspect.
  The former `--radius-km` option is rejected with migration guidance.
- `--radar-size`: left-hand radar allocation, 180–240 pixels; default 240.
  The viewport stays vertically centered. Smaller values give more panel width.
- `--viewport`: `square` (default, no outer border), `circle` (circular clipping
  and ring), or `borderless-circle` (circular clipping, no ring).
  See [comparison](#viewport-shape-comparison).
- `--wide`: opt-in 352×240 borderless radar with per-satellite off-screen arrows,
  selected-marker identification, a 48-pixel category/elevation/altitude/speed/period
  strip and bottom-left white-backed notices. Incompatible with
  `--radar-size` and `--viewport`.
  See [wide comparison](#wide-radar-comparison).
- `--debug`: start interactive `--wide` replay on the radar-free tracking/data-health
  page. Requires `--wide`; incompatible with `--snapshot`. See [debug access](#debug-access).
- `--label-hide-deg` / `--label-show-deg`: wide-view ID visibility thresholds,
  default 70°/60° vertical FOV. Both require `--wide`; see
  [label zoom visibility](#label-zoom-visibility).

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
There are no coastlines or filled occlusion mask. `FOV ... DEG` is a lens angle,
**not surface, map or slant distance**.

Only satellites above the true local geometric horizon are eligible; zoom cannot
reveal below-horizon objects. The camera also clips forward depth < 1 km (including
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

The default panel described here is still the diagnostic presentation. The
opt-in wide comparison implements the viewport/arrows portion of
[0032](../docs/decisions/0032-wide-radar-and-selected-satellite-glyphs.md), under
[0033](../docs/decisions/0033-wide-radar-comparison-contract.md), plus selected-marker
identification under [0034](../docs/decisions/0034-selected-marker-identification.md)
and [0035's zoom visibility](../docs/decisions/0035-label-zoom-visibility.md).
[0041's static circular elevation](../docs/decisions/0041-circular-elevation-only-contract.md),
`Elev` label and padded separator are implemented in the wide view, alongside
[0042's static altitude circles](../docs/decisions/0042-static-altitude-glyph-contract.md)
and `Alt` label, plus
[0043's static TEME speed arc](../docs/decisions/0043-static-speed-glyph-contract.md)
and `Spd` label, plus
[0044's static nominal-period spiral](../docs/decisions/0044-static-period-glyph-contract.md)
and `Per` label, with
[0040's bottom-left notice overlay](../docs/decisions/0040-white-backed-bottom-left-notice-contract.md).
[0045's static group-derived category](../docs/decisions/0045-static-group-category-contract.md)
and `Cat` label are also implemented; the user subsequently accepted the fresh
executable category output. [0046's selected-data transitions](../docs/decisions/0046-selected-data-glyph-transition-contract.md)
are implemented in the interactive wide view; the user subsequently reviewed
and accepted their appearance. Headless `--snapshot` and square diagnostics
remain static. [0047's text-first debug access](../docs/decisions/0047-text-first-debug-access-contract.md)
is implemented after separate authorization; the user subsequently reviewed and
accepted the fresh executable output. Elevation, altitude, speed and period are also visually accepted as static fields.
[DESIGN's retained schematic](../docs/DESIGN.md#selected-satellite-presentation)
remains a design reference, not this executable's output or a golden test.
The default square diagnostic panel is unchanged by debug access.

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
selection or physical samples. Returning discards hidden glyph-transition history
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

## Wide-radar comparison

`--wide` is a bounded real-data experiment, not a new default or the finished
normal view. Compare at the same explicit UTC, observer and **vertical** FOV:

```sh
cargo run -p overhead-sim -- sim/examples/weather-gps.json \
  2026-10-10T00:00:00Z 39.007 -104.883 2.187 \
  --fov-deg 20 --snapshot /tmp/overhead-square.png
cargo run -p overhead-sim -- sim/examples/weather-gps.json \
  2026-10-10T00:00:00Z 39.007 -104.883 2.187 \
  --wide --fov-deg 20 --snapshot /tmp/overhead-wide.png
cargo run -p overhead-sim -- --demo --wide --snapshot /tmp/overhead-demo-wide.png
```

The 352×240 radar has an aim point at (176, 120) and drawable half-extents
173×117 pixels. Horizontal FOV is
`2 * atan(tan(vertical_fov / 2) * 173 / 117)`, not a second independent zoom.
Earth and satellites keep the square baseline's pixel magnification; more space
is admitted horizontally. The 48-pixel strip holds the selected satellite's
[category](#static-category-glyph), [elevation](#static-elevation-glyph),
[altitude](#static-altitude-glyph), [speed](#static-speed-glyph) and
[period](#static-period-glyph) glyphs; status notices are at bottom-left
inside the radar, not in the strip.

One hollow outward arrow is drawn for each successful, above-horizon satellite
whose finite projection is outside the view and whose camera depth is at least
1 km. Arrows follow the projection direction, inset to fit the complete glyph.
They are not velocities, physical edge positions, or distance encodings.
**Overlaps are allowed: no arrows are suppressed, merged, or spread apart.**
Exact overlaps can look like a single arrow. Near-plane/behind-camera objects
receive no arrow; zoom cannot change their depth. Objects become ordinary
markers when their true projections enter the viewport.

Selection remains visible-marker-only, with the same Tab/zoom behavior as the
baseline; arrows never count as visible markers. The snapshot report adds
`indicators=N`: the number of arrows submitted, not necessarily distinguishable
symbols (zero in baseline modes). All physical diagnostics remain available in
host reports and the baseline panel.

When the zoom gate permits it, the selected visible marker carries its bare NORAD
ID, except `25544` shows `ISS`.
Only identity controls this exception, not the catalogue name. The 6×10 text
prefers the diamond's right side, flips left if needed, and clamps vertically to
keep the entire label inside the drawable radar. Longer IDs are not truncated to
five or six digits. Text is transparent: overlaps with scene graphics are allowed,
without a clearing mask or collision search. Selection/zoom updates the label
immediately; arrows are never labelled. Wide zoom hides the label as described
below; the selected diamond and square baseline are unchanged.

The strip shows a labelled static circular cutout of true elevation,
concentric circles of true WGS-84 altitude and an arc of true TEME speed
when a visible satellite is selected;
its separator remains without selection. Under
[0040](../docs/decisions/0040-white-backed-bottom-left-notice-contract.md), complete
notice lines are bottom-left at `x=12`, ending at baseline `y=222`, with 13px
spacing. A borderless white backing pads the active text block by 2px and is drawn
last. It may visually cover markers (including the selected diamond), IDs, arrows
or Earth within that small area. This does not change eligibility, physical state,
selection or reported counts; counts are not an unoccluded-pixel tally.

Notices are condition-driven, not timed pop-ups:
- `NO MARKERS`: no in-view markers in a valid-time frame, even if arrows remain.
  It clears when motion or zoom brings a marker into view; it does not claim a
  complete empty sky.
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
box. The maximum three-line backing occupies only `x=10..79`, `y=187..226`.
Debug exposes the same conditions separately without replacing these comparison
notices. No routine FOV/count text is shown in the wide normal image.

At the pinned weather/GPS time/site above, all frames have SKY 22 and zero failures:

| Vertical FOV | Square markers | Wide markers | Wide arrows |
|---|---:|---:|---:|
| 0.1° | 0 | 0 | 22 |
| 5° | 4 | 4 | 18 |
| 20° | 6 | 8 | 14 |
| 40° | 18 | 18 | 4 |
| 100° | 22 | 22 | 0 |

The demo at 20° has zero markers and four arrows; at 100° it has four markers
and no arrows. These are historical replay observations, not capacity targets.
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
ingestion warning. `failed.png` has no valid markers and all three notice lines.
Compare `crossing-clean.png` with `crossing-ingest.png`: the overlay can cover the
ISS diamond/ID while all four data fields and the `visible=1` report remain.
Clean loading restores that scene; this is not a live ingestion-warning timeout.
`NO FRAME` and same-app redraw/recovery are covered by renderer tests, not these CLI images.
These are executable snapshots, unlike DESIGN's illustrative full-stack mocks.

### Static elevation glyph

Under [0041](../docs/decisions/0041-circular-elevation-only-contract.md), `--wide`
draws a 23px circle within a 24×24 footprint, with a crisp `Elev` label and padded
vertical separator. There is no capsule, glyph box, fill or numeric value. The
cutout opens upward from the rightward radius: at 0° the circle is complete
without a spoke, at 45° it has an eighth-circle opening, and at 90° a quarter-circle
opening. Both radial edges and the remaining circle are outlined. The decision
owns exact geometry, angular snapping and single-pixel raster rules; quantization
never changes the true physical sample.

The glyph reads the selected satellite's current physical elevation directly.
In these headless snapshots, Tab selection, frame updates and zoom-driven
reselection show current values without animation. Interactive selection changes
use [0046's transition](../docs/decisions/0046-selected-data-glyph-transition-contract.md).
Hiding the marker's ID at wide zoom does **not** hide `Elev`
or the glyph. Without selection, clear both label and glyph while retaining the
separator; no stale reading remains. Nonfinite input likewise suppresses the
label/glyph. Notices retain their independent bottom-left overlay.

To review selected, ID-hidden overview and no-selection executable output:

```sh
review_dir="$(mktemp -d "${TMPDIR:-/tmp}/overhead-elevation-review.XXXXXX")"
for fov in 20 100 0.1; do
  cargo run -p overhead-sim -- sim/examples/weather-gps.json \
    2026-10-10T00:00:00Z 39.007 -104.883 2.187 \
    --wide --fov-deg "$fov" --snapshot "$review_dir/elevation-$fov.png"
done
printf 'Review images: %s\n' "$review_dir"
```

Repeat with UTC `2026-10-10T00:00:30Z` for a later physical frame. The demo at
20° also has no selection. The [notice review recipe](#notice-overlay-review)
exercises a selected ISS near 90° with synthetic warnings, empty/failure state,
and a real selected marker/ID visually covered by the notice backing. These
executable glyphs use true historical samples, unlike DESIGN's illustrative
stack and endpoint references. The same snapshots now include altitude under
0042, speed under 0043, nominal period under 0044 and category under 0045;
headless snapshots do not animate transitions.

### Static altitude glyph

Under [0042](../docs/decisions/0042-static-altitude-glyph-contract.md), `--wide`
adds `Alt` and two concentric outlines below elevation. The outer circle stays
fixed; the inner circle shrinks as the selected sample's true WGS-84 ellipsoidal
altitude increases. A fixed compressed 0–40,000 km scale spans low Earth,
navigation and geostationary heights using eight inner sizes. Nearby altitudes
can share a raster; high values saturate at the smallest ring. This is a broad
height cue, not a precise numeric readout or a tracking limit. The decision owns
the exact scale, rounding and pixel rules; zoom and catalogue never rescale it.

These static snapshots show immediate true-state fields; interactive selected-ID
changes animate the data glyphs under 0046. The fields remain visible when the
marker ID is hidden. Nonfinite altitude clears only `Alt` and its circles,
preserving other valid fields. Without selection,
all selected labels/glyphs clear, leaving the separator. Notices and square diagnostics
are unchanged. These behaviors are implemented, and the user reviewed and
accepted fresh executable altitude output separately from the illustrative mocks.

The [elevation recipe](#static-elevation-glyph) covers selected, ID-hidden and
no-selection states at the usual site; repeat its UTC at `2026-10-10T00:00:30Z`
for another frame. The [notice recipe](#notice-overlay-review) covers valid
altitude alongside synthetic warnings and a real marker crossing under the mask.

For a clean low/navigation/GEO-height comparison, run the following from the
workspace root. It extracts three unmodified records from the existing
[historical pool](../tools/fixtures/m2-pool/README.md), uses `overhead-track` to
place each observer at its approximate subpoint, then captures real executable
frames at the same explicit UTC. Python uses only its standard library; files
are disposable and fixtures are not changed. These deliberately chosen sites
are visual test inputs, not production settings or claims of operational status.
The GEO-height sample need not be stationary over Earth.

```sh
review_dir="$(mktemp -d "${TMPDIR:-/tmp}/overhead-altitude-review.XXXXXX")"
cargo build -p overhead-sim -p overhead-tools --bin overhead-sim --bin overhead-track
python3 - "$review_dir" <<'PY'
import json
import subprocess
import sys
from pathlib import Path

out = Path(sys.argv[1])
records = json.loads(Path("tools/fixtures/m2-pool/pool.json").read_text())
utc = "2026-10-10T00:00:00Z"
for name, norad in [("low", 28054), ("navigation", 26407), ("geostationary", 19548)]:
    record = next(r for r in records if r["NORAD_CAT_ID"] == norad)
    data = out / f"{name}-data.json"
    data.write_text(json.dumps([record]))
    manifest = out / f"{name}.json"
    manifest.write_text(json.dumps({"groups": [
        {"name": "altitude-review", "path": data.name}
    ]}))
    report = subprocess.check_output([
        "target/debug/overhead-track", str(data), utc, "0", "0", "0"
    ], text=True)
    fields = dict(line.split(": ", 1) for line in report.splitlines() if ": " in line)
    latitude = fields["Satellite WGS-84 latitude (deg)"]
    longitude = fields["Satellite WGS-84 longitude (deg east)"]
    subprocess.run([
        "target/debug/overhead-sim", str(manifest), utc, latitude, longitude, "0",
        "--wide", "--snapshot", str(out / f"{name}.png")
    ], check=True)
    print(name, "nominal period minutes:", 1440 / record["MEAN_MOTION"])
    print(name, "altitude km:", fields["Satellite WGS-84 ellipsoidal altitude (km)"])
    velocity = [float(v) for v in fields["TEME velocity (km/s)"].split()]
    print(name, "TEME speed km/s (from rounded report):",
          sum(v * v for v in velocity) ** 0.5)
PY
printf 'Review images: %s\n' "$review_dir"
```

These outputs use propagated altitude, not DESIGN's illustrative values.
Independent CLI assertions additionally exercise all 16 pool objects individually
at their epochs, plus both standard catalogues at multiple times and zooms.

### Static speed glyph

Under [0043](../docs/decisions/0043-static-speed-glyph-contract.md), `--wide`
adds `Spd` and a thin clockwise arc beginning at the top of the same 23px circle
used by the other fields. The input is instantaneous **TEME velocity magnitude**
in km/s, not Earth-fixed ground speed, observer range rate or apparent screen
motion. The app derives it from the same timestamped propagation as position.

A fixed linear 0–12 km/s scale does not change with zoom, observer or catalogue.
Zero retains `Spd` with no arc; any positive speed draws at least the top pixel;
12 km/s and higher give a full circle. Endpoint snapping means nearby speeds
share a raster, and the circle becomes complete slightly below 12. Finite
negative inputs defensively draw as zero. Nonfinite speed clears only its label
and arc, never substituting a plausible reading or hiding other valid fields.
These are drawing limits, not tracking limits; physical speed is not clamped.

All selected fields update immediately on frames and selection changes, without
animation in headless snapshots; interactive ID changes follow 0046. Nominal
period is constant for a given element set. ID-hidden views
still show the fields. No selection clears the labels/glyphs while retaining the separator; no stale speed survives a failed
frame or former selection. Notices remain explicit alongside valid fields.
0044 adds nominal period and 0045 adds group-derived category. Speed implementation is
complete and the user accepted its executable output separately from the
contract/mocks. The user also subsequently accepted fresh executable period output.

For selected, ID-hidden and no-selection output, run from the workspace root:

```sh
review_dir="$(mktemp -d "${TMPDIR:-/tmp}/overhead-speed-review.XXXXXX")"
for fov in 20 100 0.1; do
  cargo run -p overhead-sim -- sim/examples/weather-gps.json \
    2026-10-10T00:00:00Z 39.007 -104.883 2.187 \
    --wide --fov-deg "$fov" --snapshot "$review_dir/speed-$fov.png"
done
printf 'Review images: %s\n' "$review_dir"
```

Repeat at `2026-10-10T00:00:30Z` for a later physical frame. The self-contained
[low/navigation/GEO comparison recipe](#static-altitude-glyph) now renders all
five fields (Unknown category for its unclassified inputs) and prints TEME speed
derived from the rounded tracking report;
its orbital records are unchanged. The [notice recipe](#notice-overlay-review)
adds selected-plus-warning, failed/no-selection and real-marker-under-overlay
cases, with explicitly synthetic faults. Zero/nonfinite speed controls are
renderer tests, not claims about these historical samples. CLI tests independently
propagate all 16 pool objects and both replay catalogues at multiple times/zooms.

### Static period glyph

Under [0044](../docs/decisions/0044-static-period-glyph-contract.md), `--wide`
adds `Per` and a thin clockwise-outward spiral below speed. Input is the selected
**element set's nominal period**, `1440 / MEAN_MOTION` minutes, not an
instantaneous period, pass duration or countdown. It uses existing validated
catalogue metadata without another propagation or retained per-sample scalar.

A fixed logarithmic scale runs from 90 minutes/2½ turns to 24 hours/one turn;
12 hours gives 1⅜ turns. Shorter/longer periods saturate only the drawing. The
same elements give the same period across frames, observer and zoom; selection
changes may change the reading. Nonfinite/nonpositive/unavailable period clears
only its label/spiral. No selection clears all selected fields, retaining the
separator. ID hiding does not hide the fields, and notices remain explicit.
No animation in these headless snapshots or new CLI option. 0045 adds category
above the four data fields; interactive ID changes follow 0046.

The user approved meaning, scale, 2½-turn appearance and static lifecycle, then
authorized documentation and implementation. They subsequently reviewed and
accepted fresh executable period output, separately from the illustrative
comparison and numeric scale approval.

For selected, ID-hidden, no-selection and later-frame comparisons, use the
[speed recipe](#static-speed-glyph), which now renders period too. For actual
low/half-day/day-long examples use the [mixed-orbit recipe](#static-altitude-glyph):
it prints the source-derived nominal period alongside altitude and speed. The
sample periods are approximately 101.80, 717.99 and 1436.20 minutes, not the
illustrative 90/720/1440 values. The [notice recipe](#notice-overlay-review)
covers valid period alongside synthetic faults and a real notice-covered marker.

The exact raster uses a bounded 4097-point curve in the approved footprint;
reducing samples can change pixels. This has not been timed on S3 or validated
on the physical display. No 20 Hz target-budget claim follows from host output.

### Static category glyph

[0045](../docs/decisions/0045-static-group-category-contract.md) adds a static
`Cat` label and Station body-and-wings, Navigation cross, Weather cloud or Unknown
question mark. The user approved the icons and behavior and authorized
implementation, then separately reviewed and accepted the fresh executable
four-category comparison.

Roles come from [explicit group configuration](../tools/README.md#display-categories),
not OMM purpose metadata, names, URLs or a per-ID lookup. The app considers all
retained valid memberships (including older contributing records) and chooses
**Station > Navigation > Weather > Unknown**. Missing group roles contribute
nothing; overlapping known roles do not become Unknown. Invalid records cannot
establish membership. This is a display preference, not active status or exclusive
mission purpose; loading only a subset of groups also limits known membership.

The weather/GPS example assigns Weather and Navigation; the ISS-only tools
example assigns Station. The mixed `--demo` pool is uncategorized, so selected
objects show Unknown. Old manifests still work. A successful selection always
shows Cat and one icon, independently of the four data fields and ID visibility.
No selection clears all five fields, retaining the separator. Notices, counts,
physical state and square diagnostics are unchanged. Cat still switches immediately;
interactive wide replay animates only the four data glyphs on selected-ID changes
under [0046](../docs/decisions/0046-selected-data-glyph-transition-contract.md).
Standalone headless snapshots remain static. No fetching.

For fresh real-orbit examples of all icons, run from the workspace root. This
creates disposable **single-object review groups**, not a production per-ID map.
It uses the existing pinned records and deliberately chosen subpoint observers;
no fixture is refreshed or changed. The unknown example has no assigned role,
not an assertion that its actual mission is unknowable. The source identities
were checked during design: ISS (25544), GPS BIIRM-6 (32711), Metop-B (38771).

```sh
review_dir="$(mktemp -d "${TMPDIR:-/tmp}/overhead-category-review.XXXXXX")"
cargo build -p overhead-sim -p overhead-tools --bin overhead-sim --bin overhead-track
python3 - "$review_dir" <<'PY'
import json
import subprocess
import sys
from pathlib import Path

out = Path(sys.argv[1])
cases = [
    ("station", "core/tests/fixtures/iss-25544.json", 25544),
    ("navigation", "tools/fixtures/m2-pool/sources/gnss.json", 32711),
    ("weather", "tools/fixtures/m2-pool/sources/leo.json", 38771),
    ("unknown", "tools/fixtures/m2-pool/pool.json", 19548),
]
for role, source, norad in cases:
    record = next(r for r in json.loads(Path(source).read_text()) if r["NORAD_CAT_ID"] == norad)
    utc = record["EPOCH"].removesuffix("Z") + "Z"
    data = out / f"{role}-data.json"
    data.write_text(json.dumps([record]))
    group = {"name": "category-review", "path": data.name}
    if role != "unknown":
        group["category"] = role
    manifest = out / f"{role}.json"
    manifest.write_text(json.dumps({"groups": [group]}))
    report = subprocess.check_output([
        "target/debug/overhead-track", str(data), utc, "0", "0", "0"
    ], text=True)
    fields = dict(line.split(": ", 1) for line in report.splitlines() if ": " in line)
    subprocess.run([
        "target/debug/overhead-sim", str(manifest), utc,
        fields["Satellite WGS-84 latitude (deg)"],
        fields["Satellite WGS-84 longitude (deg east)"], "0",
        "--wide", "--snapshot", str(out / f"{role}.png")
    ], check=True)
PY
for fov in 0.1 20 100; do
  target/debug/overhead-sim sim/examples/weather-gps.json \
    2026-10-10T00:00:00Z 39.007 -104.883 2.187 \
    --wide --fov-deg "$fov" --snapshot "$review_dir/replay-$fov.png"
done
printf 'Review images: %s\n' "$review_dir"
```

The replay cases cover no selection, selected Navigation and ID-hidden readings.
The [notice recipe](#notice-overlay-review) exercises Unknown beside synthetic
faults and a real notice-covered selected marker; assigning `"category":"station"`
to those disposable review groups instead supplies Station without changing
notices. Renderer tests cover same-app selection/frame loss and recovery; loader
and executable tests cover hierarchy, duplicate/reordered groups and valid versus
invalid membership. Exact pixel tests use independent geometric references, not
production packed masks. No target/display validation follows from host images.

### Label zoom visibility

Under [0035](../docs/decisions/0035-label-zoom-visibility.md), the wide-view label
hides at **≥70° vertical FOV** and restores at **≤60°**. Between those thresholds,
it retains its previous visibility. The selected diamond remains visible. This
is clean hide/show with hysteresis: no fade, dithering or animation.

At launch, including headless snapshots, the label is shown below the hide
threshold and hidden at/above it; the intervening band starts visible. Frame
updates, Tab selection and empty/invalid frames do not reset zoom history.

Tune either threshold independently with `--label-hide-deg` and
`--label-show-deg`; omitted values retain their defaults. Both options require
`--wide`, and the pair must satisfy finite `0.1 <= show < hide <= 100` degrees.
For a same-frame comparison at 70° (default hidden versus configured visible):

```sh
cargo run -p overhead-sim -- sim/examples/weather-gps.json \
  2026-10-10T00:00:00Z 39.007 -104.883 2.187 \
  --wide --fov-deg 70 --snapshot /tmp/overhead-label-hidden.png
cargo run -p overhead-sim -- sim/examples/weather-gps.json \
  2026-10-10T00:00:00Z 39.007 -104.883 2.187 \
  --wide --fov-deg 70 --label-hide-deg 80 --label-show-deg 50 \
  --snapshot /tmp/overhead-label-visible.png
```

The thresholds are provisional visual tuning, not density/collision guarantees
or device controls. Labels may still overlap scene graphics below the cutoff.

### Identification review

Reproduce right-edge fallback, ISS and a real six-digit ID with existing local
catalogues (no generated input files). These are historical visual checks, not
independent geometry references or production observer choices:

```sh
cargo run -p overhead-sim -- sim/examples/weather-gps.json \
  2026-10-10T00:00:00Z 39.007 -104.883 2.187 \
  --wide --fov-deg 9.4 --snapshot /tmp/overhead-label-edge.png
cargo run -p overhead-sim -- tools/examples/catalogue.json \
  2026-10-04T12:43:41.833056Z 0.000017409 -85.180357315 0 \
  --wide --fov-deg 20 --snapshot /tmp/overhead-label-iss.png
cargo run -p overhead-sim -- sim/examples/weather-gps.json \
  2026-10-10T00:00:00Z 0.926636747 3.630339358 0 \
  --wide --fov-deg 0.1 --snapshot /tmp/overhead-label-six-digit.png
```

The first shows `32711` left of its near-right-edge diamond (5 markers, 17 arrows).
The others show `ISS` (1 marker, no arrows) and `100530` (1 marker, 25 arrows)
near the aim point; all have zero tracking failures. The latter sites approximate
those satellites' subpoints from `overhead-track` at the supplied UTC. Repeat the
weather/GPS comparison at 100° to review the now-unlabelled overview: user review
of its formerly crowded label led to 0035. Below the hide cutoff, transparent
overlap remains allowed by 0034; no collision avoidance has been added.

## Verify

```sh
cargo check --workspace
cargo test -p overhead-app -p overhead-render -p overhead-sim
```

Tests cover actual-position perspective, altitude/depth effects, fixed-camera
magnification, near-plane/circular/square boundaries, poles/dateline, independent
ellipsoid tangency, true-state traceability, horizon counts, failed-frame
replacement, selection/zoom, clipped outline/glyph pixels, explicit clock/config,
and real CLI PNG snapshots with SDL unavailable. Existing engine/prediction
regression tests remain unchanged. CLI tests compare explicit/default square
snapshots, corner counts against independent physical-look-angle references,
and determinism. Wide tests additionally cover rectangular aspect and pixel scale,
per-object arrows despite coincident positions, directional glyph clipping,
marker entry, and visible warnings in empty and partially successful frames.
Identification tests cover ISS/five-/six-/longer IDs, complete edge/corner fit,
transparent overlaps, selection changes and stale-label clearing, baseline
isolation, and headless identity-versus-name behavior using a real orbit with
explicitly synthetic identity metadata. Zoom-label tests cover inclusive
thresholds, retained history in both directions, startup in the intervening band,
validated CLI overrides, unchanged selection/physical state, retained diamonds
and warnings, and unchanged baseline diagnostics. Elevation tests cover exact
outline/radial pixels, snapping boundaries and downward ties, single-pixel lines,
8-connectivity, monotonic arc removal, label/separator geometry, finite saturation
and nonfinite suppression. Lifecycle tests cover true selected-state updates,
ID-hidden independence, missing-frame/selection clearing, actual propagation
failure after a successful selection, warning coexistence and recovery. Notice
tests additionally cover exact bottom-left backing bounds,
opaque padding/gaps, a real selected marker/ID crossing under the overlay,
unchanged app state and pixels outside the backing, shrinking/missing-frame
recovery, and no stale white mask. CLI tests check notice pixels with valid
markers plus synthetic tracking failures/omitted records and a subsequent clean
load, without changing the strip. Altitude tests cover all eight exact ring
rasters, radius boundaries/ties, finite saturation, field-local nonfinite
suppression and real frame/selection changes across radius boundaries using
explicit synthetic orbit controls. Headless PNG tests compare both glyphs to
independent look-angle/geodetic samples and inverse altitude thresholds, including
real low/navigation/GEO-height selected objects. Speed tests cover true TEME
magnitude and observer/zoom independence, robust large/small-component norms,
every arc boundary/tie, connected monotonic circle prefixes, exact independent
raster comparisons, zero-label/invalid distinction, field-local suppression,
and actual selection/frame raster changes. CLI assertions derive speed outside
app state, including alongside synthetic warnings. Existing elevation, notices,
selection-loss/recovery and square regressions remain covered. Period tests
cover source mean-motion conversion, reciprocal overflow, fixed scale and
saturation, connected bounded hollow spirals, independent raster references,
field-local invalid suppression, selection changes and constant period across
physical frames. Independent CLI assertions derive period from element metadata
for all 16 pool objects, both replay catalogues and synthetic-warning states.
Debug tests cover exact fixed cells, signed fractional-time arithmetic, text
boundaries/substitution/overflow, scientific fallback, field-local availability,
unsampled/empty/mixed-failure states and stale-ink clearing. Simulator unit tests
exercise wide-only nonrepeating toggles, unchanged app truth/geometry/failures,
Tab/zoom and replay while debugging, static return during interrupted animation,
normal transition recovery and preserved notices. CLI tests reject debug snapshot
combinations and verify complete host names/causes; normal real-data snapshot
regressions remain unchanged.

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
