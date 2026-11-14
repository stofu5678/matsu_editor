#!/usr/bin/env bash
# Runs inside a Fedora container (.github/scripts/test-linux-install.sh): installs the .rpm, checks it, removes it,
# and checks that the user's files stay.
#
#   test-rpm-in-fedora.sh <path of the .rpm>
set -euo pipefail

rpm_file=${1:?path of the rpm}
app_id=io.github.stofu5678.matsu-editor
config="$HOME/.config/matsu-editor"
fail() { echo "::error::$*"; exit 1; }

dnf install -y --setopt=install_weak_deps=False "$rpm_file"
test -x /usr/lib/matsu-editor/matsu || fail "matsu is missing"
test "$(readlink -f /usr/bin/matsu)" = /usr/lib/matsu-editor/matsu || fail "the symlink is wrong"
test -f "/usr/share/applications/$app_id.desktop" || fail "the desktop file is missing"
test -f /usr/lib/matsu-editor/help/index.html || fail "the help is not found next to the program"
if ldd /usr/lib/matsu-editor/matsu | grep 'not found'; then fail "matsu misses a shared library"; fi
if ldd /usr/lib/matsu-editor/matsu-script | grep 'not found'; then fail "matsu-script misses a shared library"; fi
set +e
usage=$(/usr/lib/matsu-editor/matsu-script 2>&1)
status=$?
set -e
echo "$usage"
test "$status" = 2 || fail "matsu-script exited with $status, not 2"

mkdir -p "$config/unsaved"
printf '[Window]\nWidth=123\n' > "$config/matsu.ini"
printf 'unsaved text\n' > "$config/unsaved/x.txt"
before=$(cd "$config" && find . -type f -exec sha256sum {} + | sort)
dnf remove -y matsu-editor
for p in /usr/lib/matsu-editor /usr/bin/matsu "/usr/share/applications/$app_id.desktop" /usr/share/doc/matsu-editor /usr/share/matsu-editor; do
  test ! -e "$p" && test ! -L "$p" || fail "$p is still there"
done
after=$(cd "$config" && find . -type f -exec sha256sum {} + | sort)
test "$before" = "$after" || fail "the user's files changed"
echo "The user's files are unchanged (Fedora)."
