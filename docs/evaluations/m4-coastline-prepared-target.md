# M4 coastline — prepared/cached layer on the S3

Status: **one approved target measurement, complete**. Measures the
[0058](../decisions/0058-prepared-wide-coastline-slice.md) prepared layer with the
[0059](../decisions/0059-current-view-coastline-raster-cache.md) current-view raster
cache on the ESP32-S3 at 240 MHz, drawing into a 1-bit RAM framebuffer. Not the
device app: no Sharp transfer, PSRAM, scheduling or frame cadence.
[Evidence, image identities and commands](m4-coastline-prepared-s3/README.md).

## Result

| Measurement (11 scenes × 5 samples × 2 boots) | Range | Earlier comparison |
| --- | --- | --- |
| Prepare once (`prepare_for`) | **4,923–5,112 µs** | — |
| Coast, cached view (steady state) | **386–1,153 µs** | candidate 9,961–22,489 µs every frame |
| Coast, fresh raster (one zoom step) | **7,532–21,177 µs** | ≈ candidate |
| Complete frame, cached coast | 19,257–138,622 µs | coast-free frame 18,208–136,919 µs |
| Complete frame, fresh raster | 28,456–153,495 µs | — |

- **The steady-state coastline now passes the provisional 5 ms screen**, by
  ≥4.3× in every scene. Per scene, a cached frame costs 1.1–2.4 ms more than
  the coast-free control.
- **A zoom step costs one fresh raster**, up to 21 ms (polar 20°), only on the
  frame where the zoom changes. No acceptable zoom-step delay has been chosen.
- **Non-coast frame cost dominates:** the coast-free control already takes
  18–20 ms in ordinary scenes and 137 ms in the selected ISS-ground scene. That
  independent blocker is unchanged.

## Memory and stack

Allocated section differences, prepared image versus the coast-free control:

| Section | Off | On | Difference | Attribution |
| --- | ---: | ---: | ---: | --- |
| `.bss` | 89,720 | 100,672 | **+10,952** | the `CoastLayer` (10,560 B raster + state) |
| `.rodata` | 83,408 | 148,752 | +65,344 | **two** 32,211 B asset copies: the layer's and the measurement fixture for the on-target candidate check; one copy in a device image |
| `.text` | 273,549 | 303,645 | +30,096 | layer, candidate (for exactness checks) and adapter |
| `.data` | 3,556 | 3,716 | +160 | |

Every sample asserts zero allocation calls and no heap growth, including
preparation. Observed stack write depth (watermark, not a safe bound): cached
coast **1,712 B**; prepare and fresh raster 3,312 B (candidate coast was 3,328 B);
cached frame 3,616 B, **equal to the coast-free frame**, so the cached path does
not raise the frame's observed peak. These do not prove the provisional 1 KiB
scratch screen or interrupt/task stack safety.

A first boot exposed two measurement-design problems, fixed before the retained
run: identical `const` asset slices were emitted three times in `.rodata` (now one
`static` per crate), and `CoastLayer::prepare` returned the 11 KB layer by value,
inflating stack. `CoastLayer::new` (const) plus in-place `prepare_for` now let
long-lived storage be prepared without a stack temporary.

## Verification

- Both boots completed all 330 rows; `validate.py` checks the matrix, completion
  and failure markers.
- Every non-Prepare checksum and write count matches the host oracle in both boots.
- On target, every coast-only buffer equals `candidate_draw` and every frame
  equals the shared candidate frame (`EXACT` lines), cached and fresh alike.

Exact agreement with the candidate is not independent cartography, an
all-observer no-omission proof (0055 deferral stands) or physical-panel evidence.

## Remaining

Non-coast frame cost, Sharp transfer, whole-device 20 Hz, stack safety, and
whether ~21 ms zoom steps are acceptable on the panel. Any further target work
needs separate approval.
