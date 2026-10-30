# Overhead — Handoff

_Last updated: 2026-10-30 UTC. M3 complete; M4 done for now. M2 operating policy and M6 standalone device remain open._

## Current state
- Shared `app/`, 1-bit `render/` and simulator provide the accepted wide radar; [PLAN](PLAN.md) owns scope and [DESIGN](DESIGN.md) intent. Time/source contracts are [0062–0067](decisions/0062-no-trusted-time-display-preflight.md); current device images are **diagnostics**, not the standalone app. Knob, fetching, production capacity and cadence remain open.
- Board: ESP32-S3-DevKitC-1-N8R8 MAC `80:45:6b:2a:45:08`, Sharp #4694 GND→G, Vin→5V, Clk→IO13, DI→IO11, CS→IO10. UART `/dev/cu.usbserial-21410` was confirmed for this trial; recheck next time. Board now runs the **fixed-historical multi-marker BOOT diagnostic**, not the live-UTC image. No further board access, reset, capture or flash is approved; ask first. Do not change toolchains without asking.
- A prior approved dated-ISS live-UTC diagnostic showed user-observed ISS motion and horizon notice return on 2026-10-29 PDT ([history](PROGRESS.md#2026-10-30-utc-user-observed-iss-diagnostic-pass)). This was not clock/image identity, physical-location accuracy or cadence evidence.

## BOOT session
- [0068](decisions/0068-boot-selection-gesture.md) records the agreed behavior: in-view-only ascending cycling via 0049, one event on a completed debounced press with no hold-repeat, and discard completed presses without a valid frame. `firmware/src/button_input.rs` is GPIO-independent, host-tested and composed with `RadarApp::select_next()`; active-low GPIO0/pull-up and 20 ms are **provisional diagnostic choices**, not production timing. Knob zoom stays separate.
- `firmware/src/live_utc_main.rs` opts into BOOT with `live-utc-boot`; two user presses were logged on the approved MAC-confirmed live-UTC ISS diagnostic (`valid=true selected=None` at each press). The single ISS was not in view, so that capture verifies events, not cycling/visual change. User tentatively recalled `NO FRAME` without focused panel observation; UART showed valid UTC at both presses. An attempted no-reset monitor connected via flash stub without app output; it is not negative evidence.
- A separate [`sharp-boot-cycle` image](../firmware/README.md#historical-multi-marker-boot-cycle-diagnostic-flashed-short-uart-capture) uses the pinned 16-OMM pool at fixed historical UTC 2026-10-10T00:00:00Z; `firmware/src/coast_measurement.rs` assembles the scene and `firmware/src/sharp_boot_cycle_main.rs` owns BOOT, render and Sharp transfer. It was host-tested, offline-built/exact-gated, and flashed after separate approval. In a bounded reset/UART capture, `visible=4`, selected IDs advanced **19548 → 21639 → 22314** on two presses; the user separately confirmed the selected label/marker changed with **each press** on the panel. This is bounded physical/visual behavior, **not live UTC** or guaranteed short-press/hold/timing behavior. Raw captures and credential-bearing artifacts are ignored local files; never commit/share them.

## Next bounded step
- **Host-first:** review sampling gaps across blocking render/SPI and the provisional 20 ms press/release interval; propose a bounded physical validation protocol only if needed, without claiming a service-latency bound or starting a new target trial. Keep the existing diagnostic image unchanged unless separately authorized. No unattended waits.

## Verification and remaining risks
<a id="verification"></a>
- Fresh wrap-up: `cargo check --workspace`, `cargo test --workspace`, firmware host `--lib --locked` suites (default **10**, `live-utc-boot` **61**, `sharp-boot-cycle` **50** passing), workspace/firmware fmt checks, and both opt-in release builds with offline exact-image gates passed (live-UTC build used dummy credentials; historical build needs none). Earlier local-credential live-UTC build/gate and approved captures are in [firmware README](../firmware/README.md#opt-in-boot-selection-on-the-live-utc-diagnostic-flashed-event-capture). Existing linker RWX warning persists. The fixed-historical host test confirms different selected IDs **and 1-bit pixels**. No frozen M2 gates refreshed.
- Remaining: blocked-loop press capture and latency, physical COM edges, long-duration service, heap/stack peaks, orbital accuracy at real location, SPI panic recovery, standalone integration and 20 Hz. Accepted BOOT behavior is narrower than those claims.

<a id="this-session-host-only-2026-10-29"></a>
Historical 2026-10-29 host pass/snapshot provenance lives in [PROGRESS](PROGRESS.md#2026-10-29-utc-accepted-host-review-physical-pass-deferred) and [tool/simulator usage](../tools/README.md#overhead-passes); earlier generated OMMs/PNGs remain ignored local inputs, not committed evidence.
