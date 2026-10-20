# M4 coastline comparison asset

`coastline-110m-40km.bin` is an **executable comparison asset**, not a selected production map or evidence of S3 capacity. It was generated from Natural Earth 1:110m coastline v4.1.0, public-domain, WGS-84. Pinned archive SHA-256: `664449b39070027e882abb295974d182afec18ca21107273d17e9e8bf6f64817`; original URL/terms and image-source provenance are recorded in [0051's comparison](../../docs/mockups/0051-coastline-legibility-comparison.md#candidate-and-provenance). No source archive is bundled.

Regenerate from a separately obtained, verified archive (no fetching or extra package in the generator):

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/offline/export_coastline.py /path/to/ne_110m_coastline.zip
shasum -a 256 render/assets/coastline-110m-40km.bin
```

Expected export SHA-256: `951ed542a445201c24241bfb263b81567cef20641854be281778788a7a7a6796`. Generator verifies the archive SHA, version, WGS-84 projection, finite source coordinates and topology, then applies the previously reviewed **40 km ECEF chord-distance RDP** (preserving part breaks and closure). It checks the reduced 134 parts / 1,868 points and 15,484 bytes. The runtime consumes 1,734 segments. The format is 1,868 consecutive little-endian `(i32 longitude, i32 latitude)` pairs in 1e-4 degrees, followed by 135 little-endian `u32` vertex offsets starting at zero and ending at 1,868. No header, index, heap allocation or shapefile decoder on the device. The offline max coordinate rounding is 0.00005°; this is **not** a bound on screen pixels at 0.1°.

[0054](../../docs/decisions/0054-first-executable-coastline-comparison.md) owns the provisional renderer behavior and remaining review gates. The 40 km *look* is approved for the orientation comparison, not pixel-perfect reference geography. No measurement of physical-panel legibility, whole-device time, stack or Sharp transfer is implied by the asset's size.
