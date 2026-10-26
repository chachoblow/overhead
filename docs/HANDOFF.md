# Overhead — Handoff

_Last updated: 2026-10-26 UTC. M3 complete, M4 done for now; M2 operating policy and M6 standalone device remain open. The board runs the approved **Sharp natural-speed command-audit v2** historical single-ISS fixture, not a live device app._

## Current state
- Shared clockless `app/`, 1-bit `render/` and simulator provide the accepted wide radar, glyphs, notices and prepared coastline. [PLAN](PLAN.md) owns scope; [DESIGN](DESIGN.md) owns interface intent. [0060](decisions/0060-defer-timing-to-physical-prototype.md) defers timing choices to the physical prototype; button/knob zoom is not implemented.
- **UTC is now settled for the prototype, but not yet integrated into the device loop.** [0062](decisions/0062-no-trusted-time-display-preflight.md) no-time `NO FRAME` behavior; [0063](decisions/0063-bounded-utc-holdover-direction.md) bounded holdover and ≤100 ms target; [0064](decisions/0064-prototype-sntp-screen-and-leap-boundary.md) SNTP reply screen (unique NTP era, LI=1/2/3 revoke, KoD, `(RTT−τ)/2` uncertainty) and no holdover across a UTC month end; [0065](decisions/0065-prototype-drift-bound-and-holdover.md) `D = 20 ppm`, `H = 3,600 s`; [0066](decisions/0066-prototype-live-sntp-stack-and-source.md) `esp-radio`/`esp-rtos`/`embassy-net` stack behind an opt-in feature, Cloudflare then NIST, build-time credentials. [Proposal](utc-trust-loss-proposal.md) is the reasoning record.
- Board: ESP32-S3-DevKitC-1-N8R8 (MAC `80:45:6b:2a:45:08`), Sharp #4694 GND→G, Vin→5V, Clk→IO13, DI→IO11, CS→IO10. UART last seen at **`/dev/cu.usbserial-10`** (port name changes on replug; check MAC with `espflash board-info`). During this session the user waived per-flash approval; AGENTS.md still says to ask, so **confirm at the start of the next session**. Always run exact ELF/image gate checks; never change toolchains silently.

## This session
- Host preflights: `RadarApp::invalidate_frame` (`app/src/lib.rs`); `TimeLossPreflight` (`firmware/src/device_timing.rs`); `UtcTrustPreflight` with `prototype()` (`firmware/src/utc_trust.rs`); pure reply screen, request builder, bracketing capture bound and consistency check (`firmware/src/sntp_screen.rs`).
- Target diagnostics (opt-in, not the device app): `clock-drift` (`firmware/src/clock_drift_main.rs`) measured the S3 timer at ≈+1.2 ppm over 3 h (host-reference-limited); `live-sntp` (`firmware/src/live_sntp_main.rs`) ran 1 h on home Wi-Fi: 59/59 Cloudflare replies accepted, `U0` 21–58 ms (median 28), 58/58 consistency checks in bound, S3 ≈−0.4 ppm vs UTC. Host tooling `firmware/drift_capture.py` (+ tests). Evidence, hashes and commands: [firmware README](../firmware/README.md#s3-clock-drift-capture-flashed-3-hour-uart-run), [live SNTP](../firmware/README.md#live-sntp-source-evidence-flashed-1-hour-uart-run).
- `firmware/Cargo.toml`/`Cargo.lock`: new optional deps for `live-sntp` only (37 lock entries added, no existing version changed). `.gitignore` excludes `firmware/.wifi.env` (local `SSID=`/`PASSWORD=`; exists on this machine). A `live-sntp` ELF embeds those credentials: never commit or share it.

## Next bounded step
- **Live-UTC radar on the device:** compose the `live-sntp` adapter, `UtcTrustPreflight::prototype()` and the existing natural-speed render/send loop in a new opt-in image, so the ISS fixture is propagated at trusted live UTC (showing `NO FRAME` before sync/after expiry). Agree its scope first (fixture elements vs fetch; panel service while Wi-Fi blocks; failure behavior). Element fetch/persistence, controls and production cadence stay out.

## Unresolved decisions and risks
- `D = 20 ppm` rests on one board, room temperature, a few hours; revisit with temperature/multi-board evidence. The ≤100 ms target is shown only on one home network for one hour.
- `esp-radio 1.0.0-beta.1` is pre-release with closed Wi-Fi drivers and no WPA3; credential provisioning for the handheld is undecided.
- Holdover ends near each UTC month end while offline (brief `NO FRAME`); no smearing-source detection exists.
- Carried over: M2 catalogue/prediction cadence and over-budget policy; failed SPI command panics; blocked-loop/VCOM service, physical COM edges and extended operation unverified; 8-second full refresh is diagnostic only; linker RWX warning remains. [Frozen M2 gates](evaluations/s3-m2/README.md) need their pinned revision.

## Verification
- Fresh at wrap-up: `cargo check --workspace`; `cargo test --workspace` (318 passed); firmware host tests default 6, `sharp-radar` 43, `sharp-natural-audit` 53, `live-sntp` 17 (all pass); `cargo fmt --check` (workspace and firmware); `python3 -m unittest test_drift_capture` (4); target `--locked` release builds of `overhead-s3-clock-drift` and `overhead-s3-live-sntp` (dummy credentials); local doc links; `git diff --check`. No flash, image gate or hardware run at wrap-up.
- Earlier in session (not rerun): both flashed images passed the offline image gate; board restored to v2 (ELF `d87028ff…`), boot line confirmed. Linker RWX warning unchanged.
