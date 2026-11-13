#!/usr/bin/env bash
# Makes the .dmg of MATSU Editor from `MATSU Editor.app` (ADR-0032 stage A), for .github/workflows/package.yml.
#
#   scripts/package-mac-dmg.sh <folder that holds "MATSU Editor.app"> <the .dmg to write>
#
# The .dmg holds the app and a link to /Applications, and nothing else: the user drags the app onto the link.
# A .dmg installs nothing, so it never touches ~/Library/Application Support/matsu-editor (ADR-0032 decision 3).
# The volume name is "MATSU Editor" (ADR-0021). The format is UDZO (zlib, read-only): every macOS that
# runs on arm64 reads it, and so do the common tools that unpack .dmg files on other systems (ULFO is
# smaller but needs macOS 10.11+ and is not read by them). The app keeps its ad-hoc signature (ditto copies the
# bundle as it is); the .dmg itself is not signed. Needs hdiutil and ditto, which macOS has. No secret, and it
# works on the self-hosted VM and on GitHub's macos-15 alike.
set -euo pipefail

src=${1:?folder that holds "MATSU Editor.app"}
out=${2:?the .dmg to write}
app="$src/MATSU Editor.app"
test -d "$app"

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
ditto "$app" "$stage/MATSU Editor.app"
ln -s /Applications "$stage/Applications"

rm -f "$out"
# hdiutil sometimes answers "Resource busy" right after the files were written (seen on CI VMs); try again.
for attempt in 1 2 3; do
  if hdiutil create -volname "MATSU Editor" -srcfolder "$stage" -fs HFS+ -format UDZO -ov "$out"; then
    break
  fi
  if [ "$attempt" = 3 ]; then
    echo "hdiutil create failed" >&2
    exit 1
  fi
  rm -f "$out"
  sleep 5
done
