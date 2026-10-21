"""Lists the DLLs a Windows program imports, and fails if it needs the C runtime DLLs.

Usage: python pe_imports.py <program.exe>...

The programs link the C runtime statically (.cargo/config.toml), so that they start on a Windows without the
Visual C++ Redistributable. A program that imports VCRUNTIME*.dll, MSVCP*.dll or the api-ms-win-crt-* DLLs of the
Universal CRT has lost that.
"""

import struct
import sys

FORBIDDEN = ("vcruntime", "msvcp", "api-ms-win-crt-")


def imports(path):
    with open(path, "rb") as f:
        data = f.read()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe : pe + 4] != b"PE\0\0":
        raise ValueError(f"{path}: not a PE file")
    sections_count = struct.unpack_from("<H", data, pe + 6)[0]
    optional_size = struct.unpack_from("<H", data, pe + 20)[0]
    optional = pe + 24
    magic = struct.unpack_from("<H", data, optional)[0]
    # The data directories follow the fixed part of the optional header: 96 bytes in PE32, 112 in PE32+.
    directories = optional + (112 if magic == 0x20B else 96)
    import_rva = struct.unpack_from("<I", data, directories + 8)[0]
    sections = []
    for i in range(sections_count):
        header = optional + optional_size + 40 * i
        virtual_size, virtual_address, raw_size, raw_pointer = struct.unpack_from("<IIII", data, header + 8)
        sections.append((virtual_address, max(virtual_size, raw_size), raw_pointer))

    def offset(rva):
        for address, size, raw in sections:
            if address <= rva < address + size:
                return rva - address + raw
        raise ValueError(f"{path}: RVA {rva:#x} is in no section")

    names = []
    if import_rva == 0:
        return names
    descriptor = offset(import_rva)
    while True:
        name_rva = struct.unpack_from("<I", data, descriptor + 12)[0]
        if name_rva == 0:
            return names
        start = offset(name_rva)
        names.append(data[start : data.index(b"\0", start)].decode("ascii"))
        descriptor += 20


def main(paths):
    failed = False
    for path in paths:
        names = imports(path)
        print(f"{path}: {', '.join(names)}")
        bad = [name for name in names if name.lower().startswith(FORBIDDEN)]
        if bad:
            print(f"::error::{path} imports {', '.join(bad)}: the C runtime must be linked statically")
            failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
