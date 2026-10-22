//! mruby 4.0 for the script host of MATSU Editor: the C source is compiled by `build.rs`, and this
//! crate offers one safe function, [`run`]: start a VM, bind host functions as the methods of a
//! module `Editor`, run a boot script and then the user's script.
//!
//! Values cross the boundary as [`Value`], a copy; no mruby object is visible outside.
//!
//! mruby reports errors with `longjmp`. Rust is only called from one native method, and it
//! answers with a status that the C glue (`glue.c`) turns into an exception after Rust has
//! returned, so no Rust frame is jumped over by a script error. The mruby calls that Rust makes
//! to copy a value (which can fail when the value is too large) run under `mrb_protect_error`
//! in the glue and answer with a status. Reading a value calls no Ruby method.

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
    /// Ruby source run first; it defines `EditorError` and `MatsuBoot.describe(exception, name)`
    /// (the text for the standard error), and `Editor.info`'s reader. See
    /// `matsu-script/src/mruby_boot.rb`.
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
    fn matsu_open(host: *mut c_void, call: CallFn) -> *mut MrbState;
    fn matsu_close(mrb: *mut MrbState);
    fn matsu_define_function(mrb: *mut MrbState, name: *const c_char, len: usize, index: usize);
    fn matsu_set_info(mrb: *mut MrbState, info: MrbValue);
    fn matsu_run(
        mrb: *mut MrbState,
        name: *const c_char,
        source: *const c_char,
        len: usize,
    ) -> MrbValue;

    fn matsu_kind(v: MrbValue) -> c_int;
    fn matsu_integer(v: MrbValue) -> i64;
    fn matsu_float(v: MrbValue) -> f64;
    fn matsu_string(v: MrbValue, len: *mut usize) -> *const u8;
    fn matsu_symbol(mrb: *mut MrbState, v: MrbValue, len: *mut usize) -> *const u8;
    fn matsu_array_len(v: MrbValue) -> usize;
    fn matsu_array_ref(mrb: *mut MrbState, v: MrbValue, i: usize) -> MrbValue;
    fn matsu_hash_keys(mrb: *mut MrbState, hash: MrbValue, out: *mut MrbValue) -> c_int;
    fn matsu_hash_values(mrb: *mut MrbState, hash: MrbValue, out: *mut MrbValue) -> c_int;
    fn matsu_class_name(mrb: *mut MrbState, v: MrbValue) -> *const c_char;

    fn matsu_nil() -> MrbValue;
    fn matsu_bool(b: c_int) -> MrbValue;
    // These make objects, which can fail (too large): they answer 0 then, never jump.
    fn matsu_integer_value(mrb: *mut MrbState, i: i64, out: *mut MrbValue) -> c_int;
    fn matsu_float_value(mrb: *mut MrbState, f: f64, out: *mut MrbValue) -> c_int;
    fn matsu_string_new(mrb: *mut MrbState, p: *const u8, len: usize, out: *mut MrbValue) -> c_int;
    fn matsu_array_new(mrb: *mut MrbState, capacity: usize, out: *mut MrbValue) -> c_int;
    fn matsu_array_push(mrb: *mut MrbState, array: MrbValue, item: MrbValue) -> c_int;
    fn matsu_hash_new(mrb: *mut MrbState, out: *mut MrbValue) -> c_int;
    fn matsu_hash_set(mrb: *mut MrbState, hash: MrbValue, key: MrbValue, value: MrbValue) -> c_int;
}

/// How deep a value may nest, and how many values a copy may visit, before the value is taken
/// for one that contains itself. An array that holds itself twice (`a << a << a`) has a small
/// depth but an astronomical number of paths, so the number of visits is limited too.
const MAX_DEPTH: usize = 64;
const MAX_VISITS: usize = 1_000_000;
const TOO_DEEP: &str = "a nested structure that is too deep or large (or contains itself)";

