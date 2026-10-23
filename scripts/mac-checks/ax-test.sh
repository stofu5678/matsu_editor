#!/usr/bin/env bash
# An example check for .github/workflows/mac-check.yml: drives the built `matsu` through the
# Accessibility API (crates/matsu/tests/macos_ax.rs) and saves pictures in $MATSU_E2E_SCREENS_DIR.
set -euo pipefail
cargo test --release --locked -p matsu --test macos_ax -- --ignored --test-threads=1 --nocapture
