# Overhead — Handoff

_Last updated: 2026-10-24 UTC. M4 display items done for now; coastline is a prepared, cached layer measured on the S3. Timing work deferred to the physical prototype; next is M6 scoping once the user finishes soldering._

## State
- [M3 complete](PLAN.md#m3--minimal-real-data-radar-complete). All M4 checklist items are now checked "for now" by the user; [0060](decisions/0060-defer-timing-to-physical-prototype.md) defers frame-timing work (including non-coast frame cost) to a working physical prototype. M2 operating budgets/scheduling and Sharp bring-up remain open.
- **Wide-view coastline (implemented, host/simulator):** [0058](decisions/0058-prepared-wide-coastline-slice.md) replaces the 0054 renderer there with the accepted quadratic coastline from a fixed-observer `CoastLayer` (`render/src/coast/representation_spike/prepared.rs`): prepared once (`CoastLayer::new` + in-place `prepare_for`), zero heap, exact parity with the candidate. [0059](decisions/0059-current-view-coastline-raster-cache.md) caches the current view's 10,560 B raster; only a zoom change re-rasters. The simulator shows a provisional full-screen `PREPARING MAP` frame first. 0053/0055 and exceptional-only `COAST ERROR` are unchanged. Location setting is future work (0004/0057).
- **Normal-view refinements (implemented, user-chosen):** [0061](decisions/0061-filled-arrows-id-backing-none-in-view.md) — filled off-screen arrows, 1px white backing behind the selected ID and the selected diamond (drawn above other markers), `NONE IN VIEW` replacing `SKY OUTSIDE VIEW`. Unselected `+` markers, whole-Earth coastline and empty-sky feel were reviewed and kept.
- **Control intent (agreed, 0060):** a button cycles satellites, a knob zooms continuously; presets as fallback if the knob is infeasible. First firmware may start with BOOT-button control.

## Session result
- Approved [S3 measurement](evaluations/m4-coastline-prepared-target.md) (new `overhead-s3-coast-prepared` image, `firmware/src/prepared_*.rs`): cached coast **386–1,153 µs** (passes provisional 5 ms), zoom-step raster **7.5–21.2 ms**, preparation ~5 ms, zero heap, `.bss` +10,952 B, observed stack 1,712 B cached (cached frame equals coast-free frame peak). All outputs match the host oracle and on-target candidate checks across two boots. Coast-free frames still take 18–20 ms (137 ms selected-ISS scene).
- A first boot exposed triplicated `const` assets and an 11 KB by-value `prepare`; both fixed (single `static` assets, in-place preparation) before the retained run. Host-only preparation trials (uncompressed/compact/heap probe) remain test-only evidence in [the host evaluation](evaluations/m4-coastline-observer-preparation-host.md).

## Hardware/evidence
- Board: ESP32-S3 v0.2, 8 MB flash, on `/dev/cu.usbserial-21410`; left running the **prepared-layer measurement image**. Toolchain unchanged (Xtensa Rust 1.97.0.0, espflash 4.6.0); no Sharp/PSRAM use. Future builds/flashes need new approval. [Evidence and commands](evaluations/m4-coastline-prepared-s3/README.md); ignored local archive `firmware/target/coast-prepared-20261024/`. Frozen M2 stays pinned separately ([instructions](evaluations/s3-m2/README.md)).

## One next bounded task
- When the user's soldering is done: **scope the first physical prototype (M6)** — Sharp wiring/driver choice, how the renderer's frame reaches the display buffer, `CoastLayer` in static storage, and what first boot shows. Scoping only; no flash without approval. Until then, no open M4 item requires work.

## Open decisions/risks
- `PREPARING MAP` wording is provisional. No zoom-delay ceiling chosen (judge on device). Continuous knob zoom re-rasters coast every frame while turning (≤21 ms measured). Non-coast frame cost, Sharp transfer, whole-device 20 Hz and stack safety are unmeasured/unproven.

## Verification
- Fresh at handoff: `cargo check --workspace`, `cargo test --workspace` (314 passed), `cargo fmt --check` (workspace and firmware), render `representation-measurement` check, firmware host tests (default and `coast-prepared`), image-gate unit tests, `validate.py` over retained captures, links and whitespace. No new target build/flash at handoff.
