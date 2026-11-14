#!/usr/bin/env bash
# Indexes the built book with Pagefind and checks that the Japanese text was indexed (ADR-0031). Run it from the root
# of the repository. pagefind must be on PATH (tools/help/install-tools.sh).
#
#   tools/help/index.sh <site folder printed by tools/help/build.sh>
#
# Only <main> (the page itself, not the sidebar) is indexed; the language is set so that the Japanese text is split
# into words for the index.
set -euo pipefail

site=${1:?site folder}
pagefind --site "$site" --root-selector main --force-language ja
python3 .github/scripts/help-check-index.py "$site" 'このヘルプについて'
