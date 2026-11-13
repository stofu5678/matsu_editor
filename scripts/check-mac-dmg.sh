#!/usr/bin/env bash
# Checks the .dmg made by scripts/package-mac-dmg.sh: mounts it read-only, looks for the app (its ad-hoc signature
# must verify) and the link to /Applications, and detaches it.
#
#   scripts/check-mac-dmg.sh <the .dmg>
set -euo pipefail

dmg=${1:?the .dmg to check}
mnt=$(mktemp -d)
attached=
cleanup() {
  if [ -n "$attached" ]; then
    hdiutil detach "$mnt" -force >/dev/null 2>&1 || true
  fi
  rmdir "$mnt" 2>/dev/null || true
}
trap cleanup EXIT

hdiutil attach "$dmg" -readonly -nobrowse -noverify -noautoopen -mountpoint "$mnt"
attached=1

app="$mnt/MATSU Editor.app"
test -d "$app"
codesign --verify --deep --strict --verbose=2 "$app"
test -L "$mnt/Applications"
test "$(readlink "$mnt/Applications")" = /Applications
# The volume holds the app and the link, nothing more (the system adds dot files such as .fseventsd).
test "$(ls "$mnt" | sort | tr '\n' '|')" = "Applications|MATSU Editor.app|"
hdiutil imageinfo "$dmg" | grep -q '^Format: UDZO'

hdiutil detach "$mnt"
attached=
echo "dmg ok: $dmg"
