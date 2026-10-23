#!/usr/bin/env python3
"""Turns Onigmo's testpy.py into a Rust test table (src/onigmo_suite/cases.rs).

testpy.py is a script of calls like `x2("a", "a", 0, 1)`. Rather than parsing
its text (it has `if onig_encoding == ...` branches), this script runs the
real `main()` of testpy.py against a fake `onigmo` module that records every
call to `xx()` for the UTF-8 encoding, and prints the records as Rust.

Usage:
    python3 tools/gen_onigmo_suite.py <Onigmo checkout> > src/onigmo_suite/cases.rs

The Onigmo checkout is the one vendored in `onigmo/` (see onigmo/README.md);
`testpy.py` itself is not vendored, so pass a clone of the same commit.
"""

import ast
import contextlib
import io
import re
import sys
import types
import warnings
from pathlib import Path

class Sym:
    """Stands for any name of the real `onigmo` module: a constant or a function."""

    _cache = {}

    def __new__(cls, name):
        if name not in cls._cache:
            obj = super().__new__(cls)
            obj.name = name
            cls._cache[name] = obj
        return cls._cache[name]

    def __call__(self, *args, **kwargs):
        if self.name in ("onig_set_match_stack_limit_size", "onig_set_parse_depth_limit"):
            events.append(("event", self.name, args[0]))
        return 0

    def __or__(self, other):
        return Sym("%s | %s" % (self.name, other.name))

    def __repr__(self):
        return self.name


class FakeOnigmo:
    def __getattr__(self, name):
        return Sym(name)


events = []  # ("case", dict) and ("event", name, value) in script order
ENCODING = "UTF-8"


def cc_to_cb(s, cc):
    """Char count to byte count, as testpy.py does for the UTF-8 encoding."""
    if cc == -1:
        return -1
    chars = list(s)
    if cc > len(chars):
        raise IndexError
    return len("".join(chars[:cc]).encode("utf-8"))


def record(pattern, target, s_from, s_to, mem, not_match, searchtype=0,
           gpos=-1, startpos=0, endpos=-1, syn=None, opt=None, err=None, execerr=None):
    if not isinstance(target, bytes):
        s_from, s_to = cc_to_cb(target, s_from), cc_to_cb(target, s_to)
        gpos, startpos, endpos = (cc_to_cb(target, v) for v in (gpos, startpos, endpos))
        target = target.encode("utf-8")
    if not isinstance(pattern, bytes):
        pattern = pattern.encode("utf-8")
    events.append(("case", dict(
        pattern=pattern, target=target, from_=s_from, to=s_to, mem=mem,
        not_match=not_match, kind=searchtype, gpos=gpos, start=startpos, end=endpos,
        syn=syn, opt=opt, err=err, execerr=execerr)))


def load_testpy(path):
    """Compiles only the helpers and main() of testpy.py (not its ctypes setup)."""
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        tree = ast.parse(Path(path).read_text(encoding="utf-8"))
    wanted = {"is_unicode_encoding", "is_ascii_incompatible_encoding", "SearchType",
              "x2", "x3", "n", "main"}
    body = [node for node in tree.body
            if isinstance(node, (ast.FunctionDef, ast.ClassDef)) and node.name in wanted]
    module = ast.Module(body=body, type_ignores=[])
    onigmo = FakeOnigmo()
    scope = {
        "onigmo": onigmo,
        "sys": types.SimpleNamespace(argv=["testpy.py"], exit=lambda *a: None),
        "init": lambda *a: None,
        "nsucc": 0,
        "nfail": 0,
        "nerror": 0,
        "xx": lambda *a, **k: record(*a, **k),
        "onig_encoding": onigmo.ONIG_ENCODING_UTF8,
        "syntax_default": Sym("SYNTAX_DEFAULT"),
        "get_encoding_name": lambda enc: ENCODING,
        "print": lambda *a, **k: None,
        "exit": lambda *a: None,
    }
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")  # "\g" etc. stay as backslash + letter, as in testpy.py
        code = compile(module, str(path), "exec")
    exec(code, scope)
    return scope


def rust_bytes(data):
    out = []
    for b in data:
        if b == 0x22 or b == 0x5C:
            out.append("\\" + chr(b))
        elif 0x20 <= b < 0x7F:
            out.append(chr(b))
        else:
            out.append("\\x%02x" % b)
    return "".join(out)