struct Reader {
    mrb: *mut MrbState,
    visits: usize,
}

impl Reader {
    /// Safety: `self.mrb` is a live VM and `v` a value of it.
    ///
    /// Nothing here calls a Ruby method: a hash is read through its stored keys and values, not
    /// by looking the keys up (`Hash#[]` would call the `hash` and `eql?` of a user's key, and an
    /// exception in them would jump over the Rust frames).
    unsafe fn read(&mut self, v: MrbValue, depth: usize) -> Value {
        let mrb = self.mrb;
        self.visits += 1;
        let too_deep = depth >= MAX_DEPTH || self.visits > MAX_VISITS;
        unsafe {
            match matsu_kind(v) {
                kind::NIL => Value::Nil,
                kind::FALSE => Value::Bool(false),
                kind::TRUE => Value::Bool(true),
                kind::INTEGER => Value::Integer(matsu_integer(v)),
                kind::FLOAT => Value::Float(matsu_float(v)),
                kind::STRING => {
                    let mut len = 0;
                    let p = matsu_string(v, &mut len);
                    Value::Str(bytes(p, len))
                }
                kind::SYMBOL => {
                    let mut len = 0;
                    let p = matsu_symbol(mrb, v, &mut len);
                    Value::Str(bytes(p, len))
                }
                kind::ARRAY | kind::HASH if too_deep => Value::Other(TOO_DEEP.into()),
                kind::ARRAY => {
                    let len = matsu_array_len(v);
                    let mut items = Vec::new();
                    for i in 0..len {
                        if self.visits > MAX_VISITS {
                            return Value::Other(TOO_DEEP.into());
                        }
                        items.push(self.read(matsu_array_ref(mrb, v, i), depth + 1));
                    }
                    Value::Array(items)
                }
                kind::HASH => {
                    let (mut keys, mut values) = (matsu_nil(), matsu_nil());
                    if matsu_hash_keys(mrb, v, &mut keys) == 0
                        || matsu_hash_values(mrb, v, &mut values) == 0
                    {
                        return Value::Other("a hash that is too large".into());
                    }
                    let len = matsu_array_len(keys).min(matsu_array_len(values));
                    let mut pairs = Vec::new();
                    for i in 0..len {
                        if self.visits > MAX_VISITS {
                            return Value::Other(TOO_DEEP.into());
                        }
                        pairs.push((
                            self.read(matsu_array_ref(mrb, keys, i), depth + 1),
                            self.read(matsu_array_ref(mrb, values, i), depth + 1),
                        ));
                    }
                    Value::Hash(pairs)
                }
                _ => Value::Other(
                    CStr::from_ptr(matsu_class_name(mrb, v))
                        .to_string_lossy()
                        .into_owned(),
                ),
            }
        }
    }
}

/// Safety: `mrb` is a live VM and `v` a value of it.
unsafe fn read(mrb: *mut MrbState, v: MrbValue) -> Value {
    unsafe { Reader { mrb, visits: 0 }.read(v, 0) }
}

unsafe fn bytes(p: *const u8, len: usize) -> Vec<u8> {
    if len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(p, len) }.to_vec()
    }
}

/// The value could not be made in mruby (it is too large).
struct TooLarge;

