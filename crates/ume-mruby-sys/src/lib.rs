//! mruby 4.0 for the script host of UME Editor: the C source is compiled by `build.rs`, and this
//! crate offers one safe function, [`run`]: start a VM, bind host functions as the methods of a
//! module `Editor`, run a boot script and then the user's script.
//!
//! Values cross the boundary as [`Value`], a copy; no mruby object is visible outside.
//!
//! mruby reports errors with `longjmp`. Rust is only called from one native method, and it
//! answers with a status that the C glue (`glue.c`) turns into an exception after Rust has
//! returned, so no Rust frame is jumped over by a script error. The one exception is running
//! out of memory inside an mruby call that Rust makes while it builds a value.

use std::ffi::{CStr, c_char, c_int, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// A value of the script as plain data.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Integer(i64),
    Float(f64),
    /// The bytes of a string; mruby strings need not be UTF-8. A Symbol arrives as a `Str`.
    Str(Vec<u8>),
    Array(Vec<Value>),
    Hash(Vec<(Value, Value)>),
    /// Anything else (a Range, an Object, an Integer too large...); the name of its class.
    Other(String),
}

/// Why a host function did not return a value.
#[derive(Debug)]
pub enum Failure {
    /// The script sees `EditorError.new(code, message)` raised at the call.
    Editor { code: i64, message: String },
    /// The script sees a `RuntimeError`: the editor is gone and nothing can be done.
    Fatal(String),
}

/// The functions that the script can call.
pub trait Host {
    fn call(&mut self, function: &str, args: Vec<Value>) -> Result<Value, Failure>;
}

pub struct Script<'a> {
    /// Ruby source run first; it defines `EditorError` and `UmeBoot.describe(exception, name)`
    /// (the text for the standard error), and `Editor.info`'s reader. See
    /// `ume-script/src/mruby_boot.rb`.
    pub boot: &'a str,
    /// The name that errors and backtraces show.
    pub name: &'a str,
    pub source: &'a str,
    /// The names of the functions to define on `Editor`.
    pub functions: &'a [&'a str],
    /// What `Editor.info` returns (set as the instance variable `@info` of `Editor`).
    pub info: &'a Value,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MrbValue(usize);

#[repr(C)]
struct MrbState {
    _private: [u8; 0],
}

type CallFn = extern "C" fn(
    host: *mut c_void,
    mrb: *mut MrbState,
    index: usize,
    args: MrbValue,
    out: *mut MrbValue,
) -> c_int;

mod kind {
    use std::ffi::c_int;
    pub const NIL: c_int = 0;
    pub const FALSE: c_int = 1;
    pub const TRUE: c_int = 2;
    pub const INTEGER: c_int = 3;
    pub const FLOAT: c_int = 4;
    pub const STRING: c_int = 5;
    pub const SYMBOL: c_int = 6;
    pub const ARRAY: c_int = 7;
    pub const HASH: c_int = 8;
}

unsafe extern "C" {
    fn ume_open(host: *mut c_void, call: CallFn) -> *mut MrbState;
    fn ume_close(mrb: *mut MrbState);
    fn ume_define_function(mrb: *mut MrbState, name: *const c_char, len: usize, index: usize);
    fn ume_set_info(mrb: *mut MrbState, info: MrbValue);
    fn ume_run(
        mrb: *mut MrbState,
        name: *const c_char,
        source: *const c_char,
        len: usize,
    ) -> MrbValue;

    fn ume_kind(v: MrbValue) -> c_int;
    fn ume_integer(v: MrbValue) -> i64;
    fn ume_float(v: MrbValue) -> f64;
    fn ume_string(v: MrbValue, len: *mut usize) -> *const u8;
    fn ume_symbol(mrb: *mut MrbState, v: MrbValue, len: *mut usize) -> *const u8;
    fn ume_array_len(v: MrbValue) -> usize;
    fn ume_array_ref(mrb: *mut MrbState, v: MrbValue, i: usize) -> MrbValue;
    fn ume_hash_keys(mrb: *mut MrbState, v: MrbValue) -> MrbValue;
    fn ume_hash_get(mrb: *mut MrbState, hash: MrbValue, key: MrbValue) -> MrbValue;
    fn ume_class_name(mrb: *mut MrbState, v: MrbValue) -> *const c_char;

    fn ume_nil() -> MrbValue;
    fn ume_bool(b: c_int) -> MrbValue;
    fn ume_integer_value(mrb: *mut MrbState, i: i64) -> MrbValue;
    fn ume_float_value(mrb: *mut MrbState, f: f64) -> MrbValue;
    fn ume_string_new(mrb: *mut MrbState, p: *const u8, len: usize) -> MrbValue;
    fn ume_array_new(mrb: *mut MrbState, capacity: usize) -> MrbValue;
    fn ume_array_push(mrb: *mut MrbState, array: MrbValue, item: MrbValue);
    fn ume_hash_new(mrb: *mut MrbState) -> MrbValue;
    fn ume_hash_set(mrb: *mut MrbState, hash: MrbValue, key: MrbValue, value: MrbValue);
}

/// How deep a value may nest before it is taken for a cycle (an array that contains itself).
const MAX_DEPTH: usize = 64;

