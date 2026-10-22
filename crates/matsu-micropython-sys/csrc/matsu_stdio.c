// sys.stdin, sys.stdout and sys.stderr for the macros, and the output hooks of MicroPython.
//
// Output is written by Rust (`matsu_mp_write`), which writes to the process's standard output or
// standard error and flushes at once, so that it stays in order with the requests that are
// written to the same pipe. Standard input carries the JSON-RPC protocol, so a script cannot
// read it: it looks like an empty file.

#include <stddef.h>

#include "py/lexer.h"
#include "py/obj.h"
#include "py/runtime.h"
#include "py/stream.h"
#include "py/mperrno.h"
#include "py/mphal.h"

#include "matsu_mp.h"

#define STDIO_FD_IN  (0)
#define STDIO_FD_OUT (1)
#define STDIO_FD_ERR (2)

typedef struct _matsu_stdio_obj_t {
    mp_obj_base_t base;
    int fd;
} matsu_stdio_obj_t;

static void stdio_print(const mp_print_t *print, mp_obj_t self_in, mp_print_kind_t kind) {
    matsu_stdio_obj_t *self = MP_OBJ_TO_PTR(self_in);
    mp_printf(print, "<io.TextIOWrapper %d>", self->fd);
}

static mp_uint_t stdio_read(mp_obj_t self_in, void *buf, mp_uint_t size, int *errcode) {
    matsu_stdio_obj_t *self = MP_OBJ_TO_PTR(self_in);
    if (self->fd == STDIO_FD_IN) {
        return 0;
    }
    *errcode = MP_EPERM;
    return MP_STREAM_ERROR;
}

static mp_uint_t stdio_write(mp_obj_t self_in, const void *buf, mp_uint_t size, int *errcode) {
    matsu_stdio_obj_t *self = MP_OBJ_TO_PTR(self_in);
    if (self->fd == STDIO_FD_OUT || self->fd == STDIO_FD_ERR) {
        matsu_mp_write(self->fd, (const char *)buf, size);
        return size;
    }
    *errcode = MP_EPERM;
    return MP_STREAM_ERROR;
}

static mp_uint_t stdio_ioctl(mp_obj_t self_in, mp_uint_t request, uintptr_t arg, int *errcode) {
    (void)self_in;
    (void)arg;
    if (request == MP_STREAM_CLOSE || request == MP_STREAM_FLUSH) {
        return 0;
    }
    *errcode = MP_EINVAL;
    return MP_STREAM_ERROR;
}

static mp_obj_t stdio_flush(mp_obj_t self_in) {
    (void)self_in;
    return mp_const_none;
}
static MP_DEFINE_CONST_FUN_OBJ_1(stdio_flush_obj, stdio_flush);

static const mp_rom_map_elem_t stdio_locals_dict_table[] = {
    { MP_ROM_QSTR(MP_QSTR_read), MP_ROM_PTR(&mp_stream_read_obj) },
    { MP_ROM_QSTR(MP_QSTR_readline), MP_ROM_PTR(&mp_stream_unbuffered_readline_obj) },
    { MP_ROM_QSTR(MP_QSTR_readlines), MP_ROM_PTR(&mp_stream_unbuffered_readlines_obj) },
    { MP_ROM_QSTR(MP_QSTR_write), MP_ROM_PTR(&mp_stream_write_obj) },
    { MP_ROM_QSTR(MP_QSTR_flush), MP_ROM_PTR(&stdio_flush_obj) },
    { MP_ROM_QSTR(MP_QSTR_close), MP_ROM_PTR(&mp_identity_obj) },
    { MP_ROM_QSTR(MP_QSTR___enter__), MP_ROM_PTR(&mp_identity_obj) },
    { MP_ROM_QSTR(MP_QSTR___exit__), MP_ROM_PTR(&mp_stream___exit___obj) },
};
static MP_DEFINE_CONST_DICT(stdio_locals_dict, stdio_locals_dict_table);

static const mp_stream_p_t stdio_stream_p = {
    .read = stdio_read,
    .write = stdio_write,
    .ioctl = stdio_ioctl,
    .is_text = true,
};

static MP_DEFINE_CONST_OBJ_TYPE(
    matsu_stdio_type,
    MP_QSTR_TextIOWrapper,
    MP_TYPE_FLAG_ITER_IS_STREAM,
    print, stdio_print,
    protocol, &stdio_stream_p,
    locals_dict, &stdio_locals_dict
    );

const matsu_stdio_obj_t mp_sys_stdin_obj = {{&matsu_stdio_type}, .fd = STDIO_FD_IN};
const matsu_stdio_obj_t mp_sys_stdout_obj = {{&matsu_stdio_type}, .fd = STDIO_FD_OUT};
const matsu_stdio_obj_t mp_sys_stderr_obj = {{&matsu_stdio_type}, .fd = STDIO_FD_ERR};

// The hooks that MicroPython itself calls (print goes through sys.stdout, see mpprint.h; the
// platform printer is for the messages of the runtime).
mp_uint_t mp_hal_stdout_tx_strn(const char *str, size_t len) {
    matsu_mp_write(STDIO_FD_OUT, str, len);
    return len;
}

void mp_hal_stdout_tx_strn_cooked(const char *str, size_t len) {
    matsu_mp_write(STDIO_FD_OUT, str, len);
}

void mp_hal_stdout_tx_str(const char *str) {
    size_t len = 0;
    while (str[len] != '\0') {
        len++;
    }
    matsu_mp_write(STDIO_FD_OUT, str, len);
}

// `open` (also `io.open`) is there because the `io` module lists it, but a macro has no file
// system: it fails like a file that cannot be opened.
static mp_obj_t matsu_open(size_t n_args, const mp_obj_t *args, mp_map_t *kwargs) {
    (void)n_args;
    (void)args;
    (void)kwargs;
    mp_raise_OSError(MP_EPERM);
}
MP_DEFINE_CONST_FUN_OBJ_KW(mp_builtin_open_obj, 1, matsu_open);

// `execfile` reads a file with this; there is no file system.
mp_lexer_t *mp_lexer_new_from_file(qstr filename) {
    (void)filename;
    mp_raise_OSError(MP_EPERM);
}
