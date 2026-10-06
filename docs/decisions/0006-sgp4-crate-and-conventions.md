# 0006 — SGP4 dependency and physical conventions

Date: 2026-10-04
Status: accepted (clarified by 0007–0009)

## Decision and rationale
Use `sgp4` 2.4 (neuromorphicsystems) for parsing/propagation; own the downstream
geometry and budget-aware pass search. It ports the CelesTrak reference and
includes Vallado AIAA 2006-6753 verification vectors.

- `default-features = false, features = ["libm"]` supports no_std propagation
  without alloc; transitive Chrono also has defaults disabled.
- Core's default-off `omm` feature enables `sgp4/serde`: alloc, not std.
  Host tools enable it; firmware will need an allocator for on-device parsing.
- Rejected: `satkit` (std/data-heavy), `kshana` (broad PNT framework), and young
  `ephemerust`/`sgp4-predict` crates. Small, independently tested transforms are
  preferable to oversized or immature dependencies.
- Coordinate math declares the already-transitive `libm` directly (0007).
  Python reference generators are offline tools, not runtime dependencies.

## Physical conventions
- **Propagation:** AFSPC compatibility mode, WGS-72 constants and AFSPC epoch
  handling, matching the published verification path.
- **Time:** explicit UTC inputs; signed minutes since element epoch. UT1≈UTC,
  no ΔUT1/EOP data. Representation and supported era are defined in
  [0009](0009-core-time-and-frame-api.md).
- **Frames:** TEME at propagation time → PEF via IAU-1982 GMST Z-rotation;
  treat PEF as ECEF, omitting polar motion. “TEME of epoch” in the original
  record was corrected by [0007](0007-coordinate-implementation-conventions.md).
- **Geometry:** WGS-84 ellipsoidal geodetic coordinates; observer look angles
  via south/east/zenith (SEZ), azimuth clockwise from true north, elevation
  from the local horizon, direct slant range.
- **Units:** km, km/s, radians; degrees only at reporting/display boundaries.
  Altitude is above the ellipsoid, not mean sea level.

## Test gates
| Layer | Tolerance | Independent reference |
|---|---|---|
| Propagation | 1e-6 km position; 1e-9 km/s velocity | Vallado AIAA 2006-6753 vectors |
| Earth-fixed/geodetic | 1 m position/height; 1e-6° angles | ERFA / pymap3d forward geometry |
| Full observer pipeline | 0.1 km range; 0.01° angles | Skyfield |

The original reference plan proposed Vallado geodetic/`razel` worked examples;
0007–0008 specify the implemented references. Cases, versions, tighter isolated
geometry gates, and regeneration live in the
[fixture README](../../core/tests/fixtures/README.md).

These are implementation comparisons, not absolute tracking accuracy. UT1≈UTC
can introduce ~0.4 km surface displacement (0.9 s Earth rotation); omitted polar
motion contributes ~15 m. Orbital-element age/quality adds error independently.
