# 0045 — Static group-derived category contract

Date: 2026-10-17
Status: accepted (bounded M3 contract; implemented and user visually accepted)

## Decision

Add one static `Cat` icon above the four accepted data fields in the opt-in wide
view. Use explicitly configured source-group roles and retained valid group
memberships, with priority **Station > Navigation > Weather > Unknown**. This is
a display preference, not a claim of exclusive mission purpose or active status.

## Why

Configured groups already provide retained membership, so roles can scale with
those populations without manually classifying every NORAD ID. Explicit role
configuration avoids guessing from names or orbital parameters; a deterministic
hierarchy avoids turning ordinary overlap into an unhelpful question mark. The
initial per-ID-only approach and conflict-to-Unknown policy were considered and
rejected. A static icon completes the selected fields without introducing timing,
interpolation or a new selection policy.

The user approved Station, Navigation, Weather and Unknown; retained 0038's
navigation cross and station body-and-wings; reviewed and accepted the cloud and
question mark at native size; and approved the immediate selection lifecycle.
They subsequently chose group-based assignment and the hierarchy, replacing the
session's initial per-ID-only mapping and conflict-to-Unknown proposals. They
then authorized documentation and implementation. They subsequently reviewed and
accepted the fresh executable four-category comparison, closing the visual gate
separately from approval of the illustrative icons.

