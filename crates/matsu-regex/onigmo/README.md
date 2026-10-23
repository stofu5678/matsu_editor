# Vendored Onigmo

This directory is a source copy of Onigmo, the regular expression engine
inside Sakura Editor's bregonig.dll. It is built by `../build.rs`. Do not edit
these files: changes go to a fork of Onigmo first (ADR-0006), and then the
copy is refreshed.

- Upstream: https://github.com/k-takata/Onigmo
- Commit: `1d7ee878b3e4` (2024-06-15, version 6.2.0)
- License: the BSD 2-Clause license in `COPYING`

## Files

Copied unchanged from the upstream tree (only what the UTF-8-only build needs):

- `regcomp.c`, `regenc.c`, `regerror.c`, `regexec.c`, `regparse.c`,
  `regsyntax.c`, `regversion.c`, `st.c`
- `onigmo.h`, `regint.h`, `regparse.h`, `regenc.h`, `st.h`
- `enc/ascii.c`, `enc/unicode.c`, `enc/utf_8.c`
- `enc/unicode/casefold.h`, `enc/unicode/name2ctype.h` (Unicode 12.1 tables,
  generated upstream by `tool/enc-unicode.rb`)
- `COPYING`

Left out: the other encodings under `enc/`, the POSIX and GNU APIs
(`regposix.c`, `reggnu.c`, ...), `regext.c`, `regtrav.c`, the tests, the build
files for autotools and nmake, and the docs.

## How it is built

`../build.rs` compiles these files and `../c/shim.c` with the `cc` crate.
`../c/config.h` replaces Onigmo's `config.h` (the upstream one is made by
autotools or is `win32/config.h` for nmake).
