#!/usr/bin/env python3
"""Moves the macOS recovery partition of a stopped VM's raw disk image to the end of the disk.

`tart set --disk-size` makes the disk image larger, but the system's APFS container cannot grow into the new space:
the recovery container sits right after it. diskutil refuses to delete or move an "APFS Recovery Physical Store"
while System Integrity Protection is on, in the VM and on the host alike, and `gpt` needs root to write. So this
edits the GPT of the image file itself, as Cirrus Labs' Packer plugin for Tart does with recovery_partition =
"relocate": it copies the recovery partition to the end of the disk and moves its entry there (same partition GUID).
After that, `diskutil apfs resizeContainer disk0s2 0` in the VM grows the system's container (ADR-0027).

    relocate-recovery.py <disk.img>

The VM must be stopped. Exits 0 without changes when the recovery partition is already at the end.
"""

import os
import struct
import sys
import uuid
import zlib

SECTOR = 512
HEADER_FORMAT = "<8sIIIIQQQQ16sQIII"
HEADER_SIZE = struct.calcsize(HEADER_FORMAT)  # 92
RECOVERY_TYPE = uuid.UUID("52637672-7900-11AA-AA11-00306543ECAC").bytes_le
EMPTY_TYPE = bytes(16)
CHUNK = 64 * 1024 * 1024


def fail(message):
    print(f"relocate-recovery: {message}", file=sys.stderr)
    sys.exit(1)


def read_header(f, lba):
    f.seek(lba * SECTOR)
    raw = bytearray(f.read(SECTOR))
    fields = list(struct.unpack(HEADER_FORMAT, raw[:HEADER_SIZE]))
    signature, _revision, header_size, header_crc = fields[0], fields[1], fields[2], fields[3]
    if signature != b"EFI PART" or header_size != HEADER_SIZE:
        fail(f"no GPT header at LBA {lba}")
    check = bytearray(raw[:HEADER_SIZE])
    check[16:20] = b"\0\0\0\0"
    if zlib.crc32(check) != header_crc:
        fail(f"bad checksum of the GPT header at LBA {lba}")
    return {
        "raw": raw,
        "my_lba": fields[5],
        "alternate_lba": fields[6],
        "first_usable": fields[7],
        "last_usable": fields[8],
        "entries_lba": fields[10],
        "entry_count": fields[11],
        "entry_size": fields[12],
        "entries_crc": fields[13],
    }


def write_header(f, header, entries_crc):
    raw = bytearray(header["raw"])
    struct.pack_into("<I", raw, 88, entries_crc)
    raw[16:20] = b"\0\0\0\0"
    struct.pack_into("<I", raw, 16, zlib.crc32(raw[:HEADER_SIZE]))
    f.seek(header["my_lba"] * SECTOR)
    f.write(raw)


def check_geometry(primary, backup, sectors):
    """Refuses a GPT whose layout this script does not expect, before anything is written."""
    for name in ("first_usable", "last_usable", "entry_count", "entry_size"):
        if primary[name] != backup[name]:
            fail(f"the two GPT headers disagree on {name}")
    if primary["entry_size"] != 128 or not 1 <= primary["entry_count"] <= 1024:
        fail("unexpected size or number of partition entries")
    table = -(-primary["entry_count"] * primary["entry_size"] // SECTOR)  # sectors, rounded up
    if primary["entries_lba"] != 2 or primary["entries_lba"] + table > primary["first_usable"]:
        fail("the primary partition table is not where it should be")
    if backup["entries_lba"] + table != sectors - 1 or primary["last_usable"] >= backup["entries_lba"]:
        fail("the backup partition table is not where it should be")
    if not 2 < primary["first_usable"] <= primary["last_usable"] < sectors:
        fail("the usable part of the disk is out of bounds")


def main():
    if len(sys.argv) != 2:
        print(__doc__.strip().splitlines()[0])
        sys.exit(2)
    path = sys.argv[1]
    with open(path, "r+b") as f:
        sectors = os.fstat(f.fileno()).st_size // SECTOR
        primary = read_header(f, 1)
        if primary["my_lba"] != 1 or primary["alternate_lba"] != sectors - 1:
            fail("the backup GPT header is not at the end of the image (repair the disk in the VM first)")
        backup = read_header(f, primary["alternate_lba"])
        if backup["my_lba"] != sectors - 1 or backup["alternate_lba"] != 1:
            fail("the backup GPT header does not point back to the primary one")
        check_geometry(primary, backup, sectors)
        size = primary["entry_count"] * primary["entry_size"]
        f.seek(primary["entries_lba"] * SECTOR)
        entries = bytearray(f.read(size))
        if zlib.crc32(entries) != primary["entries_crc"]:
            fail("bad checksum of the partition entries")
        f.seek(backup["entries_lba"] * SECTOR)
        if f.read(size) != entries:
            fail("the backup partition entries differ from the primary ones")

        used = []
        for i in range(primary["entry_count"]):
            offset = i * primary["entry_size"]
            kind = bytes(entries[offset : offset + 16])
            if kind == EMPTY_TYPE:
                continue
            first, last = struct.unpack_from("<QQ", entries, offset + 32)
            used.append((i, kind, first, last))
        for _i, _kind, first, last in used:
            if not (primary["first_usable"] <= first <= last <= primary["last_usable"]):
                fail("a partition lies outside the usable part of the disk")
        spans = sorted((e[2], e[3]) for e in used)
        for (_a_first, a_last), (b_first, _b_last) in zip(spans, spans[1:]):
            if b_first <= a_last:
                fail("partitions overlap")
        recovery = [e for e in used if e[1] == RECOVERY_TYPE]
        if len(recovery) != 1:
            fail(f"expected one recovery partition, found {len(recovery)}")
        index, _kind, first, last = recovery[0]
        if any(e[2] > first for e in used):
            fail("the recovery partition is not the last partition")
        end = primary["last_usable"]
        if last == end:
            print("the recovery partition is already at the end of the disk")
            return
        length = last - first + 1
        new_first = end - length + 1
        if new_first <= last:
            fail("not enough free space after the recovery partition to move it")
        if new_first < primary["first_usable"] or end >= backup["entries_lba"]:
            fail("the new place of the recovery partition would cover the partition tables")

        # Copy the contents first, then point the entry at the copy: if anything stops halfway, the old entry still
        # points at the untouched original.
        remaining = length * SECTOR
        source = first * SECTOR
        target = new_first * SECTOR
        while remaining:
            n = min(CHUNK, remaining)
            f.seek(source)
            data = f.read(n)
            if len(data) != n:
                fail("short read while copying the recovery partition")
            f.seek(target)
            f.write(data)
            source += n
            target += n
            remaining -= n
        f.flush()
        os.fsync(f.fileno())

        struct.pack_into("<QQ", entries, index * primary["entry_size"] + 32, new_first, end)
        crc = zlib.crc32(entries)
        # One complete copy at a time, each synced before the other is touched: the backup first, then the primary.
        # If anything stops in between, one of the two copies is whole and valid.
        for header in (backup, primary):
            f.seek(header["entries_lba"] * SECTOR)
            f.write(entries)
            write_header(f, header, crc)
            f.flush()
            os.fsync(f.fileno())
        print(f"moved the recovery partition from LBA {first}-{last} to {new_first}-{end}")


if __name__ == "__main__":
    main()
