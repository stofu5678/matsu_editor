#!/usr/bin/env bash
# An example check for .github/workflows/mac-check.yml: drives the built `ume` through the
# Accessibility API (crates/ume/tests/macos_ax.rs) and saves pictures in $UME_E2E_SCREENS_DIR.
set -euo pipefail
cargo test --release --locked -p ume --test macos_ax -- --ignored --test-threads=1 --nocapture
