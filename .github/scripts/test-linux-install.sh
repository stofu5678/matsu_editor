#!/usr/bin/env bash
# Installs the .deb, checks it, removes it, and proves that the user's files stay (ADR-0032 decision 3). For
# .github/workflows/package.yml, on the Linux runners (it uses sudo and apt-get).
#
#   .github/scripts/test-linux-install.sh <target> <folder with the packages>
#
# Then it looks inside the .rpm (rpm -qlp, rpm -K) and, if docker is there, installs it in a Fedora container,
# removes it, and makes the same user-data check there.
set -euo pipefail

target=${1:?target (linux-x64 or linux-arm64)}
dist=${2:?folder with the packages}
dist=$(cd "$dist" && pwd)
app_id=io.github.stofu5678.matsu-editor
config="${XDG_CONFIG_HOME:-$HOME/.config}/matsu-editor"

fail() { echo "::error::$*"; exit 1; }

# --- the .deb ---
dpkg-deb --info "$dist/matsu-$target.deb"
dpkg-deb --contents "$dist/matsu-$target.deb"
# Not apt-install.sh: its second step (--no-download) rejects the path of a local .deb. The libraries are mostly
# there already (the build installed their -dev packages); a time limit keeps a slow mirror from stalling the job.
sudo timeout --kill-after=10s 20m apt-get install -y --no-install-recommends "$dist/matsu-$target.deb"

echo "--- the installed files"
test -x /usr/lib/matsu-editor/matsu || fail "matsu is missing"
test -x /usr/lib/matsu-editor/matsu-script || fail "matsu-script is missing"
test -L /usr/bin/matsu || fail "/usr/bin/matsu is not a symlink"
test "$(readlink -f /usr/bin/matsu)" = /usr/lib/matsu-editor/matsu || fail "/usr/bin/matsu points elsewhere"
test -f "/usr/share/applications/$app_id.desktop" || fail "the desktop file is missing"
grep -qx 'MimeType=text/plain;' "/usr/share/applications/$app_id.desktop" || fail "MimeType is wrong"
test -f "/usr/share/metainfo/$app_id.metainfo.xml" || fail "the metainfo file is missing"
# The help (ADR-0031): the files are in /usr/share/matsu-editor/help, and the editor finds them through the symlink
# next to the program (its exe dir is /usr/lib/matsu-editor).
test -f /usr/share/matsu-editor/help/index.html || fail "the help is missing"
test -f /usr/share/matsu-editor/help/pagefind/pagefind-entry.json || fail "the search index of the help is missing"
test -L /usr/lib/matsu-editor/help || fail "/usr/lib/matsu-editor/help is not a symlink"
test -f /usr/lib/matsu-editor/help/index.html || fail "the help is not found next to the program"
for size in 16x16 32x32 48x48 256x256 512x512; do
  test -f "/usr/share/icons/hicolor/$size/apps/$app_id.png" || fail "the $size icon is missing"
done
test -f "/usr/share/icons/hicolor/scalable/apps/$app_id.svg" || fail "the svg icon is missing"
for f in LICENSE-MIT LICENSE-APACHE THIRD_PARTY_NOTICES.txt copyright; do
  test -f "/usr/share/doc/matsu-editor/$f" || fail "$f is missing"
done
# No settings file may come with the package (a matsu.ini next to the program would make it portable).
if find /usr/lib/matsu-editor /usr/bin/matsu -name 'matsu.ini' -o -name plugins -o -name unsaved | grep -q .; then
  fail "the package carries a settings file or folder"
fi

echo "--- the programs start"
# Every shared library is found (the package depends on them).
for exe in /usr/lib/matsu-editor/matsu /usr/lib/matsu-editor/matsu-script; do
  if ldd "$exe" | grep 'not found'; then fail "$exe misses a shared library"; fi
done
# matsu has no --version (it opens a window), so run the script host: it prints its usage and exits with 2.
set +e
usage=$(/usr/lib/matsu-editor/matsu-script 2>&1)
status=$?
set -e
echo "$usage"
test "$status" = 2 || fail "matsu-script exited with $status, not 2"
echo "$usage" | grep -q 'usage: matsu-script' || fail "matsu-script printed no usage"

echo "--- the user's files"
mkdir -p "$config/unsaved"
printf '[Window]\nWidth=123\n' > "$config/matsu.ini"
printf 'unsaved text\n' > "$config/unsaved/x.txt"
before=$(cd "$config" && find . -type f -exec sha256sum {} + | sort)

sudo apt-get remove -y matsu-editor

echo "--- after removal"
for p in /usr/lib/matsu-editor /usr/bin/matsu "/usr/share/applications/$app_id.desktop" \
  "/usr/share/metainfo/$app_id.metainfo.xml" /usr/share/doc/matsu-editor /usr/share/matsu-editor; do
  test ! -e "$p" && test ! -L "$p" || fail "$p is still there"
done
if ls /usr/share/icons/hicolor/*/apps/"$app_id".* 2>/dev/null; then fail "an icon is still there"; fi
after=$(cd "$config" && find . -type f -exec sha256sum {} + | sort)
test "$before" = "$after" || fail "the user's files changed"
test -f "$config/matsu.ini" && test -f "$config/unsaved/x.txt" || fail "a user file is gone"
echo "The user's files are unchanged."

# --- the .rpm ---
echo "--- the .rpm"
rpm -qlp "$dist/matsu-$target.rpm"
rpm -qp --requires "$dist/matsu-$target.rpm"
rpm -qpi "$dist/matsu-$target.rpm"
# No signature yet (ADR-0032 stage A): -K checks the digests.
rpm -K "$dist/matsu-$target.rpm" | tee /dev/stderr | grep -q 'digests OK' || fail "rpm -K: the digests are not OK"
# rpm must have found the shared libraries itself (it has no list from us).
rpm -qp --requires "$dist/matsu-$target.rpm" | grep -q 'libc\.so\.6' || fail "the rpm has no automatic Requires"
listing=$(rpm -qlp "$dist/matsu-$target.rpm")
for p in /usr/bin/matsu /usr/lib/matsu-editor/matsu /usr/lib/matsu-editor/matsu-script \
  /usr/lib/matsu-editor/help /usr/share/matsu-editor/help/index.html \
  "/usr/share/applications/$app_id.desktop" "/usr/share/metainfo/$app_id.metainfo.xml"; do
  echo "$listing" | grep -qx "$p" || fail "the rpm lacks $p"
done
if echo "$listing" | grep -E '/(matsu\.ini|unsaved|plugins)(/|$)'; then fail "the rpm carries a settings file"; fi

# Install it in Fedora, where the library names are resolved for real.
if command -v docker >/dev/null; then
  echo "--- the .rpm in Fedora"
  docker run --rm -v "$dist:/dist:ro" -v "$PWD/.github/scripts/test-rpm-in-fedora.sh:/test.sh:ro" \
    fedora:43 bash /test.sh "/dist/matsu-$target.rpm"
else
  echo "::warning::docker is not here; the .rpm was not installed in Fedora"
fi
