//! MicroPython, built from the sources that its embed port generated (`vendor/micropython`, see
//! the README there), and the one thing UME Editor does with it: run one script, with the macro
//! API reachable through a native function.
//!
//! The VM is not thread-safe and keeps its state in C globals, so [`run`] may be called once at a
//! time in a process (the script host runs a single script and exits).
#![allow(unsafe_code)]

use std::ffi::{c_char, c_int, c_longlong, c_void};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

/// The answer of the editor to one call of the macro API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallOutcome {
    /// The result, as JSON text.
    Ok(String),
    /// The result is a string: its text itself (not JSON, so that a long one costs no escaping).
    Text(String),
    /// The editor answered with an error object.
    Error { code: i64, message: String },
    /// The connection is gone: the script ends with `EOFError` and this message.
    Closed(String),
}

/// What the script reaches the editor through.
pub trait Host {
    /// One call of the macro API: the name of the function and its arguments as a JSON array.
    /// The bytes are what the script produced; they may not be valid UTF-8 (text with a lone
    /// surrogate, say), which is for the host to refuse.
    fn call(&mut self, name: &[u8], args: &[u8]) -> CallOutcome;
}

/// A text with the name that tracebacks show.
#[derive(Debug, Clone, Copy)]
pub struct Source<'a> {
    pub name: &'a str,
    pub text: &'a str,
}

/// The heap of the VM, in bytes. It is reserved from the operating system and used page by page,
/// so a small script touches little of it. 64 MiB lets a macro hold a few large selections
/// (MicroPython keeps text as UTF-8, one byte for ASCII); past it the script gets `MemoryError`.
pub const HEAP_SIZE: usize = 64 * 1024 * 1024;

/// How much stack the VM may use before it raises `RuntimeError` for recursion that is too deep.
/// The main thread has 1 MiB on Windows, the least of the targets.
pub const STACK_LIMIT: usize = 512 * 1024;

type CallFn = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    usize,
    *const c_char,
    usize,
    *mut c_longlong,
    *mut *const c_char,
    *mut usize,
) -> c_int;

#[repr(C)]
struct RawConfig {
    heap_size: usize,
    stack_limit: usize,
    call: CallFn,
    ctx: *mut c_void,
    names_json: *const c_char,
    names_json_len: usize,
    info_json: *const c_char,
    info_json_len: usize,
    boot_name: *const c_char,
    boot_name_len: usize,
    boot_source: *const c_char,
    boot_source_len: usize,
    script_name: *const c_char,
    script_name_len: usize,
    script_source: *const c_char,
    script_source_len: usize,
}

unsafe extern "C" {
    fn ume_mp_run(
        config: *const RawConfig,
        message: *mut *mut c_char,
        message_len: *mut usize,
    ) -> c_int;
    fn ume_mp_free(pointer: *mut c_void);
}

const RUN_OK: c_int = 0;
const RUN_NO_HEAP: c_int = 2;

const CALL_OK: c_int = 0;
const CALL_TEXT: c_int = 3;
const CALL_ERROR: c_int = 1;
const CALL_CLOSED: c_int = 2;

/// The state the C callback needs: the host, and the bytes of the last answer (the C side
/// copies them at once, before the next call).
struct Context<'a> {
    host: &'a mut dyn Host,
    answer: Vec<u8>,
}

/// Writes what the script printed. It goes to the process's output (fd 1) or error (fd 2) and
/// is flushed at once, so that it stays in order with the requests written to the same pipe.
///
/// # Safety
/// Called by the C glue with a valid pointer and length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ume_mp_write(fd: c_int, bytes: *const c_char, len: usize) {
    if len == 0 {
        return;
    }
    // SAFETY: the glue passes the bytes it was asked to write.
    let bytes = unsafe { std::slice::from_raw_parts(bytes.cast::<u8>(), len) };
    // A closed pipe means the editor has gone; there is nobody to read it.
    if fd == 2 {
        let mut out = std::io::stderr().lock();
        let _ = out.write_all(bytes).and_then(|()| out.flush());
    } else {
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(bytes).and_then(|()| out.flush());
    }
}

