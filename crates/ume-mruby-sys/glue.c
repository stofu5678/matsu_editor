/*
 * The C side of ume-mruby-sys: the parts of the mruby API that are macros or inline functions
 * (so Rust cannot call them), the Editor functions, and running a script.
 *
 * mruby reports errors with setjmp/longjmp. Rust frames must never sit between a longjmp and
 * its setjmp, so Rust is only ever called from the single native method below, and it answers
 * with a status that this file turns into a raised exception after Rust has returned.
 */
#include <stdio.h>
#include <string.h>
#include <stdint.h>

#ifdef _WIN32
#include <fcntl.h>
#include <io.h>
#endif

#include <mruby.h>
#include <mruby/array.h>
#include <mruby/class.h>
#include <mruby/compile.h>
#include <mruby/error.h>
#include <mruby/hash.h>
#include <mruby/numeric.h>
#include <mruby/proc.h>
#include <mruby/string.h>
#include <mruby/variable.h>

/* Rust treats an mruby value as one machine word. */
#ifndef MRB_WORD_BOXING
#error "ume-mruby-sys expects word boxing"
#endif
mrb_static_assert2(sizeof(mrb_value) == sizeof(uintptr_t), "mrb_value must be one word");
#ifndef MRB_INT64
#error "ume-mruby-sys expects 64-bit integers"
#endif

/*
 * The call into Rust. Returns 0 and the result in *out; 1 and `[code, message]` in *out for an
 * error answer of the editor; 2 and a message string in *out for a failure that ends the macro
 * (the connection is gone); 3 when the answer could not be built in mruby (too large).
 */
typedef int (*ume_call_fn)(void *host, mrb_state *mrb, size_t index, mrb_value args,
                           mrb_value *out);

struct ume_context {
  void *host;
  ume_call_fn call;
};

static struct ume_context *context_of(mrb_state *mrb) {
  return (struct ume_context *)mrb->ud;
}

/* The native method behind every Editor function: its env holds the index of the function. */
static mrb_value editor_function(mrb_state *mrb, mrb_value self) {
  (void)self;
  mrb_value *argv;
  mrb_int argc;
  mrb_get_args(mrb, "*", &argv, &argc);

  mrb_value index_value = mrb_proc_cfunc_env_get(mrb, 0);
  mrb_value args = mrb_ary_new_from_values(mrb, argc, argv);
  mrb_value out = mrb_nil_value();
  struct ume_context *ctx = context_of(mrb);
  int status = ctx->call(ctx->host, mrb, (size_t)mrb_integer(index_value), args, &out);
  if (status == 0) {
    return out;
  }
  if (status == 1) {
    struct RClass *error_class = mrb_class_get(mrb, "EditorError");
    mrb_value ctor_args[2];
    ctor_args[0] = mrb_ary_ref(mrb, out, 0);
    ctor_args[1] = mrb_ary_ref(mrb, out, 1);
    mrb_exc_raise(mrb, mrb_obj_new(mrb, error_class, 2, ctor_args));
  }
  if (status == 3) {
    mrb_raise(mrb, E_RUNTIME_ERROR, "cannot hand the value over to or from the editor (too large?)");
  }
  mrb_exc_raise(mrb, mrb_exc_new_str(mrb, E_RUNTIME_ERROR, out));
}

/* UmeBoot.write_stdout(string) and UmeBoot.write_stderr(string): the streams are unbuffered. */
static mrb_value write_to(mrb_state *mrb, FILE *stream) {
  mrb_value text;
  mrb_get_args(mrb, "S", &text);
  fwrite(RSTRING_PTR(text), 1, (size_t)RSTRING_LEN(text), stream);
  return mrb_nil_value();
}
static mrb_value write_stdout(mrb_state *mrb, mrb_value self) {
  (void)self;
  return write_to(mrb, stdout);
}
static mrb_value write_stderr(mrb_state *mrb, mrb_value self) {
  (void)self;
  return write_to(mrb, stderr);
}

mrb_state *ume_open(void *host, ume_call_fn call) {
  /* The script's print/puts/p go through C stdio. They must not hold text back, because the
     requests that Rust writes to the same pipe have to stay in order with them, and a line
     break must stay a bare "\n" on Windows. */
  setvbuf(stdout, NULL, _IONBF, 0);
  setvbuf(stderr, NULL, _IONBF, 0);
#ifdef _WIN32
  _setmode(_fileno(stdout), _O_BINARY);
  _setmode(_fileno(stderr), _O_BINARY);
#endif

  mrb_state *mrb = mrb_open();
  if (mrb == NULL || mrb->exc) {
    if (mrb) mrb_close(mrb);
    return NULL;
  }
  struct ume_context *ctx = (struct ume_context *)mrb_malloc(mrb, sizeof(struct ume_context));
  ctx->host = host;
  ctx->call = call;
  mrb->ud = ctx;
  struct RClass *boot = mrb_define_module(mrb, "UmeBoot");
  mrb_define_module_function(mrb, boot, "write_stdout", write_stdout, MRB_ARGS_REQ(1));
  mrb_define_module_function(mrb, boot, "write_stderr", write_stderr, MRB_ARGS_REQ(1));
  return mrb;
}

