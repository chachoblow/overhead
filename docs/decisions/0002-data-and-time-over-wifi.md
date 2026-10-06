# 0002 — Orbit data and time over Wi-Fi

Date: 2026-10-04
Status: accepted

## Decision
The device fetches OMM orbital elements from CelesTrak over Wi-Fi roughly
every 8 hours (and on boot when stored data is older than that), and syncs
its clock via SNTP on each connection. No battery-backed RTC for now.

## Why
- The S3 has Wi-Fi built in; direct fetch is the lowest-friction path.
- Elements stay display-accurate for days; ~3 fetches/day is well within
  CelesTrak's usage guidance.
- Position accuracy depends on the clock as much as the elements; SNTP on an
  existing connection is free. The S3 has no battery-backed RTC, so time is
  lost on power-off.
- Sideloading (USB/phone) was considered and rejected as primary: more
  friction for no benefit while Wi-Fi exists. An RTC module was deferred, not
  rejected.

## Consequences
- A freshly powered-on device needs Wi-Fi once before positions are accurate.
- Firmware needs Wi-Fi credentials, HTTPS, and SNTP; elements and fetch
  timestamp persist in flash.
- The planned simulator uses host time and a checked-in OMM snapshot.
- Adding an RTC later remains open if offline boot matters.
