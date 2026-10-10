#!/usr/bin/env python3
"""Offline, fail-closed gate for our S3 ELF / espflash application image pair.

Standard library only; never opens a port or invokes a flashing tool.
This deliberately supports the pinned ELF32 / 64-KiB-page experiment, not
arbitrary ESP images, secure boot, merged flash images, or other chips.
"""

import argparse
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import struct


class GateError(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise GateError(message)


def span(data, offset, size):
    require(0 <= offset <= len(data) and 0 <= size <= len(data) - offset,
            f"truncated data at {offset:#x}, length {size:#x}")
    return data[offset:offset + size]


@dataclass
class Section:
    name: str
    kind: int
    flags: int
    address: int
    size: int
    alignment: int
    data: bytes


@dataclass
class Segment:
    address: int
    offset: int
    data: bytes


def elf_sections(elf):
    header = span(elf, 0, 52)
    require(header[:7] == b"\x7fELF\x01\x01\x01", "expected little-endian ELF32")
    require(struct.unpack_from("<HH", header, 16) == (2, 94),
            "expected executable Xtensa ELF")
    entry, _, section_offset = struct.unpack_from("<III", header, 24)
    section_size, count, names_index = struct.unpack_from("<HHH", header, 46)
    require(section_size == 40 and 0 < names_index < count,
            "unsupported ELF section table")
    rows = [struct.unpack("<10I", span(elf, section_offset + i * 40, 40))
            for i in range(count)]
    names_row = rows[names_index]
    names = span(elf, names_row[4], names_row[5])
    sections = []
    for name, kind, flags, address, offset, size, _, _, alignment, _ in rows:
        require(name < len(names), "invalid section name offset")
        end = names.find(b"\0", name)
        require(end >= 0, "unterminated section name")
        label = names[name:end].decode("ascii")
        payload = b"" if kind == 8 else span(elf, offset, size)
        sections.append(Section(label, kind, flags, address, size, alignment, payload))
    return entry, sections


def image_segments(image):
    header = span(image, 0, 24)
    require(header[0] == 0xE9 and 1 <= header[1] <= 16, "invalid image header")
    require(struct.unpack_from("<H", header, 12)[0] == 9, "image is not ESP32-S3")
    require(header[23] == 1, "appended SHA-256 required")
    segments, offset, checksum = [], 24, 0xEF
    for _ in range(header[1]):
        address, size = struct.unpack("<II", span(image, offset, 8))
        require(size > 0 and size % 4 == 0 and address + size <= 2**32,
                "invalid segment size")
        payload = span(image, offset + 8, size)
        segments.append(Segment(address, offset + 8, payload))
        for byte in payload:
            checksum ^= byte
        offset += 8 + size
    checksum_offset = offset + (15 - offset % 16)
    require(not any(span(image, offset, checksum_offset - offset)), "nonzero checksum padding")
    require(span(image, checksum_offset, 1)[0] == checksum, "image checksum mismatch")
    digest_offset = checksum_offset + 1
    require(len(image) == digest_offset + 32, "unexpected image length/trailing bytes")
    require(image[digest_offset:] == hashlib.sha256(image[:digest_offset]).digest(),
            "image SHA-256 mismatch")
    return struct.unpack_from("<I", header, 4)[0], segments


def region(address, size):
    for label, start, end in [
        ("DROM", 0x3C000000, 0x3E000000),
        ("IROM", 0x42000000, 0x44000000),
        ("DRAM", 0x3FC88000, 0x3FD00000),
        ("IRAM", 0x40370000, 0x403E0000),
        ("RTC_FAST", 0x600FE000, 0x60100000),
        ("RTC_SLOW", 0x50000000, 0x50002000),
    ]:
        if start <= address and address + size <= end:
            return label
    raise GateError(f"unsupported address range {address:#x}+{size:#x}")


def validate(elf, image, app_offset=0x10000, expected_alignment=None):
    require(app_offset >= 0 and app_offset % 0x10000 == 0, "app offset must be 64-KiB aligned")
    entry, sections = elf_sections(elf)
    image_entry, segments = image_segments(image)
    require(entry == image_entry, "entry point mismatch")
    loadable = []
    for section in sections:
        if not section.size or not section.flags & 2:  # SHF_ALLOC
            continue
        require(section.kind in (1, 8, 14), f"unsupported allocated section {section.name}")
        if section.kind != 8:  # NOBITS is not an image payload
            region(section.address, section.size)
            loadable.append(section)
    by_name = {s.name: s for s in loadable}
    require(len(by_name) == len(loadable), "duplicate loadable section names")
    require(all(name in by_name for name in (".flash.appdesc", ".rodata_merge", ".rodata")),
            "missing file-backed descriptor/rodata merge/rodata")
    desc, merge, rodata = (by_name[n] for n in (".flash.appdesc", ".rodata_merge", ".rodata"))
    require(desc.address == 0x3C000020 and desc.size == 256, "descriptor placement/size changed")
    require(struct.unpack_from("<I", desc.data)[0] == 0xABCD5432, "descriptor magic mismatch")
    require(desc.data[180] == 16, "expected explicit 64-KiB MMU pages")
    require(merge.kind == 1 and merge.address == desc.address + desc.size
            and merge.address + merge.size == rodata.address and not any(merge.data),
            "merge must be contiguous file-backed zero padding")
    require(rodata.alignment >= 4 and rodata.address % rodata.alignment == 0,
            "rodata alignment mismatch")
    if expected_alignment is not None:
        require(rodata.alignment == expected_alignment, "stress alignment was not retained")

    mapped = []
    for segment in segments:
        if segment.address == 0:
            require(not any(segment.data), "nonzero dummy segment")
            continue
        label = region(segment.address, len(segment.data))
        if label in ("DROM", "IROM"):
            require((app_offset + segment.offset) % 0x10000 == segment.address % 0x10000,
                    f"{label} physical/virtual page offset mismatch")
        mapped.append((segment, label))
    drom = [s for s, label in mapped if label == "DROM"]
    require(len(drom) == 1, f"expected one DROM segment, got {len(drom)}")
    require(sum(label == "IROM" for _, label in mapped) == 1, "expected one IROM segment")
    require(drom[0] is segments[0] and drom[0].offset == 32
            and drom[0].address == desc.address, "descriptor must be first image payload")
    ordered = sorted((s for s, _ in mapped), key=lambda s: s.address)
    for previous, following in zip(ordered, ordered[1:]):
        require(previous.address + len(previous.data) <= following.address, "overlapping image segments")

    # RAM sections may be split into multiple segments by espflash to fill
    # flash alignment holes. Compare every section byte across those splits.
    covered = [bytearray(len(s.data)) for s in ordered]
    section_ranges = sorted(loadable, key=lambda s: s.address)
    for previous, following in zip(section_ranges, section_ranges[1:]):
        require(previous.address + previous.size <= following.address, "overlapping ELF sections")
    for section in loadable:
        expected = bytearray(section.data)
        if section.name == ".flash.appdesc":
            # Only permitted ELF-to-image mutation; check the value, don't mask it.
            expected[144:176] = hashlib.sha256(elf).digest()
        found = 0
        for index, segment in enumerate(ordered):
            start = max(section.address, segment.address)
            end = min(section.address + section.size, segment.address + len(segment.data))
            if end <= start:
                continue
            a, b, size = start - section.address, start - segment.address, end - start
            require(segment.data[b:b + size] == expected[a:a + size],
                    f"image bytes differ from ELF: {section.name} at {start:#x}")
            covered[index][b:b + size] = b"\1" * size
            found += size
        require(found == section.size, f"image omits ELF bytes: {section.name}")
    for segment, coverage in zip(ordered, covered):
        require(all(mark or byte == 0 for mark, byte in zip(coverage, segment.data)),
                f"nonzero image bytes outside ELF sections at {segment.address:#x}")
    return {
        "elf_sha256": hashlib.sha256(elf).hexdigest(),
        "image_sha256": hashlib.sha256(image).hexdigest(),
        "image_bytes": len(image),
        "app_offset": app_offset,
        "rodata_alignment": rodata.alignment,
        "merge_bytes": merge.size,
        "checked_sections": len(loadable),
        "checked_elf_bytes": sum(s.size for s in loadable),
        "segments": [{"address": f"0x{s.address:08x}", "bytes": len(s.data),
                      "file_offset": s.offset} for s in segments],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("elf", type=Path)
    parser.add_argument("image", type=Path, help="application-only espflash save-image output")
    parser.add_argument("--app-offset", type=lambda x: int(x, 0), default=0x10000)
    parser.add_argument("--expect-alignment", type=int)
    args = parser.parse_args()
    try:
        result = validate(args.elf.read_bytes(), args.image.read_bytes(),
                          args.app_offset, args.expect_alignment)
    except (OSError, ValueError, struct.error) as error:
        parser.exit(1, f"image gate FAILED: {error}\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
