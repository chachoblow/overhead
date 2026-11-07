# Giftable-device foundations are decided
Status: blocked: user review of docs/giftable-device-proposal.md (gates 1–4)
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
Docs only. No code, no toolchain install, no Pages publish, no key generation.

## Human gates
- Accept proposal A–F: asked 2026-11-07
- ESP-IDF bootloader build (rootless Docker, espressif/idf v6.1): asked 2026-11-07
- GitHub Pages on chachoblow/overhead: asked 2026-11-07 (re-confirm at publish)
- Key generation on the Mac by the user: asked 2026-11-07

## Plan
- [x] Research pinned crates (esp-bootloader-esp-idf OTA, esp-radio AP), espflash bootloader, hosts/TLS
- [x] Write docs/giftable-device-proposal.md
- [ ] User review
- [ ] Write 0083–0087, supersede 0004, notes on 0002/0075/0076
- [ ] PLAN M7, DESIGN, HANDOFF; delete this file; handoff

## Discoveries and assumptions
- espflash's bundled S3 bootloader has no rollback (default IDF sdkconfig) → custom bootloader needed.
- GitHub Pages: one host, TLS 1.2, Let's Encrypt (ISRG Root X1/YR), Content-Length on HTTP/1.0.
- Device image ~1.39 MB loadable; 3 MiB slots proposed.

## Verification
(none yet; docs only)

## Open questions
