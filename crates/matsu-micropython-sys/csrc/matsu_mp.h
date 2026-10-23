// The glue between MATSU Editor (Rust) and the MicroPython VM.
#ifndef MATSU_MP_H
#define MATSU_MP_H

#include <stddef.h>

// Writes `len` bytes to the standard output (fd 1) or the standard error (fd 2) and flushes.
// Implemented in Rust.
void matsu_mp_write(int fd, const char *bytes, size_t len);

// What a call of an `Editor` function gives back.
enum {
    MATSU_CALL_OK = 0,     // `*out` is the result as JSON
    MATSU_CALL_TEXT = 3,   // the result is a string, and `*out` is its text itself (no JSON)
    MATSU_CALL_ERROR = 1,  // the editor answered with an error: `*code`, and `*out` is the message
    MATSU_CALL_CLOSED = 2, // the connection is gone: `*out` is the message
};

// Hands one call of the macro API to the editor and waits for the answer. `args` is a JSON
// array (UTF-8). The memory behind `*out` belongs to the callee and stays valid until the next call.
typedef int (*matsu_call_fn)(
    void *ctx,
    const char *name, size_t name_len,
    const char *args, size_t args_len,
    long long *code,
    const char **out, size_t *out_len);

typedef struct {
    // The garbage-collected heap, in bytes.
    size_t heap_size;
    // How much stack the VM may use before it raises RuntimeError (recursion too deep).
    size_t stack_limit;
    matsu_call_fn call;
    void *ctx;
    // JSON array of the names of the `Editor` functions, and the JSON object `Editor.info`.
    const char *names_json;
    size_t names_json_len;
    const char *info_json;
    size_t info_json_len;
    // The boot script (it defines `Editor` and `EditorError`) and the user's script.
    const char *boot_name;
    size_t boot_name_len;
    const char *boot_source;
    size_t boot_source_len;
    const char *script_name;
    size_t script_name_len;
    const char *script_source;
    size_t script_source_len;
} matsu_mp_config;

enum {
    MATSU_RUN_OK = 0,
    MATSU_RUN_ERROR = 1,   // the script raised an exception: `*message` has its traceback
    MATSU_RUN_NO_HEAP = 2, // the heap could not be allocated
};

// Starts the VM, runs the boot script and the user's script, and stops the VM. The text for
// the standard error comes back in `*message` (allocated with malloc; free it with
// `matsu_mp_free`).
int matsu_mp_run(const matsu_mp_config *config, char **message, size_t *message_len);

void matsu_mp_free(void *pointer);

#endif
