# Host tools

## `overhead-track` — reproducible single-satellite measurements

From the workspace root:

```sh
cargo run -p overhead-tools --bin overhead-track -- \
  core/tests/fixtures/iss-25544.json 2026-10-04T12:43:41.833056Z \
  39.007 -104.883 2.187
```

Arguments, in order:

1. Local OMM JSON file: an array containing **exactly one** satellite.
   Empty/multiple-object arrays are errors; catalogue selection belongs to M2.
2. Explicit UTC timestamp: `YYYY-MM-DDTHH:MM:SS[.fraction]Z`.
   Years 1957–2100; leap seconds, missing `Z`, and timezone offsets rejected.
3. WGS-84 observer latitude in degrees, [-90, 90].
4. East-positive observer longitude in degrees, [-180, 180].
5. Observer height in km above the WGS-84 ellipsoid, **not MSL**.
   Negative heights are accepted. All coordinates must be finite.

`--help` / `-h` prints usage. No network, wall clock, implicit location,
local-time conversion, or UI is involved. The tool enables core's optional
`omm` feature; file I/O, argument parsing, and reporting stay in `tools/`.
Core's standalone default build remains no_std/allocation-free.

Output includes name/NORAD ID, element epoch, requested UTC, signed elapsed
minutes, observer coordinates, TEME position/velocity, ECEF position,
satellite WGS-84 latitude/longitude/ellipsoidal altitude, slant range,
azimuth, and elevation. Every measurement has explicit units. At zenith or
nadir azimuth is printed as `undefined`, never a fabricated north direction.
Errors go to stderr with exit code 1 and no partial report on stdout;
success/help use exit code 0.

Physical conventions remain decisions [0006](../docs/decisions/0006-sgp4-crate-and-conventions.md),
[0007](../docs/decisions/0007-coordinate-implementation-conventions.md),
and [0008](../docs/decisions/0008-observer-geometry-conventions.md):
AFSPC propagation, GMST-only position rotation, UT1≈UTC, no polar motion,
WGS-84 geometry, no refraction. Velocity is **TEME**, not Earth-fixed.
Negative elevation is valid geometry, not a visibility decision. Printed
precision is not orbit accuracy. The runner imposes no freshness threshold;
use elements near their epoch, not this historical fixture as a live tracker.

### Reproduce T0/T1/T2 at known locations

The fixed ISS fixture and reference provenance are documented in
[`core/tests/fixtures/README.md`](../core/tests/fixtures/README.md).
Do not refresh that fixture: all reference values depend on its exact bytes.

| Time | UTC | Minutes since epoch |
|---|---|---:|
| T0 | 2026-10-04T12:43:41.833056Z | 0 |
| T1 | 2026-10-04T14:13:41.833056Z | 90 |
| T2 | 2026-10-05T12:43:41.833056Z | 1440 |

Run all three times at the four Skyfield-reference sites (12 reports):

```sh
for utc in \
  2026-10-04T12:43:41.833056Z \
  2026-10-04T14:13:41.833056Z \
  2026-10-05T12:43:41.833056Z
do
  for site in '39.007 -104.883 2.187' '-33.8688 151.2093 0.058' \
              '0 -80 0' '0 90 0'
  do
    # Intentional field splitting of each numeric latitude/longitude/height triple.
    cargo run --quiet -p overhead-tools --bin overhead-track -- \
      core/tests/fixtures/iss-25544.json "$utc" $site
  done
done
```

Run this loop with `sh` or `bash` (zsh does not split `$site` by default).
Sites are the named Vallado site on WGS-84, Sydney, equator 80°W, and
equator 90°E. T0 at the first site reports approximately:

```text
Satellite WGS-84 ellipsoidal altitude (km): 424.668829114
Slant range (km): 4825.772604378
Azimuth (deg clockwise from true north): 150.234162834
Elevation (deg): -16.803612339
```

These example numbers are runner output, not independent reference data.

### Verification

```sh
cargo test -p overhead-tools
cargo check --workspace
cargo test --workspace
cargo check -p overhead-core --no-default-features --lib
```

`tools/tests/track.rs` launches the real executable for all 12 independent
Skyfield cases and gates printed range within 0.1 km and angles within
0.01°. It checks deterministic reports, identity/timestamps, geodetic
report consistency, help, invalid/missing inputs, JSON/element errors,
pre-epoch time, boundary sites, and propagation failure. A unit test covers
undefined-azimuth formatting. Existing core tests independently verify each
calculation layer. Tests need no Python, network, SDL window, or hardware.

The default `overhead-tools` binary remains a placeholder for future offline
data conversion; specify `--bin overhead-track` for this runner.
