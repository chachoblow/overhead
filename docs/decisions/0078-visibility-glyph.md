# 0078 — Visibility glyph replaces the category icon

Date: 2026-11-03
Status: accepted direction (user, 2026-11-03: replace the category glyph with
naked-eye visibility rather than a legend). Icon appearance awaits the user's
review of executable output. Supersedes the display part of
[0045](0045-static-group-category-contract.md); 0045's roles remain for
[0077](0077-m2-operating-budget-visual-group.md)'s over-limit priority.

## Decision
The wide strip's top slot (24×24 at `(364,19)`, label at baseline
`(367,17)`) shows `Vis` and one of three icons for the selected object:

| State | When | Icon |
|---|---|---|
| Visible | the Sun is below −6° at the observer and the object is outside Earth's shadow | eye: lids at `abs(y−11) = round(5(1−((x−11)/9)²))`, x 2..20, with pupil disc r ≤ 2.5 |
| Eclipsed | dark sky, object inside Earth's shadow | filled disc r ≤ 6.5 |
| Daylight | the Sun is at or above −6° at the observer | sun: ring 3.5 ≤ r < 4.5, axis rays 7..9, diagonal rays 5..6 |

Coordinates are local to the slot, centred on (11,11). Code
(`render/src/visibility.rs`) holds the packed masks and an independent
reference; tests compare them pixel for pixel.

- **Model.** Sun from the Astronomical Almanac low-precision formula
  (~0.01°, `overhead_core::sun_ecef`); Earth's shadow as a cylinder of the
  WGS-84 equatorial radius (`in_earth_shadow`), with no penumbra, flattening
  or refraction; dark sky below −6° Sun elevation (civil twilight). It says
  whether the geometry allows seeing the object, not how bright it is or
  whether weather allows it.
- **Lifecycle.** As 0045's: immediate on frames, selection and zoom; cleared
  with no selection; no animation, notice or new selection rule. Computed
  only for the selected object, at its own sample UTC.

## Why
In the `visual` group almost every object would be Unknown, so the category
icon would carry no information. Whether an object can be seen right now is
what the group is for, and it needs no extra data or fetch. A legend was
rejected as strip space for no gain; inferring categories from names (e.g.
"R/B") was rejected as 0045 rejected inference; magnitude needs another source.

## Consequences
- `render/src/category.rs` is removed; `RadarApp::selected_category` and
  manifest roles remain. Simulator and renderer tests now expect the
  visibility icon, computed independently from engine positions.
- Shadow-edge timing can be off by seconds; twilight between −6° and darker
  is shown as dark sky.
