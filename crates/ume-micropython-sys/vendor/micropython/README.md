# Vendored MicroPython (embed package)

The C sources that MicroPython's own embed port (`ports/embed`) generates, plus three extmod
modules, for the `ume-micropython-sys` crate. Nothing in here is edited by hand.

- Upstream: https://github.com/micropython/micropython
- Version: tag `v1.29.0`, commit `0fd6c573ea815774668bbb16b8e197c8822368b2`
- License: MIT (`LICENSE` here is MicroPython's top-level file, copied unchanged). Every file
  that was taken is MicroPython's own and under that license. The files taken are:
  - `py/*.c`, `py/*.h`: the core (MIT; `emitn*.c` have no header but are part of the core and
    compile to nothing with the native emitter off)
  - `extmod/modjson.c`, `modbinascii.c`, `modheapq.c`, `modplatform.h` (MIT)
  - `shared/runtime/gchelper.h`, `gchelper_generic.c` (MIT)
  - `port/*` (the embed port's `embed_util.c` and headers, MIT; its `mphalport.c` is not built,
    `csrc/ume_stdio.c` replaces it)
  - `genhdr/*`: headers generated from the configuration (`../../embed/mpconfigport.h`)
- Left out on purpose: the `re` module. Its engine (`lib/re1.5`, "BSD-3-clause" in MicroPython's
  LICENSE summary) says "BSD-style license that can be found in the LICENSE file" in each file,
  but MicroPython has no such file next to it, so there is no license text to carry. Nothing
  under `lib/` is used. Nothing GPL-family was taken.

## Regenerating

The build of the crate (`build.rs`) compiles what is in this directory; it does not need
Python or `make`. To update MicroPython or change the configuration:

```sh
git clone --depth 1 --branch v1.29.0 https://github.com/micropython/micropython /path/to/micropython
rm -rf /path/to/out
make -C crates/ume-micropython-sys/embed MICROPYTHON_TOP=/path/to/micropython PACKAGE_DIR=/path/to/out
rm -rf crates/ume-micropython-sys/embed/build-embed
rm -rf crates/ume-micropython-sys/vendor/micropython/{py,extmod,shared,genhdr,port,LICENSE}
cp -r /path/to/out/. crates/ume-micropython-sys/vendor/micropython/
```

`make` runs MicroPython's own Python scripts (`makeqstrdata.py`, `makemoduledefs.py`, ...) with
the installed `python3` and needs a C compiler (it preprocesses the sources to find the names).
`embed/Makefile` adds the extmod modules and the crate's C files (`csrc/`, which use `MP_QSTR_`
names) to that scan. After regenerating, run `cargo test -p ume-script`.
