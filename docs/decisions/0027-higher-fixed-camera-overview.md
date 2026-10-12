# 0027 — Higher fixed camera for the overview

Date: 2026-10-12
Status: accepted

## Decision
Raise the fixed camera from 50,000 km to **75,000 km above the observer**, along
geodetic local up. This supersedes only [0026's camera height](0026-fixed-camera-perspective-slice.md);
the lens range, defaults, orientation, shared Earth/satellite perspective,
clipping and physical-state contracts remain unchanged.

## Why
After trying the perspective view, the user strongly preferred it to the ground
map, specifically liking faster apparent motion when zoomed in and the fuller
satellite overview when zoomed out. They requested an even smaller Earth at the
widest zoom and selected 75,000 km over a more distant 100,000 km alternative.
This is a moderate step back: at 100° FOV, Earth spans about 6.6% of the radar
diameter (roughly 15 pixels at the default layout), versus about 10% previously.
After reviewing the higher view, the user preferred it and liked the empty outer
band as framing. That margin is not a defect to remove by auto-fitting the scene
or adding satellites.

## Consequences
- The camera remains fixed during use: zoom still changes only FOV, with the
  same 20° default, 0.1°–100° range and 1.2× magnification steps. This is not a
  return to camera-travel zoom or a new camera-height control.
- Raising the camera reduces perspective magnification differences and apparent
  motion at a given FOV, while zooming in still magnifies motion naturally. No
  visual retiming is added; later motion policy is not settled by this adjustment.
- The smaller scene can reveal more eligible objects at a given lens setting.
  Preserve the catalogue, true horizon counts and physical readouts; do not
  compensate by increasing population or automatically retargeting the camera.
- Refresh geometry/render/CLI regressions and review snapshots; current examples
  and observed counts live in [sim usage](../../sim/README.md). The feedback is
  not a production satellite-count, cadence, or layout decision.
- The higher-camera view and its open framing have user approval. Review
  radar/information-area proportions next in M3; this does not fix panel contents.
  No engine, firmware, frozen-evidence, fetching or prediction changes.
