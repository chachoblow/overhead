# Host tools

## `overhead-catalogue`

Load local OMM groups and report a nonempty, deduplicated catalogue. No network,
wall clock, observer, propagation, or freshness cutoff is implicit.

```sh
cargo run -p overhead-tools --bin overhead-catalogue -- tools/examples/catalogue.json
```

The checked-in example references the existing historical ISS fixture without
changing it; it is a smoke test, not live data or a chosen production group set.
Fixture provenance remains in the [fixture README](../core/tests/fixtures/README.md).

### Manifest

```json
{
  "groups": [
    {
      "name": "stations",
      "path": "stations.json",
      "source_url": "https://celestrak.org/NORAD/elements/gp.php?GROUP=stations&FORMAT=JSON",
      "fetched_at": "2026-10-06T08:00:00Z"
    },
    {
      "name": "weather",
      "path": "weather.json"
    }
  ]
}
```

Save each group as a local OMM JSON array. Paths resolve relative to the manifest;
absolute paths also work. Configure at least one group, using unique nonblank
names and nonempty paths. Unknown configuration fields are rejected.

`source_url` and `fetched_at` are optional (absent/null means unknown). Source
URLs are descriptive strings, never fetched. Fetch timestamps use explicit UTC
`YYYY-MM-DDTHH:MM:SS[.fraction]Z`, 1957–2100, without leap seconds or offsets.
They are not inferred from file metadata or used to select elements.

### Report and failures

Reports are sorted by NORAD ID and include name, group memberships, selected
record location (one-based record number), element epoch, source URL, and fetch
time. Only groups contributing valid records establish membership. Equivalent
newest records use the first manifest group/record for display metadata and
provenance; orbital disagreement is never resolved by that ordering.

The [catalogue decision](../docs/decisions/0011-catalogue-ingestion-and-provenance.md)
owns merge and failure policies. Invalid records and unresolved newest-epoch
conflicts produce stderr warnings; a nonempty accepted result still exits 0.
Unreadable files, malformed documents/configuration, or an empty result exit 1
with an explanatory stderr error and no partial stdout report. Help exits 0.
Empty-result errors retain record/conflict diagnostics.

Whole-document syntax is checked before records are interpreted. Raw record
JSON preserves duplicate keys for checked OMM ingestion; converting through a
JSON object map first could silently discard them. Successful ingestion means
validation and SGP4 initialization, not guaranteed propagation or accuracy.

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

Tracking tests launch the executable for all 12 Skyfield cases (range within
0.1 km, angles within 0.01°), deterministic output, and invalid-input/error paths.
Catalogue tests cover manifests, relative paths, provenance, merge/conflict
behavior, raw-key preservation, invalid records, and whole-load failure.
Synthetic record mutations test catalogue policy, not physical accuracy.
Tests need no Python, network, SDL window, or hardware. Workspace commands are
in [AGENTS.md](../AGENTS.md).
