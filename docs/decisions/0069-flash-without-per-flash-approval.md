# 0069 — Flash without per-flash approval

Date: 2026-10-31
Status: accepted

## Decision

Agents may build, flash, reset and capture UART from the S3 without asking first.
The approval requirement in earlier decisions (0017, 0018, 0019, 0055, 0062) and
in the session docs no longer applies. Changing toolchains still requires asking.

## Why

The user removed the rule on 2026-10-31 when moving development to devbox. Asking
before every flash slowed the build, flash and capture loop. The technical
safeguards, not the approval step, are what protect the evidence.

## Consequences

The checks around each flash stay: confirm the UART port and board identity, run
the image's offline exact gate where one exists, and record what the board now
runs. Physical/visual acceptance still comes from the user observing the panel.
Frozen M2 evidence stays isolated and is never refreshed to hide drift, and
credential-bearing artifacts are still never committed.
