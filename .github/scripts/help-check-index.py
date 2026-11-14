#!/usr/bin/env python3
"""Checks that Pagefind indexed the Japanese text of the help (ADR-0031).

usage: help-check-index.py <site folder that holds pagefind/> <a Japanese word of the page "このヘルプについて">

Pagefind writes its files as a 12-byte header ("pagefind_dcd") and a gzip stream. This reads the entry file, then
every page fragment (the text of a page) and looks for the word in the text. Python 3 standard library only.
"""

import glob
import json
import os
import sys
import zlib


def unpack(path):
    """The decompressed content of a Pagefind file (or the raw bytes when it holds no gzip stream)."""
    with open(path, "rb") as f:
        data = f.read()
    offset = data.find(b"\x1f\x8b")
    if offset < 0:
        return data
    return zlib.decompressobj(16 + zlib.MAX_WBITS).decompress(data[offset:])


def main():
    site, word = sys.argv[1], sys.argv[2]
    pagefind = os.path.join(site, "pagefind")
    with open(os.path.join(pagefind, "pagefind-entry.json"), encoding="utf-8") as f:
        entry = json.load(f)
    print("pagefind-entry.json:", json.dumps(entry, ensure_ascii=False))
    ja = entry.get("languages", {}).get("ja")
    if not ja or ja.get("page_count", 0) < 1:
        sys.exit("error: Pagefind has no Japanese pages (languages.ja.page_count is 0 or missing)")

    needle = word.encode("utf-8")
    fragments = sorted(glob.glob(os.path.join(pagefind, "fragment", "*")))
    if not fragments:
        sys.exit("error: Pagefind wrote no page fragments")
    hits = [p for p in fragments if needle in unpack(p)]
    if not hits:
        sys.exit(f"error: no page fragment holds the word {word!r}")
    print(f"ok: {len(hits)} of {len(fragments)} page fragments hold {word!r}")

    # Informational: whether the search index itself holds the characters of the word. The format of the index
    # is Pagefind's own, so this does not fail the job.
    indexes = sorted(glob.glob(os.path.join(pagefind, "index", "*")))
    found = sum(1 for p in indexes if word[:2].encode("utf-8") in unpack(p))
    print(f"info: {found} of {len(indexes)} index files hold {word[:2]!r}")


main()