void ume_close(mrb_state *mrb) {
  mrb_free(mrb, mrb->ud);
  mrb->ud = NULL;
  mrb_close(mrb);
}

/* Defines `Editor.<name>` as a native method that calls Rust with `index`. */
void ume_define_function(mrb_state *mrb, const char *name, size_t len, size_t index) {
  struct RClass *editor = mrb_define_module(mrb, "Editor");
  mrb_value env = mrb_int_value(mrb, (mrb_int)index);
  struct RProc *proc = mrb_proc_new_cfunc_with_env(mrb, editor_function, 1, &env);
  mrb_method_t method;
  MRB_METHOD_FROM_PROC(method, proc);
  struct RClass *singleton = mrb_class_ptr(mrb_singleton_class(mrb, mrb_obj_value(editor)));
  mrb_define_method_raw(mrb, singleton, mrb_intern(mrb, name, len), method);
}

void ume_set_info(mrb_state *mrb, mrb_value info) {
  struct RClass *editor = mrb_define_module(mrb, "Editor");
  mrb_iv_set(mrb, mrb_obj_value(editor), mrb_intern_cstr(mrb, "@info"), info);
}

/* The text for the standard error: UmeBoot.describe(exception, name), or a plain fallback. */
static mrb_value describe(mrb_state *mrb, mrb_value exc, const char *name) {
  mrb_value top = mrb_obj_value(mrb->object_class);
  mrb_sym boot_sym = mrb_intern_cstr(mrb, "UmeBoot");
  if (mrb_const_defined(mrb, top, boot_sym)) {
    mrb_value args[2];
    args[0] = exc;
    args[1] = mrb_str_new_cstr(mrb, name);
    mrb_value text = mrb_funcall_argv(mrb, mrb_const_get(mrb, top, boot_sym),
                                      mrb_intern_cstr(mrb, "describe"), 2, args);
    if (!mrb->exc && mrb_string_p(text)) {
      return text;
    }
    mrb->exc = NULL;
  }
  char buffer[256];
  snprintf(buffer, sizeof buffer, "%s: unhandled exception (%s)", name,
           mrb_obj_classname(mrb, exc));
  return mrb_str_new_cstr(mrb, buffer);
}

/*
 * Compiles and runs `source` as a file called `name` (errors and backtraces show that name).
 * Returns nil when it ran to its end, or a string that tells the error.
 */
mrb_value ume_run(mrb_state *mrb, const char *name, const char *source, size_t len) {
  int arena = mrb_gc_arena_save(mrb);
  mrb_ccontext *cxt = mrb_ccontext_new(mrb);
  mrb_ccontext_filename(mrb, cxt, name);
  /* Syntax errors come back as the message of a SyntaxError ("line 3: ...") instead of being
     written to the standard error. */
  cxt->capture_errors = TRUE;
  mrb_load_nstring_cxt(mrb, source, len, cxt);
  mrb_ccontext_free(mrb, cxt);
  mrb_value result = mrb_nil_value();
  if (mrb->exc) {
    mrb_value exc = mrb_obj_value(mrb->exc);
    mrb->exc = NULL;
    result = describe(mrb, exc, name);
  }
  mrb_gc_arena_restore(mrb, arena);
  return result;
}

/* Kinds of values that cross to Rust. */
enum {
  UME_NIL = 0,
  UME_FALSE,
  UME_TRUE,
  UME_INTEGER,
  UME_FLOAT,
  UME_STRING,
  UME_SYMBOL,
  UME_ARRAY,
  UME_HASH,
  UME_OTHER,
};

int ume_kind(mrb_value v) {
  switch (mrb_type(v)) {
    case MRB_TT_FALSE:
      return mrb_nil_p(v) ? UME_NIL : UME_FALSE;
    case MRB_TT_TRUE:
      return UME_TRUE;
    case MRB_TT_INTEGER:
      return UME_INTEGER;
    case MRB_TT_FLOAT:
      return UME_FLOAT;
    case MRB_TT_STRING:
      return UME_STRING;
    case MRB_TT_SYMBOL:
      return UME_SYMBOL;
    case MRB_TT_ARRAY:
      return UME_ARRAY;
    case MRB_TT_HASH:
      return UME_HASH;
    default:
      return UME_OTHER;
  }
}

int64_t ume_integer(mrb_value v) { return (int64_t)mrb_integer(v); }
double ume_float(mrb_value v) { return (double)mrb_float(v); }

const char *ume_string(mrb_value v, size_t *len) {
  *len = (size_t)RSTRING_LEN(v);
  return RSTRING_PTR(v);
}

const char *ume_symbol(mrb_state *mrb, mrb_value v, size_t *len) {
  mrb_int n = 0;
  const char *p = mrb_sym_name_len(mrb, mrb_symbol(v), &n);
  *len = (size_t)n;
  return p;
}

