#!/usr/bin/env bash
# Installs Ubuntu packages on a GitHub runner, for the workflows in .github/workflows:
#
#   .github/scripts/apt-install.sh <package>...
#
# It runs `apt-get update`, then `apt-get install --no-install-recommends -y <package>...`. The steps that use the
# network have a time limit and up to three tries. On 2026-11-06 `apt-get update` (package.yml, linux-x64) printed
# nothing after the InRelease files of azure.archive.ubuntu.com for six hours, until GitHub cancelled the job at its
# default limit. apt's own timeouts (Acquire::http::Timeout, which GitHub's image sets to 15 seconds) did not end
# it. On other days the same mirror sent packages at about 44 kB/s, and an install took up to 20 minutes but
# finished.
set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "usage: $0 <package>..." >&2
  exit 2
fi

# Runs a command as root with a time limit, up to three times. A try that is cut off leaves its partial downloads
# in apt's folders, and the next try goes on from them.
retry() { # limit command...
  local limit=$1 try
  shift
  for try in 1 2 3; do
    # timeout runs as root, so that it can end apt-get and the download methods it starts (they share its process
    # group, which timeout signals). SIGKILL follows 10 seconds after SIGTERM.
    if sudo timeout --kill-after=10s "$limit" "$@"; then
      return 0
    fi
    echo "::warning::'$*' failed or ran longer than $limit (try $try of 3)"
    sleep 10
  done
  return 1
}

retry 5m apt-get update
# Download first, then install from the downloaded files only (--no-download): the install has nothing to wait for
# on the network, and is never cut off in the middle of dpkg (which would leave dpkg's database half done for the
# next try). Three tries of 15 minutes fetch about 120 MB at the slowest speed seen (44 kB/s).
retry 15m apt-get install --no-install-recommends -y --download-only "$@"
sudo apt-get install --no-install-recommends -y --no-download "$@"
