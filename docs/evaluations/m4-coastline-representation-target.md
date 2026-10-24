# M4 coastline — single quadratic-representation S3 feasibility check

2026-10-24 UTC. **Large improvement, but no-go against the current rebuild/scratch
screens. Stop general-renderer optimization and scope location preparation/reuse.**
This completes the one separately authorized target comparison in
[0056](../decisions/0056-bounded-coastline-architecture-trial.md). The user accepted
the [host candidate's appearance](m4-coastline-representation-spike.md) before
explicitly authorizing measurement-only implementation, target builds and exact-gated
flash/run. That authorization did not cover production replacement, toolchain
changes or Sharp work; none occurred. No further algorithm variant or target run
is proposed under this experiment.

## Decision evidence

| Screen / control | Measured result | Assessment |
| --- | --- | --- |
| Complete coast rebuild ≤5 ms | **9,961–22,489 µs** across 55 samples | Fails; even the sampled minimum exceeds 5 ms |
| Packed asset ≤32 KiB | **32,211 B** | Fits by 557 B |
| Compiled code/data increment ≤64 KiB | **58,156 B**, including 112 B additional initialized RAM | Fits by 7,380 B in this harness |
| Incremental heap = 0 | Zero calls/failures/requested bytes and no live/peak growth in every measured invocation | Passes these finite runs |
| Incremental scratch ≤1 KiB | Candidate caller reserves **448 B**; flattening callee reserves **992 B** direct / **1,104 B** clipped | Cannot pass as-is: candidate-specific nested frames alone reserve 1,440 / 1,552 B, before deeper callees |
| Target output | **48/48 complete framebuffer dumps byte-identical to host** | Eleven off controls and all 37 on-image case/mode outputs match |
| Exceptional rollback | Four selected/empty, warned/plain controls match byte-for-byte | Passed; deliberately injected failure, not a natural numeric failure |
| Whole-device 20 Hz | Selected FrameOff still reaches **136,849 µs**; no Sharp transfer | Not established; independent non-coast blocker remains |

The previous renderer's maximum was **9,781,141 µs**. The new maximum is roughly
**435× faster on the S3**, not an extrapolated host ratio, but still **4.50×** the
current 5 ms screen. Both experiments used the same eleven scenes, 240 MHz CPU,
1-bit RAM target and warm-sample convention. These are different measurement
images, not phase attribution or a hard worst-case comparison.

The provisional screens are not immutable product limits. Nevertheless, this
check does **not** silently relax them, approve routine coastline dropout, or
justify another optimization campaign. Keep the accepted geometry/appearance as
useful work; next scope **set location → prepare geography → reuse**. On-device
observer preparation remains a direction to agree, not an implemented cache or
per-user firmware build. Retained memory, setup latency, lens-zoom behavior and
invalidation/persistence need a separate contract. No claim is made that this
fallback automatically meets 5 ms or fixes the non-coast frame cost.

## Measurement-only extraction

- `render/src/coast/representation_spike.rs` now builds under tests or the explicit
  `representation-measurement` feature. The f32 traversal/thresholds are unchanged;
  the host-only audit/preparation stays under `cfg(test)`. The packed reader is
  extracted to `decode.rs`, checked against the original host reader. Its malformed
  offset check uses saturating addition for 32-bit safety. The explicit `s*s`
  replaces `powi(2)` because that method is unavailable in the no_std f32 API.
- The draw target is generalized only to an **infallible** 1-bit RAM target, not a
  production fallible-display contract. `measurement::frame_candidate` uses the
  existing frame compositor and its exceptional rollback through a feature-only
  hook. Normal coast selection and exceptional-error policy are unchanged.
- `firmware/src/representation_main.rs` is a separate binary using
  `representation_measurement.rs`. Historical Stage A, kernel, catalogue and frozen
  M2 sources/features retain their paths. No dependency or lockfile changed.
- `firmware/fixtures/coast-quadratic-trial.bin` is the exact **32,211-byte** accepted
  trial export, not a replacement for the production asset. Hash and regeneration:
  [fixture README](../../firmware/fixtures/README.md).
- A fresh release host audit after extraction reproduced **all prior counts,
  shape metrics, packed bytes and retained review-mask hashes exactly**. A separate
  host example uses the actual firmware app fixtures and packed `Screen`, producing
  the complete framebuffer oracle for the target. Host limb/viewport test solvers
  and the target's six-decimal scene coordinates are not silently conflated.

No observer-dependent preparation is hidden outside timed candidate drawing. The
trial still scans the global representation on every call. Existing f64 camera
setup, packed structure checks, f32 traversal, refinement/clipping and raster are
inside the coast timer. Coast-only target blanking is outside the timer, just as
in Stage A; normal full-frame clearing is inside FrameOn/FrameOff. App fixture
parsing, catalogue construction, propagation and app update are outside all draw
timers. This is not an integrated tick or a production catalogue workload.

## Target capture

The CP2102N UART enumerated at `/dev/cu.usbserial-210`; board-info confirmed an
ESP32-S3 revision v0.2, 40 MHz crystal and 8 MB flash, with secure boot/encryption
disabled. CPU is explicitly set to 240 MHz. The installed toolchain is unchanged:
Xtensa Rust **1.97.0.0** / rustc `8ea53bcd7`, esp-hal **1.2.2**, espflash **4.6.0**,
GCC **16.2.0**. PSRAM is unused; no display driver or Sharp pin/transfer operation
is present. Physical wiring was not independently inspected this session.

Both normal release ELFs were built from `firmware/` with `--locked`, opt-level 3,
no LTO override, `BENCH_SUITE=baseline BENCH_SAMPLES=5`, in separate target directories.
The binary is `overhead-s3-coast-representation`; features are `coast-representation`
(off) and `coast-representation-on` (on). [IMAGE_GATE](../../firmware/IMAGE_GATE.md)
passed each exact ELF/application image, including immediately before its flash.
Flashes used 8 MB / DIO / 40 MHz. The existing LOAD-segment RWX warning remains
visible, not suppressed or newly solved by the gate.

There was **one retained complete monitor boot per image**. Flashing/board-info and
the monitor reset the board; there was no parameter sweep or tuning/reflash cycle.
Each case/mode had one untimed warm-up then five individual timed draws. UART output
and mask transmission occur after the samples. The target asserts zero heap
activity/growth, successful geometry and in-bounds RAM writes; all runs reached DONE.
The board is left running the **candidate-on measurement image**, spinning after
completion, not the device application. Future hardware activity needs new approval.

All observers have zero altitude in this target fixture. The additional elevated
host sweep is not target coverage. The ISS-ground scene is derived from the same
pinned ISS epoch and is the only selected-satellite scene.

| Scene | Coast median µs | Coast maximum µs |
| --- | ---: | ---: |
| SF 0.1° | 10,715 | 10,895 |
| SF 1° | 10,804 | 11,011 |
| SF 5° | 13,515 | 13,716 |
| SF 100° | 17,946 | 18,131 |
| Colorado 20° | 18,677 | 18,871 |
| Polar 20° | 22,304 | **22,489** |
| Dateline 5° | 10,802 | 10,990 |
| Limb 20° | 17,481 | 17,671 |
| Limb 100° | 17,205 | 17,397 |
| Viewport 0.1° | 9,964 | 10,162 |
| ISS ground 20° | 16,085 | 16,173 |

The on-image FrameOn range is **30,775–156,649 µs**. Same-image FrameOff is
18,308–136,694 µs. Late and LateWarning maxima are 170,928 / 171,282 µs. These
injections occur **after the entire candidate succeeds with ink**, then force the
existing coast-error repaint. They are not the old renderer's 20,000-interval
injection and should not be compared as identical late-failure work.

## Compiled footprint and scratch interpretation

| Allocated section | Off bytes | On bytes | Difference |
| --- | ---: | ---: | ---: |
| `.rodata` | 82,768 | 115,792 | +33,024 |
| `.text` | 273,549 | 298,569 | +25,020 |
| `.data` | 3,580 | 3,692 | +112 |
| `.rwtext` | 11,588 | 11,588 | 0 |
| `.bss` | 89,720 | 89,720 | 0 |

`.rodata` + `.text` grows by 58,044 B; including initialized `.data` yields
**58,156 B**, also the difference in the image gate's allocated file-backed ELF
bytes. This includes instrumentation, full-frame specializations and failure
controls, not a pure shipping implementation or the device's remaining capacity.
Image-length growth is only **26,400 B** due to layout/padding and must not replace
the section comparison. The 112 B initialized-RAM increase is not independently
attributed to pure coast scratch; report it rather than hiding it in flash cost.

The new asset exists only in on (ELF offset 21,996); it and its 64-byte prefix are
absent from off. No coast text symbols remain in off. On retains only the new
`coast::representation_spike` path, not the old coastline traversal; the original
15,484-byte asset is absent from **both** images. A merged frame-failure-control
symbol alias does not imply retention of the old coast renderer.

Both images reserve the same 64 KiB instrument heap and two 12,008-byte `Screen`
objects (12,000 pixel bytes plus counters). These are measurement baselines, not
incremental coast scratch or verified spare memory. The pending-array inventory
is **640 B** on this 32-bit target, versus 768 B on the host. Exact compiled `entry`
reservations are 448 B for `candidate_draw`, 992 B for direct `flatten`, and 1,104 B
for clipped `flatten`; caller and callee are nested. Thus even the candidate's
own nested frames exceed 1 KiB before other helpers/raster/caller state. No stack
size tuning was performed to disguise that result.

The existing exclusive-CPU0 watermark was reused: no task, second core or foreign
stack owner is started by this image. Exact disassembly retains the paint/scan
probe and its 8 KiB written calibration local (8,240-byte frame). Observed smoke
baselines/loaded marks are 1,632/9,580 B off and 1,648/9,596 B on. Coast observed
write depth is 3,328 B; FrameOn and late modes are 4,448 B. Same-image FrameOff is
3,536–3,720 B, so the paired **observed-write difference** is only 728–912 B. That
subtraction is **not a live coast scratch bound**: non-coast frame work peaks on a
different path and masks some of the incremental reservation. Do not use it to
claim the 1 KiB screen passes. Guard/margin checks and smoke response do not prove
interrupt/task-safe stack capacity or whole-device memory headroom.

## Output and verification

All 48 dumped 12,000-byte framebuffers match the external host oracle exactly,
not just a checksum: eleven off FrameOff masks, 33 regular on modes and four late
controls. On-write and total-write counts also match the host in all 240 samples;
repeat samples match checksums/counts and observed stack depth. Four untimed
device rollback controls additionally compare complete buffers directly.

This establishes host/target parity for these scenes, not independent cartography,
all-observer no-omission proof, physical-panel legibility or Sharp refresh atomicity.
The accepted host visual review remains separate. [0053's ordinary-gap rule](../decisions/0053-no-resolvable-coastline-omissions.md)
and [0055's deferred universal proof](../decisions/0055-defer-all-view-coastline-proof-for-m4.md)
are unchanged; no work-limit failures occurred and none were treated as normal.

Workspace check/tests, renderer representation-measurement tests, firmware default
and candidate-feature host tests, formatting and 12 image-gate unit tests passed
before flashing. The fresh release extraction audit, byte/hash comparison and
host firmware oracle passed. The retained capture audit and six malformed/partial
capture/oracle negative-control tests pass offline. No frozen M2 source guard,
manifest or evidence was refreshed.

Exact build identities, raw logs, host masks, source hashes, commands and audit:
[evidence README](m4-coastline-representation-s3/README.md). The local ignored
archive retains the ELFs/images/disassembly/source patch and untracked source
snapshot. The published capture is evidence for these exact images, not a blanket
authorization to build or flash another image.
