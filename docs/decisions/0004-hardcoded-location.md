# 0004 — Hardcoded location for now

Date: 2026-10-04
Status: accepted (interim — expect a successor decision)

## Decision
The observer location is a configured latitude/longitude constant: a config
value in the simulator, a compile-time constant in the first firmware. A real
location source (settings UI, GPS, IP geolocation) is deferred.

## Why
- Scope: display and graphics come first; a location UI earns nothing until
  the screens look right.
- The device will mostly live at one place (home); a constant is correct in
  practice.
- Precision barely matters: anything within ~50 km produces visually
  identical passes.
- GPS rejected for now (hardware, power, poor indoors). IP geolocation and a
  settings UI remain candidates for the successor decision.

## Consequences
- Changing location means reflashing (or editing sim config).
- core/render must still take location as an input, not a constant — only the
  source is hardcoded, keeping the successor decision cheap.
