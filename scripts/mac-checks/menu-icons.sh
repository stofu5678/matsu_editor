#!/usr/bin/env bash
# A check for .github/workflows/mac-check.yml (ADR-0024): starts the built `matsu` with
# MATSU_DUMP_MENU_ICONS, which makes it write the menu bar it set (one line per item: the path of
# titles, a tab, and the size of the picture in points or "-"), and fails if no item has a picture.
# The list goes to $MATSU_E2E_SCREENS_DIR/menu-icons.log (uploaded with the artifact), and a
# picture of the screen with the menu bar to menu-icons.png.
#
#   gh workflow run mac-check.yml --ref <branch> -f script=scripts/mac-checks/menu-icons.sh
set -euo pipefail

out="${MATSU_E2E_SCREENS_DIR:-.}"
dump="$out/menu-icons.log"
rm -f "$dump"
work="$(mktemp -d)"
printf 'hello\n' > "$work/a.txt"

MATSU_DUMP_MENU_ICONS="$dump" target/release/matsu "$work/a.txt" &
pid=$!
trap 'kill "$pid" 2>/dev/null || true' EXIT

for _ in $(seq 1 60); do
  [ -s "$dump" ] && break
  sleep 1
done
[ -s "$dump" ] || { echo "the program did not write the menu bar" >&2; exit 1; }
sleep 2
screencapture -x "$out/menu-icons.png" || true

total="$(wc -l < "$dump" | tr -d ' ')"
with="$(grep -vc $'\t-$' "$dump" || true)"
echo "items: $total, with a picture: $with"
cat "$dump"
# The File menu as an example.
grep -E '^ファイル' "$dump" | head -20
[ "$with" -gt 20 ] || { echo "too few items have a picture" >&2; exit 1; }
# The 16x16 points the pictures are given (the PNG files have 32x32 pixels).
grep -q $'\t16x16$' "$dump" || { echo "no picture is 16x16 points" >&2; exit 1; }
