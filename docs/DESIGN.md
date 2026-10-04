# Overhead — Design

What the device is and how it should feel. Update this when intent changes;
record firm choices in docs/decisions/.

## What it is
A handheld device showing satellites passing over a set location in real
time. Modeled on Nathan Matsuda's flight-tracking device, but for satellites.

## Interface
- Circular radar view centered on a set location, with a readout panel
  beside it.
- Zoom from a small local radius out to wider than Earth.
- Feel: changes quickly, realtime, with good motion graphics, in the vein of
  Matsuda's interfaces.
- Fast satellites are interpolated across the screen with a minimum
  on-screen traversal time; anything faster is visually slowed down.
- A circular display would be nice but isn't required.

### Readout panel (intent, expected to evolve on screen)
The 160×240 px column right of the radar. Stateful but not paginated — it
reflects what the radar shows; no menus or pages.
- Header: local time, and the age of the orbit data.
- Selected satellite (auto-highlighted until a knob exists): name, altitude,
  range, elevation, direction of travel. Readouts always use the true
  position, never the visually slowed marker.
- Next pass: name and countdown for the next satellite overhead — carries
  the view when the local sky is empty.
- Zoom scale: current view radius (e.g. "R 500 KM").
Not included: pass lists, radio info, battery indicator.

### Zoom control before the knob (intent)
- Simulator: scroll wheel and up/down arrows drive the continuous zoom; the
  wheel approximates the eventual knob's feel.
- Device, pre-knob: the DevKit's BOOT button steps through ~4 zoom presets —
  just enough to judge the panel at different scales during bring-up.
- Auto-zoom "demo mode" (device frames the next pass on its own) is shelved
  as a possible later feature alongside the knob, not instead of it.

## Hardware
- ESP32-S3-DevKitC-1-N8R8
- Adafruit 2.7" Sharp Memory Display (#4694), 400×240, 1-bit

## Settled
- Positions computed on-device with SGP4 from CelesTrak OMM data
  (decisions/0001).
- Orbit data fetched over Wi-Fi ~every 8 h, clock set via SNTP on each
  connection; no RTC for now (decisions/0002).
- Default catalogue: a curated set from CelesTrak groups, hundreds of
  objects, sized by the SGP4 benchmark; Starlink layer deferred
  (decisions/0003).
- Location: configured constant for now; real source deferred
  (decisions/0004).

## Scope for now
The satellite display and graphics are the product focus. Implementation
starts with a tested headless calculation engine, then builds the UI on real
data (decisions/0005). Physical controls (knob) and enclosure come later.

## History
Rewrite of an earlier project, "Satellite notifier": ESP32 driving an LED
matrix, with satellite positions fetched from an API. Most of its code and
output design will be replaced.

## Open questions
None right now.
