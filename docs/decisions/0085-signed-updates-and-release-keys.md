# 0085 — Signed updates, release keys and hosting

Date: 2026-11-08
Status: accepted (user, 2026-11-08: giftable-device proposal accepted as written;
hosting agreed 2026-11-07).
From [the giftable-device proposal](../giftable-device-proposal.md) (C, D).

## Decision
**Manifest.** One small JSON manifest per channel (`stable`, `beta`): format
version, channel, monotonic `build`, version string, image path, length and
SHA-256, minimum settings-format version, and an ed25519 signature over the
canonical manifest bytes.

**Device checks.** Verify the signature against a public key compiled into the
firmware (a short accepted-key list); stream the image into the inactive slot
hashing as it goes, never holding it in RAM; activate only if length and
SHA-256 match; refuse a `build` not greater than the running one (no replay;
going back means a new build number); refuse the build recorded as failed
([0084](0084-settings-store.md)).

**Keys.** The private key is generated and kept on the user's Mac with an
offline backup; never on devbox, GitHub or in the repo. Rotation: a release
signed by the old key ships firmware trusting the new one. Losing the key
leaves USB as the only update path.

**Tools and crates.** A `tools/` subcommand `overhead-release` builds,
signs (key file given on the command line) and verifies manifests; it runs on
the Mac. `ed25519-dalek` with default features off and `verify_strict`, the
same crate on device and host (dependency reason: the most reviewed ed25519
implementation; its tables fit in flash).

**Hosting, tried in order, no redirects followed (3xx = failed check):**
1. `https://overhead.wesklein.com/ota/<channel>/manifest.json`, images at
   `…/ota/images/<build>.bin`: GitHub Pages for `chachoblow/overhead` from a
   `pages` branch holding only signed release files, custom domain
   `overhead.wesklein.com`.
2. `https://raw.githubusercontent.com/chachoblow/overhead/pages/ota/<channel>/manifest.json`.

Any update may change the list if one current address still works when it
ships. Roots: ISRG Root X1 and YR (X2 if the chain allows) for both, plus
0071's CelesTrak roots; root bundles are app content renewed by updates.

**When.** After each successful element refresh (8 h, 0075) the device checks
its channel; an available update installs once nobody has turned the knob or
pressed BOOT for 2 minutes, showing `UPDATING n%` (flash writes pause display
and input), then resets. The user's board follows `beta`, gifts `stable`.

## Why
Reasons to update come from outside (a CelesTrak TLS change, a CA rotation)
and must reach devices nobody touches. The signature, not the host, decides
what is installed, so the host can move. GitHub Release downloads were
rejected: two redirects across three hosts and long signed URLs. Wired
updates (browser flasher, UF2) were weighed (2026-11-07) and kept only as USB
recovery.

## Consequences
- Checklist: keep `wesklein.com` registered and verified on GitHub; after a
  domain move keep the old one serving for a long time; never rename the
  account or repo or make it private without first shipping a new fallback.
- A host changing CA while a device is offline longer than a release cycle
  strands it until USB.

## Note — 2026-11-08: implemented; TLS 1.2 cap; key backup

- Signed bytes: a fixed line format rebuilt from the parsed fields
  (`overhead-update::signed_bytes`, domain `overhead-manifest-v1`), so JSON
  spacing, key order or extra keys never change what was signed. Manifest
  format 1 only; a later format is served at a new address.
- `ed25519-dalek` =3.0.0 (default features off) and `sha2` 0.11. Key 1
  `961ae46e…92496` (user, 2026-11-08) is the only accepted key.
- OTA TLS is capped at 1.2: raw.githubusercontent.com negotiates TLS 1.3,
  whose encrypted certificates the 0074 date check cannot see (release 9
  failed its health check on the board for this reason and rolled back).
  If GitHub drops TLS 1.2, devices are stranded until an update changes the
  rule, as for CelesTrak.
- The user keeps the private key's only backup in 1Password (synced, not
  offline as above); accepted 2026-11-08.
- `pages` is an orphan branch with signed files and a README; build 12 is the
  first `beta` release. `ota-dev` drill builds use a LAN HTTP host and their
  own key and are never published.
