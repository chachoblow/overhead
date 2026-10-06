# Host tools

## `overhead-track`

Single-satellite measurements from explicit local inputs; no network, wall
clock, implicit location, or UI. From the workspace root:

```sh
cargo run -p overhead-tools --bin overhead-track -- \
  core/tests/fixtures/iss-25544.json 2026-10-04T12:43:41.833056Z \
  39.007 -104.883 2.187
```

| Argument | Contract |
|---|---|
| OMM JSON file | Array of **exactly one** satellite |
| UTC | `YYYY-MM-DDTHH:MM:SS[.fraction]Z`; 1957–2100; no leap seconds or offsets |
| Latitude | Degrees, [-90, 90] |
| Longitude | East-positive degrees, [-180, 180] |
| Height | km above WGS-84 ellipsoid, **not MSL**; may be negative |

Coordinates must be finite. OMM metadata is checked before numeric/epoch
validation ([0010](../docs/decisions/0010-propagation-and-omm-validation.md)).
Use `--help` / `-h` for usage; the default `overhead-tools` binary is a placeholder.

### Report
Includes identity, element epoch, requested time, signed elapsed minutes,
observer coordinates, TEME position/velocity, ECEF position, geodetic position,
and range/azimuth/elevation. Units are explicit. Vertical azimuth prints
`undefined`; negative elevation is valid geometry, not a visibility decision.

Success/help exit 0. Errors exit 1 with stderr and no partial stdout report.
Velocity is **TEME**, not Earth-fixed. Printed precision is not orbit accuracy;
there is no freshness cutoff. Do not use the fixed historical fixture as live
orbit data. Physical conventions: [0006](../docs/decisions/0006-sgp4-crate-and-conventions.md).

The example above reports approximately:

```text
Satellite WGS-84 ellipsoidal altitude (km): 424.668829114
Slant range (km): 4825.772604378
Azimuth (deg clockwise from true north): 150.234162834
Elevation (deg): -16.803612339
```

These are runner outputs, not independent references. For T0/T1/T2 timestamps,
reference sites, and provenance, see the
[fixture README](../core/tests/fixtures/README.md); do not refresh the ISS file.

### Verification
```sh
cargo test -p overhead-tools
```

Tests launch the executable for all 12 Skyfield cases (range within 0.1 km,
angles within 0.01°), deterministic output, and invalid-input/error paths.
They need no Python, network, SDL window, or hardware. Workspace commands are
in [AGENTS.md](../AGENTS.md).
