# The user's board updates itself from signed `beta` releases and rolls back bad ones
Status: active
Started: 2026-11-08. Scope: PLAN M7 item 1; decisions
[0083](../decisions/0083-flash-layout-for-updatable-devices.md),
[0084](../decisions/0084-settings-store.md) (failed-build field only),
[0085](../decisions/0085-signed-updates-and-release-keys.md),
[0086](../decisions/0086-bootloader-rollback-and-health-check.md).

## Outcome
The device app runs on the 0083 layout under a custom rollback bootloader with
`dram2` as a second internal heap region. After each element refresh it checks
its channel's signed manifest, streams a newer build into the inactive slot,
activates it and resets. A new image marks itself `Valid` only after 0086's
health check; otherwise the bootloader returns to the old slot, which refuses
that build again. Panics reset; a hung loop is reset by a hardware watchdog.

## Done when
- Board capture: a `beta` manifest signed with the user's key (made on the Mac
  with `overhead-release`) is installed by the board with no hands-on step,
  the new build passes its health check and is `Valid`.
- Brick drill (0086) on the board, each leaving it on a working image: panics,
  hangs, cannot reach the manifest, bad signature, download cut off midway,
  power cut mid-install (user action).
- Host tests: manifest canonical bytes, signature/length/hash/build checks,
  health-check state machine, settings failed-build record.
- `cargo check --workspace`, firmware host tests, release builds gated.

## Boundaries
- Not item 2: no BLE, no setup app, no settings UI; Wi-Fi and site stay
  build-time (`.wifi.env`/`.site.env`). Only the settings fields this item
  needs are written, in a format item 2 extends.
- No `stable` channel, no gift images, no Secure Boot or flash encryption.
- The release private key never touches devbox, GitHub or the repo.
- Pushing anything to the public `pages` branch: ask first (HANDOFF).
- Bootloader built in rootless Docker only (`espressif/idf` v6.1); nothing on
  the host. Rebuilding it later is a toolchain change.

## Human gates
- Test signing key on devbox for drills (trusted only by a dev feature, never
  a release image): asked 2026-11-08; default yes (recommended; "start it").
- Hosting: raw GitHub fallback first, then the custom domain once the user's
  DNS/Pages/verification is done: asked 2026-11-08; default as recommended.
- First push to `pages`: ask before it (not yet asked).
- User at the Mac: generate the release key with `overhead-release`, sign the
  final `beta` manifest. Batched at the end.
- Power cut mid-install drill: user pulls USB on cue. Batched at the end.

## Plan
- [x] Bootloader: `firmware/bootloader/build.sh` (rootless Docker,
      `espressif/idf:v6.1`), binary + sdkconfig + IDF commit + image digest +
      hash committed; segments match espflash's bundled one (dram2 ok).
- [x] 0083 partition table written (`firmware/partitions.csv`).
- [ ] Flash device app with the new bootloader + table; confirm boot, `dram2`
      heap, element store at the new offset.
- [x] `overhead-update` crate (no_std): manifest, canonical signed bytes,
      ed25519-dalek 3.0.0 `verify_strict`, streaming SHA-256, offer rules.
- [x] `overhead-release` in tools/: keygen, pubkey, sign, verify.
- [x] Settings record (`settings_store.rs`, format 1: channel, failed and
      pending build), `ota_health.rs` (boot decision, health), `otadata.rs`
      (own IDF-compatible entries), render `draw_updating`.
- [ ] Device: reset on panic, hardware watchdog, health check (PendingVerify →
      Valid/Invalid within 10 min), manifest check after each refresh, stream
      to inactive slot, 2-min idle gate, `UPDATING n%`, activate and reset.
- [ ] Dev host for drills (devbox HTTP server, dev feature, test key).
- [ ] Brick drill items on the board.
- [ ] Real key + `pages` branch + `beta` update on the board (gates above).
- [ ] Promote: decisions notes, firmware README, PLAN, HANDOFF, PROGRESS.

## Discoveries and assumptions
- `esp-bootloader-esp-idf` 0.6.0 `Ota`/`OtaUpdater` slot choice is wrong
  after an erased otadata (USB flash): Factory→ota_1 gives seq 0 and "next"
  can be the booted slot. `otadata.rs` writes entries itself (CRC = ROM
  `crc32_le(u32::MAX)`, checked against IDF's `9a 98 43 47` for seq 1).
- The health check cannot wait for the 8 h refresh: a new image checks its
  manifest as soon as it has trusted UTC, retrying each poll.
- Rollback detection uses settings `pending_build` (written before
  activation): any other build booting with it set records it as failed.
- Hosts: raw.githubusercontent.com and *.github.io chain to Let's Encrypt
  YR1 → Root YR (cross-signed by ISRG Root X1), 2026-11-08. The
  `overhead.wesklein.com` CNAME is not set yet.
- Custom bootloader: IDF v6.1 commit fff9895c, sha256 d86e62bf…; RWDT is
  disabled by esp-hal init, so the app arms its own.
- `esp-bootloader-esp-idf` 0.6.0 has `OtaUpdater` (next_partition,
  activate_next_partition, set_current_ota_state); needs otadata + 2 slots.
- `live_radar.rs` already adds `dram2` via `heap_allocator!(#[esp_hal::ram(reclaimed)])`
  under `ble-spike`; this item makes it unconditional for runtime images.

## Verification

## Open questions
