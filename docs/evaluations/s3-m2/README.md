# Frozen M2 target capture archive

Captured 2026-10-11 UTC with explicit approval, using the exact normal images in
`firmware/target/m2-normal-implementation-final-2/`. Results/interpretation belong
in [s3-m2.md](../s3-m2.md); method/commands in
[firmware/M2_CAPTURE.md](../../../firmware/M2_CAPTURE.md).

## Contents and provenance

- `measure-boot-{1,2}.log`: original serial/monitor bytes; 20 cases × three timed
  samples per boot after one warmup per case.
- `diagnostic-boot-{1,2}.log`: original serial/monitor bytes; 75 numerical checks
  plus one observation of each case for each stack pattern per boot, not timings.
- `*-flash.log`, `*-capture.log`, `runner.log`: actual command output, including
  successful validations. `*.log.json` are original pre-reset gate/timeout
  sidecars; their attempt status is deliberately not rewritten as acceptance.
- `artifacts.json`: complete build/source/image/section/frame inventories, hashes,
  command start/end times, whole-set validation result and exact boot-segment
  review. `build-commands.log` preserves the normal build/image gate commands.
- `results-samples.csv`: all 120 timed samples; microseconds and absolute memory
  counters, blank aggregation fields only for expected failed-document case 16.
- `results-stack.csv`: all 252 phase observations, retaining boot, pattern,
  absolute stack/paint bounds, untouched margin and observed depth.
- `results-summary.json`: unfiltered min/median/max over six samples per case,
  independently measured pipeline peaks and repeated collection observations.
- `results-summary-inputs.json`: original input-byte hashes and reporting-helper
  fingerprint. These generated tables are not a second validation oracle.

The nested `.gitattributes` preserves raw CRLF/bootloader whitespace without
changing the frozen root attributes. Git-filtered and raw blob hashes agree.
No raw log has been normalized, trimmed or relabeled as a different run.
Both images were flashed once; each capture command separately reset the board.
No failed capture attempt, retry, timing outlier removal, firmware rebuild or heap
change occurred.

Exact ELF/images, full disassembly/symbol text, the runner and capture-start source
tarball remain locally in `firmware/target/m2-capture-20261011T0423/`; hashes are
retained here, but ignored local files are **not a durable binary archive**. The
source snapshot predates the reporting helper, whose own fingerprint is separate.
The board was left on the diagnostic image, with no monitor process running.

## Offline validation and table regeneration

Use the matching frozen preparation source revision; do not renew the manifest
to make old evidence pass after unrelated source changes. From repository root:

```sh
python3 firmware/m2_capture.py validate-set \
  --measure docs/evaluations/s3-m2/measure-boot-1.log docs/evaluations/s3-m2/measure-boot-2.log \
  --diagnostic docs/evaluations/s3-m2/diagnostic-boot-1.log docs/evaluations/s3-m2/diagnostic-boot-2.log
python3 tools/summarize_m2_capture.py \
  --measure docs/evaluations/s3-m2/measure-boot-1.log docs/evaluations/s3-m2/measure-boot-2.log \
  --diagnostic docs/evaluations/s3-m2/diagnostic-boot-1.log docs/evaluations/s3-m2/diagnostic-boot-2.log \
  --output-prefix docs/evaluations/s3-m2/results --check
python3 -m unittest discover -s tools -p 'test_summarize_m2_capture.py'
```

Without `--check`, choose a new output prefix; existing evidence is never overwritten.
The helper validates the complete four-log set before extracting any results. It
uses only the Python standard library and never accesses hardware. Synthetic
reporting tests use invented counters, not captured measurements. Boot/build/reset
provenance and numerical/stack limitations still require the linked reviews.
