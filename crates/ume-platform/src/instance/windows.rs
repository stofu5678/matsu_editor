//! The receiver and the sender on Windows: a named pipe.
//!
//! The pipe is `\\.\pipe\ume-editor-<SID of the user>-<key>`. Whoever creates its first instance
//! (`FILE_FLAG_FIRST_PIPE_INSTANCE`) is the receiver; a second try fails, also when two launches
//! race. The security descriptor lets only the user's own SID in, and the pipe refuses clients on
//! other machines (`PIPE_REJECT_REMOTE_CLIENTS`). The sender asks the receiver's process to be
//! allowed to take the foreground (`AllowSetForegroundWindow`), so that its window can come to
//! the front.
//!
//! A pipe has no read timeout, so the sender does its I/O on a thread and waits for it with a
//! timeout; a receiver that never answers leaves that thread behind, and the sender goes on alone.
//! The receiver likewise serves each client on a thread of its own.

// The Win32 calls are `unsafe`. This module and the other OS modules of the crate are the only
// places of the program that need it (ADR-0013).
#![allow(unsafe_code)]

use std::fs::File;
use std::io;
use std::os::windows::io::{FromRawHandle, OwnedHandle};
use std::ptr::{null, null_mut};
use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED,
    GENERIC_READ, GENERIC_WRITE, GetLastError, HANDLE, INVALID_HANDLE_VALUE, LocalFree,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_FIRST_PIPE_INSTANCE, FlushFileBuffers, OPEN_EXISTING,
    PIPE_ACCESS_DUPLEX, SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeServerProcessId, PIPE_READMODE_BYTE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    WaitNamedPipeW,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow;

use super::{Handler, Key, Reply, Request, SendError, Timeouts, exchange, serve_connection};

/// The buffer of the pipe in each direction. A command line is far smaller.
const BUFFER: u32 = 64 * 1024;

/// Nothing to keep: the pipe belongs to the process until it ends.
pub(super) struct Guard;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// The SID of the user of this process, as text (`S-1-5-21-...`).
fn user_sid() -> io::Result<String> {
    let mut token: HANDLE = null_mut();
    // SAFETY: `token` is written by the call; it is closed below.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let result = sid_of_token(token);
    // SAFETY: `token` is the handle opened above.
    unsafe { CloseHandle(token) };
    result
}

fn sid_of_token(token: HANDLE) -> io::Result<String> {
    let mut size = 0u32;
    // SAFETY: asks for the size only (a null buffer of length 0); the call fails and sets `size`.
    unsafe { GetTokenInformation(token, TokenUser, null_mut(), 0, &mut size) };
    if size == 0 {
        return Err(io::Error::last_os_error());
    }
    // 8-byte units, so that the `TOKEN_USER` in the buffer is aligned.
    let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
    // SAFETY: the buffer has at least `size` bytes.
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            size,
            &mut size,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the call filled the buffer with a `TOKEN_USER`, which points into the same buffer.
    let sid = unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    let mut text: *mut u16 = null_mut();
    // SAFETY: `sid` is valid while `buffer` lives; the call allocates `text`, freed below.
    if unsafe { ConvertSidToStringSidW(sid, &mut text) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `text` is a NUL-terminated UTF-16 string from the call above.
    let string = unsafe {
        let mut len = 0;
        while *text.add(len) != 0 {
            len += 1;
        }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(text, len));
        LocalFree(text.cast());
        s
    };
    Ok(string)
}

fn pipe_name(sid: &str, key: &Key) -> String {
    format!(r"\\.\pipe\ume-editor-{sid}-{}", key.as_str())
}

/// Makes one instance of the pipe, for the user's SID only.
fn create_instance(name: &[u16], sid: &str, first: bool) -> io::Result<OwnedHandle> {
    // "D:P": a protected DACL; one entry that allows everything ("GA") to the user.
    let sddl = wide(&format!("D:P(A;;GA;;;{sid})"));
    let mut descriptor: *mut core::ffi::c_void = null_mut();
    // SAFETY: `sddl` is NUL-terminated; the call allocates `descriptor`, freed below.
    let ok = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            null_mut(),
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let mut open_mode = PIPE_ACCESS_DUPLEX;
    if first {
        open_mode |= FILE_FLAG_FIRST_PIPE_INSTANCE;
    }
    // SAFETY: `name` is NUL-terminated and `attributes` points to a live descriptor; the pipe
    // keeps its own copy of the descriptor.
    let handle = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            open_mode,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES,
            BUFFER,
            BUFFER,
            0,
            &attributes,
        )
    };
    let error = io::Error::last_os_error();
    // SAFETY: `descriptor` was allocated by the conversion above.
    unsafe { LocalFree(descriptor) };
    if handle == INVALID_HANDLE_VALUE {
        return Err(error);
    }
    // SAFETY: a valid handle that nothing else owns.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

