# 0011 — Catalogue ingestion and provenance

Date: 2026-10-07
Status: accepted

## Decision
Assemble configured local groups through checked OMM ingestion, deduplicate by
NORAD ID, and select the newest valid element epoch. Publish only a nonempty
catalogue after all configured documents have loaded successfully.

| Situation | Policy |
|---|---|
| Equivalent orbit records | Deduplicate; retain contributing valid group memberships |
| Different valid epochs | Select one complete record at the newest epoch, never mix/average elements |
| Orbital disagreement at the newest valid epoch | Omit that satellite and report all candidate input locations; no older fallback during an initial build |
| Invalid individual record | Skip and diagnose; another valid record for that ID remains eligible |
| Unreadable file, malformed JSON document, or invalid configuration | Fail the whole load; no partial catalogue |
| Empty group | Allowed, provided the combined result is nonempty |
| No usable satellites remain | Fail the load, retaining rejection diagnostics |

Compare parsed orbital values, not JSON formatting or names. Equivalent ties
use manifest/record order for non-orbital metadata and selected provenance.
An unambiguous newer epoch supersedes conflicts at older epochs. Exact field
comparisons and ordering are owned by the core implementation/tests.

The local manifest names groups and files, with optional source URL and fetch
time. Missing provenance remains unknown: neither file modification time nor
element epoch substitutes for fetch time. Keep group memberships and the
selected record's origin; source metadata can be shared rather than copied
into every satellite. Manifest syntax/usage lives in [tools](../../tools/README.md).

## Why
Overlapping group snapshots can contain different updates for the same object.
Newest epoch is a useful current-tracking default, not an accuracy guarantee;
fetch recency cannot resolve orbital freshness. Isolating bad records keeps
useful objects, while rejecting failed documents prevents silent omission of
configured groups. Diagnostics are structured results, not a logging dependency.

## Consequences
- Shared catalogue logic is opt-in `no_std` + alloc; default core stays
  allocation-free. Host tools own files, raw JSON document parsing, and output.
- M2 is offline and initial-load only. No live fetching, cache, active-catalogue
  replacement, stale-data cutoff, or embedded logging framework is implemented.
- Future embedded refreshes prepare/validate before replacement. A failed
  refresh keeps the previous accepted catalogue; a per-satellite conflict may
  retain its previously accepted unambiguous elements, subject to a future
  stale-data policy. This does not require two full catalogues in RAM.
- Without usable data the device stays running with a no-orbit-data status;
  cached data still requires valid time ([0002](0002-data-and-time-over-wifi.md)).
- Capacity, allocation/diagnostic budgets, cadence, retry/storage mechanics,
  and prediction costs remain unmeasured follow-up work in [PLAN](../PLAN.md).
