# M1 Earth-relative coordinates

_Historical implementation notes from 2026-10-04. See [PLAN](../PLAN.md) for
current tasks, [decision 0007](../decisions/0007-coordinate-implementation-conventions.md)
for accepted conventions, and the [verification notes](earth-coordinates-verification.md)
for completed checks._

## Goal
Convert propagated TEME positions to Earth-fixed positions and WGS-84 geodetic latitude, longitude, and altitude, independently verified before observer geometry.

## Contract
- Success: IAU-1982 GMST rotation at explicit UTC time (UT1≈UTC), WGS-84 inverse and forward position conversions; independent fixtures and geometric boundary tests.
- Gates: Earth-fixed/reference and round-trip positions within 1 m, geodetic angles within 1e-6 degrees; workspace checks/tests pass.
- Constraints: no_std, no allocation, bounded iteration, km/radians, explicit errors. Follow decision 0006.
- Non-goals: velocity transforms, observer look angles, runner, UI, hardware, changes to propagation.

## Assumptions
- [active] Position-only transforms satisfy this M1 task; raw TEME velocity remains untouched.
- [active] UTC input years 1957–2100 match ingestion's supported era. Explicit leap-second representations are rejected, not silently interpreted as UT1.
- [active] Geodetic inverse targets satellites and terrestrial positions (including below sea level); geocenter is undefined and iteration failure is an error.

## Research
- sgp4 2.4 `iau_epoch_to_sidereal_time` implements the required IAU-1982 GMST polynomial and takes Julian years since J2000 noon. This is separate from AFSPC propagation mode.
- Its calendar helper uses a simplified Gregorian expression that fails after February 2100. Use Chrono elapsed seconds from J2000 instead, preserving subsecond precision.
- Vallado `teme2ecef.m`: position rotation `[cx+sy, -sx+cy, z]`; polar motion and additional kinematic terms omitted per the GMST-only convention.
- Vallado `ecef2ll.m`: iterative ellipsoid-normal latitude. Use WGS-84 constants (not that example's older constants) and a pole-safe height expression.
- ERFA `gmst82` is an independent IAU-1982 implementation. pymap3d uses You (2000), a non-iterative inverse, suitable for independent terrestrial/LEO reference cases. Use forward pymap3d references at higher altitudes rather than assuming its approximate inverse meets the angular gate there.
- libm 0.2 is already transitive through sgp4; declare directly for no_std trig/hypot/sqrt.
- Sources: https://docs.rs/crate/sgp4/2.4.0/source/src/model.rs ; https://github.com/CelesTrak/fundamentals-of-astrodynamics/blob/main/software/matlab/teme2ecef.m ; https://github.com/CelesTrak/fundamentals-of-astrodynamics/blob/main/software/matlab/ecef2ll.m ; https://github.com/liberfa/erfa/blob/master/src/gmst82.c ; https://github.com/geospace-code/pymap3d/blob/v3.2.0/src/pymap3d/ecef.py

## Structure
One completed unit: public position conversion functions and geodetic type in core, independent offline-generated fixtures, integration/boundary tests. PLAN.md remains the milestone task list.

## Completion / alignment
Contract met; 12 new tests, workspace and feature checks pass. No observer, velocity, UI, or hardware work. Decision 0007 records boundary policies and clarifies the earlier “TEME of epoch” wording to TEME at propagation time, consistent with the already-agreed direct GMST reduction. Propagation math is unchanged. Next task is observer geometry, reusing the forward WGS-84 conversion.
