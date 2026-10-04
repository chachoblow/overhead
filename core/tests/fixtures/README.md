# Test fixtures

## iss-25544.json

OMM (JSON) for the ISS (ZARYA), NORAD 25544, as published by Celestrak.

- Source: https://celestrak.org/NORAD/elements/gp.php?CATNR=25544&FORMAT=json
- Retrieved: 2026-10-04T20:15:08Z
- Element epoch: 2026-10-04T12:43:41.833056 UTC
- Stored byte-for-byte as retrieved (single-line JSON array of one object).

Fixed test timestamps for pipeline tests (chosen relative to the element
epoch; propagation is only meaningful near the epoch):

- T0 = 2026-10-04T12:43:41.833056 UTC (the epoch itself, t = 0 min)
- T1 = 2026-10-04T14:13:41.833056 UTC (t = +90 min, ~one orbit)
- T2 = 2026-10-05T12:43:41.833056 UTC (t = +1440 min, one day)

Do not refresh this file casually: expected values in tests are tied to this
exact element set. If it is ever replaced, update the retrieval metadata here
and regenerate every derived expectation.
