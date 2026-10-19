# 0050 — Keep range in debug; defer display prediction

Date: 2026-10-18
Status: accepted M4 scope choice; no implementation

## Decision

Keep observer slant range on the existing debug page, not the wide normal radar.
After the [native-size comparison](../mockups/0050-range-vs-no-readout.md), the
user chose the unchanged normal view. Defer pass-prediction display work,
including the suggested empty-sky countdown feasibility study, **for now**.
Keep the implemented pass-search engine and its existing physical semantics;
this is not a decision to remove prediction from the product permanently.

## Why

The normal view prioritizes an uncluttered radar and already has the selected
five-field strip and condition-driven empty-view notices. A range number did
not earn space there; debug provides the exact reading on demand. The user also
preferred not to pursue prediction as the next M4 display slice. Neither choice
implies that the sky is complete, that no future passes exist, or that pass
search results are ready for device scheduling.

## Consequences

- No range label, new numeric readout, countdown, prediction-based empty-view
  wording, or change to [0048's notices](0048-refined-empty-view-notices.md) or
  [0049's selection](0049-stable-wide-selection.md) is authorized.
- [0012's physical-pass semantics](0012-physical-pass-semantics.md),
  [0016's incomplete/ambiguous aggregation](0016-catalogue-pass-aggregation.md),
  core search and the offline `overhead-passes` tool remain intact. Future
  prediction UI would need a separately scoped agreement and measured budget;
  M2's operating limits and scheduling remain open.
- M4's next **recommendation** is a bounded offline coastline-legibility
  comparison at several zooms, without changing the renderer by default.
  Coastline source, mapping, raster rules and implementation have not been
  approved. This record does not close all secondary-information or empty-sky
  refinement, nor does it select production population/cadence.
