// The glue between UME Editor (Rust) and the MicroPython VM. See ume_mp.h.

#include <stdlib.h>
#include <string.h>

#include "py/builtin.h"
#include "py/compile.h"
#include "py/gc.h"
#include "py/mphal.h"
#include "py/objstr.h"
#include "py/parse.h"
#include "py/runtime.h"
#include "py/stackctrl.h"
#include "port/micropython_embed.h"

#include "ume_mp.h"

// The VM runs one script in one thread and then stops, so the configuration is a plain static.
static const ume_mp_config *g_config;

typedef struct {
    char *data;
    size_t len;
    size_t cap;
} text_buffer;

// Collects text in a malloc'ed buffer: the arguments of a call as JSON, and the traceback of an
// exception (which goes to the standard error after the VM has stopped). It grows by doubling;
// the strings of a macro can be megabytes, and the growth of MicroPython's own `vstr` (16 bytes
// at a time) and of `json.dumps` and `json.loads` is quadratic for them.
static void buffer_write(void *data, const char *str, size_t len) {
    text_buffer *buffer = data;
    if (buffer->len + len > buffer->cap) {
        size_t cap = (buffer->len + len) * 2 + 256;
        char *grown = realloc(buffer->data, cap);
        if (grown == NULL) {
            return;
        }
        buffer->data = grown;
        buffer->cap = cap;
    }
    memcpy(buffer->data + buffer->len, str, len);
    buffer->len += len;
}

static void buffer_puts(text_buffer *buffer, const char *text) {
    buffer_write(buffer, text, strlen(text));
}

// A string as a JSON string. Non-ASCII text stays UTF-8; a lone surrogate (which MicroPython
// can hold) comes out as invalid UTF-8, which the host refuses.
static void json_string(text_buffer *buffer, mp_obj_t object) {
    size_t len;
    const char *text = mp_obj_str_get_data(object, &len);
    static const char hex[] = "0123456789abcdef";
    buffer_write(buffer, "\"", 1);
    size_t run = 0;
    for (size_t i = 0; i <= len; i++) {
        unsigned char c = i < len ? (unsigned char)text[i] : 0;
        if (i < len && c >= 0x20 && c != '"' && c != '\\') {
            continue;
        }
        buffer_write(buffer, text + run, i - run);
        run = i + 1;
        if (i == len) {
            break;
        }
        switch (c) {
            case '"': buffer_puts(buffer, "\\\""); break;
            case '\\': buffer_puts(buffer, "\\\\"); break;
            case '\n': buffer_puts(buffer, "\\n"); break;
            case '\r': buffer_puts(buffer, "\\r"); break;
            case '\t': buffer_puts(buffer, "\\t"); break;
            default: {
                char escape[6] = {'\\', 'u', '0', '0', hex[c >> 4], hex[c & 15]};
                buffer_write(buffer, escape, sizeof escape);
            }
        }
    }
    buffer_write(buffer, "\"", 1);
}

// A value as JSON. What has no JSON form (an object, a set, `nan`) is written as text that is not
// JSON, so that the host answers with an invalid-params error that names the function.
static void json_value(text_buffer *buffer, mp_obj_t object) {
    mp_stack_check();
    mp_print_t print = {buffer, buffer_write};
    if (object == mp_const_none) {
        buffer_puts(buffer, "null");
    } else if (object == mp_const_true) {
        buffer_puts(buffer, "true");
    } else if (object == mp_const_false) {
        buffer_puts(buffer, "false");
    } else if (mp_obj_is_int(object) || mp_obj_is_float(object)) {
        mp_obj_print_helper(&print, object, PRINT_REPR);
    } else if (mp_obj_is_str(object)) {
        json_string(buffer, object);
    } else if (mp_obj_is_type(object, &mp_type_list) || mp_obj_is_type(object, &mp_type_tuple)) {
        size_t len;
        mp_obj_t *items;
        mp_obj_get_array(object, &len, &items);
        buffer_puts(buffer, "[");
        for (size_t i = 0; i < len; i++) {
            if (i > 0) {
                buffer_puts(buffer, ",");
            }
            json_value(buffer, items[i]);
        }
        buffer_puts(buffer, "]");
    } else if (mp_obj_is_type(object, &mp_type_dict)) {
        mp_map_t *map = mp_obj_dict_get_map(object);
        bool first = true;
        buffer_puts(buffer, "{");
        for (size_t i = 0; i < map->alloc; i++) {
            if (!mp_map_slot_is_filled(map, i)) {
                continue;
            }
            if (!first) {
                buffer_puts(buffer, ",");
            }
            first = false;
            if (mp_obj_is_str(map->table[i].key)) {
                json_string(buffer, map->table[i].key);
            } else {
                buffer_puts(buffer, "<key>");
            }
            buffer_puts(buffer, ":");
            json_value(buffer, map->table[i].value);
        }
        buffer_puts(buffer, "}");
    } else {
        buffer_puts(buffer, "<unsupported>");
    }
}

