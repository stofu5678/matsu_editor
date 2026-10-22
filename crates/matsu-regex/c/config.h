/*
 * Minimal config.h for building the vendored Onigmo with the cc crate.
 *
 * Onigmo's own win32/config.h is for the nmake build and assumes 32-bit
 * defaults, and the autotools build generates its own. This one derives the
 * sizes from the compiler instead, so it works for MSVC (x64 and ARM64),
 * clang (macOS), and gcc/clang (Linux) alike. Only what Onigmo reads is set.
 */
#ifndef MATSU_ONIGMO_CONFIG_H
#define MATSU_ONIGMO_CONFIG_H

#include <limits.h>
#include <stdint.h>

#define HAVE_LIMITS_H 1
#define HAVE_STDINT_H 1
#define HAVE_INTTYPES_H 1
#define HAVE_STDLIB_H 1
#define HAVE_STRING_H 1

#define SIZEOF_INT 4
#define SIZEOF_LONG_LONG 8

#if defined(_WIN32)
/* LLP64: long is 32 bits on 64-bit Windows too. */
# define SIZEOF_LONG 4
#elif defined(__SIZEOF_LONG__)
# define SIZEOF_LONG __SIZEOF_LONG__
#else
# error "cannot determine sizeof(long)"
#endif

#if UINTPTR_MAX == 0xffffffffu
# define SIZEOF_VOIDP 4
#else
# define SIZEOF_VOIDP 8
#endif

#endif
