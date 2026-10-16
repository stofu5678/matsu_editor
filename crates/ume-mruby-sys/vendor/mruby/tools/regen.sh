#!/usr/bin/env bash
# Regenerates crates/ume-mruby-sys/vendor/mruby/ from the official mruby source.
# Needs: git and Ruby (3.x) with rake. bison is not needed (mruby ships the parser as y.tab.c).
#
#   tools/regen.sh <work dir> [tag]
#
# The work dir must be empty or absent. The tag defaults to the one in VERSION.
set -euo pipefail

here="$(cd "$(dirname "$0")/.." && pwd)"
work="$(realpath -m "${1:?usage: regen.sh <work dir> [tag]}")"
tag="${2:-$(cat "$here/VERSION")}"

mkdir -p "$work"
git clone --depth 1 --branch "$tag" https://github.com/mruby/mruby.git "$work/mruby"
(cd "$work/mruby" && git rev-parse HEAD)

# Generate the presym tables, the gem init code and the C arrays of the Ruby parts.
(
  cd "$work/mruby"
  UME_MRUBY_BUILD_DIR="$work/build" MRUBY_CONFIG="$here/tools/build_config.rb" rake -j4
)

src="$work/mruby"
out="$(mktemp -d)"
cp "$src/LICENSE" "$src/LEGAL" "$out/"
cp -r "$src/include" "$out/include"
mkdir -p "$out/src"
cp "$src"/src/*.c "$src"/src/*.h "$out/src/"

# Every gem that rake built (the generated gem_init.c files name them).
gems=$(cd "$work/build/mrbgems" && ls -d mruby-*/ | tr -d /)
for gem in $gems; do
  for sub in src core include; do
    if [ -d "$src/mrbgems/$gem/$sub" ]; then
      mkdir -p "$out/gems/$gem"
      cp -r "$src/mrbgems/$gem/$sub" "$out/gems/$gem/$sub"
    fi
  done
  if [ -f "$src/mrbgems/$gem/LICENSE" ]; then
    mkdir -p "$out/gems/$gem"
    cp "$src/mrbgems/$gem/LICENSE" "$out/gems/$gem/LICENSE"
  fi
done
# The compiler: only what the build needs (y.tab.c is the parser that mruby ships generated).
mkdir -p "$out/gems/mruby-compiler/core"
cp "$src"/mrbgems/mruby-compiler/core/{codegen.c,y.tab.c,node.h,lex.def} "$out/gems/mruby-compiler/core/"
cp "$src/mrbgems/mruby-bigint/README-fgmp.md" "$out/gems/mruby-bigint/"

# What rake generated.
mkdir -p "$out/generated/include/mruby/presym" "$out/generated/mrbgems"
cp "$work/build/include/mruby/presym/id.h" "$work/build/include/mruby/presym/table.h" \
  "$out/generated/include/mruby/presym/"
cp "$work/build/mrblib/mrblib.c" "$out/generated/mrblib.c"
cp "$work/build/mrbgems/gem_init.c" "$out/generated/gem_init.c"
for gem in $gems; do
  # mruby-bigint has only core objects and no gem_init.c.
  if [ -f "$work/build/mrbgems/$gem/gem_init.c" ]; then
    cp "$work/build/mrbgems/$gem/gem_init.c" "$out/generated/mrbgems/$gem.c"
  fi
done

# Keep our own files, replace the rest.
for keep in README.md VERSION tools; do cp -r "$here/$keep" "$out/"; done
find "$here" -mindepth 1 -maxdepth 1 -exec rm -rf {} +
cp -r "$out"/. "$here/"
rm -rf "$out"
echo "done: $tag"
