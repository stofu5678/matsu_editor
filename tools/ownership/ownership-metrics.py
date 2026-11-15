"""Counts ownership-related patterns in the Rust code of a checkout, production code and tests apart.

ownership-metrics.py <checkout>. Test code: files under a tests/ folder, files named tests.rs or *_tests.rs, and the
body of `#[cfg(test)] mod ... {` blocks (found by brace counting; strings with braces can skew it a little).
"""
import collections
import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1])
PATTERNS = {
    ".clone()": r"\.clone\(\)",
    ".cloned()": r"\.cloned\(\)",
    ".to_vec()": r"\.to_vec\(\)",
    ".to_owned()": r"\.to_owned\(\)",
    ".to_string()": r"\.to_string\(\)",
    "Arc (type or new)": r"\bArc(<|::new)",
    "Rc (type or new)": r"\bRc(<|::new)",
    "RefCell<": r"\bRefCell<",
    "Mutex< / RwLock<": r"\b(Mutex|RwLock)<",
    "explicit lifetime 'a": r"'(?!static\b)[a-z_][a-z0-9_]*\b(?!')",
    "'static": r"'static\b",
    "unsafe": r"\bunsafe\b",
    ".unwrap()": r"\.unwrap\(\)",
    ".expect(": r"\.expect\(",
    "#[allow(clippy::": r"#\[allow\(clippy::",
    "Box<dyn": r"\bBox<dyn\b",
}
compiled = {k: re.compile(v) for k, v in PATTERNS.items()}


def is_test_file(p: pathlib.Path) -> bool:
    parts = p.parts
    return "tests" in parts or p.name == "tests.rs" or p.name.endswith("_tests.rs")


def split_lines(text: str):
    """Yields (line, in_test) for each line; inline #[cfg(test)] modules are test code."""
    lines = text.split("\n")
    i = 0
    pending_cfg_test = False
    depth = 0
    in_test = False
    for line in lines:
        stripped = line.strip()
        if not in_test:
            if stripped.startswith("#[cfg(test)]"):
                pending_cfg_test = True
                yield line, True
                continue
            if pending_cfg_test and re.match(r"(pub(\(crate\))?\s+)?mod\s+\w+\s*\{", stripped):
                in_test = True
                depth = line.count("{") - line.count("}")
                pending_cfg_test = False
                yield line, True
                if depth <= 0:
                    in_test = False
                continue
            if stripped and not stripped.startswith("#["):
                pending_cfg_test = False
            yield line, False
        else:
            depth += line.count("{") - line.count("}")
            yield line, True
            if depth <= 0:
                in_test = False


def code_line(line: str) -> bool:
    s = line.strip()
    return bool(s) and not s.startswith("//")


totals = {"prod": collections.Counter(), "test": collections.Counter()}
loc = collections.Counter()
per_file_clone = collections.Counter()
if (root / "crates").is_dir():
    files = sorted(p for p in root.glob("crates/**/*.rs")) + sorted(root.glob("xtask/**/*.rs"))
else:  # a single crate (for example one from the cargo registry)
    files = sorted(root.glob("**/*.rs"))
for p in files:
    text = p.read_text(encoding="utf-8", errors="replace")
    test_file = is_test_file(p.relative_to(root))
    for line, in_test in split_lines(text):
        kind = "test" if (test_file or in_test) else "prod"
        if not code_line(line):
            continue
        loc[kind] += 1
        body = line.split("//")[0] if "//" in line and '"' not in line else line
        for name, rx in compiled.items():
            n = len(rx.findall(body))
            if n:
                totals[kind][name] += n
                if kind == "prod" and name == ".clone()":
                    per_file_clone[str(p.relative_to(root))] += n

print(f"files: {len(files)}; code lines (non-blank, non-comment): production {loc['prod']}, tests {loc['test']}")
print(f"{'pattern':26} {'prod':>7} {'/KLOC':>7} {'tests':>7} {'/KLOC':>7}")
for name in PATTERNS:
    a, b = totals["prod"][name], totals["test"][name]
    print(f"{name:26} {a:7} {a * 1000 / max(loc['prod'], 1):7.1f} {b:7} {b * 1000 / max(loc['test'], 1):7.1f}")
print("\nproduction files with the most .clone():")
for f, n in per_file_clone.most_common(12):
    print(f"  {n:4}  {f}")
