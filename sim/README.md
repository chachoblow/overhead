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
- `--fov-deg`: full angle across viewport width/height, 0.1°–100°; default 20°.
  The former `--radius-km` option is rejected with migration guidance.
- `--radar-size`: left-hand radar allocation, 180–240 pixels; default 240.
  The viewport stays vertically centered. Smaller values give more panel width.
- `--viewport`: `square` (default, no outer border), `circle` (circular clipping
  and ring), or `borderless-circle` (circular clipping, no ring).
  See [comparison](#viewport-shape-comparison).

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
not capacity or density targets. User feedback on the initial perspective view
favored zoom-magnified motion and the fuller overview. The user also prefers the
higher camera and its breathing room, and subsequently selected the 240×240
borderless square. Right-panel contents remain open.

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
regression tests remain unchanged. From a fresh staged-source Git export with
an empty build target, offline locked workspace check/default/all-feature tests,
core/app/render no-default-feature checks, strict workspace Clippy/fmt and the
simulator build pass. CLI tests compare explicit/default square snapshots,
corner counts against independent physical-look-angle references, and determinism.

The fresh executable runs from an unrelated directory with SDL unavailable:
pool 20°/100° shows 0/4 markers; weather/GPS 20°/40°/100°/5°/0.1° shows
6/18/22/4/0, with circular modes showing 17 at 40°. All have zero failures.
The 20° default PNG matches the reviewed square comparison byte-for-byte;
a two-second SDL dummy-loop smoke also passes.

Existing Rust tool/dependency caches and Homebrew SDL2 remain prerequisites;
no ignored build artifacts or disposable panel mockups are needed. `--demo`
uses the checkout's paths when built there. The user reviewed snapshots and
selected square; no agent interactive input review or hardware test was
performed. Verification history is in [PROGRESS](../docs/PROGRESS.md).

Frozen M2 evidence is a separate historical verification scope: use its
[pinned revision workflow](../docs/evaluations/s3-m2/README.md#offline-validation-and-table-regeneration),
not the current workspace lockfile. No target-capacity, whole-device RAM,
safe-stack or interactive usability claim follows from simulator checks.
