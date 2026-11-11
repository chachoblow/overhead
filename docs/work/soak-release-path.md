# Soak and release path for gift units
Status: active
Started: 2026-11-11. Scope: PLAN M7 gift readiness (soak + release/recovery checklist parts); decisions [0083](../decisions/0083-flash-layout-for-updatable-devices.md)–[0087](../decisions/0087-ios-setup-app-over-ble.md)

## Outcome
The board survives a 72 h soak on app-saved settings, takes a mid-soak release
over the air, that release is also the first `stable` release, and an owner-
grade release/recovery checklist exists and has been exercised once.

## Done when
- (a) 72 h continuous devbox UART capture (`firmware/target/soak/soak72.log`, host-timestamped): no `NO FRAME`, no unexpected reset, 8 h refreshes on time.
- (b) Release 15 published to `beta` mid-soak installs over the air, goes `VALID`, settings intact.
- (c) The same `.bin` published as `ota/stable/manifest.json` on `pages`; manifest fetched from `overhead.wesklein.com` and verified with `overhead-release verify` on devbox.
- (d) Release/recovery checklist in `firmware/README.md`; USB recovery flash (release 15 image, `OTA_CHANNEL=beta`, settings kept) tried once after the soak, board back to tracking.

## Boundaries
Out of scope: `ios/`, App Store, another iPhone, gift-unit flashing. Board stays on `beta`. No toolchain/bootloader rebuild. Never force-push `pages`.

## Human gates
- Defaults for release content/timing/recovery: asked 2026-11-10, answered 2026-11-11 "go" (version bump only; publish ~36 h; sign beta+stable in one Mac session; recovery flash keeps settings, seeded `beta`; capture via `systemd-run --user`).
- User signs release 15 (beta + stable) on the Mac: user plans Thu 2026-11-12 morning PST, fallback Wed 11-11 evening PST. Any publish before a refresh works: before ~07:15Z (Wed 11:15 pm PST), ~15:15Z (Thu 7:15 am PST) or ~23:15Z (Thu 3:15 pm PST) installs at the following :46 refresh. Have `15.bin` + sha256 + exact commands ready by Wed 2026-11-11 afternoon PST.
- `NO FRAME` recurrence: stop, report to user.

## Plan
- [x] Start capture: unit `overhead-soak.service` (transient, linger on), started 2026-11-11T03:05:24Z, appends to `firmware/target/soak/soak72.log`. Check: `systemctl --user status overhead-soak`. Stop: `systemctl --user stop overhead-soak`.
- [x] Confirm board is on build 14 / `beta` from the capture: 2026-11-11T07:48Z refresh `attempt=3 outcome=Success uptime_s=86525`, `OTA_CHECK build=14 offer=Current` (no reset since 11-10 07:45Z; `attempt` counts per boot, so the uncaptured 23:46Z refresh was attempt 2).
- [x] Draft release/recovery checklist in firmware README (`## Release and recovery checklist`, marked draft).
- [ ] ~36 h: build release 15 (version bump only), image gate, personal-data check, hand `.bin` + sha256 to user for signing (beta + stable).
- [ ] Push 15 to `pages` (`ota/images/15.bin`, beta + stable manifests), fast-forward only; verify stable manifest from devbox.
- [ ] Watch next 8 h refresh: install, `VALID`, settings intact.
- [ ] At 72 h (≥ 2026-11-14T03:05Z): stop capture, scan log (resets, `NO FRAME`, refresh times).
- [ ] Recovery flash per checklist; confirm tracking; finish checklist text.
- [ ] Complete: promote to README/PLAN/HANDOFF/PROGRESS, delete this file.

## Discoveries and assumptions
- `pages` has no `stable` manifest yet; a `stable` board (incl. any factory-reset board) currently fails its health check (404) and rolls back. (c) fixes that.
- Release 15: `OVERHEAD_VERSION=0.1.15` (no `-beta`: one image serves both channels), `OTA_CHANNEL` unset (stable seed, right for USB-flashed gift units). The user's board keeps `beta` because OTA and USB flashes keep `settings`.
- Recovery flash therefore uses the release 15 ELF as published; a `beta`-seeded build is only needed for an empty store. Keep `target/release/15.elf`.
- The board logs only at 8 h refreshes; last seen 2026-11-10T15:46Z, so expected ~23:46Z (uncaptured) and ~2026-11-11T07:46Z. `uptime_s` there (~86,400 s since 07:45Z on 11-10) shows whether it reset in the uncaptured gap.

## Verification
- Refresh times so far (UTC): 11-10 15:46:39, 11-11 07:48:41 (each cycle starts after the previous fetch ends, so ~1 min drift per cycle is expected).

## Open questions