/// Takes the key if nobody holds it: `Ok(Some)` is the receiver, `Ok(None)` means someone else is.
pub(super) fn try_receive(key: &Key, handler: Arc<Handler>) -> io::Result<Option<Guard>> {
    let sid = user_sid()?;
    let name = wide(&pipe_name(&sid, key));
    let first = match create_instance(&name, &sid, true) {
        Ok(handle) => handle,
        // The first instance is taken (access denied) or all instances are in use.
        Err(e)
            if matches!(
                e.raw_os_error().map(|c| c as u32),
                Some(ERROR_ACCESS_DENIED | ERROR_PIPE_BUSY)
            ) =>
        {
            return Ok(None);
        }
        Err(e) => return Err(e),
    };
    std::thread::Builder::new()
        .name("ume-instance-accept".to_owned())
        .spawn(move || accept_loop(first, &name, &sid, &handler))?;
    Ok(Some(Guard))
}

fn accept_loop(mut pipe: OwnedHandle, name: &[u16], sid: &str, handler: &Arc<Handler>) {
    use std::os::windows::io::AsRawHandle;
    loop {
        // SAFETY: `pipe` is a pipe handle; no OVERLAPPED, so the call waits for a client.
        let connected = unsafe { ConnectNamedPipe(pipe.as_raw_handle(), null_mut()) } != 0
            // A client that connected before the call is fine.
            || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED;
        // The next client finds a free instance while this one is served.
        let Ok(next) = create_instance(name, sid, false) else {
            return;
        };
        let served = std::mem::replace(&mut pipe, next);
        if !connected {
            continue;
        }
        let handler = handler.clone();
        let _ = std::thread::Builder::new()
            .name("ume-instance-client".to_owned())
            .spawn(move || {
                use std::os::windows::io::AsRawHandle;
                let mut file = File::from(served);
                let _ = serve_connection(&mut file, &handler);
                // `File::flush` does nothing. Closing the pipe before the client has read the
                // answer would lose it, and the client would run on its own after the request
                // was handled: wait until the client has read it.
                // SAFETY: `file` owns a pipe handle.
                unsafe { FlushFileBuffers(file.as_raw_handle()) };
            });
    }
}

/// Sends `request` to the receiver of `key`.
pub(super) fn send(key: &Key, request: &Request, timeouts: Timeouts) -> Result<Reply, SendError> {
    let sid = user_sid()?;
    let name = wide(&pipe_name(&sid, key));
    let file = connect(&name, timeouts.receiver)?;
    allow_foreground(&file);
    // The I/O on a thread, so that a receiver that does not answer cannot hold us.
    let (tx, rx) = mpsc::channel();
    let request = request.clone();
    std::thread::Builder::new()
        .name("ume-instance-send".to_owned())
        .spawn(move || {
            let mut file = file;
            let _ = tx.send(exchange(&mut file, &request));
        })?;
    match rx.recv_timeout(timeouts.reply) {
        Ok(result) => Ok(result?),
        Err(_) => Err(SendError::TimedOut),
    }
}

/// Opens the pipe, trying again for `wait` while it does not exist yet or all its instances are
/// busy.
fn connect(name: &[u16], wait: Duration) -> Result<File, SendError> {
    let start = Instant::now();
    loop {
        // SAFETY: `name` is NUL-terminated. SQOS with identification only: the receiver cannot
        // act as us.
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                null(),
                OPEN_EXISTING,
                SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
                null_mut(),
            )
        };
        if handle != INVALID_HANDLE_VALUE {
            // SAFETY: a valid handle that nothing else owns.
            return Ok(File::from(unsafe { OwnedHandle::from_raw_handle(handle) }));
        }
        let code = unsafe { GetLastError() };
        if start.elapsed() >= wait {
            return Err(match code {
                ERROR_FILE_NOT_FOUND => SendError::NoReceiver,
                ERROR_PIPE_BUSY => SendError::TimedOut,
                _ => SendError::Io(io::Error::from_raw_os_error(code as i32)),
            });
        }
        match code {
            // SAFETY: `name` is NUL-terminated.
            ERROR_PIPE_BUSY => {
                unsafe { WaitNamedPipeW(name.as_ptr(), 100) };
            }
            ERROR_FILE_NOT_FOUND => std::thread::sleep(Duration::from_millis(20)),
            _ => return Err(SendError::Io(io::Error::from_raw_os_error(code as i32))),
        }
    }
}

/// Lets the process on the other end of the pipe take the foreground.
fn allow_foreground(file: &File) {
    use std::os::windows::io::AsRawHandle;
    let mut pid = 0u32;
    // SAFETY: the handle is a pipe client handle; `pid` is written by the call.
    if unsafe { GetNamedPipeServerProcessId(file.as_raw_handle(), &mut pid) } != 0 {
        // SAFETY: plain call; failure only means the window may not come to the front.
        unsafe { AllowSetForegroundWindow(pid) };
    }
}
