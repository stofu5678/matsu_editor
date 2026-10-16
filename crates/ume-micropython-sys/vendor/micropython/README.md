# Vendored MicroPython (embed package)

The C sources that MicroPython's own embed port (`ports/embed`) generates, plus four extmod
modules, for the `ume-micropython-sys` crate. Nothing in here is edited by hand, except the two small changes under "Local changes" below.

- Upstream: https://github.com/micropython/micropython
- Version: tag `v1.29.0`, commit `0fd6c573ea815774668bbb16b8e197c8822368b2`
- License: MIT (`LICENSE` here is MicroPython's top-level file, copied unchanged), except the `re`
  engine (see below). The files taken are:
  - `py/*.c`, `py/*.h`: the core (MIT; `emitn*.c` have no header but are part of the core and
    compile to nothing with the native emitter off)
  - `extmod/modjson.c`, `modbinascii.c`, `modheapq.c`, `modplatform.h`, `modre.c` (MIT)
  - `lib/re1.5/charclass.c`, `compilecode.c`, `dumpcode.c`, `recursiveloop.c`, `re1.5.h`: the engine of
    `re` (`modre.c` `#include`s the `.c` files, so `build.rs` does not compile them by themselves).
    BSD-3-Clause, copyright Russ Cox / Google (2007-2009) and Paul Sokolovsky (2014 on); see below
  - `shared/runtime/gchelper.h`, `gchelper_generic.c` (MIT)
  - `port/*` (the embed port's `embed_util.c` and headers, MIT; its `mphalport.c` is not built,
    `csrc/ume_stdio.c` replaces it)
  - `genhdr/*`: headers generated from the configuration (`../../embed/mpconfigport.h`)
- Nothing GPL-family was taken (every added file was checked for it).

## The license of `lib/re1.5`

The re1.5 files say "BSD-style license that can be found in the LICENSE file", and MicroPython has
no such file next to them (its top-level `LICENSE` only lists `/re15 (BSD-3-clause)`). The text
comes from the upstream project instead: `lib/re1.5/LICENSE` is the `LICENSE` of
https://github.com/pfalcon/re1.5 (Paul Sokolovsky's fork of Russ Cox's re1), commit
`dd37aa2a4f4742edcadff6e75124898c71d5dc89`, copied unchanged. It is the BSD-3-Clause text with
"Copyright (c) 2007-2009 Russ Cox, Google Inc.". All five files exist upstream (MicroPython's copy
is an older snapshot with MicroPython's own changes, which are under MicroPython's MIT license).
`compilecode.c`, `dumpcode.c` and `re1.5.h` also carry Paul Sokolovsky's copyright (2014 on) in their
headers, under the same BSD-3-Clause terms: they point to the same `LICENSE` file, which is
upstream's, unchanged, with Russ Cox / Google's line.

## Local changes

Two files differ from MicroPython v1.29.0. Each change is marked with a `UME Editor:` comment.
Upstream master still has both problems as of 2026-10-17.

- `extmod/modre.c`, `re_sub_helper`: the match object handed to a `re.sub` callback lives on the
  C stack (`mp_local_alloc`) and is overwritten by the next match, so a callback that kept it
  crashed the process after `sub()` returned. Each callback call now gets its own match object on
  the GC heap (a copy of the captures; the subject string is kept alive by the match). A string
  replacement still uses the single stack match.
- `lib/re1.5/compilecode.c`, character class: for a pattern like `[a-`, `re += 2` stepped onto the
  end of the pattern and the loop went on reading past it. It now fails to compile
  (`ValueError`) like other malformed patterns.

## Regenerating

The build of the crate (`build.rs`) compiles what is in this directory; it does not need
Python or `make`. To update MicroPython or change the configuration:

```sh
git clone --depth 1 --branch v1.29.0 https://github.com/micropython/micropython /path/to/micropython
rm -rf /path/to/out
make -C crates/ume-micropython-sys/embed MICROPYTHON_TOP=/path/to/micropython PACKAGE_DIR=/path/to/out
rm -rf crates/ume-micropython-sys/embed/build-embed
rm -rf crates/ume-micropython-sys/vendor/micropython/{py,extmod,shared,genhdr,port,lib/re1.5/*.[ch],LICENSE}
cp -r /path/to/out/. crates/ume-micropython-sys/vendor/micropython/   # keeps lib/re1.5/LICENSE
```

`make` runs MicroPython's own Python scripts (`makeqstrdata.py`, `makemoduledefs.py`, ...) with
the installed `python3` and needs a C compiler (it preprocesses the sources to find the names).
`embed/Makefile` adds the extmod modules and `lib/re1.5` and the crate's C files (`csrc/`, which use `MP_QSTR_`
names) to that scan. After regenerating, run `cargo test -p ume-script`.
