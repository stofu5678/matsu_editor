#!/usr/bin/env bash
# A check for .github/workflows/mac-check.yml that hands out the macOS build: puts the release
# `ume` and `ume-script` (built by the workflow) into $UME_E2E_SCREENS_DIR/ume-macos-arm64.tar.gz,
# laid out like the package: one folder `ume-macos-arm64/` that holds both programs. Download it
# with `gh run download <run id> -n mac-check` to run a pull request's UME on a Mac that has no
# Rust toolchain.
set -euo pipefail
out=${UME_E2E_SCREENS_DIR:?UME_E2E_SCREENS_DIR is not set}
stage=$(mktemp -d)
mkdir "$stage/ume-macos-arm64"
cp target/release/ume target/release/ume-script "$stage/ume-macos-arm64/"
tar -czf "$out/ume-macos-arm64.tar.gz" -C "$stage" ume-macos-arm64
tar -tzf "$out/ume-macos-arm64.tar.gz"
rm -rf "$stage"
