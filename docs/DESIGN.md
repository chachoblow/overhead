# Overhead — Design

A handheld, realtime satellite radar for a set location, inspired by Nathan
Matsuda's flight-tracking interfaces. A rewrite of “Satellite notifier,” which
used an LED matrix and a position API.

## Hardware
- ESP32-S3-DevKitC-1-N8R8: 8 MB flash, 8 MB PSRAM.
- Adafruit 2.7" Sharp Memory Display (#4694): 400×240, 1-bit; target 20 Hz.
  Handle the panel/ribbon gently during bring-up.

## Interface intent
A location-centered circular radar with a 160×240 readout column to its right.
Zoom ranges from local to wider than Earth. Motion should feel fluid and lively;
fast markers have a minimum on-screen traversal time. **Readouts always use
true positions, never visually slowed markers.**

The panel reflects the radar, with no menus or pages:
- Local time and orbit-data age.
- Auto-selected satellite: name, altitude, range, elevation, travel direction.
- Next satellite overhead and countdown, especially when the local sky is empty.
- View radius (e.g. `R 500 KM`).

No pass lists, radio information, or battery indicator. Layout, selection, and
projection policies should evolve with simulator feedback.

## Controls
- Simulator: continuous zoom via scroll wheel and up/down arrows.
- Initial device: BOOT button cycles through roughly four zoom presets.
- Knob and optional auto-zoom demo mode come later; neither blocks the prototype.

## Product choices
[On-device SGP4](decisions/0001-on-device-sgp4.md),
[Wi-Fi elements/SNTP](decisions/0002-data-and-time-over-wifi.md),
[curated catalogue](decisions/0003-curated-catalogue.md), and
[configured location](decisions/0004-hardcoded-location.md) are settled;
follow those records for rationale and constraints. Implementation order and
deferred scope belong to [PLAN.md](PLAN.md).