unsafe extern "C" fn call_host(
    ctx: *mut c_void,
    name: *const c_char,
    name_len: usize,
    args: *const c_char,
    args_len: usize,
    code: *mut c_longlong,
    out: *mut *const c_char,
    out_len: *mut usize,
) -> c_int {
    // SAFETY: `ctx` is the `Context` that `run` made and keeps alive; the strings are those
    // of the VM, alive during this call.
    let (ctx, name, args) = unsafe {
        (
            &mut *ctx.cast::<Context>(),
            std::slice::from_raw_parts(name.cast::<u8>(), name_len),
            std::slice::from_raw_parts(args.cast::<u8>(), args_len),
        )
    };
    let (status, text) = match ctx.host.call(name, args) {
        CallOutcome::Ok(result) => (CALL_OK, result),
        CallOutcome::Text(text) => (CALL_TEXT, text),
        CallOutcome::Error { code: c, message } => {
            // SAFETY: valid out-pointer of the glue.
            unsafe { *code = c };
            (CALL_ERROR, message)
        }
        CallOutcome::Closed(message) => (CALL_CLOSED, message),
    };
    ctx.answer = text.into_bytes();
    // SAFETY: valid out-pointers of the glue; the bytes live in `ctx` until the next call.
    unsafe {
        *out = ctx.answer.as_ptr().cast();
        *out_len = ctx.answer.len();
    }
    status
}

/// The VM keeps its state in C globals: one run at a time in the process.
static RUNNING: AtomicBool = AtomicBool::new(false);

struct RunGuard;

impl RunGuard {
    fn acquire() -> Option<Self> {
        (!RUNNING.swap(true, Ordering::Acquire)).then_some(Self)
    }
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        RUNNING.store(false, Ordering::Release);
    }
}

/// Runs the boot script (it defines `Editor` and `EditorError`; it calls `_ume_setup()` for
/// `names_json` and `info_json` and `_ume_call(name, args_json)` for the API) and then the
/// user's script, in the same globals.
///
/// `Err` is the text for the standard error: the traceback of an exception that the script did
/// not catch (or that another run is going on in this process). The script's own output has
/// already been written by then.
pub fn run(
    host: &mut dyn Host,
    names_json: &str,
    info_json: &str,
    boot: Source<'_>,
    script: Source<'_>,
) -> Result<(), String> {
    let Some(_guard) = RunGuard::acquire() else {
        return Err("MicroPython is already running a script in this process".into());
    };
    let mut ctx = Context {
        host,
        answer: Vec::new(),
    };
    let ptr = |s: &str| s.as_ptr().cast::<c_char>();
    let config = RawConfig {
        heap_size: HEAP_SIZE,
        stack_limit: STACK_LIMIT,
        call: call_host,
        ctx: (&raw mut ctx).cast(),
        names_json: ptr(names_json),
        names_json_len: names_json.len(),
        info_json: ptr(info_json),
        info_json_len: info_json.len(),
        boot_name: ptr(boot.name),
        boot_name_len: boot.name.len(),
        boot_source: ptr(boot.text),
        boot_source_len: boot.text.len(),
        script_name: ptr(script.name),
        script_name_len: script.name.len(),
        script_source: ptr(script.text),
        script_source_len: script.text.len(),
    };
    let mut message: *mut c_char = std::ptr::null_mut();
    let mut message_len = 0usize;
    // SAFETY: `config` and what it points to outlive the call; the glue sets `message`.
    let status = unsafe { ume_mp_run(&config, &mut message, &mut message_len) };
    let text = if message.is_null() {
        String::new()
    } else {
        // SAFETY: malloc'ed by the glue, `message_len` bytes.
        let bytes = unsafe { std::slice::from_raw_parts(message.cast::<u8>(), message_len) };
        let text = String::from_utf8_lossy(bytes).into_owned();
        // SAFETY: allocated by the glue's malloc, freed once.
        unsafe { ume_mp_free(message.cast()) };
        text
    };
    match status {
        RUN_OK => Ok(()),
        RUN_NO_HEAP => Err("cannot allocate the heap of MicroPython".into()),
        _ => Err(text.trim_end_matches('\n').to_string()),
    }
}
