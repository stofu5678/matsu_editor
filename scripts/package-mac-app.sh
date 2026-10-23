#!/usr/bin/env bash
# Builds `MATSU Editor.app` for macOS from the release programs, for .github/workflows/package.yml and
# scripts/mac-checks/pack-build.sh. Opening a bare executable from Finder starts Terminal.app, which
# shows a black window; a bundle opens the program as a normal application.
#
#   scripts/package-mac-app.sh <folder with matsu and matsu-script> <folder to put "MATSU Editor.app" in>
#
# Layout:
#   MATSU Editor.app/Contents/Info.plist
#   MATSU Editor.app/Contents/MacOS/matsu          (CFBundleExecutable)
#   MATSU Editor.app/Contents/MacOS/matsu-script   (the editor looks for it next to its own executable:
#                                               crates/matsu-plugin/src/session.rs)
#   MATSU Editor.app/Contents/Resources/matsu_editor.icns  (made from assets/icons)
#   MATSU Editor.app/Contents/Resources/LICENSE-MIT, LICENSE-APACHE, THIRD_PARTY_NOTICES.txt
# The bundle is signed ad hoc (no Developer ID, no notarization). Needs sips, iconutil, plutil and
# codesign, which macOS has. Run it from the root of the repository.
set -euo pipefail

programs=${1:?folder with matsu and matsu-script}
dest=${2:?folder to put the app in}
app="$dest/MATSU Editor.app"
# The version of the workspace (release-please keeps it; the nightly builds say 0.0.0).
version=$(awk -F'"' '/^version = /{print $2; exit}' Cargo.toml)
test -n "$version"

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$programs/matsu" "$programs/matsu-script" "$app/Contents/MacOS/"
# The licenses travel with the app (it is often moved out of the archive alone).
cp LICENSE-MIT LICENSE-APACHE THIRD_PARTY_NOTICES.txt "$app/Contents/Resources/"

# The icon: the hand-drawn 16 and 32 px pictures and the larger ones of assets/icons; the sizes that
# do not exist (64 and 128) are made from the 256 px one. The 1024 px entry is left out.
iconset=$(mktemp -d)/matsu_editor.iconset
mkdir "$iconset"
icons=assets/icons
cp "$icons/matsu_editor_16.png" "$iconset/icon_16x16.png"
cp "$icons/matsu_editor_32.png" "$iconset/icon_16x16@2x.png"
cp "$icons/matsu_editor_32.png" "$iconset/icon_32x32.png"
sips -z 64 64 "$icons/matsu_editor_256.png" --out "$iconset/icon_32x32@2x.png" >/dev/null
sips -z 128 128 "$icons/matsu_editor_256.png" --out "$iconset/icon_128x128.png" >/dev/null
cp "$icons/matsu_editor_256.png" "$iconset/icon_128x128@2x.png"
cp "$icons/matsu_editor_256.png" "$iconset/icon_256x256.png"
cp "$icons/matsu_editor_512.png" "$iconset/icon_256x256@2x.png"
cp "$icons/matsu_editor_512.png" "$iconset/icon_512x512.png"
iconutil -c icns "$iconset" -o "$app/Contents/Resources/matsu_editor.icns"
rm -rf "$(dirname "$iconset")"

# LSMinimumSystemVersion 11.0: the oldest macOS that runs arm64 programs, and the default deployment
# target of rustc for aarch64-apple-darwin.
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>io.github.stofu5678.matsu-editor</string>
  <key>CFBundleExecutable</key><string>matsu</string>
  <key>CFBundleName</key><string>MATSU Editor</string>
  <key>CFBundleDisplayName</key><string>MATSU Editor</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleIconFile</key><string>matsu_editor</string>
  <key>CFBundleShortVersionString</key><string>${version}</string>
  <key>CFBundleVersion</key><string>${version}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key><string>Text</string>
      <key>CFBundleTypeRole</key><string>Editor</string>
      <key>LSHandlerRank</key><string>Alternate</string>
      <key>LSItemContentTypes</key>
      <array><string>public.plain-text</string><string>public.source-code</string></array>
    </dict>
  </array>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSHumanReadableCopyright</key><string>MIT OR Apache-2.0</string>
</dict>
</plist>
PLIST
plutil -lint "$app/Contents/Info.plist"

# Ad hoc signature (the linker already signs the bare programs the same way). The inner program first.
codesign --force -s - "$app/Contents/MacOS/matsu-script"
codesign --force --deep -s - "$app"
codesign --verify --deep --strict "$app"
echo "built $app ($version)"
