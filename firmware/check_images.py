#!/usr/bin/env python3
"""Build/check both S3 binaries and linker alignment fixtures, without hardware.

Run after sourcing ~/export-esp.sh. Outputs go in a NEW directory; stress
artifacts are linker tests, never measurement or flash candidates.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

from image_gate import GateError, elf_sections, image_segments, region, require, validate


ROOT = Path(__file__).resolve().parent


def run(command, directory, env, log):
    print("+", " ".join(map(str, command)), flush=True)
    result = subprocess.run(list(map(str, command)), cwd=directory, env=env,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    with log.open("a") as output:
        output.write("+ " + " ".join(map(str, command)) + "\n" + result.stdout)
    if result.returncode:
        raise RuntimeError(f"command failed ({result.returncode}); see {log}")
    return result.stdout.strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="new artifact directory (ignored target/ recommended)")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    log = output / "commands.log"
    env = dict(os.environ, BENCH_SUITE="baseline", BENCH_SAMPLES="3")
    # Separate outputs prevent stress builds from replacing measurement ELFs.
    env["CARGO_TARGET_DIR"] = str(output / "cargo-target")
    results = {
        "scope": "offline only; stress/fixture artifacts are not flash candidates",
        "complete": False,
        "tools": {tool: run([tool, "--version"], ROOT, env, log) for tool in
                  ("rustc", "cargo", "espflash", "xtensa-esp32s3-elf-as", "xtensa-esp32s3-elf-ld")},
        "source_sha256": {name: hashlib.sha256((ROOT / name).read_bytes()).hexdigest()
                          for name in ("rodata.x", "Cargo.lock", ".cargo/config.toml",
                                       "build.rs", "image_gate.py", "check_images.py")},
        "cases": {},
    }

    def gate(name, elf, alignment=None):
        image = output / f"{name}.bin"
        run(["espflash", "save-image", "--chip", "esp32s3", "--flash-size", "8mb",
             "--flash-mode", "dio", "--flash-freq", "40mhz", "--skip-update-check",
             elf, image], ROOT, env, log)
        result = validate(elf.read_bytes(), image.read_bytes(), expected_alignment=alignment)
        results["cases"][name] = result
        (output / "report.json").write_text(json.dumps(results, indent=2) + "\n")
        print(f"PASS {name}: single DROM; {result['checked_elf_bytes']} ELF bytes checked", flush=True)

    # A retained input object forces large alignment in the actual binaries,
    # without adding a production feature or changing their Rust sources.
    stress_source = output / "stress.S"
    stress_source.write_text('.section .rodata.overhead_image_stress,"a",@progbits\n'
                             '.balign 65536\n.global overhead_image_stress\n'
                             'overhead_image_stress:\n.byte 0x19,0x82,0x37,0x46\n')
    stress_object = output / "stress.o"
    run(["xtensa-esp32s3-elf-as", stress_source, "-o", stress_object], ROOT, env, log)
    for binary, features in [("overhead-s3-benchmark", []),
                             ("overhead-s3-catalogue-memory", ["--features", "catalogue-memory"])]:
        for stress in (False, True):
            name = binary + ("-stress-65536" if stress else "")
            command = ["cargo", "rustc", "--release", "--locked", "--bin", binary, *features]
            if stress:
                command += ["--", "-C", f"link-arg={stress_object}",
                            "-C", "link-arg=-Wl,-u,overhead_image_stress"]
            run(command, ROOT, env, log)
            built = Path(env["CARGO_TARGET_DIR"]) / "xtensa-esp32s3-none-elf/release" / binary
            elf = output / f"{name}.elf"
            elf.write_bytes(built.read_bytes())
            gate(name, elf, 65536 if stress else None)

    # Minimal source-level linker fixtures exercise the no-gap case as well as
    # small, large and page-sized alignment, using the SAME local rodata.x.
    for alignment in (4, 64, 4096, 65536):
        name = f"linker-alignment-{alignment}"
        assembly = output / f"{name}.S"
        assembly.write_text(
            '.section .flash.appdesc,"a",@progbits\n.global esp_app_desc\nesp_app_desc:\n'
            '.long 0xabcd5432\n.zero 176\n.byte 16\n.zero 75\n'
            f'.section .rodata.probe,"a",@progbits\n.balign {alignment}\n'
            '.byte 0x19,0x82,0x37,0x46\n'
            '.section .text,"ax",@progbits\n.global _start\n_start:\n'
            '.byte 0x11,0x22,0x33,0x44\n'
            '.section .data,"aw",@progbits\n.byte 0x55,0x66,0x77,0x88\n')
        script = output / f"{name}.x"
        script.write_text(
            'ENTRY(_start)\nMEMORY {\n'
            'RODATA (r) : ORIGIN = 0x3c000020, LENGTH = 2M\n'
            'ROTEXT (rx) : ORIGIN = 0x42020020, LENGTH = 2M\n'
            'RWDATA (rw) : ORIGIN = 0x3fc90000, LENGTH = 4K\n}\n'
            f'INCLUDE "{ROOT / "rodata.x"}"\n'
            'SECTIONS { .text : { *(.text) } > ROTEXT\n'
            '.data : { *(.data) } > RWDATA }\n')
        obj, elf = output / f"{name}.o", output / f"{name}.elf"
        run(["xtensa-esp32s3-elf-as", assembly, "-o", obj], ROOT, env, log)
        run(["xtensa-esp32s3-elf-ld", "-T", script, obj, "-o", elf], ROOT, env, log)
        gate(name, elf, alignment)

    # Reproduce the original failure from source, without historical artifacts
    # or modifications to registry/toolchain files. Remove only the remedy byte.
    original_gap = output / "original-gap.x"
    source = (ROOT / "rodata.x").read_text()
    require(source.count("    BYTE(0);\n") == 1, "remedy source changed; review negative control")
    original_gap.write_text(source.replace("    BYTE(0);\n", ""))
    script = output / "negative-nobits.x"
    script.write_text((output / "linker-alignment-64.x").read_text().replace(
        str(ROOT / "rodata.x"), str(original_gap)))
    elf, image = output / "negative-nobits.elf", output / "negative-nobits.bin"
    run(["xtensa-esp32s3-elf-ld", "-T", script, output / "linker-alignment-64.o",
         "-o", elf], ROOT, env, log)
    run(["espflash", "save-image", "--chip", "esp32s3", "--flash-size", "8mb",
         "--flash-mode", "dio", "--flash-freq", "40mhz", "--skip-update-check",
         elf, image], ROOT, env, log)
    elf_data, image_data = elf.read_bytes(), image.read_bytes()
    _, sections = elf_sections(elf_data)
    _, segments = image_segments(image_data)
    require(any(s.name == ".rodata_merge" and s.kind == 8 and s.size == 32 for s in sections),
            "negative control did not reproduce NOBITS gap")
    require(sum(s.address != 0 and region(s.address, len(s.data)) == "DROM"
                for s in segments) == 2, "negative control did not split DROM")
    try:
        validate(elf_data, image_data)
    except GateError as error:
        require(str(error) == "missing file-backed descriptor/rodata merge/rodata",
                f"unexpected negative-control rejection: {error}")
        results["negative_control"] = {
            "expected_rejection": str(error), "drom_segments": 2,
            "elf_sha256": hashlib.sha256(elf_data).hexdigest(),
            "image_sha256": hashlib.sha256(image_data).hexdigest(),
        }
    else:
        raise RuntimeError("NOBITS negative control unexpectedly passed")
    results["complete"] = True
    (output / "report.json").write_text(json.dumps(results, indent=2) + "\n")
    print(f"All 8 positive cases and the negative control passed. Report: {output / 'report.json'}")


if __name__ == "__main__":
    main()
