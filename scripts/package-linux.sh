#!/usr/bin/env bash
# Builds the .deb and the .rpm of MATSU Editor from the release programs, for .github/workflows/package.yml
# (ADR-0032 stage A). Run it from the root of the repository.
#
#   scripts/package-linux.sh <target: linux-x64 | linux-arm64> <folder with matsu and matsu-script> <out folder>
#
# It writes <out>/matsu-<target>.deb and <out>/matsu-<target>.rpm. Needs dpkg-deb and dpkg-shlibdeps (dpkg-dev),
# rpmbuild (rpm), and appstreamcli and desktop-file-validate if you want the metadata checked (CI does).
#
# Layout (the same in both packages):
#   /usr/lib/matsu-editor/matsu, matsu-script   (the editor looks for matsu-script next to its own executable:
#                                                crates/matsu-plugin/src/session.rs; std::env::current_exe is the
#                                                resolved path on Linux, so it works through the symlink)
#   /usr/bin/matsu                              symlink to the above
#   /usr/share/applications, metainfo, icons/hicolor
#   /usr/share/doc/matsu-editor/                licenses and notices
# The packages have no install or removal scripts: they never touch the user's settings (ADR-0032 decision 3).
set -euo pipefail

target=${1:?target (linux-x64 or linux-arm64)}
programs=${2:?folder with matsu and matsu-script}
out=${3:?folder for the packages}
app_id=io.github.stofu5678.matsu-editor

case "$target" in
  linux-x64) deb_arch=amd64 ;;
  linux-arm64) deb_arch=arm64 ;;
  *) echo "unknown target: $target" >&2; exit 2 ;;
esac

version=$(awk -F'"' '/^version = /{print $2; exit}' Cargo.toml)
test -n "$version"
date=$(date -u +%F)
mkdir -p "$out"
out=$(cd "$out" && pwd)
programs=$(cd "$programs" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
root="$work/root"

# --- the tree ---
install -D -m 755 "$programs/matsu" "$root/usr/lib/matsu-editor/matsu"
install -D -m 755 "$programs/matsu-script" "$root/usr/lib/matsu-editor/matsu-script"
mkdir -p "$root/usr/bin"
ln -s ../lib/matsu-editor/matsu "$root/usr/bin/matsu"
install -D -m 644 "packaging/linux/$app_id.desktop" "$root/usr/share/applications/$app_id.desktop"
mkdir -p "$root/usr/share/metainfo"
sed -e "s/@VERSION@/$version/" -e "s/@DATE@/$date/" "packaging/linux/$app_id.metainfo.xml.in" \
  > "$root/usr/share/metainfo/$app_id.metainfo.xml"
chmod 644 "$root/usr/share/metainfo/$app_id.metainfo.xml"
# The 16 and 32 px pictures are the simplified designs for small sizes (assets/icons/README.md).
for size in 16 32 48 256 512; do
  install -D -m 644 "assets/icons/matsu_editor_$size.png" \
    "$root/usr/share/icons/hicolor/${size}x${size}/apps/$app_id.png"
done
install -D -m 644 assets/icons/matsu_editor.svg "$root/usr/share/icons/hicolor/scalable/apps/$app_id.svg"
doc="$root/usr/share/doc/matsu-editor"
install -d "$doc"
install -m 644 LICENSE-MIT LICENSE-APACHE THIRD_PARTY_NOTICES.txt "$doc/"
cat > "$doc/copyright" <<COPYRIGHT
MATSU Editor is licensed under MIT OR Apache-2.0 (LICENSE-MIT, LICENSE-APACHE in this folder). The icon has the
same license. THIRD_PARTY_NOTICES.txt has the notices and licenses of the code the programs include.
https://github.com/stofu5678/matsu_editor
COPYRIGHT
chmod 644 "$doc/copyright"

# --- check the metadata ---
if command -v desktop-file-validate >/dev/null; then
  desktop-file-validate "$root/usr/share/applications/$app_id.desktop"
fi
if command -v appstreamcli >/dev/null; then
  appstreamcli validate --no-net "$root/usr/share/metainfo/$app_id.metainfo.xml"
fi

# --- .deb ---
# dpkg-shlibdeps turns the shared libraries the programs link into Depends: lines (Debian/Ubuntu package names).
mkdir "$work/shlibs"
(
  cd "$work/shlibs"
  mkdir debian
  printf 'Source: matsu-editor\n\nPackage: matsu-editor\nArchitecture: any\n' > debian/control
  dpkg-shlibdeps -O -e"$root/usr/lib/matsu-editor/matsu" -e"$root/usr/lib/matsu-editor/matsu-script" \
    > "$work/shlibs.out"
)
depends=$(sed -n 's/^shlibs:Depends=//p' "$work/shlibs.out")
echo "Depends: $depends"
test -n "$depends"
size=$(du -sk "$root" | cut -f1)
mkdir "$root/DEBIAN"
cat > "$root/DEBIAN/control" <<CONTROL
Package: matsu-editor
Version: $version
Architecture: $deb_arch
Maintainer: stofu5678 <stofu5678@users.noreply.github.com>
Installed-Size: $size
Depends: $depends
Recommends: libvulkan1, libwayland-client0, fonts-noto-cjk
Section: editors
Priority: optional
Homepage: https://github.com/stofu5678/matsu_editor
Description: A text editor that feels like SAKURA Editor
 MATSU Editor is a text editor written in Rust. It brings the key bindings
 and the behavior of SAKURA Editor to macOS, Windows and Linux.
CONTROL
dpkg-deb --root-owner-group -Zxz --build "$root" "$out/matsu-$target.deb"
rm -rf "$root/DEBIAN"

# --- .rpm ---
# rpm finds Requires: itself, from the sonames of the programs (Fedora and others resolve them by capability).
mkdir -p "$work/rpm"
sed -e "s|@VERSION@|$version|" -e "s|@ROOT@|$root|" packaging/linux/matsu-editor.spec.in > "$work/matsu-editor.spec"
rpmbuild -bb --define "_topdir $work/rpm" --define "_rpmdir $work/rpms" "$work/matsu-editor.spec"
rpm_file=$(find "$work/rpms" -name '*.rpm')
test -n "$rpm_file"
cp "$rpm_file" "$out/matsu-$target.rpm"
ls -l "$out/matsu-$target".*
