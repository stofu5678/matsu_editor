#!/usr/bin/env bash
# Downloads the screenshots of the E2E workflow (.github/workflows/e2e.yml, job `screens`).
#
#   scripts/e2e-screens.sh <PR number | run id | branch name> [output folder]
#
# * A PR number (up to 6 digits) or a branch name: the latest finished run of e2e.yml on that PR's
#   branch (or the branch).
# * A run id (7 digits or more): that run.
#
# The pictures go to <output folder>/<run id>/<artifact name>/<platform>-<scenario>.png. The
# default folder is outside the repository: $UME_E2E_SCREENS_OUT, or ${TMPDIR:-/tmp}/ume-e2e-screens.
# Needs the GitHub CLI (`gh`), logged in, run inside a clone of the repository.
set -euo pipefail

if [ $# -lt 1 ] || [ $# -gt 2 ]; then
  echo "usage: $0 <PR number | run id | branch name> [output folder]" >&2
  exit 2
fi
target=$1
out_root=${2:-${UME_E2E_SCREENS_OUT:-${TMPDIR:-/tmp}/ume-e2e-screens}}

if [[ $target =~ ^[0-9]{7,}$ ]]; then
  run=$target
else
  if [[ $target =~ ^[0-9]+$ ]]; then
    branch=$(gh pr view "$target" --json headRefName --jq .headRefName)
  else
    branch=$target
  fi
  run=$(gh run list --workflow e2e.yml --branch "$branch" --status completed --limit 1 \
    --json databaseId --jq '.[0].databaseId // empty')
  if [ -z "$run" ]; then
    echo "no finished run of e2e.yml on the branch $branch" >&2
    exit 1
  fi
fi

dir="$out_root/$run"
rm -rf "$dir"
mkdir -p "$dir"
echo "run $run -> $dir"
gh run download "$run" --pattern 'e2e-screens-*' --dir "$dir"
find "$dir" -name '*.png' | sort
