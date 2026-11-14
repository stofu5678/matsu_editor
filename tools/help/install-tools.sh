#!/usr/bin/env bash
# Installs mdBook and Pagefind (release binaries of pinned versions, checked against their SHA-256) for the build of the
# help in docs/help/ (ADR-0031). The `help` job of ci.yml and the `help` job of package.yml both call it, so the pins
# are in this one place. Linux x64 only (the book is built once, on ubuntu-24.04, and is the same everywhere).
#
#   tools/help/install-tools.sh <folder to put mdbook and pagefind in>
#
# With GITHUB_PATH set (a GitHub Actions step) the folder is added to PATH for the next steps. The names of the
# variables avoid the prefixes MDBOOK_ and PAGEFIND_, which the tools read as settings.
set -euo pipefail

HELP_MDBOOK_VERSION=0.5.4
# SHA-256 of mdbook-v0.5.4-x86_64-unknown-linux-gnu.tar.gz (the digest on the GitHub release).
HELP_MDBOOK_SHA256=3f28de05dafca9d0f2eab99c662116b0e37b89b1d96a08f8f430b9eeae958cd7
HELP_PAGEFIND_VERSION=1.6.0
# SHA-256 of pagefind-v1.6.0-x86_64-unknown-linux-musl.tar.gz.
HELP_PAGEFIND_SHA256=004d40c9a644c995dadc42b8031196c6f17e7a6dacf9a6c15c63286dd6ead5f2

bin=${1:?folder for mdbook and pagefind}
mkdir -p "$bin"
bin=$(cd "$bin" && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

curl --proto '=https' --tlsv1.2 -sSfL --retry 3 -o "$tmp/mdbook.tar.gz" \
  "https://github.com/rust-lang/mdBook/releases/download/v${HELP_MDBOOK_VERSION}/mdbook-v${HELP_MDBOOK_VERSION}-x86_64-unknown-linux-gnu.tar.gz"
echo "${HELP_MDBOOK_SHA256}  $tmp/mdbook.tar.gz" | sha256sum --check
tar -xzf "$tmp/mdbook.tar.gz" -C "$bin" mdbook
curl --proto '=https' --tlsv1.2 -sSfL --retry 3 -o "$tmp/pagefind.tar.gz" \
  "https://github.com/Pagefind/pagefind/releases/download/v${HELP_PAGEFIND_VERSION}/pagefind-v${HELP_PAGEFIND_VERSION}-x86_64-unknown-linux-musl.tar.gz"
echo "${HELP_PAGEFIND_SHA256}  $tmp/pagefind.tar.gz" | sha256sum --check
tar -xzf "$tmp/pagefind.tar.gz" -C "$bin" pagefind

if [ -n "${GITHUB_PATH:-}" ]; then
  echo "$bin" >> "$GITHUB_PATH"
fi
"$bin/mdbook" --version
"$bin/pagefind" --version
