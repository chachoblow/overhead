# 0059 — Coast-only fallback sensitivity (not a device frame)

**Offline diagnostic, not an approved look, threshold or renderer.** The six 400×240 1-bit tiles below contain only the provisional coastline stroke; there is **no limb, satellites, selected label, strip, notices or finished-frame layering**. Source reduction, stroke and bounds remain trials. See the [evaluation](../evaluations/m4-coastline-fallback-appearance-study.md) for numbers, method and reproduction.

![Coast-only local gap and fallback poster, three columns and two rows](0059-coastline-fallback-diagnostic.png)

| Column | Top row: local-gap trial | Bottom row: illustrative fallback |
| --- | --- | --- |
| Left: Colorado 20° | 980 coast pixels; 16 tiny unknown tangent intervals | **Blank geography** under a *hypothetical* 0.0001° unknown-arc cutoff. Foreground would still exist in any future device frame. |
| Middle: San Francisco 20° | 890 coast pixels; 10 tiny unknowns | Same coast under a *hypothetical* 0.001° cutoff. |
| Right: synthetic 20° mixed scene | Five certified coast pixels remain despite a separately budget-exhausted horizon crossing | **Blank geography** because work exhaustion overrides the hypothetical arc threshold. |

The right-hand five-pixel speck is intentionally a synthetic control, not a real geographical map. The example shows that a blanket fallback can erase independently certified geography; the Colorado example shows that a small accumulated unknown arc can trigger visually large suppression. Neither example sets a production cutoff or proves the underlying geometry/raster/target-float correctness.
