# 2026-10-24 prepared/cached coastline layer S3 capture

[Results and limits](../m4-coastline-prepared-target.md). Provenance and
reproduction instructions, **not permission for another build or flash**.
The board was left running the prepared-layer measurement image.

## Retained evidence

- `capture-on-1.log` / `capture-on-2.log`: unmodified monitor bytes from two
  complete boots of the same flashed image (`.gitattributes` keeps CRLF).
- `host-expected.csv`: host oracle (checksums and write counts per case/mode)
  from `firmware/examples/prepared_expected.rs`, same source revision.
- `summary.json`: output of `validate.py` over both captures.
- `preflash-*-gate.json`, `flash-on.log`: exact image gates and the flash log.

An earlier boot of a previous build (before `CoastLayer::prepare_for`) is not
retained here: its adapter shared an 11 KB temporary across modes and inflated
every stack figure. Its timings and exactness agreed with the retained run.

## Images

| | Source | ELF SHA-256 | Application image SHA-256 |
| --- | --- | --- | --- |
| Off control (`coast-representation`, size baseline only; not flashed) | `6dae5b9` | `185c88dd…560b6b` | `21f36b34…343b9f` |
| On (`coast-prepared`) | `a8b4be2` | `7e8ce5d8bf474d5f0b2d0ccdd0f94276bb8a92647df1f27224674c13d21fb1cd` | `9c118395a696b99a274b31ac0ba21f86dc0c307fd9d79083ead35def06126431` |

The off image is coast-free; the later in-place-preparation change does not
touch its linked code. Toolchain unchanged: Xtensa Rust 1.97.0.0, espflash 4.6.0.
Board: ESP32-S3 revision v0.2, 8 MB flash, MAC `80:45:6b:2a:45:08`.

## Offline audit — no hardware

```sh
python3 docs/evaluations/m4-coastline-prepared-s3/validate.py
cargo +stable run --release --locked --manifest-path firmware/Cargo.toml \
  --example prepared_expected --features coast-prepared > /tmp/prepared-host.csv
cmp /tmp/prepared-host.csv docs/evaluations/m4-coastline-prepared-s3/host-expected.csv
```

## Target commands used — new execution requires approval

From `firmware/` with `. "$HOME/export-esp.sh"`:

```sh
A="$PWD/target/coast-prepared-20261024"
BENCH_SUITE=baseline BENCH_SAMPLES=5 CARGO_TARGET_DIR="$A/build-on" \
  cargo build --release --locked --bin overhead-s3-coast-prepared --features coast-prepared
E="$A/build-on/xtensa-esp32s3-none-elf/release/overhead-s3-coast-prepared"
espflash save-image --chip esp32s3 --flash-size 8mb --flash-mode dio --flash-freq 40mhz \
  --skip-update-check "$E" "$A/on.bin"
python3 image_gate.py "$E" "$A/on.bin"
espflash flash --port <UART> --chip esp32s3 --flash-size 8mb --flash-mode dio \
  --flash-freq 40mhz --non-interactive --skip-update-check "$E"
python3 representation_capture.py <UART> "$E" "$A/capture-on-1.log" \
  --timeout 300 --marker OVERHEAD_S3_COAST_PREPARED
```

The off control used `--bin overhead-s3-coast-representation --features
coast-representation` into `$A/build-off`. The ignored local archive
`firmware/target/coast-prepared-20261024/` keeps ELFs, images and build logs.
