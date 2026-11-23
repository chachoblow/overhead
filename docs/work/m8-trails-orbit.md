# Trails and the selected orbit on the wide radar
Status: blocked: panel review by the user (diag5 image on the board)
Started: 2026-11-22. Scope: PLAN M8 item 2; decisions [0089](../decisions/0089-full-width-radar-revision.md), [0077](../decisions/0077-m2-operating-budget-visual-group.md), [0088](../decisions/0088-selection-button-and-captured-knob-switch.md)

## Outcome
The wide radar shows a four-step past-line trail behind each unselected
in-view above-horizon marker (spacing scales with the lens) and the selected
satellite's one-period orbit as a solid line broken where the Earth hides it,
in the simulator and on the device. Trail history and orbit points are
presentation state owned outside the renderer (`overhead_app::motion`); a
frame without it, or with state from another frame/selection, draws no
trails or orbit. Cost measured on the S3.

## Done when
- Host tests: history/orbit validity rules (catalogue change, time step back,
  gap, selection change, mismatched frame → nothing drawn), trail spacing,
  orbit broken behind the Earth, render with/without motion.
- Simulator `--wide --snapshot` shows trails and orbit (warm-up before start).
- `live-fetch-radar` on the board: FRAME_WINDOW frame time, heap/PSRAM, and
  selection/knob latency within 0077/0088 limits, recorded.
- Device app image with trails/orbit flashed; user accepts on the panel.

## Boundaries
No extra propagation for markers or trails; orbit ≤ ~160 propagations per
selection, spread over frames. No new device settings. Release 16 is not
part of this item (PLAN M8 done-when). No toolchain change.

## Human gates
- Panel acceptance: needed (after the board image is flashed).

## Plan
- [x] app: `RadarApp::{camera, sample_times, catalogue_generation}`;
      `app/src/motion.rs` (two-tier trail history with per-object sample
      times; slot-ring orbit in TEME, rolled by time, filled nearest-now
      first under a budget). Tests in `motion_tests.rs`.
- [x] render: `draw_with_motion`; Liang–Barsky clipped lines; orbit then
      trails before below-horizon diamonds and markers; `FrameStage::Motion`.
- [x] sim: Motion each frame, 30 min warm-up (10 s frames) before the start.
- [x] firmware: Motion in live_radar, `ORBIT_BUDGET` 4/frame;
      INPUT_CAPTURE_STATS gains `motion_max_us orbit_points trails`.
- [x] host tests, check, clippy; snapshots (scratchpad, visual group 20/60/100°).
- [x] board: live-fetch-radar measurement, then frame-cost fixes (below).
- [ ] panel review on `target/m8c/diag5.elf` (same picture as the device
      image, plus telemetry): selection presses and knob turns, measured.
- [ ] flash device image `target/m8c/m8c-8h.elf` (`0.1.15-m8c`, gated) and
      confirm boot; then complete (docs, PLAN, HANDOFF).

## Discoveries and assumptions
- Trail step dt = clamp(60 s · tan(fov/2)/tan 10°, 1, 900) (lab). Lens max
  100° gives dt 405 s, so history must reach 1620 s. Two tiers: 20 s × 14
  and 120 s × 16 (linear-interpolation error < ~1 px at every lens).
  ~480 B/object, ~120 KB at 256 objects (PSRAM).
- Device FOV 60° → dt ≈ 196 s, trails ≈ 13 min. The board test capped
  history at 240 s, so the panel has not shown full-length trails yet.
- PSRAM propagation ~2.2 ms each (0077): orbit fill budget kept small per frame.

## Verification
- Board (Mac, MAC checked, every image gated; logs `firmware/target/m8c/`):
  - base (HEAD `fc2dea0`, item 1 frame, no motion): 14 Hz, idle work max
    ~70 ms, update ~22 ms (`base-run1.log`). Item 1 had regressed from the
    pre-M8 20 Hz / 41–45 ms (firmware README 2026-11-04/05).
  - diag (motion, unoptimized): 11.4 Hz, ~86 ms; motion update ≤1.7 ms.
  - diag2 per-stage maxima (µs): clear 14.6k, coast 1.9k, earth 5.4k,
    motion 13–15k, markers 17.8k, overlays 9.4k, send ~8k, update ~30k.
  - diag5 (after fixes): 20 Hz, 199 frames/10 s, ~1 skip per window, idle
    work max 45–52 ms; stages clear 1.2k, coast 1.9–2.7k, earth ~6k, motion
    3.4–4.5k, markers 4.8–6.1k, overlays 6.7–8k, send ~6.8k; update
    21–24k, motion update ≤3k; heap_now ~472 KB (PSRAM ~427 KB, internal
    ~45 KB), no panic (`diag5-run1.log`).
  - diag5 30 min after reset (2026-11-23 01:04–01:34 UTC, no user input;
    `diag5-review2.log`): FRAME_BOOT idle 22,999 frames / 4 skipped, p50 41,
    p99 46, p99.9 51 ms (pre-M8: 41/45 p50/p99.9); 3 automatic selection
    changes 56–59 ms, no skips (pre-M8 select ~165 ms); full resend p50 88;
    heap_now 504 KB with full trail history (PSRAM 460 KB, internal 44.7 KB
    used / 127 KB free); no panic. `mac.sh read` returned garbage bytes
    (6 KB/15 min) on this board state; `monitor` works.
- 2026-11-22: `cargo check --workspace` ok; `cargo test --workspace` 341
  passed; clippy clean (app/render/sim, render representation-measurement);
  firmware host tests default 38, `device` 172 passed.

- Frame-cost fixes (all pixel-identical in snapshots): firmware `Screen`
  byte-wide `fill_solid` (clear 14.6→1.2 ms); `Sample::hidden_by_earth`
  cached at sampling; orbit camera points cached in `Motion`, 20 re-projected
  per update; trail camera points cached per entry keyed by (sample UTC,
  step); halos and pill fills as spans; f32 line clipping and ring trig.

## Open questions
- Full-length trails at the device's 60° may look longer than the board test
  (capped at 240 s); default is 0089's lab formula, judged on the panel.
