# Giftable-device foundations are decided
Status: blocked: user acceptance of the revised proposal (A–C, E, F as iOS BLE app) and gates 2–4
Started: 2026-11-07. Scope: proposed PLAN M7 (not yet in PLAN); replaces 0004, amends 0002/0075/0076

## Outcome
The user has accepted (or amended) the giftable-device proposal. Its decisions
0083–0087 are written, PLAN has M7 with its three work items, and DESIGN states
recipient-configured location and over-the-air updates.

## Done when
- User acceptance recorded in the proposal's status line.
- docs/decisions/0083–0087 exist; 0004 is marked superseded; 0002/0075/0076 carry notes.
- PLAN has M7; DESIGN product intent updated; HANDOFF points at M7 item 1.
- `cargo check --workspace` (docs only, but run per AGENTS.md).

## Boundaries
Docs, plus one throwaway diagnostic: the `ble-spike` firmware feature (never a
device image), authorized 2026-11-07. No toolchain install, no Pages publish,
no key generation, no iOS app yet.

## Human gates
- Accept proposal A–C, E, F: asked 2026-11-07; user wants to revisit in a fresh session
- Hosting (D): answered 2026-11-07: main `overhead.wesklein.com` (Pages custom
  domain on the `overhead` repo), fallback raw.githubusercontent.com `pages` branch,
  no redirects followed. User's DNS/Pages setup steps still to do (proposal gate 3).
- ESP-IDF bootloader build (rootless Docker, espressif/idf v6.1): asked 2026-11-07, unanswered
- Key generation on the Mac by the user: asked 2026-11-07, unanswered

- Setup path (F): discussed 2026-11-07. Recipients all use iPhones; user will
  publish to the App Store and has an Apple Developer account. Direction: iOS
  setup app over BLE (Wi-Fi + phone GPS location), device still pulls signed
  OTA itself; app code in `ios/` in this repo (user left it to me). Gate:
  BLE + Wi-Fi coexistence spike first (user: "now, before we get too deep").
  Captive-portal page is the fallback if the spike fails.
- Spike phone test: answered 2026-11-08: connected, wrote, read reply; radar
  and knob normal while connected (user).
- Accept revised proposal (A–C, E, F): asked 2026-11-08

## Plan
- [x] Research pinned crates (esp-bootloader-esp-idf OTA, esp-radio AP), espflash bootloader, hosts/TLS
- [x] Write docs/giftable-device-proposal.md
- [x] Section D hosting agreed (overhead.wesklein.com + raw fallback)
- [x] Walk-through; user asked for conventional alternatives (USB, captive portal, BLE app)
- [x] BLE spike: `ble-spike` feature on live-fetch-radar (esp-radio `ble`+`coex`,
      trouble-host =0.7.0, bt-hci 0.9 as esp-radio pins); GATT write characteristic,
      advertising `OVERHEAD-xxxx`; log internal heap free/peak with BLE up, through
      a TLS fetch, frame stats; phone connect + write by user
- [x] Revise proposal: F → iOS BLE setup app (+ captive-portal fallback), spike evidence;
      evidence in firmware/README.md#ble-coexistence-spike
- [ ] When the spike capture ends (~07:00 UTC 2026-11-08): check the 2 h refresh
      lines, then reflash the device app (8 h, `--locked`) and confirm boot
- [ ] User acceptance of A–C, E, revised F; gates 2–4
- [ ] Write 0083–0087, supersede 0004, notes on 0002/0075/0076
- [ ] PLAN M7, DESIGN, HANDOFF; delete this file; handoff

## Discoveries and assumptions
- BLE spike build: BLE adds ~33 KB static RAM (BT controller code in shared
  D/IRAM +20.6 KB, .data +5 KB, .bss +6 KB; trouble DEFAULT_POOL 4 KB) → 160 KiB
  static heap no longer links. Fix used: static heap 128 KiB + the unused
  bootloader-reclaimed `dram2` region (0x3FCDB700..0x3FCED710, 72 KiB) as a
  second internal region (`#[ram(reclaimed)]`); CPU0 stack 23,640 B (base
  23,888). dram2 was unused in the base image too: free headroom for M7.
- Spike boot (2026-11-08 ~04:00 UTC): BLE_INIT heap cost 35,440 B internal
  (free 160,880 → 125,440 of 200 KiB); boot TLS fetch OK 156/156, 15.0 s;
  BLE_MEM internal_free_min 116,112 through boot+fetch (50 ms sampling);
  FRAME_WINDOW skipped 0–2/200 as before. No phone connected yet.
- Spike image: `--features ble-spike`, REFRESH_MINUTES=120, ELF sha256 4f6708af…,
  gate passed, rev 8a31109 + uncommitted spike; log target/ble-spike.log (ignored).
  Board must go back to the device app afterwards.
- espflash's bundled S3 bootloader has no rollback (default IDF sdkconfig) → custom bootloader needed.
- GitHub Pages: one host, TLS 1.2, Let's Encrypt (ISRG Root X1/YR), Content-Length on HTTP/1.0.
- Device image ~1.39 MB loadable; 3 MiB slots proposed.
- raw.githubusercontent.com: no redirect, TLS 1.2, Content-Length on HTTP/1.0, same ISRG chain.
- Settled in conversation 2026-11-07 (also in the proposal): served setup page, no
  companion app (an app would replace only F; BLE setup could come later by OTA);
  coordinates entry; key on the user's Mac with offline backup, local signing;
  check at refresh, install when idle; user board `beta`, gifts `stable`; a handful
  of units, no date.
- User's personal site wesklein.com (GitHub Pages) had `www` CNAME
  `chachowblow.github.io` (typo, user confirmed); HTTPS on the apex timed out. User's
  to fix; domain verification on GitHub is on the gift checklist.

## Verification
- 2026-11-08: `cargo check --workspace` ok; firmware host tests default 37 ok;
  `ble-spike` release build, image gate passed, flashed, UART capture
  (target/ble-spike.log, ignored), user phone test.

## Open questions
