# 0023 — Verify frozen M2 evidence at its pinned revision

Date: 2026-10-12
Status: accepted

## Decision
Run frozen `m2-bounded-20-v1` evidence checks from the complete tracked source
revision `ab267ae587585d1a0fa9983e49bad49537055c0d`, separately from current
workspace/simulator checks. Preserve the original manifest, strict guards,
fixtures, generated tables and captured evidence without renewal or hash bypasses.

## Why
M3 adds shared application state and workspace dependency edges. The root
`Cargo.lock` necessarily changes, although every external locked package entry
is unchanged. The frozen manifest intentionally hashes that entire file.
Fresh-source verification confirmed it is the only pinned-source mismatch and
all non-source manifest fields still agree. Its rejection is therefore expected:
the new workspace is not the exact frozen preparation tree.

The user chose revision-isolated verification over designing a new source-archive
verifier in this simulator slice. Re-freezing to hide unrelated workspace changes
would sever the link between historical capture identities and their source
contract; relaxing the guard would weaken detection of real evidence drift.

## Consequences
- Current source remains authoritative for app/core/render/simulator behavior.
  Run workspace tests and simulator smoke checks from a clean current export.
  These do not establish new target performance or validate a new M2 capture.
- Use a separate `git archive` export of the full pinned revision for frozen
  preparation, capture validation, reporting and their Python test suites.
  Commands live in the [capture archive README](../evaluations/s3-m2/README.md#offline-validation-and-table-regeneration).
  It includes the original lockfile, checker, tests, sources and raw captures;
  ignored binaries/build directories are not prerequisites.
- A fresh clone needs the pinned Git object, existing tools/dependency caches,
  Python 3.11+ and (for workspace builds) SDL2. A latest-source-only tarball or
  shallow clone lacking that object is insufficient for historical reproduction;
  obtain the pinned revision explicitly, not bytes from an ignored build tree.
- `prepare_m2_manifest.py --check`, `generate_m2.py --check` and Python suites
  importing that freeze continue to reject the evolving current tree. This is
  not a passing current-tree evidence claim or permission to skip a failed gate
  when preparing a new capture. Run the historical suite only at its named pin.
- Changes to measured engine/inputs/dependencies, new captures, or adoption of
  new operating limits still require reviewed preflight and the appropriate
  new evidence/decision. Historical success is not validation of later changes.
- No target image rebuild, hardware access, toolchain change, new workload matrix
  or production scheduling choice follows from this workflow correction.
