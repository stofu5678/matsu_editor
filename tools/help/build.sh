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
# search.html: print.html without its automatic print call, for the search of the app (it opens it and uses Ctrl+F).
# The script fails if mdBook no longer has the call to remove.
python3 -I tools/help/make-search-page.py "${site}"
echo "site=${site}"
if [ -n "${GITHUB_OUTPUT:-}" ]; then
  echo "site=${site}" >> "$GITHUB_OUTPUT"
fi
