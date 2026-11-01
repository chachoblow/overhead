# 0073 — Run the fetch diagnostic without certificate date checks

Date: 2026-11-01
Status: accepted (user, 2026-11-01). Amends item 4 of
[0071](0071-https-element-fetch-first-slice.md) for the diagnostic only.

## Decision
The display-free `live-fetch` diagnostic may run on the board with MbedTLS's
default profile: chain, signatures and hostname are verified against the two
bundled roots, but certificate `notBefore`/`notAfter` dates are not checked.
Before fetched elements reach the radar or any non-diagnostic image, date
checking must be revisited in its own decision.

## Why
0071 planned to give MbedTLS screened UTC as its wall clock. Slice 2 found that
`mbedtls-rs-sys`'s `hook-wall-clock` disables its prebuilt library and forces
an on-the-fly C build needing `clang`, `cmake` and `ninja`, which is a
toolchain and devbox change. Option (2), installing those tools, was not
chosen for now. A diagnostic that only measures handshake, transfer and memory
does not need date checks; the main risk, an expired or not-yet-valid
certificate, matters once data drives the display.

## Consequences
Diagnostic fetch evidence says nothing about validity-date enforcement. The
pinned roots still bound who can serve the data. Trusted UTC is still obtained
before fetching, so the device remains ready to supply a wall clock later.