/// Safety: `mrb` is a live VM and `v` a value of it.
unsafe fn read(mrb: *mut MrbState, v: MrbValue, depth: usize) -> Value {
    unsafe {
        match ume_kind(v) {
            kind::NIL => Value::Nil,
            kind::FALSE => Value::Bool(false),
            kind::TRUE => Value::Bool(true),
            kind::INTEGER => Value::Integer(ume_integer(v)),
            kind::FLOAT => Value::Float(ume_float(v)),
            kind::STRING => {
                let mut len = 0;
                let p = ume_string(v, &mut len);
                Value::Str(bytes(p, len))
            }
            kind::SYMBOL => {
                let mut len = 0;
                let p = ume_symbol(mrb, v, &mut len);
                Value::Str(bytes(p, len))
            }
            kind::ARRAY if depth < MAX_DEPTH => {
                let len = ume_array_len(v);
                Value::Array(
                    (0..len)
                        .map(|i| read(mrb, ume_array_ref(mrb, v, i), depth + 1))
                        .collect(),
                )
            }
            kind::HASH if depth < MAX_DEPTH => {
                let keys = ume_hash_keys(mrb, v);
                let len = ume_array_len(keys);
                Value::Hash(
                    (0..len)
                        .map(|i| {
                            let key = ume_array_ref(mrb, keys, i);
                            (
                                read(mrb, key, depth + 1),
                                read(mrb, ume_hash_get(mrb, v, key), depth + 1),
                            )
                        })
                        .collect(),
                )
            }
            _ => Value::Other(
                CStr::from_ptr(ume_class_name(mrb, v))
                    .to_string_lossy()
                    .into_owned(),
            ),
        }
    }
}

unsafe fn bytes(p: *const u8, len: usize) -> Vec<u8> {
    if len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(p, len) }.to_vec()
    }
}

/// Safety: `mrb` is a live VM. `Other` has no mruby form and becomes nil.
unsafe fn write(mrb: *mut MrbState, value: &Value) -> MrbValue {
    unsafe {
        match value {
            Value::Nil | Value::Other(_) => ume_nil(),
            Value::Bool(b) => ume_bool(c_int::from(*b)),
            Value::Integer(i) => ume_integer_value(mrb, *i),
            Value::Float(f) => ume_float_value(mrb, *f),
            Value::Str(s) => ume_string_new(mrb, s.as_ptr(), s.len()),
            Value::Array(items) => {
                let array = ume_array_new(mrb, items.len());
                for item in items {
                    ume_array_push(mrb, array, write(mrb, item));
                }
                array
            }
            Value::Hash(pairs) => {
                let hash = ume_hash_new(mrb);
                for (key, item) in pairs {
                    ume_hash_set(mrb, hash, write(mrb, key), write(mrb, item));
                }
                hash
            }
        }
    }
}

/// What the C glue calls back with: the host, and the names behind the function indexes.
struct Context<'a> {
    host: &'a mut dyn Host,
    names: &'a [&'a str],
}

extern "C" fn trampoline(
    context: *mut c_void,
    mrb: *mut MrbState,
    index: usize,
    args: MrbValue,
    out: *mut MrbValue,
) -> c_int {
    // Safety: `context` is the `Context` that `run` passed to `ume_open`, alive for the whole
    // run, and the VM runs on this thread only.
    let context = unsafe { &mut *context.cast::<Context<'_>>() };
    let answer = catch_unwind(AssertUnwindSafe(|| {
        let args = match unsafe { read(mrb, args, 0) } {
            Value::Array(items) => items,
            _ => Vec::new(),
        };
        context.host.call(context.names[index], args)
    }))
    .unwrap_or_else(|_| Err(Failure::Fatal("internal error in the script host".into())));
    // Safety: `out` points at a local of the C glue.
    unsafe {
        match answer {
            Ok(value) => {
                *out = write(mrb, &value);
                0
            }
            Err(Failure::Editor { code, message }) => {
                *out = write(
                    mrb,
                    &Value::Array(vec![Value::Integer(code), Value::Str(message.into_bytes())]),
                );
                1
            }
            Err(Failure::Fatal(message)) => {
                *out = write(mrb, &Value::Str(message.into_bytes()));
                2
            }
        }
    }
}

/// Closes the VM when the run ends, however it ends.
struct Vm(*mut MrbState);

impl Drop for Vm {
    fn drop(&mut self) {
        // Safety: `self.0` came from `ume_open` and is closed once.
        unsafe { ume_close(self.0) }
    }
}

/// Runs the boot script and then the script. The `Err` is the text for the standard error: the
/// error of the script as `UmeBoot.describe` (in the boot script) tells it.
pub fn run(script: &Script<'_>, host: &mut dyn Host) -> Result<(), String> {
    let mut context = Context {
        host,
        names: script.functions,
    };
    // Safety: `context` outlives `vm`, which is dropped (and the VM closed) before this
    // function returns; the callbacks run only inside the `ume_run` calls below.
    unsafe {
        let mrb = ume_open((&raw mut context).cast(), trampoline);
        if mrb.is_null() {
            return Err("cannot start mruby".into());
        }
        let vm = Vm(mrb);
        for (index, name) in script.functions.iter().enumerate() {
            ume_define_function(mrb, name.as_ptr().cast(), name.len(), index);
        }
        if let Some(message) = run_source(mrb, "ume-script boot", script.boot) {
            return Err(message);
        }
        ume_set_info(mrb, write(mrb, script.info));
        let result = run_source(mrb, script.name, script.source);
        drop(vm);
        result.map_or(Ok(()), Err)
    }
}

/// Safety: `mrb` is a live VM.
unsafe fn run_source(mrb: *mut MrbState, name: &str, source: &str) -> Option<String> {
    // A NUL ends a C string early; no real file name has one.
    let name = std::ffi::CString::new(name.replace('\0', "?")).unwrap_or_default();
    let error = unsafe { ume_run(mrb, name.as_ptr(), source.as_ptr().cast(), source.len()) };
    match unsafe { read(mrb, error, 0) } {
        Value::Str(text) => Some(String::from_utf8_lossy(&text).into_owned()),
        _ => None,
    }
}
