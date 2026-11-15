"""Summarizes Clippy's JSON messages by lint, production code and tests apart (same test rule as ownership-metrics.py).

clippy-summary.py <clippy json> <checkout>
"""
import collections
import importlib.util
import json
import pathlib
import sys

json_path, root = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
spec = importlib.util.spec_from_file_location("om", pathlib.Path(__file__).with_name("ownership-metrics.py"))
# Reuse the test classification without running the metrics: read the helpers from the source.
src = pathlib.Path(__file__).with_name("ownership-metrics.py").read_text()
helpers = src.split("totals = {")[0].replace("root = pathlib.Path(sys.argv[1])", "")
ns: dict = {}
exec(compile(helpers, "ownership-metrics-helpers", "exec"), ns)

test_lines: dict[str, set[int]] = {}


def is_test(file: str, line: int) -> bool:
    rel = pathlib.Path(file)
    if ns["is_test_file"](rel):
        return True
    if file not in test_lines:
        text = (root / file).read_text(encoding="utf-8", errors="replace")
        test_lines[file] = {i + 1 for i, (_, t) in enumerate(ns["split_lines"](text)) if t}
    return line in test_lines[file]


by_lint = {"prod": collections.Counter(), "test": collections.Counter()}
by_file = collections.Counter()
examples: dict[str, list[str]] = collections.defaultdict(list)
seen = set()
for raw in json_path.read_text().splitlines():
    try:
        m = json.loads(raw)
    except json.JSONDecodeError:
        continue
    if m.get("reason") != "compiler-message":
        continue
    msg = m["message"]
    code = (msg.get("code") or {}).get("code") or ""
    if not code.startswith("clippy::"):
        continue
    spans = [s for s in msg.get("spans", []) if s.get("is_primary")]
    if not spans:
        continue
    s = spans[0]
    key = (code, s["file_name"], s["line_start"], s["column_start"])
    if key in seen:  # the same spot reported for the lib and its test target
        continue
    seen.add(key)
    kind = "test" if is_test(s["file_name"], s["line_start"]) else "prod"
    by_lint[kind][code] += 1
    if kind == "prod":
        by_file[s["file_name"]] += 1
        if len(examples[code]) < 3:
            examples[code].append(f"{s['file_name']}:{s['line_start']}")

lints = sorted(set(by_lint["prod"]) | set(by_lint["test"]), key=lambda c: -by_lint["prod"][c])
print(f"{'lint':45} {'prod':>6} {'tests':>6}")
for c in lints:
    print(f"{c:45} {by_lint['prod'][c]:6} {by_lint['test'][c]:6}")
print(f"{'total':45} {sum(by_lint['prod'].values()):6} {sum(by_lint['test'].values()):6}")
print("\nproduction files with the most findings:")
for f, n in by_file.most_common(10):
    print(f"  {n:4}  {f}")
print("\nexamples:")
for c in lints:
    if examples[c]:
        print(f"  {c}: {', '.join(examples[c])}")
