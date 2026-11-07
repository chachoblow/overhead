# Giftable device: signed updates, setup page and fixed foundations — proposal

Status: **proposed 2026-11-07, awaiting the user's review.** Nothing here is
implemented or authorized until accepted. On acceptance it becomes decisions
0083–0087 (listed at the end) and a new PLAN milestone, M7.

## Why now

The user will give a handful of devices as gifts, with no date set, and must be
able to update them afterwards. Remote updates are a core requirement, not
later polish (user, 2026-11-07). Today every device-app setting that differs
per home is built into the image: `SSID`/`PASSWORD` (0002) and
`SITE_LAT_DEG`/`SITE_LON_DEG`/`SITE_ALT_M` (0004, 0075). Moving or changing
either one means a USB reflash. A gifted device is out of reach, so it needs
three things:

1. **Updates that arrive by themselves**: signed, with rollback, installed
   without the recipient doing anything.
2. **Setup the recipient can do**: Wi-Fi and coordinates entered from a phone
   web page served by the device. There is no companion app.
3. **Foundations that don't change after the first gift**: the partition
   table, the bootloader, the settings format, the signing scheme and the
   first public key, and the manifest location. An update can replace the app
   but none of these.

Already agreed in conversation (2026-11-07): the setup page is served by the
device, with no app. Location is entered as coordinates. The signing key lives
on the user's Mac with an offline backup, and signing is done locally. Updates
are checked at the element refresh and installed while the device is idle.
The user's board follows a `beta` channel and the gifts follow `stable`.
There is no gift date, and the units are "a handful".

## What was checked (2026-11-07, from devbox)

- **OTA support in the pinned crates (source read).**
  - `esp-bootloader-esp-idf` =0.6.0 has `OtaUpdater`. It needs an `otadata`
    partition and at least two OTA app slots, and provides `next_partition`,
    `activate_next_partition` and `set_current_ota_state`.
  - Its `OtaImageState` has `New` → `PendingVerify` → `Valid`/`Invalid`/
    `Aborted`. The crate's documentation says the bootloader changes `New` to
    `PendingVerify` **only if auto-rollback is enabled in the bootloader**.
  - `esp-storage` =0.10.0, already used by 0079, writes the slots.