size_t ume_array_len(mrb_value v) { return (size_t)RARRAY_LEN(v); }
mrb_value ume_array_ref(mrb_state *mrb, mrb_value v, size_t i) {
  return mrb_ary_ref(mrb, v, (mrb_int)i);
}
const char *ume_class_name(mrb_state *mrb, mrb_value v) { return mrb_obj_classname(mrb, v); }

mrb_value ume_nil(void) { return mrb_nil_value(); }
mrb_value ume_bool(int b) { return b ? mrb_true_value() : mrb_false_value(); }

/*
 * The calls that make mruby objects can raise (a hash of more than 131072 entries cannot be
 * turned into an array, a string of 1 MiB can fail to allocate...). Rust frames are on the
 * stack when they are called, so each runs under mrb_protect_error and answers 0 instead of
 * jumping. On success the result is in *out, and it stays alive (mrb_protect_error puts it in
 * the GC arena).
 */
enum {
  OP_INTEGER,
  OP_FLOAT,
  OP_STRING,
  OP_ARRAY_NEW,
  OP_ARRAY_PUSH,
  OP_HASH_NEW,
  OP_HASH_SET,
  OP_HASH_KEYS,
  OP_HASH_VALUES,
};

struct op {
  int kind;
  mrb_value a, b, c;
  int64_t i;
  double f;
  const char *p;
  size_t n;
};

static mrb_value op_body(mrb_state *mrb, void *data) {
  struct op *op = (struct op *)data;
  switch (op->kind) {
    case OP_INTEGER:
      return mrb_int_value(mrb, (mrb_int)op->i);
    case OP_FLOAT:
      return mrb_float_value(mrb, (mrb_float)op->f);
    case OP_STRING:
      return mrb_str_new(mrb, op->p, (mrb_int)op->n);
    case OP_ARRAY_NEW:
      return mrb_ary_new_capa(mrb, (mrb_int)op->n);
    case OP_ARRAY_PUSH:
      mrb_ary_push(mrb, op->a, op->b);
      return op->a;
    case OP_HASH_NEW:
      return mrb_hash_new(mrb);
    case OP_HASH_SET:
      mrb_hash_set(mrb, op->a, op->b, op->c);
      return op->a;
    case OP_HASH_KEYS:
      return mrb_hash_keys(mrb, op->a);
    case OP_HASH_VALUES:
      return mrb_hash_values(mrb, op->a);
  }
  return mrb_nil_value();
}

static int run_op(mrb_state *mrb, struct op *op, mrb_value *out) {
  mrb_bool error = FALSE;
  mrb_value result = mrb_protect_error(mrb, op_body, op, &error);
  if (error) {
    return 0;
  }
  *out = result;
  return 1;
}

int ume_integer_value(mrb_state *mrb, int64_t i, mrb_value *out) {
  struct op op = {.kind = OP_INTEGER, .i = i};
  return run_op(mrb, &op, out);
}
int ume_float_value(mrb_state *mrb, double f, mrb_value *out) {
  struct op op = {.kind = OP_FLOAT, .f = f};
  return run_op(mrb, &op, out);
}
int ume_string_new(mrb_state *mrb, const char *p, size_t len, mrb_value *out) {
  struct op op = {.kind = OP_STRING, .p = p, .n = len};
  return run_op(mrb, &op, out);
}
int ume_array_new(mrb_state *mrb, size_t capacity, mrb_value *out) {
  struct op op = {.kind = OP_ARRAY_NEW, .n = capacity};
  return run_op(mrb, &op, out);
}
int ume_array_push(mrb_state *mrb, mrb_value array, mrb_value item) {
  struct op op = {.kind = OP_ARRAY_PUSH, .a = array, .b = item};
  mrb_value ignored;
  return run_op(mrb, &op, &ignored);
}
int ume_hash_new(mrb_state *mrb, mrb_value *out) {
  struct op op = {.kind = OP_HASH_NEW};
  return run_op(mrb, &op, out);
}
int ume_hash_set(mrb_state *mrb, mrb_value hash, mrb_value key, mrb_value value) {
  struct op op = {.kind = OP_HASH_SET, .a = hash, .b = key, .c = value};
  mrb_value ignored;
  return run_op(mrb, &op, &ignored);
}
/* The keys and the values of a hash, in the same order, as arrays. A hash is read this way and
   not by looking the keys up, because a lookup calls the `hash` and `eql?` of the user's keys. */
int ume_hash_keys(mrb_state *mrb, mrb_value hash, mrb_value *out) {
  struct op op = {.kind = OP_HASH_KEYS, .a = hash};
  return run_op(mrb, &op, out);
}
int ume_hash_values(mrb_state *mrb, mrb_value hash, mrb_value *out) {
  struct op op = {.kind = OP_HASH_VALUES, .a = hash};
  return run_op(mrb, &op, out);
}
