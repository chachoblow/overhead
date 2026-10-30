# Overhead — Handoff

_Last updated: 2026-10-30 UTC. M3 complete, M4 done for now; M2 operating policy and M6 standalone device remain open. Board last known to run the **separate pinned-OMM live-UTC diagnostic**, not a production device app._

## Current state
- Shared `app/`, 1-bit `render/` and simulator implement accepted wide radar/glyphs/notices and prepared coast. [PLAN](PLAN.md) owns scope; [DESIGN](DESIGN.md) intent. Button/knob controls, fetching and production capacity/cadence remain unimplemented.
- Time contracts [0062–0066](decisions/0062-no-trusted-time-display-preflight.md) cover `NO FRAME`, screened SNTP, bounded holdover (`D=20 ppm`, `H=3,600 s`) and opt-in Wi-Fi/UDP. [0067](decisions/0067-opt-in-live-utc-iss-radar-scope.md) bounds the original fixture-only image; the [dated diagnostic trial](live-utc-visible-marker-proposal.md) is distinct, not a freshness/fetch contract.
- Board: ESP32-S3-DevKitC-1-N8R8 MAC `80:45:6b:2a:45:08`, Sharp #4694 GND→G, Vin→5V, Clk→IO13, DI→IO11, CS→IO10. UART last verified `/dev/cu.usbserial-10` (recheck). **No further board access, capture, reset or flash was approved; ask before doing any of these.** Never change toolchains without asking.

## Physical observation (2026-10-29 20:25–20:35 PDT / 2026-10-30 UTC)
- The [pinned 2026-10-26 ISS OMM diagnostic](../firmware/README.md#pinned-omm-live-utc-marker-diagnostic-flashed-no-visual-observation) was flashed on 2026-10-27 with separate approval; its prior UART capture logged accepted SNTP and changed/full sends, but the user was not watching then. Original empty view at an earlier UTC is explained in `firmware/src/live_utc_preflight.rs`.
- This session the user watched the already-running Sharp panel: first `NONE ABOVE THE HORIZON` (user's wording), then **ISS slowly moving through the frame**, then the marker left and the notice returned. This is user-reported physical visual observation, not a new UART capture, independently verified clock/image identity, actual-location pass, orbital-accuracy or sustained-cadence proof. No board access/reset/flash occurred. Ignored `.wifi.env`, credential-bearing ELF/image and raw UART log must not be shared.

## Host scheduling and review
<a id="this-session-host-only-2026-10-29"></a>
- On 2026-10-29 the user accepted historical headless images from the [checked-in dated OMM](../firmware/fixtures/README.md): one empty view and three moving-marker frames. The earlier one-off 2026-10-29 14:11–14:22 UTC pass was declined and is not a reusable appointment.
- On 2026-10-30 ~03:18 UTC, fetched a new CelesTrak ISS OMM (epoch 2026-10-29 13:31:42Z) into ignored `target/iss-next-pass/iss.json`. At the **configured test observer**, not the user's physical location (lat `0.000017408882706672767`, lon `-85.18035731481179`, height `0` km), `overhead-passes` with 30 s detection, 5 s crossing tolerance and 0° elevation found the fresh-element horizon window ~03:25:09–03:34:28 UTC (24 h search complete, 2,911 evaluations). A separate 1 h pinned search completed in 127 evaluations. The pinned-OMM search gave ~03:25:07–03:34:26 UTC. Both OMMs yielded `sky=1 visible=1 failed=0` at sampled 5° wide snapshots inside that window. Modelled horizon timing does not guarantee real orbital accuracy; no OMM fetching on device. [Pass tool](../tools/README.md#overhead-passes) and [snapshot usage](../sim/README.md#deterministic-snapshots-and-density-comparison) own reproduction; ignored inputs/PNGs are not committed evidence.

## Next bounded step
- **Discuss** BOOT-button satellite cycling for multiple/one/zero selectable objects and outline host-test cases; behavior and implementation are not yet approved. The knob remains separate. If more physical tests are wanted, coordinate availability and obtain separate hardware approval before accessing the board; no unattended waits.

## Verification and remaining risks
<a id="verification"></a>
- This session: fresh 24 h host pass search completed (2,911 evaluations), pinned 1 h search completed (127); five fresh and five pinned 5° snapshots at 03:26–03:34 UTC reported `catalogue=1 sky=1 visible=1 failed=0`. User-reported panel motion/notice return is visual only; no new hardware telemetry. Documentation-only wrap-up: whitespace and local-link/anchor checks passed; **workspace/firmware checks and tests not rerun** because no code changed. Prior exact-image gates are historical, not this session's verification.
- Physical COM edges, blocked-loop service, long-duration operation, heap/stack peaks, real-world orbital accuracy, SPI panic recovery and production 20 Hz remain open. RWX linker warning persists; M2 freeze uses its pinned revision, not the evolving workspace lockfile.