def long_expr(data):
    """Handles patterns like X * n + Y + Z * n by splitting into runs."""
    runs = [(m.group(0)[:1], len(m.group(0))) for m in re.finditer(rb"(.)\1*", data, re.S)]
    if len(data) > 200:
        parts = []
        for ch, count in runs:
            if count > 20:
                parts.append('Bytes::Rep(b"%s", %d)' % (rust_bytes(ch), count))
            else:
                parts.append('Bytes::Lit(b"%s")' % rust_bytes(ch * count))
        return "Bytes::Cat(&[" + ", ".join(parts) + "])"
    return 'Bytes::Lit(b"%s")' % rust_bytes(data)


def parse_header(onigmo_h):
    """Evaluates the #define integers of onigmo.h that the cases refer to."""
    defs = {}
    for line in Path(onigmo_h).read_text(encoding="utf-8").splitlines():
        m = re.match(r"#define\s+(ONIG(?:ERR)?_[A-Z0-9_]+)\s+(.+?)\s*(?:/\*.*)?$", line)
        if m:
            defs[m.group(1)] = m.group(2)

    def value(name):
        expr = defs[name]
        expr = re.sub(r"\b(\d+)U\b", r"\1", expr)
        expr = re.sub(r"\bONIG(?:ERR)?_[A-Z0-9_]+\b", lambda mm: str(value(mm.group(0))), expr)
        return eval(expr, {"__builtins__": {}})  # integer arithmetic from onigmo.h

    return value


def main():
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    onigmo_dir = Path(sys.argv[1])
    scope = load_testpy(onigmo_dir / "testpy.py")
    try:
        scope["main"]()
    except SystemExit:
        pass
    value = parse_header(onigmo_dir / "onigmo.h")

    used = set()
    lines = []
    kinds = {0: "Kind::Forward", 1: "Kind::Backward", 2: "Kind::Match"}
    syntaxes = {None: None, "SYNTAX_DEFAULT": None, "ONIG_SYNTAX_PERL": "Syntax::Perl",
                "ONIG_SYNTAX_GREP": "Syntax::Grep"}
    for event in events:
        if event[0] == "event":
            _, name, arg = event
            fn = {"onig_set_match_stack_limit_size": "StackLimit",
                  "onig_set_parse_depth_limit": "ParseDepthLimit"}[name]
            lines.append("    Item::%s(%d)," % (fn, arg))
            continue
        c = event[1]
        fields = ["pattern: %s" % long_expr(c["pattern"]), "target: %s" % long_expr(c["target"])]
        if not c["not_match"]:
            fields += ["from: %d" % c["from_"], "to: %d" % c["to"]]
        if c["mem"]:
            fields.append("mem: %d" % c["mem"])
        if c["not_match"]:
            fields.append("not_match: true")
        if c["kind"]:
            fields.append("kind: %s" % kinds[c["kind"]])
        for key in ("gpos", "start", "end"):
            default = 0 if key == "start" else -1
            if c[key] != default:
                fields.append("%s: %d" % (key, c[key]))
        syn = syntaxes[repr(c["syn"]) if c["syn"] is not None else None]
        if syn:
            fields.append("syn: %s" % syn)
        if c["opt"] is not None and repr(c["opt"]) != "ONIG_OPTION_DEFAULT":
            used.update(repr(c["opt"]).split(" | "))
            fields.append("opt: %s" % repr(c["opt"]))
        for key, field in (("err", "err"), ("execerr", "execerr")):
            if c[key] is not None and repr(c[key]) != "ONIG_NORMAL":
                used.add(repr(c[key]))
                fields.append("%s: %s" % (field, repr(c[key])))
        lines.append("    Item::Case(Case { %s, ..Case::DEFAULT })," % ", ".join(fields))

    print("// @generated by tools/gen_onigmo_suite.py from Onigmo's testpy.py (UTF-8 branch).")
    print("// Do not edit by hand; see the script for how to regenerate it.")
    print("//")
    print("// Positions are byte offsets (testpy.py counts characters; the script converts).")
    print()
    print("use super::{Bytes, Case, Item, Kind};")
    print("use crate::ffi::Syntax;")
    print()
    for name in sorted(used):
        print("pub(super) const %s: i64 = %d;" % (name, value(name)))
    print()
    print("#[rustfmt::skip]")
    print("pub(super) static ITEMS: &[Item] = &[")
    print("\n".join(lines))
    print("];")


if __name__ == "__main__":
    main()