/// Safety: `mrb` is a live VM. `Other` has no mruby form and becomes nil.
unsafe fn write(mrb: *mut MrbState, value: &Value) -> Result<MrbValue, TooLarge> {
    let mut out = unsafe { matsu_nil() };
    let made = unsafe {
        match value {
            Value::Nil | Value::Other(_) => return Ok(matsu_nil()),
            Value::Bool(b) => return Ok(matsu_bool(c_int::from(*b))),
            Value::Integer(i) => matsu_integer_value(mrb, *i, &mut out),
            Value::Float(f) => matsu_float_value(mrb, *f, &mut out),
            Value::Str(s) => matsu_string_new(mrb, s.as_ptr(), s.len(), &mut out),
            Value::Array(items) => {
                let mut made = matsu_array_new(mrb, items.len(), &mut out);
                for item in items {
                    if made == 0 {
                        break;
                    }
                    made = matsu_array_push(mrb, out, write(mrb, item)?);
                }
                made
            }
            Value::Hash(pairs) => {
                let mut made = matsu_hash_new(mrb, &mut out);
                for (key, item) in pairs {
                    if made == 0 {
                        break;
                    }
                    made = matsu_hash_set(mrb, out, write(mrb, key)?, write(mrb, item)?);
                }
                made
            }
        }
    };
    if made == 0 { Err(TooLarge) } else { Ok(out) }
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
    // Safety: `context` is the `Context` that `run` passed to `matsu_open`, alive for the whole
    // run, and the VM runs on this thread only.
    let context = unsafe { &mut *context.cast::<Context<'_>>() };
    let answer = catch_unwind(AssertUnwindSafe(|| {
        let args = match unsafe { read(mrb, args) } {
            Value::Array(items) => items,
            _ => Vec::new(),
        };
        context.host.call(context.names[index], args)
    }))
    .unwrap_or_else(|_| Err(Failure::Fatal("internal error in the script host".into())));
    // Safety: `out` points at a local of the C glue.
    unsafe {
        let (status, value) = match answer {
            Ok(value) => (0, value),
            Err(Failure::Editor { code, message }) => (
                1,
                Value::Array(vec![Value::Integer(code), Value::Str(message.into_bytes())]),
            ),
            Err(Failure::Fatal(message)) => (2, Value::Str(message.into_bytes())),
        };
        match write(mrb, &value) {
            Ok(made) => {
                *out = made;
                status
            }
            Err(TooLarge) => 3,
        }
    }
}

/// Closes the VM when the run ends, however it ends.
struct Vm(*mut MrbState);

impl Drop for Vm {
    fn drop(&mut self) {
        // Safety: `self.0` came from `matsu_open` and is closed once.
        unsafe { matsu_close(self.0) }
    }
}

/// Runs the boot script and then the script. The `Err` is the text for the standard error: the
/// error of the script as `MatsuBoot.describe` (in the boot script) tells it.
pub fn run(script: &Script<'_>, host: &mut dyn Host) -> Result<(), String> {
    let mut context = Context {
        host,
        names: script.functions,
    };
    // Safety: `context` outlives `vm`, which is dropped (and the VM closed) before this
    // function returns; the callbacks run only inside the `matsu_run` calls below.
    unsafe {
        let mrb = matsu_open((&raw mut context).cast(), trampoline);
        if mrb.is_null() {
            return Err("cannot start mruby".into());
        }
        let vm = Vm(mrb);
        for (index, name) in script.functions.iter().enumerate() {
            matsu_define_function(mrb, name.as_ptr().cast(), name.len(), index);
        }
        if let Some(message) = run_source(mrb, "matsu-script boot", script.boot) {
            return Err(message);
        }
        match write(mrb, script.info) {
            Ok(info) => matsu_set_info(mrb, info),
            Err(TooLarge) => return Err("cannot pass the editor's information to mruby".into()),
        }
        let result = run_source(mrb, script.name, script.source);
        drop(vm);
        result.map_or(Ok(()), Err)
    }
}

/// Safety: `mrb` is a live VM.
unsafe fn run_source(mrb: *mut MrbState, name: &str, source: &str) -> Option<String> {
    // A NUL ends a C string early; no real file name has one.
    let name = std::ffi::CString::new(name.replace('\0', "?")).unwrap_or_default();
    let error = unsafe { matsu_run(mrb, name.as_ptr(), source.as_ptr().cast(), source.len()) };
    match unsafe { read(mrb, error) } {
        Value::Str(text) => Some(String::from_utf8_lossy(&text).into_owned()),
        _ => None,
    }
}
