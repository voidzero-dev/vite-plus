#!/usr/bin/env python3
"""Measure a Mach-O artifact and attribute section bytes to symbol extents.

Symbol extents include alignment and any unnamed bytes before the next symbol.
They are attribution estimates, not compiler-reported function sizes. Analyze
an unstripped copy for attribution; compare stripped artifacts for release size.
"""

import argparse
import collections
import gzip
import hashlib
import json
import re
import struct
import subprocess
from pathlib import Path


def cstring(data):
    return data.split(b"\0", 1)[0].decode("utf-8", errors="replace")


def analyze(path, cxxfilt, limit):
    data = path.read_bytes()
    if data[:4] != b"\xcf\xfa\xed\xfe":
        raise ValueError("Expected a little-endian 64-bit Mach-O file")
    command_count = struct.unpack_from("<I", data, 16)[0]
    offset = 32
    sections = []
    symtab = None
    for _ in range(command_count):
        command, length = struct.unpack_from("<II", data, offset)
        if command == 0x19:  # LC_SEGMENT_64
            count = struct.unpack_from("<I", data, offset + 64)[0]
            for index in range(count):
                start = offset + 72 + 80 * index
                name, segment, address, size = struct.unpack_from("<16s16sQQ", data, start)
                sections.append({"name": cstring(segment) + "," + cstring(name),
                                 "address": address, "bytes": size})
        elif command == 2:  # LC_SYMTAB
            symtab = struct.unpack_from("<IIII", data, offset + 8)
        offset += length
    symbols = collections.defaultdict(lambda: collections.defaultdict(list))
    if symtab:
        symbol_offset, count, string_offset, string_size = symtab
        strings = data[string_offset:string_offset + string_size]
        for index in range(count):
            name_index, kind, section, _, address = struct.unpack_from(
                "<IBBHQ", data, symbol_offset + index * 16)
            if kind & 0xe0 or kind & 0x0e != 0x0e or not section:
                continue
            name = cstring(strings[name_index:])
            symbols[section - 1][address].append(name)
    names = sorted({name for group in symbols.values()
                    for aliases in group.values() for name in aliases})
    demangled = {}
    if names:
        process = subprocess.run([cxxfilt, "--strip-underscore"],
                                 input="\n".join(names) + "\n", text=True,
                                 capture_output=True, check=True)
        demangled = dict(zip(names, process.stdout.splitlines()))
    extents = []
    groups = collections.Counter()
    for index, section in enumerate(sections):
        end = section["address"] + section["bytes"]
        addresses = sorted(address for address in symbols[index]
                           if section["address"] <= address < end)
        for position, address in enumerate(addresses):
            next_address = addresses[position + 1] if position + 1 < len(addresses) else end
            aliases = symbols[index][address]
            name = demangled.get(aliases[0], aliases[0])
            extent = {"name": name, "section": section["name"],
                      "bytes": next_address - address}
            extents.append(extent)
            prefix = re.match(r"[<&]*(?:mut )?([a-z_][a-z0-9_]*)::", name)
            group = prefix.group(1) if prefix else "[C or anonymous]"
            if section["name"] in ("__TEXT,__text", "__TEXT,__text_startup"):
                groups[group] += extent["bytes"]
    return {"path": str(path), "raw": len(data),
            "gzip9_python_zlib": len(gzip.compress(data, compresslevel=9, mtime=0)),
            "sha256": hashlib.sha256(data).hexdigest(), "sections": sections,
            "largest_symbol_extents": sorted(extents, key=lambda x: -x["bytes"])[:limit],
            "text_by_symbol_prefix": dict(groups.most_common(limit))}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--cxxfilt", default="llvm-cxxfilt")
    parser.add_argument("--limit", type=int, default=50)
    args = parser.parse_args()
    print(json.dumps(analyze(args.binary, args.cxxfilt, args.limit), indent=2))