- **The bootloader.** espflash 4.6.0 flashes its bundled ESP32-S3 bootloader
  unless given `--bootloader <file>`. Its
  [manifest](https://github.com/esp-rs/espflash/blob/main/espflash/resources/bootloaders/manifest.yaml)
  builds that bootloader from ESP-IDF `release/v6.1` with **no sdkconfig
  overrides for the S3**. ESP-IDF's
  `CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE` is off by default, so **the bundled
  bootloader picks a slot but never rolls back**. Gifts need a custom
  bootloader built from ESP-IDF with rollback enabled. Building it is a
  toolchain addition, which is a gate (see the decisions below). Without it, a
  new image that boots and then hangs or panics before marking itself bad
  would be booted forever.
- **Access-point Wi-Fi.** `esp-radio` =1.0.0-beta.1 has
  `Config::AccessPoint` and `Config::AccessPointStation`, so the device can run
  its own network and join another at the same time. Its WPA2 access point
  requires a non-empty password, and it can scan for networks
  (`scan_async`).
- **Update hosting.**
  - GitHub Release downloads redirect twice: `github.com` →
    `release-assets.githubusercontent.com`. The first is TLS 1.3/1.2 with an
    ECDSA leaf on a Sectigo E46 chain; the second has a Let's Encrypt RSA leaf.
    That's three hosts' worth of handshakes and long signed redirect URLs.
  - **GitHub Pages** (`chachoblow.github.io`, the repo is public) is one host
    and fixed paths, with no redirects. It negotiates TLS 1.2
    (ECDHE-RSA-CHACHA20-POLY1305, RSA-2048 leaf).
  - Its chain is leaf ← Let's Encrypt `YR1` ← `ISRG Root YR`, served
    cross-signed by `ISRG Root X1`.
  - Its HTTP/1.0 replies carry `Content-Length`, which matches the bounded
    parser of 0071.
- **Image size.** The current `overhead-s3-device` release ELF loads about
  1.39 MB (text 1,368,131 B and data 20,464 B; bss 648,972 B takes no flash).
- **Web-search leads, not yet verified against the pinned stack:**
  - `picoserve` 0.19 (an async `no_std` HTTP server; 0.x API, GET/POST/HEAD
    only).
  - `edge-net`'s `edge-dhcp` (DHCP server) and `edge-captive` (captive DNS),
    which target embassy-net.
  - `ed25519-dalek` (`no_std` with default features off, widely reviewed,
    roughly 40 kB of tables) and `salty` (smaller, built for
    microcontrollers).

## Proposed decisions

### A. Flash layout (fixed at the first gift)
All gift units use the same module as the current board, an ESP32-S3 N8R8
(8 MB flash, 8 MB PSRAM). One partition table, 4 KiB-aligned:

| Name | Type/subtype | Offset | Size | Purpose |
|---|---|---|---|---|
| nvs | data/nvs | 0x9000 | 0x4000 | kept for the radio stack; unused by our code |
| otadata | data/ota | 0xd000 | 0x2000 | which slot boots, and its state |
| phy_init | data/phy | 0xf000 | 0x1000 | unchanged |
| ota_0 | app/ota_0 | 0x10000 | 3 MiB | app slot A |
| ota_1 | app/ota_1 | 0x310000 | 3 MiB | app slot B |
| settings | data/undefined | 0x610000 | 64 KiB | B's settings, two slots |
| elements | data/undefined | 0x620000 | 320 KiB | 0079's two element slots, moved |
| spare | data/undefined | 0x670000 | 1,600 KiB | reserved: future data (for example a city list), never written by today's firmware |

- Each slot is about 2.2× today's image size. The factory slot goes; USB
  flashing writes `ota_0` and erases `otadata`.
- Moving `elements` clears the stored copy once on the first flash of this
  layout. 0079's format is otherwise unchanged.
- The `spare` partition matters because the table can't grow later.

### B. Settings store (format fixed; contents can grow)
- A versioned, CRC-checked record in `settings`, using 0079's pattern: two
  alternating slots, header written last, so a power cut during a save leaves
  the previous record intact.
- Fields today:
  - format version
  - Wi-Fi SSID and passphrase (one network; more later through new format
    versions)
  - observer latitude, longitude and optional altitude
  - update channel (`stable` by default)
  - setup-network password (random, made on first boot)
  - the last update version that failed its health check (so it isn't
    reinstalled)
- Future firmware must read every earlier format version (or migrate it).
- Values built into the image become **developer-only**. A `dev-settings`
  feature seeds an empty store from `firmware/.wifi.env`/`.site.env`. Release
  images are built without it and **contain no personal data**, because they
  are published.
- The passphrase is stored unencrypted in flash, readable by anyone who has
  the board. That is accepted for home gifts. Flash encryption and Secure Boot
  burn eFuses permanently and are **not** proposed.
- Replaces 0004 (build-time location) and the build-time credential parts of
  0002 and 0075.

### C. Signed updates (signing scheme and first key fixed)
- **What is signed.** A small JSON manifest per channel holds:
  - format version
  - channel
  - a monotonic `build` number
  - human version string
  - image URL path, length and SHA-256
  - minimum settings-format version
  - the ed25519 signature over the canonical manifest bytes
- **Checks on the device.**
  - The device verifies the signature against a public key compiled into the
    firmware.
  - It streams the image into the inactive slot, hashing as it goes, and never
    holds the whole image in RAM.
  - It activates the slot only if length and SHA-256 match.
  - It refuses a `build` that isn't greater than the running one, so an old
    signed image can't be replayed. Going back to old code means a new build
    number.
- **Keys.**
  - The private key is generated on and kept on the user's Mac, with an
    offline backup. It never goes to devbox, GitHub or the repo.
  - Firmware holds a small list of accepted public keys. Rotation: a release
    signed by the old key ships firmware that trusts the new one.
  - Losing the key means updates only over USB, so the backup is part of the
    release checklist.
- **Signing tool.** A `tools/` subcommand (`overhead-release`): it builds the
  manifest from an image, signs it with a key file given on the command line,
  and verifies a signed pair. It runs on the Mac.
- **Verification crate.** The same ed25519 crate verifies on the device and on
  the host. I recommend `ed25519-dalek` with default features off and
  `verify_strict`: it is the most reviewed, and its tables fit in flash. This
  is a dependency added with this reason given, per AGENTS.md.

### D. Hosting and delivery (manifest location fixed)
- **Host.** GitHub Pages for this repo: `https://chachoblow.github.io/overhead/`
  with `ota/<channel>/manifest.json` and `ota/images/<build>.bin`, published
  from a `pages` branch that holds only signed release files. It is one host,
  with no redirects or tokens, and it is static.
  - Moving the host later means shipping, under the old host, a release that
    points elsewhere. The manifest path is therefore one of the fixed items.
  - Publishing Pages is an outward-facing step and needs the user's approval
    when it happens.
- **Certificate roots.**
  - Ship several roots per host: for Pages, ISRG Root X1 and ISRG Root YR (and
    X2 if the chain allows); for CelesTrak, 0071's two Sectigo/USERTrust roots.
  - Root bundles are ordinary app content, renewed by updates.
  - The risk is a host changing CA while a device is offline for longer than a
    release cycle.
- **When to check.**
  - The device checks its channel manifest after each successful element
    refresh (every 8 hours, 0075), using the same TLS stack: one more fetch
    from a second host.
  - If an update is available, it installs when nobody has turned the knob or
    pressed BOOT for 2 minutes.
  - During the install it shows `UPDATING n%`, since flash erase and write
    disable interrupts in `esp-storage`, which pauses display and input.
  - It then resets.
- **Channels.** `stable` and `beta`. The user's board is set to `beta` (a
  setting), so each release runs on it first.

