# 0017 — Bounded target catalogue memory experiment

Date: 2026-10-10
Status: accepted

## Decision
Prepare a separate feature-gated S3 experiment for checked catalogue initialization
and retained pass aggregation, using a fixed internal heap and CPU0 written-stack
probe. This is measurement infrastructure, not a device allocator, capacity, or
scheduling decision. Target captures are still required.

## Why
Host requested bytes cannot establish target RAM use. Reusing the same historical
4/8-object inputs and work matrix isolates architecture/allocator differences
before broadening samples. Flash-resident OMM makes input storage lifetime explicit;
it deliberately does not claim to measure the future network ingestion path.

Use the maintained `esp-alloc` 0.11.0 LLFF backend rather than hand-writing an
allocator. Disable C exports/implicit global allocation, bound its internal region
to 64 KiB, and instrument requested bytes separately from backend occupancy.
Allocate-copy-free growth must include both live blocks in the measured peak.
Keep the default kernel benchmark allocation-free and the host workspace isolated.

A guarded, no-call paint loop avoids overwriting live CPU0 frames or introducing
second-core scheduling solely for stack measurement. Scan only after work returns;
require a known-local smoke check and untouched guard margin. It measures observed
writes including harness overhead, not maximum reserved stack: unwritten regions
and pattern collisions prevent a safety/worst-case claim.

## Consequences
No PSRAM, display, network, new toolchain, asynchronous predictor, production
cadence, supported catalogue size, or graceful memory-pressure policy is selected.
OOM/panic/probe failures or inconsistent/released-storage checks invalidate a run;
incomplete logs are evidence of failure, never completed measurements. Hardware
flashing still requires approval.

The fixed sample population remains narrow; independent target numerical checks,
density/collection-growth cases, and broader operating evidence remain M2 work.
Implementation/tests own the protocol. Input lifetimes, counter interpretation,
build/capture commands and API references belong in
[the experiment guide](../../firmware/CATALOGUE_MEMORY.md).
