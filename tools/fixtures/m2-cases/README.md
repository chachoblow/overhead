# Candidate M2 case documents

Offline inputs for the [bounded preparation contract](../../../docs/evaluations/m2-next-evidence.md).
**Candidate host evidence, not a frozen capture manifest.**
[Results and remaining gates](../../../docs/evaluations/m2-host-preflight.md).
No historical fixture, source snapshot, element epoch, or production policy is refreshed.

## Provenance and exact representation

[inputs.json](inputs.json) archives ten named document strings with UTF-8 byte
lengths, SHA-256 hashes, record counts, and ingestion-order IDs/epochs. Decode each
`text` JSON string to obtain the **exact case document bytes**; the surrounding
archive is not an ingestion input. Each case's group table, bytes/hashes, accepted
identities/provenance, UTC/site, and work are in the
[host report](../../../docs/evaluations/m2-host-preflight.json).
Groups use zero-based indices; source/name strings remain outside measured storage.

- `historical`: unmodified `catalogue-costs.tle`, parsed by sgp4 and serialized
  directly from typed `Elements`, exactly as the historical host/firmware helper.
  Original [provenance](../README.md#catalogue-coststle) and
  `2006-06-26T00:00:00Z` are unchanged. This document is 3,331 bytes; SHA-256
  `2e18244cfec8152122bd7ca4575513862f429ad7f7f4c2ea58e856abf8895972`.
- `mixed-5/9/12/16`, `leo-heavy`, `deep-space-heavy`: raw objects from the separate
  [pinned pool](../m2-pool/README.md), copied without numeric reserialization.
  Subsets keep its `[`/LF, comma/LF, LF/`]`/LF representation and prescribed order.
  They use `2026-10-10T00:00:00Z`, not the historical UTC.
- `conflict`: two explicitly synthetic copies of historical NORAD 6251: epoch
  minus one day, then original epoch with mean anomaly +1° modulo 360. Typed
  element serialization avoids changing unrelated orbital fields through a
  generic JSON-value round trip. Case 14 adds this as group 1 after the originals.
- `invalid`: synthetic original 6251 with `CENTER_NAME=MARS`, then a copy with
  eccentricity 1. No other fields are intentionally altered. Case 15 adds group 1.
- `malformed`: the single byte `[`. Record count/order are `null` (document is
  unparseable), not an empty valid array. Case 16 counts the eight valid group-0
  records plus this byte; no complete record is supplied by malformed group 1.

Case 13 references `historical` four times: all 32 input records and all 13,324
bytes count, not eight records/3,331 bytes. RAM cases reuse their control's exact
bytes; they are not additional source populations. Case names and source metadata
are descriptive external tables, not dynamically allocated URLs measured here.
The final target metadata/numerical/capture annex remains to be frozen.

## Offline regeneration

A fresh session starts with `docs/HANDOFF.md`, `docs/PLAN.md`, and `docs/DESIGN.md`.
The numerical-reference/manifest-freeze work listed there remains open; running
these candidate gates does not authorize target implementation or flashing.

From the repo root with the existing Rust toolchain/cached dependencies, Python
3.11+ (standard library only), and Homebrew SDL2 (linker path in `.cargo/config.toml`):

```sh
cargo check --offline --locked --workspace
cargo test --offline --locked --workspace
cargo test --offline --locked --workspace --all-features
cargo check --offline --locked -p overhead-core --no-default-features
cargo clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
python3 tools/prepare_m2_pool.py --check
python3 tools/prepare_m2_cases.py --check
python3 -m unittest discover -s tools -p 'test_prepare_m2*.py'
cargo run --offline --release --locked -p overhead-tools --bin overhead-m2-preflight \
  > /tmp/m2-host-preflight.json
cmp /tmp/m2-host-preflight.json docs/evaluations/m2-host-preflight.json
cargo test --offline --locked -p overhead-tools --bin overhead-m2-preflight --test m2_preflight
```

The separate firmware host regression checks also run from the **repo root**,
using the already-installed stable host toolchain, not the target configuration:

```sh
cargo +stable test --offline --locked --manifest-path firmware/Cargo.toml --lib
cargo +stable test --offline --locked --manifest-path firmware/Cargo.toml --lib --features catalogue-memory
cargo +stable clippy --offline --locked --manifest-path firmware/Cargo.toml --lib --tests -- -D warnings
cargo +stable clippy --offline --locked --manifest-path firmware/Cargo.toml --lib --tests --features catalogue-memory -- -D warnings
python3 -m unittest discover -s firmware -p 'test_*.py'
```

All commands above were verified from a clean staged-source export with no ignored
artifacts. They do not build/flash target images, refresh snapshots, install a
toolchain, or require previously captured hardware logs. Offline dependency caches
and SDL2 are prerequisites, not bundled in Git. Exact report comparison uses the
published Darwin arm64/compiler environment; other hosts still run semantic gates.

`prepare_m2_cases.py` invokes the Rust tool's `--inputs` mode, which uses only
checked-in historical/pool sources. It hashes the resulting document bytes.
Omit `--check` only for deliberate reconstruction/review; the normal runner rejects
any mismatch between generated and archived text. No network or ignored firmware
artifact is needed. Host collection/size observations depend on compiler/architecture;
portable integration tests always check inputs/work/diagnostics, and compare exact
memory evidence only on the published OS/architecture. Changed evidence requires
review before any eventual manifest freeze, not a silent expected-output refresh.