### E. Rollback and health (the bootloader is fixed at the first gift)
- **Custom bootloader.** It is built from ESP-IDF `release/v6.1` (matching
  espflash's own) with `CONFIG_BOOTLOADER_APP_ROLLBACK_ENABLE=y`. The binary
  is committed with its sdkconfig, IDF commit and hash, and is flashed with
  `--bootloader` for every runtime image.
- **Health check.** After an update the image boots as `PendingVerify` and
  marks itself `Valid` only after all of these, within 10 minutes:
  - a radar or `NO FRAME` frame drawn
  - Wi-Fi joined
  - trusted UTC
  - a catalogue loaded
  - the channel manifest fetched and its signature verified (so a build that
    breaks updating rolls back)
- **Failure.** If the check fails it marks itself `Invalid` and resets into
  the old slot. The old app records the failed `build` and doesn't reinstall
  it. A Wi-Fi outage at the wrong moment rolls back a good build, which is
  harmless: it retries with the next build.
- **Required alongside.**
  - The device app **resets on panic** instead of halting (today it halts,
    0076).
  - A **hardware watchdog** resets a hung loop.
  - Without both, a stuck new image never reaches the bootloader again to
    roll back.
- A one-off **brick drill** before the first gift: a release that panics, one
  that hangs, one that can't reach the manifest, one with a bad signature, one
  cut off mid-download, and a power cut mid-install. All must leave the board
  on a working image.

### F. Setup page (behavior; the look is reviewed later)
- **When setup starts.** The device enters setup mode when it has no stored
  Wi-Fi, when it can't join for 5 minutes after boot, or when the knob switch
  (IO6) is held for 3 s at boot.
- **On screen.** It runs WPA2 network `OVERHEAD-xxxx` (last four MAC hex
  digits) with its stored random password, alongside its own station
  connection (`AccessPointStation`). The Sharp shows the name, the password
  and a Wi-Fi join QR code.
- **Captive portal.** A captive DNS answers every name with the device's
  address, a small DHCP server hands out addresses, and an HTTP/1.0 server
  serves one page from flash.
- **The page.**
  - A network list from a scan, plus a passphrase field.
  - Coordinates as one field (`lat, lon`, as copied from a maps app) and an
    optional altitude.
  - Read-only status: firmware build, channel, MAC, data age, last fetch,
    update result.
- **Saving.**
  - Before saving, the device tests the new network while staying in setup
    mode. Success or the error shows on the page **and** on the Sharp.
  - Then it saves, leaves setup mode, prepares the coastline for the new
    location (0057/0058) and runs.
- **Recovery.**
  - Factory reset: hold the knob switch for 10 s at boot. It clears settings
    and stored elements, then enters setup.
  - The info page is also reachable by a short knob press, for the recipient
    to photograph.
- **Dependencies.** Decided in its work item. My lean: hand-write the HTTP
  server (one form, like 0071's hand-written client) and the captive DNS
  (answer every A query). Use `edge-dhcp` unless a minimal DHCP responder
  proves simpler.
- **Not proposed:** serving the page on the home network (`overhead.local`),
  the knob menu, a city list, IP geolocation, and phone GPS (the browser
  blocks location on plain HTTP).

## Proposed milestone and work items

**M7 — Giftable device** (new PLAN milestone, after M6's open items).
**Done when:** a device set up only through the setup page on a phone that
isn't the user's tracks at its recipient's coordinates, and receives a signed
`stable` release with no hands-on step. The brick drill has passed on the
user's board.

Work items, in order:
1. **Signed OTA with rollback on the user's board.** This item covers:
   - Layout A
   - the custom bootloader (E)
   - signing tool and manifest (C)
   - Pages hosting (D)
   - reset on panic plus watchdog
   - health check
   - the brick drill

   It ends with a board that updates itself from `beta`. It comes first
   because every later fix reaches the gifts through it.
2. **Settings store and setup page.** B and F, release images with no
   personal data, factory reset and the info screen. It ends with the board
   set up only through the phone, then updated over the air with its settings
   intact.
3. **Gift readiness.** A soak test of a release image set up only through the
   page, setup on a phone that isn't the user's, the release and recovery
   checklist (key backup, USB recovery flash), and flashing the gift units.

M6's open items (BOOT risks, knob switch role) stay in M6; F gives the knob
switch its first role. The `NO FRAME` of 2026-11-06 is still an open concern;
a recurrence on any board blocks gift readiness.

## Decisions on acceptance

- 0083 Flash layout for updatable devices (A)
- 0084 Settings store; built-in personal values are developer-only (B;
  replaces 0004, amends 0002/0075)
- 0085 Signed updates and release keys (C, D)
- 0086 Bootloader rollback and the health check (E; amends 0076's
  halt-on-panic)
- 0087 Phone setup page (F)

## Gates for you

1. **Accept A–F** as written, or say what to change.
2. **Build the custom bootloader with ESP-IDF.** This is a toolchain addition,
   which needs your approval. My recommendation: build it once on devbox in
   rootless Docker from Espressif's official `espressif/idf` image pinned to
   the v6.1 release, with nothing installed on the host, and commit the
   binary, sdkconfig and hash. The alternative is installing ESP-IDF on your
   Mac.
3. **Turn on GitHub Pages** for `chachoblow/overhead` from a `pages` branch.
   This is outward-facing, and I'd ask again just before doing it.
4. **Key generation.** I write the tool. You run it on the Mac to make the key
   and its backup, and only the public key comes back into the repo.

Nothing above changes the toolchain, touches credentials or publishes anything
until the matching gate is answered.
