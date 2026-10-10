"""Synthetic format regressions; real binary/stress checks use check_images.py."""
import hashlib
import struct
import unittest

from image_gate import GateError, elf_sections, image_segments, validate


DESC = 0x3C000020
TEXT = 0x42000200
RAM = 0x3FC90000


def make_elf(merge_kind=1, alignment=64):
    descriptor = bytearray(256)
    struct.pack_into("<I", descriptor, 0, 0xABCD5432)
    descriptor[180] = 16
    entries = [
        (".flash.appdesc", 1, 2, DESC, bytes(descriptor), 4),
        (".rodata_merge", merge_kind, 2, DESC + 256, bytes(32), 4),
        (".rodata", 1, 2, DESC + 288, b"rodata!!", alignment),
        (".text", 1, 6, TEXT, b"code!!!!", 4),
        (".data", 1, 3, RAM, b"ramdata!", 4),
        (".bss", 8, 3, RAM + 8, bytes(16), 4),
    ]
    names = b"\0" + b"".join(e[0].encode() + b"\0" for e in entries) + b".shstrtab\0"
    entries.append((".shstrtab", 3, 0, 0, names, 1))
    data = bytearray(52)
    rows = [bytes(40)]
    for name, kind, flags, address, payload, align in entries:
        offset = len(data)
        if kind != 8:
            data.extend(payload)
        rows.append(struct.pack("<10I", names.index(name.encode() + b"\0"), kind,
                                flags, address, offset, len(payload), 0, 0, align, 0))
    section_offset = len(data)
    data.extend(b"".join(rows))
    data[:16] = b"\x7fELF\x01\x01\x01" + bytes(9)
    struct.pack_into("<HHIIIIIHHHHHH", data, 16, 2, 94, 1, TEXT, 0,
                     section_offset, 0, 52, 0, 0, 40, len(rows), len(rows) - 1)
    return bytes(data)


def pack_image(segments, entry=TEXT):
    data = bytearray(24)
    data[0:2] = bytes([0xE9, len(segments)])
    struct.pack_into("<I", data, 4, entry)
    struct.pack_into("<H", data, 12, 9)
    data[23] = 1
    checksum = 0xEF
    for address, payload in segments:
        data.extend(struct.pack("<II", address, len(payload)))
        data.extend(payload)
        for byte in payload:
            checksum ^= byte
    data.extend(bytes(15 - len(data) % 16))
    data.append(checksum)
    data.extend(hashlib.sha256(data).digest())
    return bytes(data)


def good_segments(elf):
    _, sections = elf_sections(elf)
    descriptor = bytearray(sections[1].data)
    descriptor[144:176] = hashlib.sha256(elf).digest()
    return [
        (DESC, bytes(descriptor) + bytes(32) + b"rodata!!"),
        # Exercise espflash's RAM section splitting.
        (RAM, b"ramd"),
        (RAM + 4, b"ata!"),
        (0, bytes(144)),
        (TEXT, b"code!!!!"),
    ]


class ImageGateTests(unittest.TestCase):
    def setUp(self):
        self.elf = make_elf()
        self.segments = good_segments(self.elf)
        self.image = pack_image(self.segments)

    def reject_segments(self, segments, message=None):
        with self.assertRaisesRegex(GateError, message or "."):
            validate(self.elf, pack_image(segments))

    def test_valid_and_split_ram(self):
        result = validate(self.elf, self.image, expected_alignment=64)
        self.assertEqual(result["merge_bytes"], 32)
        self.assertEqual(result["checked_sections"], 5)

    def test_nobits_gap_rejected(self):
        elf = make_elf(merge_kind=8)
        with self.assertRaisesRegex(GateError, "file-backed"):
            validate(elf, pack_image(good_segments(elf)))

    def test_multiple_drom_segments(self):
        payload = self.segments[0][1]
        tail = self.segments[1:].copy()
        tail[2] = (0, bytes(136))  # Added segment header must not shift IROM.
        self.reject_segments([(DESC, payload[:256]), (DESC + 264, payload[256:])]
                             + tail, "one DROM")

    def test_corruption_in_each_memory_class_and_descriptor_hash(self):
        for index, offset in [(0, 144), (0, 290), (1, 0), (4, 0)]:
            with self.subTest(index=index, offset=offset):
                segments = self.segments.copy()
                address, payload = segments[index]
                changed = bytearray(payload)
                changed[offset] ^= 1
                segments[index] = address, bytes(changed)
                self.reject_segments(segments, "bytes differ")

    def test_wrong_elf_even_with_valid_image_checksums(self):
        other = self.elf + b"different debug content"
        with self.assertRaisesRegex(GateError, "bytes differ"):
            validate(other, self.image)

    def test_missing_ram_bytes(self):
        segments = self.segments.copy()
        segments[2] = (0, bytes(4))
        self.reject_segments(segments, "omits ELF bytes")

    def test_nonzero_padding(self):
        segments = self.segments.copy()
        segments[3] = (0, b"!" + bytes(143))
        self.reject_segments(segments, "nonzero dummy")
        segments = self.segments.copy()
        segments[-1] = (TEXT, b"code!!!!oops")
        self.reject_segments(segments, "outside ELF")

    def test_overlap_and_bad_flash_mapping(self):
        segments = self.segments.copy()
        segments[2] = (RAM, b"ata!")
        self.reject_segments(segments, "overlapping")
        segments = self.segments.copy()
        segments[-1] = (TEXT + 4, b"code!!!!")
        self.reject_segments(segments, "page offset")

    def test_entry_chip_and_digest_required(self):
        for offset, replacement in [(4, b"\0\0\0\0"), (12, b"\0\0"), (23, b"\0")]:
            with self.subTest(offset=offset):
                image = bytearray(self.image)
                image[offset:offset + len(replacement)] = replacement
                image[-32:] = hashlib.sha256(image[:-32]).digest()
                with self.assertRaises(GateError):
                    validate(self.elf, bytes(image))

    def test_checksum_digest_truncation_and_trailing_bytes(self):
        images = [self.image[:-1], self.image + b"!", b"", self.image[:30]]
        checksum_bad = bytearray(self.image)
        checksum_bad[-33] ^= 1
        checksum_bad[-32:] = hashlib.sha256(checksum_bad[:-32]).digest()
        images.append(bytes(checksum_bad))
        digest_bad = bytearray(self.image)
        digest_bad[-1] ^= 1
        images.append(bytes(digest_bad))
        for image in images:
            with self.subTest(size=len(image)):
                with self.assertRaises(GateError):
                    validate(self.elf, image)

    def test_invalid_elf_and_alignment(self):
        for elf in [b"", self.elf[:30], self.elf[:-1]]:
            with self.assertRaises(GateError):
                validate(elf, self.image)
        with self.assertRaisesRegex(GateError, "stress alignment"):
            validate(self.elf, self.image, expected_alignment=4096)
        with self.assertRaisesRegex(GateError, "app offset"):
            validate(self.elf, self.image, app_offset=0x10001)

    def test_segment_length_cannot_run_past_file(self):
        image = bytearray(self.image)
        struct.pack_into("<I", image, 28, 0xFFFFFFFC)
        with self.assertRaises(GateError):
            image_segments(bytes(image))


if __name__ == "__main__":
    unittest.main()
