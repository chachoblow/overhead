# 2026-10-24 single coastline representation feasibility capture

[Results, recommendation, method and limits](../m4-coastline-representation-target.md).
The one separately authorized target comparison is complete. These commands are
provenance/reproduction instructions, **not permission for another build or flash**.
The board was left on the candidate-on measurement image. No Sharp operation,
toolchain change or frozen-M2 evidence update occurred.

## Retained evidence

- `capture-off.log` / `capture-on.log`: unmodified espflash monitor bytes from one
  complete boot per image, including full 1-bit framebuffer dumps. `.gitattributes`
  preserves CRLF/spacing. SHA-256:
  - off: `7f3900092d857f1b6181b4584f7e02b37be392aa7f35ac4313780527732a487c`
  - on: `d0d53f03298ad0e413e1a54e1bdf8e0869c512de1cfbe2c9534a4332eafa4ef2`
- `host-expected.csv` / `host-masks.zip`: external host draw counts/checksums and
  37 losslessly stored packed 12,000-byte framebuffers, one per case/mode. Each row
  is 50 bytes, bit `x % 8` in byte `y*50 + x/8`; 1 means ink. These came from the
  actual shared app/measurement fixture, not target warm-up values or a screenshot.
- `host-extraction.txt`: fresh release audit after no_std/target extraction. Its
  deterministic counts/shape metrics and every retained review-input hash matched
  the original accepted host trial; timings are not target evidence.
- `summary.json`: generated from the raw captures plus host oracle by
  `validate_capture.py`. The default audit is independent of evolving Rust source;
  it does not refresh/re-pin any historical freeze.
- `build-report.json`: settings, exact ELF/image hashes, allocated-section deltas,
  asset exclusion checks and selected compiled stack-frame reservations.
  `compiled-frames-*.txt` retain the actual disassembly headers/prologues.
- `preflash-*-gate.json` and `flash-*.log`: exact image gate results and flash logs.
  `toolchain.txt` records the unchanged installed compiler/flasher.
- `source-hashes.json`: compiled-source/fixture inventory at build time. This is
  provenance, not a new immutable-source requirement on future workspace tests.

The base revision was `33fcf43fc467fa0936972fac782bf8737d377809` plus **uncommitted**
changes. The ignored local archive `firmware/target/coast-representation-20261024/`
retains the full ELFs, application images, build/save logs, sections/symbols/full
disassembly, board-info, tracked `source.patch`, and `source-extra/` copies of the
untracked render/firmware code/asset. The patch hash is in `build-report.json`.
Those large/local artifacts are not published here. A later rebuild can have
different image hashes due to metadata/layout; it requires fresh gates.

Exact image identities:

| | ELF SHA-256 | Application image SHA-256 |
| --- | --- | --- |
| Off | `e90530be007601fa2fe2626a915f3c78520510bdea2e8cfd1d78ac17dd7e51f8` | `782270125301ad366736ed646e0d1e689ec5cfb1a0bcd808cb5aa5eb3e043d15` |
| On | `8be11ae7b6587c95aa0411a865deda6a7475ca57fa6d785f5d5976a1d31f8ffa` | `8ed3d2a10106fe6b761ff78eef4d211316ff1c1b75cc906e104f09225f742495` |

## Offline audit — no hardware

From repository root:

```sh
python3 docs/evaluations/m4-coastline-representation-s3/validate_capture.py
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s docs/evaluations/m4-coastline-representation-s3 -p 'test_validate_capture.py'
python3 docs/evaluations/m4-coastline-representation-spike/validate_spike.py \
  docs/evaluations/m4-coastline-representation-spike/host-run2.txt \
  docs/evaluations/m4-coastline-representation-s3/host-extraction.txt
```

The audit rejects missing/duplicate/reordered records, wrong cases/settings,
failed/incomplete runs, malformed/incomplete masks, non-repeatable controls and
host/target byte/count mismatches. All 240 rows and 48 mask dumps pass. Six negative
control tests cover malformed completion, samples, masks, settings/rollback and
an incorrect external oracle. Exact byte agreement is not an independent geography
oracle, full-view proof, physical panel test or stack-safety proof.

To recreate the host firmware oracle, use a **fresh** output directory:

```sh
cargo +stable run --release --locked --manifest-path firmware/Cargo.toml \
  --example representation_expected --features coast-representation-on \
  -- /tmp/coast-representation-host-masks > /tmp/coast-representation-host-expected.csv
```

This is a host executable, not a target build. The fixture
[README](../../../firmware/fixtures/README.md) owns packed-asset regeneration.

## Exact target commands used — new execution requires approval

Build from `firmware/`, with the installed toolchain and a fresh archive directory.
For off/on respectively use features `coast-representation` /
`coast-representation-on`, otherwise identical settings:

```sh
. "$HOME/export-esp.sh"
cd firmware
ARCHIVE="$PWD/target/coast-representation-20261024"
SIDE=off                 # on for the second image
FEATURE=coast-representation  # coast-representation-on for the second image
BENCH_SUITE=baseline BENCH_SAMPLES=5 CARGO_TARGET_DIR="$ARCHIVE/build-$SIDE" \
  cargo build --release --locked --bin overhead-s3-coast-representation --features "$FEATURE"
ELF="$ARCHIVE/build-$SIDE/xtensa-esp32s3-none-elf/release/overhead-s3-coast-representation"
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio --flash-freq 40mhz \
  --skip-update-check "$ELF" "$ARCHIVE/$SIDE.bin"
python3 image_gate.py "$ELF" "$ARCHIVE/$SIDE.bin"
```

Section/symbol/disassembly inspection used `xtensa-esp32-elf-size -A`, `nm -C -S`
and `objdump -d -C`. Both exact images passed the gate before flash. The actual
current UART was confirmed through `espflash list-ports` and `board-info`, not
assumed from an older port name. After those approved checks:

```sh
# Gate again immediately before the approved flash; never rebuild in between.
python3 image_gate.py "$ELF" "$ARCHIVE/$SIDE.bin"
espflash flash --port /dev/cu.usbserial-210 --chip esp32s3 \
  --flash-size 8mb --flash-mode dio --flash-freq 40mhz \
  --non-interactive --skip-update-check "$ELF"
python3 representation_capture.py /dev/cu.usbserial-210 "$ELF" \
  "$ARCHIVE/capture-$SIDE.log" --timeout 240
```

The capture helper resets/monitors but never flashes; it refuses to overwrite and
retains partial logs on failure/timeout. DONE detection is not the full audit.
Flash writes bootloader/partition/application and replaces the prior app. Raw masks
and reporting are untimed; captured monitor durations were 34.2 s off / 111.5 s on,
not rendering latency. No additional target run or alternative build followed.
