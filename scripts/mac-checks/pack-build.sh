#!/usr/bin/env bash
# A check for .github/workflows/mac-check.yml that hands out the macOS build: puts the release
# `matsu` and `matsu-script` (built by the workflow) into $MATSU_E2E_SCREENS_DIR/matsu-macos-arm64.tar.gz,
# laid out like the package: one folder `matsu-macos-arm64/` that holds `MATSU Editor.app` (both programs
# are inside it, scripts/package-mac-app.sh). Download it
# with `gh run download <run id> -n mac-check` to run a pull request's MATSU on a Mac that has no
# Rust toolchain. The licenses and THIRD_PARTY_NOTICES.txt go with the programs, as in the package.
set -euo pipefail
out=${MATSU_E2E_SCREENS_DIR:?MATSU_E2E_SCREENS_DIR is not set}
stage=$(mktemp -d)
mkdir "$stage/matsu-macos-arm64"
scripts/package-mac-app.sh target/release "$stage/matsu-macos-arm64"
cp LICENSE-MIT LICENSE-APACHE THIRD_PARTY_NOTICES.txt "$stage/matsu-macos-arm64/"
tar -czf "$out/matsu-macos-arm64.tar.gz" -C "$stage" matsu-macos-arm64
tar -tzf "$out/matsu-macos-arm64.tar.gz"
rm -rf "$stage"