This completes the bounded static fields of
[0038](0038-labelled-borderless-glyph-direction.md), not animation, final debug
access or a production catalogue. [DESIGN](../DESIGN.md#static-category-contract)
owns the approved preview provenance; [usage](../../sim/README.md#static-category-glyph)
owns reproduction. Preserve 0040–0044, radar/selection/ID behavior and square default.

## Consequences

### Assignment and data flow

OMM records in the CelesTrak GP JSON input do not carry mission roles or a list
of group memberships. `GROUP` selects the response population, as documented in
[CelesTrak's query documentation](https://celestrak.org/NORAD/documentation/gp-data-formats.php).
`CLASSIFICATION_TYPE` is not a mission category. The loader already assigns group
indices and core retains valid contributing memberships under
[0011](0011-catalogue-ingestion-and-provenance.md). Use that provenance rather than
infer purpose from orbital parameters, names, IDs, URLs or group-name spelling.

- A manifest group may declare `"category": "station"`, `"navigation"` or
  `"weather"`. Omission/null contributes no category. Other values/types fail
  configuration loading atomically, before publishing a catalogue. `"unknown"`
  is not another configured role; omit the field instead.
- The configuration author explicitly chooses which groups warrant a role.
  Broad groups such as Active/Brightest, heterogeneous benchmark pools and
  unreviewed sources normally have no role. This does not automatically treat
  every member of an arbitrarily named `stations` group as a station.
- Resolve every accepted entry's retained memberships using the highest declared
  role: Station, then Navigation, then Weather. Repeated equal roles do not
  change the result. Uncategorized groups do not override known roles. No role
  yields Unknown; overlapping recognized roles do not.
- Preserve all memberships and the chosen orbital record. Category priority is
  independent of group order, record order, epoch tie breaking and selected
  record origin. Valid **older** records establish membership too; invalid
  records and empty groups do not. This is existing 0011 behavior, not a new
  newest-record-only category rule. Do not reinterpret membership as current
  operational status or fetch freshness.
- The host loader returns a group-indexed role table. The clockless app owns
  the table and resolves the currently selected entry without a per-ID cache,
  new propagation, I/O, or runtime allocation during resolution. Missing table
  indices defensively contribute Unknown. The renderer consumes only the
  resolved optional category; `None` means no selection, not Unknown.
- `sim/examples/weather-gps.json` explicitly assigns Weather and Navigation;
  `tools/examples/catalogue.json` assigns Station to its ISS-only fixture.
  The mixed demo pool stays uncategorized. No per-NORAD-ID mappings, overrides,
  automatic group discovery, new fetching, or changes to archived orbital data.

This adds a local tools→app dependency for the shared role type, not a new
external package or an app→host dependency. Serde configuration parsing remains
host-only; core propagation/catalogue semantics and firmware remain unchanged.
The workspace lockfile gains only that local dependency edge. Historical M2
verification still uses [0023's pinned revision](0023-frozen-m2-revision-isolation.md).

### Exact label and raster

Only the new category slot changes. Keep the existing 48px strip, padded
separator and every elevation/altitude/speed/period pixel fixed. Use existing
`FONT_6X10`, `Cat` cells starting at `(367,10)`, baseline `(367,17)`. The 24×24 icon
footprint starts at `(364,19)`. Coordinates below are local x-right/y-down;
endpoints and ranges are inclusive. No border, numeric readout, antialiasing,
dithering, animation or layout displacement.

Navigation is the union of x=11, y=2..21 and y=11, x=2..21: a 1px square-ended
cross. Station is an outlined body x=9..14, y=8..15, with wings at y=11,
x=2..9 and x=14..21. These copy the accepted 0038 icon pixels exactly, including
their original centering; do not recenter or enlarge them to match data circles.

The accepted cloud and question mark use the following exact nonblank rows.
Unlisted rows/columns are white; commas separate disjoint x values/ranges.

| y | Weather x | Unknown x |
|---|---|---|
| 3 | — | 10..13 |
| 4 | — | 9,14 |
| 5 | 8..11 | 8,15 |
| 6 | 7,12 | 8,15 |
| 7 | 6,13 | 15 |
| 8 | 6,13..16 | 15 |
| 9 | 5..6,17 | 14 |
| 10 | 3..4,18 | 14 |
| 11 | 3,18..20 | 13 |
| 12 | 2,21 | 12 |
| 13..15 | 2,21 | 11 |
| 16 | 3,20 | 11 |
| 17 | 4..19 | — |
| 19..20 | — | 11..12 |

The renderer uses four immutable 24-row bitmasks, 384 bytes total, with no new
framebuffer or allocation. This storage accounting is not target RAM, placement,
latency or physical-display validation. No S3 frame-budget claim follows.

### Static lifecycle

- Update immediately on frames, Tab selection and zoom-driven reselection.
  Category is independent of physical field validity and does not interpolate.
- A successful visible selection always has `Cat` and one icon, including the
  question mark when unclassified. Unknown is not a tracking/ingestion error
  and does not add a notice.
- With no successful visible selection, clear all five labels/icons and retain
  the separator. This includes no frame, invalid time, no in-view markers and
  failed former selections. Off-screen arrows do not supply a selected category.
- ID hiding does not hide category. Notices still draw last and do not change
  category availability, even when covering the selected marker. Other successful
  selections may supply all fields alongside explicit failures/ingestion warnings.
- Keep camera, eligibility, counts, selection, ID hysteresis, notices and square
  diagnostics unchanged. No animation, new selection policy or debug control.

## Verification

Tests cover strict optional schema, unchanged legacy manifests/reports, all role
pairs/order/duplicates/missing indices, older/invalid/empty group membership,
selected-source independence, label-hidden and frame/selection lifecycle,
exact icon/label pixels, field/radar/notice isolation, square invariance and
independent executable expectations. CLI tests exercise explicit overlapping
roles and real replay geometry without an SDL window. Synthetic role assignments
exercise policy and are not new claims about ISS mission roles.

Implementation files are `app/src/{category,lib}.rs`, `tools/src/catalogue.rs`,
`sim/src/main.rs` and `render/src/{category,lib}.rs`, with corresponding loader,
app, renderer and headless snapshot tests. The app remains no_std + alloc;
render remains no_std. Code/tests own API details.

Required implementation gates: serial workspace check/tests, formatting,
app/render no-default-feature checks, strict all-target app/render/sim/tools
Clippy, before/after square-byte and wide-outside-slot comparisons, and exact
preview/executable category-pixel parity. Fresh executable visual acceptance is
a separate user gate. No hardware, firmware, target timing, external dependency,
fixture refresh, production limits or frozen-M2 evidence renewal is authorized.

### Implementation verification

Serial `cargo check --workspace`, `cargo test --workspace`, formatting,
app/render no-default-feature checks and strict all-target app/render/sim/tools
Clippy passed. There are 26 app, 40 renderer, 14 loader integration and 10 CLI
snapshot tests, alongside the rest of the workspace tests.

Thirteen before/after captures preserved all three square PNGs byte-for-byte,
all ten wide images outside x=364..387/y=10..42, and all count/diagnostic reports
(normalizing only snapshot destinations). Cases covered selected/ID-hidden/empty
weather/GPS, a later frame, empty/selected mixed demo and four synthetic-fault/
real-ISS notice cases, including a notice-covered marker. The documented category
review recipe ran headlessly with real pinned ISS, GPS, Metop-B and unclassified
TDRS inputs; all four Cat/icon rasters exactly matched the approved illustrative
pixels after accounting for LcdWhite's two display colors. The comparison does
not turn the illustrative data values into measurements or establish a production
per-ID table. The user subsequently reviewed and accepted the fresh executable
comparison showing Station/ISS, Navigation/GPS, Weather/Metop-B and unclassified
TDRS with real orbital readings at deliberate subpoint review sites. This closes
the visual-acceptance gate separately from the earlier illustrative review. The
acceptance update changed docs only; no new Cargo checks/tests accompanied it.
Wrap-up subsequently reran all Rust gates above and the documented headless review
recipe, and rechecked preview parity, retained assets, local links and whitespace.
No interactive, hardware, physical-display or target-cost validation was run.
