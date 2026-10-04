# 0003 — Curated satellite catalogue by default

Date: 2026-10-04
Status: accepted

## Decision
The device tracks a curated set of satellites assembled from CelesTrak's
published groups (e.g. brightest, stations, weather, GNSS) — on the order of
hundreds of objects. The exact size is set by the SGP4 benchmark. A
full-catalogue / Starlink layer is a possible later addition, not a
commitment.

## Why
- Compute: SGP4 per satellite per update is expensive on the S3; the full
  catalogue (10,000+, mostly Starlink) likely exceeds the budget.
- Legibility: 10,000 dots in a 240 px circle is noise; most are visually
  identical Starlink units.
- CelesTrak groups make curation free — no hand-maintained ID list.
- "Show everything with aggregate rendering" was considered and set aside: it
  bets the design on unverified performance.

## Consequences
- Every on-screen object is nameable; the readout panel stays meaningful.
- The fetch path (0002) downloads a handful of group files, not the catalogue.
- Wide zoom will look sparse compared to reality; the "crowded sky" picture
  is deferred with the optional Starlink layer.
- Which specific groups to include is a tuning question, not a decision.
