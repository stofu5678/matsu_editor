// The MicroPython configuration that UME Editor builds with. It is passed to MicroPython's own
// embed port (ports/embed) to generate the sources in vendor/micropython (see the README there).
//
// This file is part of UME Editor (MIT OR Apache-2.0); the options it sets are MicroPython's.

// Common embed configuration (the `mp_off_t` type, alloca, the HAL header).
#include <port/mpconfigport_common.h>

// Start from the smallest feature set, then switch on what a macro needs.
#define MICROPY_CONFIG_ROM_LEVEL                (MICROPY_CONFIG_ROM_LEVEL_CORE_FEATURES)

#define MICROPY_ENABLE_COMPILER                 (1)
#define MICROPY_ENABLE_GC                       (1)
#define MICROPY_PY_GC                           (1)

// No assembly anywhere (Windows ARM64 has no MicroPython port). Non-local returns use
// setjmp/longjmp, and the garbage collector finds the registers with setjmp too.
#define MICROPY_NLR_SETJMP                      (1)
#define MICROPY_GCREGS_SETJMP                   (1)
#define MICROPY_PERSISTENT_CODE_LOAD            (0)
#define MICROPY_PERSISTENT_CODE_SAVE            (0)

// Text is Unicode (UTF-8 inside), numbers are 64-bit floats and arbitrary-size integers, and
// a file name or a message longer than 255 bytes is fine.
#define MICROPY_PY_BUILTINS_STR_UNICODE         (1)
#define MICROPY_PY_BUILTINS_FLOAT               (1)
#define MICROPY_FLOAT_IMPL                      (MICROPY_FLOAT_IMPL_DOUBLE)
#define MICROPY_LONGINT_IMPL                    (MICROPY_LONGINT_IMPL_MPZ)
#define MICROPY_QSTR_BYTES_IN_LEN               (2)
#define MICROPY_QSTR_BYTES_IN_HASH              (2)

// Deep recursion is an exception (RuntimeError), not a crash. The limit is set at start-up.
#define MICROPY_STACK_CHECK                     (1)
#define MICROPY_ENABLE_PYSTACK                  (0)

// The messages are those of CPython where MicroPython has them, and a traceback shows the
// file name and the line number.
#define MICROPY_ERROR_REPORTING                 (MICROPY_ERROR_REPORTING_DETAILED)
#define MICROPY_WARNINGS                        (0)
#define MICROPY_ENABLE_SOURCE_LINE              (1)
#define MICROPY_PY_BUILTINS_HELP                (0)
#define MICROPY_PY_BUILTINS_INPUT               (0)
#define MICROPY_PY_BUILTINS_EXECEXPR            (1)
#define MICROPY_PY_BUILTINS_COMPILE             (1)

// The modules a macro needs. No file system, time or network.
#define MICROPY_PY_SYS                          (1)
#define MICROPY_PY_SYS_PATH                     (0)
#define MICROPY_PY_SYS_ARGV                     (0)
#define MICROPY_PY_SYS_EXIT                     (1)
#define MICROPY_PY_SYS_MODULES                  (1)
#define MICROPY_PY_SYS_PLATFORM                 "ume"
#define MICROPY_PY_SYS_STDFILES                 (1)
#define MICROPY_PY_SYS_STDIO_BUFFER             (0)
#define MICROPY_PY_IO                           (1)
#define MICROPY_PY_IO_IOBASE                    (1)
#define MICROPY_PY_IO_FILEIO                    (0)
#define MICROPY_PY_IO_BYTESIO                   (1)
#define MICROPY_PY_IO_BUFFEREDWRITER            (0)
#define MICROPY_PY_JSON                         (1)
#define MICROPY_PY_MATH                         (1)
#define MICROPY_PY_CMATH                        (0)
#define MICROPY_PY_STRUCT                       (1)
#define MICROPY_PY_COLLECTIONS                  (1)
#define MICROPY_PY_COLLECTIONS_DEQUE            (1)
#define MICROPY_PY_COLLECTIONS_ORDEREDDICT      (1)
#define MICROPY_PY_ARRAY                        (1)
#define MICROPY_PY_BINASCII                     (1)
#define MICROPY_PY_HEAPQ                        (1)
#define MICROPY_PY_ERRNO                        (1)
#define MICROPY_PY_RANDOM                       (0)
#define MICROPY_PY_TIME                         (0)
#define MICROPY_PY_OS                           (0)
#define MICROPY_PY_ASYNCIO                      (0)
#define MICROPY_PY_HASHLIB                      (0)
#define MICROPY_PY_UCTYPES                      (0)
#define MICROPY_PY_DEFLATE                      (0)
#define MICROPY_PY_MARSHAL                      (0)
#define MICROPY_PY_PLATFORM                     (0)
#define MICROPY_PY_RANDOM_SEED_INIT_FUNC        (0)

// Import is not available: there is no file system to import from.
#define MICROPY_ENABLE_EXTERNAL_IMPORT          (0)
#define MICROPY_MODULE_BUILTIN_INIT             (1)
#define MICROPY_READER_POSIX                    (0)
#define MICROPY_READER_VFS                      (0)

#define MICROPY_HELPER_REPL                     (0)
#define MICROPY_KBD_EXCEPTION                   (0)
#define MICROPY_PY_RE                           (0)
