# Release 16 is published and installed over the air
Status: blocked: user signs 16.bin on the Mac (beta + stable) and copies OUT back
Started: 2026-11-23. Scope: PLAN M8, last box (release 16); decisions
[0085](../decisions/0085-signed-updates-and-release-keys.md),
[0088](../decisions/0088-selection-button-and-captured-knob-switch.md),
[0089](../decisions/0089-full-width-radar-revision.md)

## Outcome
Build 16 (`0.1.16`, 0088 + setup protocol 2 + 0089 with trails/orbit + the
2026-11-23 review fixes) is signed for `beta` and `stable`, served from
`overhead.wesklein.com`, and the user's board has installed it over the air.

## Done when
- `target/release/16.{bin,elf}` built from clean pushed `main` per the
  [checklist](../../firmware/README.md#release-and-recovery-checklist);
  `image_gate.py` pass; personal-data check 0 (SSID, password, lat, lon).
- User-signed `beta` and `stable` manifests verify on devbox against
  `release_keys.rs`.
- `pages` fast-forward pushed; live manifests and image verify after the CDN.
- Board log: `OTA_CHECK … offer=Install`, `OTA_INSTALL done`, `OTA_HEALTH
  valid`, radar first frame on build 16.
- PLAN M8 marked done; evidence in the firmware README.

## Boundaries
No firmware code changes, no toolchain or bootloader rebuild, no iOS work.
Review fixes ship host-tested; board drills for them stay open risks.

## Human gates
- Channels: answered 2026-11-23: both `beta` and `stable` (one image).
- Pre-signing board smoke test: answered 2026-11-23: skipped; the OTA install is the check.
- Signing on the Mac (git pull, hash check, sign, copy OUT back): asked 2026-11-23.

## Plan
- [x] Build 16 on devbox, save-image, gate, personal-data check, sha256.
- [ ] Hand over to the user for signing.
- [ ] Verify manifests on devbox; publish to `pages`; verify live.
- [ ] Watch the board take 16 at its next 8 h refresh (`mac.sh read`).
- [ ] Docs: README evidence, PLAN, HANDOFF, PROGRESS; delete this file.

## Discoveries and assumptions
- MIN_SETTINGS left at the default 1, as for 15 (settings format unchanged since 15).
- Devbox `OUT` lands in `firmware/target/release/out16/` (ignored).

## Verification
- Built at `4af1896` (clean, = origin/main), `--features device` only, `OVERHEAD_BUILD=16 OVERHEAD_VERSION=0.1.16`, no OTA_*/Wi-Fi/site env → `target/release/16.{elf,bin}`, 2,113,200 B (67.18% of slot).
- `image_gate.py` exit 0; ELF sha256 `cc07a819…5ace2`.
- Marker: one `overhead-build:16` in 16.bin.
- Personal data: SSID 0, password 0, lat 0, lon 0; 2-digit altitude 40 (41 in 15.bin, chance).
- 16.bin sha256 `752f1bf019e3d980db62d38b37b0e0e9e031dd9f1619ef53463c9705127d7a94`.

## Open questions
