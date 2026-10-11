#!/usr/bin/env python3
"""Archive/gate exact normal M2 images and disassembly; NO hardware access.

Run after sourcing ~/export-esp.sh. A new output directory is required. Does not
replace host preflight/tests or authorize flashing. Never builds stress images.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil

from check_images import run
from generate_m2 import ROOT, DEST, generate, manifest
from image_gate import elf_sections, validate

FIRMWARE = ROOT / 'firmware'


def frames(disassembly):
    """Inventory ENTRY reservations only, not call-chain/worst-case stack use."""
    symbol = None
    address = None
    result = []
    for line in disassembly.splitlines():
        match = re.match(r'([0-9a-f]+) <(.*)>:', line)
        if match:
            address, symbol = int(match[1], 16), match[2]
        instruction = re.match(r'\s*([0-9a-f]+):', line)
        match = re.search(r'\bentry\s+a1,\s*(0x[0-9a-f]+|[0-9]+)', line)
        if match and instruction and int(instruction[1], 16) == address:
            result.append({'symbol': symbol, 'entry_bytes': int(match[1], 0)})
    return sorted(result, key=lambda r: (-r['entry_bytes'], r['symbol']))


def source_hashes():
    paths = [p for p in FIRMWARE.rglob('*') if p.is_file()
             and 'target' not in p.relative_to(FIRMWARE).parts
             and '__pycache__' not in p.parts and p.suffix in ('.rs', '.py', '.toml', '.lock', '.x')]
    paths += [ROOT / 'tools/fixtures/m2-cases/manifest.json']
    return {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(paths)}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('output', type=Path)
    args = p.parse_args()
    frozen, digest = manifest()
    if DEST.read_text() != generate():
        raise SystemExit('generated tables differ; stop before build')
    for name in ['RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER']:
        if os.environ.get(name):
            raise SystemExit(f'{name} override requires reviewed build context')
    if any(k.startswith('CARGO_PROFILE_') for k in os.environ):
        raise SystemExit('profile override requires reviewed build context')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    log = output / 'commands.log'
    env = dict(os.environ, BENCH_SUITE='baseline', BENCH_SAMPLES='3',
               CARGO_TARGET_DIR=str(output / 'cargo-target'))
    report = {'complete': False, 'scope': 'normal offline images, not target evidence or flash approval',
              'manifest_id': frozen['manifest_id'], 'manifest_sha256': digest,
              'source_sha256': source_hashes(), 'images': {}}
    report['tools'] = {tool: run([tool, '--version'], FIRMWARE, env, log)
                       for tool in ['rustc', 'cargo', 'espflash', 'xtensa-esp32s3-elf-objdump']}
    report['git_base'] = run(['git', 'rev-parse', 'HEAD'], ROOT, env, log)
    report['git_status'] = run(['git', 'status', '--short'], ROOT, env, log)
    for example, features in [('m2_measure', 'catalogue-memory'),
                              ('m2_diagnostic', 'catalogue-memory,numerical-reference')]:
        run(['cargo', 'build', '--offline', '--release', '--locked', '--example', example,
             '--features', features], FIRMWARE, env, log)
        built = Path(env['CARGO_TARGET_DIR']) / 'xtensa-esp32s3-none-elf/release/examples' / example
        elf, image = output / (example + '.elf'), output / (example + '.bin')
        shutil.copyfile(built, elf)
        run(['espflash', 'save-image', '--chip', 'esp32s3', '--flash-size', '8mb',
             '--flash-mode', 'dio', '--flash-freq', '40mhz', '--skip-update-check', elf, image], FIRMWARE, env, log)
        gate = validate(elf.read_bytes(), image.read_bytes())
        disassembly = run(['xtensa-esp32s3-elf-objdump', '-d', '-C', elf], FIRMWARE, env, output / 'disassembly-command.log')
        dis_path = output / (example + '.disassembly.txt')
        dis_path.write_text(disassembly + '\n')
        inventory = frames(disassembly)
        _, sections = elf_sections(elf.read_bytes())
        section_inventory = [{'name': s.name, 'address': hex(s.address), 'bytes': s.size,
                              'kind': s.kind, 'flags': s.flags, 'alignment': s.alignment}
                             for s in sections if s.flags & 2]
        symbols = run(['xtensa-esp32s3-elf-nm', '-S', '--size-sort', '-C', elf], FIRMWARE, env, output / 'symbols-command.log')
        (output / (example + '.symbols.txt')).write_text(symbols + '\n')
        if not inventory or not any('m2::load' in f['symbol'] for f in inventory):
            raise RuntimeError('missing parser frame inventory')
        if example == 'm2_diagnostic':
            smoke = [f for f in inventory if 'exercise_stack' in f['symbol']]
            if len(smoke) != 1 or smoke[0]['entry_bytes'] < 8192:
                raise RuntimeError('known 8KiB diagnostic local reservation not retained')
        report['images'][example] = {'features': features, 'gate': gate, 'sections': section_inventory, 'frames': inventory,
            'raw_disassembly_callx_tokens': len(re.findall(r'\bcallx(?:0|4|8|12)\s', disassembly)),
            'disassembly_sha256': hashlib.sha256(dis_path.read_bytes()).hexdigest(),
            'frame_limits': 'Symbol-start ENTRY inventory only; no call-chain bound. Includes unwritten reservations; raw callx tokens include decoded literal pools, literal-resolved calls and indirect calls. Interrupts/ABI spill/probe frames not bounded.'}
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    if source_hashes() != report['source_sha256']:
        raise RuntimeError('sources changed during build; reject archive')
    report['complete'] = True
    (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(f'Both exact normal images pass: {output}/report.json; no hardware accessed')


if __name__ == '__main__':
    main()
