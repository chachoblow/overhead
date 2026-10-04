# Unit: Position transforms

## Plan
1. Generate checked-in reference data with pinned offline ERFA/pymap3d dependencies and retain the generator/provenance.
2. Add TEME→ECEF position conversion, WGS-84 geodetic inverse/forward, validation, and bounded iteration.
3. Test reference cases, poles/equator/antimeridian, negative altitude, GEO, subsecond and calendar boundaries, invalid inputs, and fixture propagation→geometry composition.
4. Run workspace checks/tests, explicit default-feature-free core check, and feature-enabled tests; update milestone/handoff docs.

## Acceptance Criteria
- [x] Independent Earth-fixed reference positions agree within 1 m.
- [x] Independent geodetic references agree within 1e-6 degrees / 1 m.
- [x] Forward/inverse round trips cover terrestrial, LEO, GEO, poles, equator, antimeridian, and negative altitudes.
- [x] Invalid input/time/geocenter handling and finite bounded computation are tested.
- [x] Workspace and no_std/OMM feature checks/tests pass.

## Implementation Notes
- Baseline workspace check/tests passed (15 tests); final workspace/default and all-feature tests pass (27 tests, 12 new).
- Implemented `teme_to_ecef`, `ecef_to_geodetic`, `GeodeticPosition::to_ecef`, and `CoordinateError`; fixed work limit of 16 inverse iterations.
- Nine ERFA rotation references, 13 pymap3d forward geodetic references, 594 round trips, analytic/invalid-input tests, Vanguard reference composition, and ISS fixture sanity checks.
- Default-feature-free core check, strict all-target/all-feature Clippy, formatting, and byte-identical fixture regeneration pass. A Clippy digit-grouping warning was fixed.
- No observer/velocity/UI/hardware implementation. Python dependencies installed only in a temporary reference-generation venv.

## Discoveries
- Decision 0006's “TEME of epoch” wording was inconsistent with its direct GMST reduction. Decision 0007 clarifies TEME at propagation time; propagation computations unchanged.
- Coordinate time uses Chrono instead of sgp4's calendar helper to avoid the 2100 century bug. Upstream propagation epoch handling remains unchanged.
- Alignment check: contract fulfilled with no scope expansion. Next M1 unit can reuse forward geodetic coordinates for the observer position; undefined look angles still need explicit policy.
