#!/usr/bin/env bash
# Builds the book of docs/help/ with mdBook and checks that its main pages exist. Run it from the root of the
# repository. mdbook must be on PATH (tools/help/install-tools.sh). Prints the folder of the built site (the one
# with index.html at its top) and, when GITHUB_OUTPUT is set, writes it there as `site=`.
set -euo pipefail

mdbook build docs/help
site=target/help
if [ -f target/help/html/index.html ]; then site=target/help/html; fi
for page in index.html about.html print.html; do
  test -f "${site}/${page}" || { echo "::error::${site}/${page} was not built"; exit 1; }
done
echo "site=${site}"
if [ -n "${GITHUB_OUTPUT:-}" ]; then
  echo "site=${site}" >> "$GITHUB_OUTPUT"
fi
