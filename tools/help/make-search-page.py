#!/usr/bin/env python3
"""Writes search.html next to print.html: the same one-page book, without the automatic print call.

usage: make-search-page.py <site folder>

mdBook's print.html calls window.print() when it loads, so opening it only to search it (Ctrl+F finds all pages)
pops up a print dialog. search.html is a copy of print.html in which every `window.print()` is replaced by `void 0`.
The call may be inline or in a local script that print.html loads; a script that holds it is copied as
`search-<name>` and search.html loads the copy (print.html keeps the original). If no call is found anywhere, or one
is left, the script fails: an mdBook upgrade that changes this must break the build, not ship a page that prints.
Python 3 standard library only.
"""

import os
import re
import sys

CALL = re.compile(r"window\s*\.\s*print\s*\(\s*\)")
ANY = re.compile(r"window\s*\.\s*print")
SRC = re.compile(r'(<script\b[^>]*?\bsrc=")([^"#?]+)((?:[?#][^"]*)?")')


def fail(message):
    print(f"::error::{message}")
    sys.exit(1)


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read()


def write(path, text):
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write(text)


def main():
    site = sys.argv[1]
    page = read(os.path.join(site, "print.html"))
    replaced = 0

    page, n = CALL.subn("void 0", page)
    replaced += n

    def local_script(match):
        nonlocal replaced
        name = match.group(2)
        if re.match(r"^[a-z]+:|^//", name):
            return match.group(0)
        path = os.path.join(site, name)
        if not os.path.isfile(path):
            return match.group(0)
        script = read(path)
        if not ANY.search(script):
            return match.group(0)
        new_script, count = CALL.subn("void 0", script)
        replaced += count
        copy = os.path.join(os.path.dirname(name), "search-" + os.path.basename(name))
        write(os.path.join(site, copy), new_script)
        return match.group(1) + copy.replace(os.sep, "/") + match.group(3)

    page = SRC.sub(local_script, page)
    if replaced == 0:
        fail("no window.print() call was found in print.html or the scripts it loads; check tools/help/make-search-page.py against this mdBook version")
    write(os.path.join(site, "search.html"), page)
    check(site)
    print(f"search.html written ({replaced} print call(s) removed)")


def check(site):
    """search.html exists and neither it nor a local script it loads mentions window.print."""
    path = os.path.join(site, "search.html")
    if not os.path.isfile(path):
        fail("search.html was not written")
    page = read(path)
    if ANY.search(page):
        fail("search.html still has a window.print call")
    for match in SRC.finditer(page):
        name = match.group(2)
        script = os.path.join(site, name)
        if not re.match(r"^[a-z]+:|^//", name) and os.path.isfile(script) and ANY.search(read(script)):
            fail(f"{name}, loaded by search.html, still has a window.print call")


if __name__ == "__main__":
    main()
