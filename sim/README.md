# Simulator

400×240, 1-bit real-data radar replay. The SDL window uses the LcdWhite theme
at 2×; rendering and clockless application state live in `overhead-render` and
`overhead-app`, not in SDL. Requires the existing Homebrew SDL2/linker setup.

## Run

From the workspace root:

```sh
cargo run -p overhead-sim -- --demo
cargo run -p overhead-sim -- --help
```

`--demo` explicitly selects the pinned historical 16-object pool, UTC
`2026-10-10T00:00:00Z`, and observer `39.007 -104.883 2.187`. It uses paths in
this build's checkout. It is not live tracking or a production catalogue.
Without arguments the program prints an input error rather than guessing time.

For another local catalogue, time or observer:

```sh
cargo run -p overhead-sim -- sim/examples/weather-gps.json \
  2026-10-10T00:00:00Z 39.007 -104.883 2.187 \
  --radius-km 8000 --radar-size 240
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

- Wheel/up arrow: zoom in. Down arrow: zoom out. Zoom is geometric, not presets.
- Tab: select the next visible NORAD ID, wrapping. Selection stays while visible;
  otherwise the first visible ID is used. This is inspection, not final ranking.
- Escape/window close: quit.
- `--radius-km`: initial radius, 50–20,000 projected km; default 8,000.
- `--radar-size`: left-hand radar allocation, 180–240 pixels; default 240.
  The circle stays vertically centered. Smaller values give more panel width.

The [ground-map contract](../docs/decisions/0022-minimal-ground-map-simulator.md)
owns projection and clipping semantics. North is up; the central cross is the
observer, small crosses are satellite ground positions, and the diamond is the
selected satellite. The outer circle is the viewport; when the Earth fits,
the inner circle is the display globe's limb. There are no coastlines yet.
`R ... KM (MAP)` measures projected map radius, **not surface or slant distance**.
Only satellites above the true local geometric horizon are eligible; zoom cannot
reveal below-horizon objects. The map also clips its far hemisphere.

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
  --radius-km 2500 --radar-size 200 --snapshot /tmp/overhead-local.png
```

Snapshots use exactly the supplied start UTC, write a 400×240 PNG, print counts
and exit without opening SDL. A later explicit UTC gives another physical frame;
there is no hidden accelerated clock. At the example time/site the pool has
4 visible markers; weather/GPS has 22 at 8,000 km and 6 at 2,500 km, all with zero
calculation failures. These are reproducible example observations, not capacity
or density targets. Interactive movement/input review remains necessary.

Both manifests reference existing immutable snapshots; they do not copy or
modify orbital data. `catalogue.json` uses the selected 16-object pool;
`weather-gps.json` uses the archived 73-record weather and 32-record GPS groups.
[Acquisition, exact bytes, selection and provenance](../tools/fixtures/m2-pool/README.md)
remain there. These archives include historical/inactive objects and are not a
chosen production group set. Source URLs are provenance only, never fetched.

## Verify

```sh
cargo check --workspace
cargo test -p overhead-app -p overhead-render -p overhead-sim
```

Tests cover projection/boundaries, true-state traceability and separation,
failed-frame replacement, selection/zoom, clipping of whole marker glyphs,
explicit clock/configuration, and real CLI PNG snapshots with SDL unavailable.
Existing engine/prediction regression tests remain unchanged. These commands and
the snapshot examples pass from a clean current source export with an empty build
directory and the existing offline tool/dependency caches. No ignored binaries
are needed. `--demo` uses the new checkout's paths when built there.

Frozen M2 evidence is a separate historical verification scope: use its
[pinned revision workflow](../docs/evaluations/s3-m2/README.md#offline-validation-and-table-regeneration),
not the current workspace lockfile. No target-capacity, whole-device RAM,
safe-stack or interactive usability claim follows from simulator checks.