// `_ume_call(name, args)`: one call of the macro API; `args` is a list. Returns
//   - a str: the result, when it is a string (the text itself, however long);
//   - a 1-tuple `(json,)`: any other result, as JSON text;
//   - a 2-tuple `(code, message)`: the editor answered with an error (the Python side raises
//     EditorError from it, so that no frame of C is in between).
// When the connection is gone it raises EOFError, which ends the macro.
static mp_obj_t ume_call(mp_obj_t name_in, mp_obj_t args_in) {
    size_t name_len;
    const char *name = mp_obj_str_get_data(name_in, &name_len);
    text_buffer args = {NULL, 0, 0};
    json_value(&args, args_in);
    long long code = 0;
    const char *out = NULL;
    size_t out_len = 0;
    int status = g_config->call(g_config->ctx, name, name_len, args.data, args.len, &code, &out, &out_len);
    free(args.data);
    switch (status) {
        case UME_CALL_TEXT:
            return mp_obj_new_str(out, out_len);
        case UME_CALL_OK: {
            mp_obj_t item = mp_obj_new_str(out, out_len);
            return mp_obj_new_tuple(1, &item);
        }
        case UME_CALL_ERROR: {
            mp_obj_t items[2] = {mp_obj_new_int_from_ll(code), mp_obj_new_str(out, out_len)};
            return mp_obj_new_tuple(2, items);
        }
        default:
            nlr_raise(mp_obj_new_exception_arg1(&mp_type_EOFError, mp_obj_new_str(out, out_len)));
    }
}
static MP_DEFINE_CONST_FUN_OBJ_2(ume_call_obj, ume_call);

// `_ume_setup()`: `(names_json, info_json)` for the boot script.
static mp_obj_t ume_setup(void) {
    mp_obj_t items[2] = {
        mp_obj_new_str(g_config->names_json, g_config->names_json_len),
        mp_obj_new_str(g_config->info_json, g_config->info_json_len),
    };
    return mp_obj_new_tuple(2, items);
}
static MP_DEFINE_CONST_FUN_OBJ_0(ume_setup_obj, ume_setup);

// Compiles and runs the text as a module, in the globals of `__main__`. Returns 0, or 1 with
// the traceback in `buffer`. `sys.exit()` and `sys.exit(0)` are not errors.
static int exec_text(const char *name, size_t name_len, const char *source, size_t source_len, text_buffer *buffer) {
    nlr_buf_t nlr;
    if (nlr_push(&nlr) == 0) {
        qstr file = qstr_from_strn(name, name_len);
        mp_lexer_t *lexer = mp_lexer_new_from_str_len(file, source, source_len, 0);
        mp_parse_tree_t tree = mp_parse(lexer, MP_PARSE_FILE_INPUT);
        mp_obj_t module_function = mp_compile(&tree, file, false);
        mp_call_function_0(module_function);
        nlr_pop();
        return 0;
    }
    mp_obj_t exception = MP_OBJ_FROM_PTR(nlr.ret_val);
    if (mp_obj_is_type(exception, &mp_type_SystemExit)) {
        mp_obj_t value = mp_obj_exception_get_value(exception);
        if (value == mp_const_none || (mp_obj_is_small_int(value) && MP_OBJ_SMALL_INT_VALUE(value) == 0)) {
            return 0;
        }
    }
    mp_print_t print = {buffer, buffer_write};
    mp_obj_print_exception(&print, exception);
    return 1;
}

int ume_mp_run(const ume_mp_config *config, char **message, size_t *message_len) {
    g_config = config;
    *message = NULL;
    *message_len = 0;

    void *heap = malloc(config->heap_size);
    if (heap == NULL) {
        return UME_RUN_NO_HEAP;
    }
    // The garbage collector scans the stack from here down: everything the VM holds in C
    // frames is below this one.
    int stack_top;
    mp_embed_init(heap, config->heap_size, &stack_top);
    mp_stack_set_limit(config->stack_limit);
    mp_store_global(qstr_from_str("_ume_call"), MP_OBJ_FROM_PTR(&ume_call_obj));
    mp_store_global(qstr_from_str("_ume_setup"), MP_OBJ_FROM_PTR(&ume_setup_obj));

    text_buffer buffer = {NULL, 0, 0};
    int failed = exec_text(config->boot_name, config->boot_name_len, config->boot_source, config->boot_source_len, &buffer);
    if (!failed) {
        failed = exec_text(config->script_name, config->script_name_len, config->script_source, config->script_source_len, &buffer);
    }
    mp_embed_deinit();
    free(heap);
    *message = buffer.data;
    *message_len = buffer.len;
    return failed ? UME_RUN_ERROR : UME_RUN_OK;
}

void ume_mp_free(void *pointer) {
    free(pointer);
}
