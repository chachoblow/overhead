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
- Escape/window close: quit.
- `--fov-deg`: full vertical angle, 0.1°–100°; default 20°. Also the horizontal
  angle in square/circular modes; wide mode derives horizontal coverage from aspect.
  The former `--radius-km` option is rejected with migration guidance.
- `--radar-size`: left-hand radar allocation, 180–240 pixels; default 240.
  The viewport stays vertically centered. Smaller values give more panel width.
- `--viewport`: `square` (default, no outer border), `circle` (circular clipping
  and ring), or `borderless-circle` (circular clipping, no ring).
  See [comparison](#viewport-shape-comparison).
- `--wide`: opt-in 352×240 borderless radar with per-satellite off-screen arrows
  and a reserved 48-pixel strip. Incompatible with `--radar-size` and `--viewport`.
  See [wide comparison](#wide-radar-comparison).

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
opt-in wide comparison implements only the viewport/arrows portion of
[0032](../docs/decisions/0032-wide-radar-and-selected-satellite-glyphs.md), under
[0033](../docs/decisions/0033-wide-radar-comparison-contract.md).
The full normal/debug split, glyphs and marker-adjacent identifiers are not
implemented. [DESIGN's retained schematic](../docs/DESIGN.md#selected-satellite-presentation)
remains a design reference, not this executable's output or a golden test.
Final debug-access controls remain open.

The synchronous loop targets 20 Hz on the host, without a catch-up queue. This
is a visual test loop, **not a selected device cadence or bounded scheduler**.
No pass countdown, motion slow-down or hardware access is included.

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
is admitted horizontally. The 48-pixel strip is reserved for later glyph work.

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

No selected ID or data glyphs are drawn yet. The reserved strip shows `NO MARKERS`
when there are no in-view markers, not a claim of a complete empty sky. `NO FRAME`,
`TRACK ERROR` and `INGEST WARN` preserve explicit missing/incomplete-state notices;
warnings remain present alongside valid markers. These are comparison notices,
not the final empty-state or debug-access design. No routine FOV/count text is
shown in the wide image.

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
