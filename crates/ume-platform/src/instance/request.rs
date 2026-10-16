//! What a second launch sends to the running UME Editor, and the bytes it travels as.
//!
//! The message is `[length: u32][version: u32][body]`, all numbers little endian. The length counts
//! everything after itself, so a receiver can always read a whole message, also one of a version
//! it does not understand, and then answer "refused" ([`Reply::Refused`]) instead of leaving the
//! sender waiting. Version 1's body is the working folder and the arguments as given, each as
//! `[length: u32][bytes]` (the arguments after a `[count: u32]`). A path or an argument is not
//! always text, so it travels as the bytes of the OS (UTF-8 bytes on Unix, UTF-16 units on Windows).
//! A new field goes into a new version, and a receiver reads only the versions it knows.

use std::ffi::OsString;
use std::path::PathBuf;

/// The version of the request that this build sends and reads.
pub const VERSION: u32 = 1;

/// The largest message a receiver accepts. A command line is far smaller; the limit keeps a bad
/// client from making the receiver allocate without end.
pub const MAX_MESSAGE: usize = 4 * 1024 * 1024;

/// What a launch asks the running UME Editor to do: the working folder and the arguments as given.
/// The sender makes the paths absolute, and the receiver still resolves a relative one against
/// `cwd`, so that options can be added later (Sakura's `-R`, `-X`/`-Y`, `-CODE`, ...) without
/// changing the format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    /// The folder the second launch was started in.
    pub cwd: PathBuf,
    /// The arguments after the program name, in order.
    pub args: Vec<OsString>,
}

/// The receiver's answer, one byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reply {
    /// The request was read and handed over.
    Accepted,
    /// The request is of a version, or in a form, that the receiver cannot read.
    Refused,
}

impl Reply {
    pub(super) fn to_byte(self) -> u8 {
        match self {
            Reply::Accepted => 0,
            Reply::Refused => 1,
        }
    }

    pub(super) fn from_byte(byte: u8) -> Reply {
        if byte == 0 {
            Reply::Accepted
        } else {
            Reply::Refused
        }
    }
}

/// Why a message could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The request is of another version. Carries that version.
    Version(u32),
    /// The bytes are cut short, too long, or not what the version says.
    Malformed,
}

impl Request {
    /// A request for the current process: its working folder and arguments (the paths made
    /// absolute against the working folder).
    pub fn from_current_process(args: impl IntoIterator<Item = OsString>) -> Request {
        let cwd = std::env::current_dir().unwrap_or_default();
        Request {
            args: args.into_iter().map(|a| absolute_arg(&cwd, a)).collect(),
            cwd,
        }
    }

    /// The whole message, with its length in front.
    pub fn encode(&self) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&VERSION.to_le_bytes());
        put_bytes(&mut body, &os_to_bytes(self.cwd.as_os_str()));
        body.extend_from_slice(&(self.args.len() as u32).to_le_bytes());
        for arg in &self.args {
            put_bytes(&mut body, &os_to_bytes(arg));
        }
        let mut message = (body.len() as u32).to_le_bytes().to_vec();
        message.extend(body);
        message
    }

    /// Reads what follows the length of a message: the version and the body.
    pub fn decode(body: &[u8]) -> Result<Request, DecodeError> {
        let mut input = Input(body);
        let version = input.u32()?;
        if version != VERSION {
            return Err(DecodeError::Version(version));
        }
        let cwd = PathBuf::from(os_from_bytes(input.bytes()?));
        let count = input.u32()? as usize;
        // Each argument takes at least four bytes, so a count that cannot fit is a lie.
        if count > input.0.len() / 4 {
            return Err(DecodeError::Malformed);
        }
        let mut args = Vec::with_capacity(count);
        for _ in 0..count {
            args.push(os_from_bytes(input.bytes()?));
        }
        if !input.0.is_empty() {
            return Err(DecodeError::Malformed);
        }
        Ok(Request { cwd, args })
    }
}

/// An argument as the sender passes it on: made absolute against `cwd` unless it is empty.
/// (Every argument is a path for now; the options of later versions will be left as they are.)
fn absolute_arg(cwd: &std::path::Path, arg: OsString) -> OsString {
    if arg.is_empty() {
        return arg;
    }
    cwd.join(arg).into_os_string()
}

fn put_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
}

struct Input<'a>(&'a [u8]);

impl<'a> Input<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if self.0.len() < n {
            return Err(DecodeError::Malformed);
        }
        let (head, tail) = self.0.split_at(n);
        self.0 = tail;
        Ok(head)
    }

    fn u32(&mut self) -> Result<u32, DecodeError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn bytes(&mut self) -> Result<&'a [u8], DecodeError> {
        let len = self.u32()? as usize;
        self.take(len)
    }
}

#[cfg(unix)]
fn os_to_bytes(s: &std::ffi::OsStr) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    s.as_bytes().to_vec()
}

#[cfg(unix)]
fn os_from_bytes(bytes: &[u8]) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(bytes.to_vec())
}

#[cfg(windows)]
fn os_to_bytes(s: &std::ffi::OsStr) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;
    s.encode_wide().flat_map(u16::to_le_bytes).collect()
}

#[cfg(windows)]
fn os_from_bytes(bytes: &[u8]) -> OsString {
    use std::os::windows::ffi::OsStringExt;
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    OsString::from_wide(&units)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Request {
        Request {
            cwd: PathBuf::from(if cfg!(windows) { r"C:\work" } else { "/work" }),
            args: vec![OsString::from("日本語.txt"), OsString::from("b c.txt")],
        }
    }

    #[test]
    fn a_request_round_trips() {
        let request = sample();
        let message = request.encode();
        let length = u32::from_le_bytes(message[..4].try_into().unwrap()) as usize;
        assert_eq!(length, message.len() - 4);
        assert_eq!(Request::decode(&message[4..]), Ok(request));
    }

    #[test]
    fn an_empty_request_round_trips() {
        let request = Request {
            cwd: PathBuf::new(),
            args: Vec::new(),
        };
        assert_eq!(Request::decode(&request.encode()[4..]), Ok(request));
    }

    #[cfg(unix)]
    #[test]
    fn an_argument_that_is_not_utf8_survives() {
        use std::os::unix::ffi::OsStringExt;
        let request = Request {
            cwd: PathBuf::from("/"),
            args: vec![OsString::from_vec(vec![b'a', 0xff, 0xfe])],
        };
        assert_eq!(Request::decode(&request.encode()[4..]), Ok(request));
    }

    #[test]
    fn another_version_is_told_apart_from_a_broken_message() {
        let mut body = sample().encode()[4..].to_vec();
        body[..4].copy_from_slice(&(VERSION + 1).to_le_bytes());
        assert_eq!(
            Request::decode(&body),
            Err(DecodeError::Version(VERSION + 1))
        );
        // The rest of a newer version may look like anything.
        body.truncate(5);
        assert_eq!(
            Request::decode(&body),
            Err(DecodeError::Version(VERSION + 1))
        );
        assert_eq!(Request::decode(&[1]), Err(DecodeError::Malformed));
    }

    #[test]
    fn a_cut_or_padded_message_is_malformed() {
        let body = sample().encode()[4..].to_vec();
        for cut in 1..body.len() {
            assert_eq!(
                Request::decode(&body[..cut]),
                Err(DecodeError::Malformed),
                "cut at {cut}"
            );
        }
        let mut padded = body;
        padded.push(0);
        assert_eq!(Request::decode(&padded), Err(DecodeError::Malformed));
    }

    #[test]
    fn a_count_that_cannot_fit_is_malformed() {
        let mut body = VERSION.to_le_bytes().to_vec();
        put_bytes(&mut body, b"");
        body.extend_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(Request::decode(&body), Err(DecodeError::Malformed));
    }

    #[test]
    fn the_sender_makes_the_arguments_absolute() {
        let request = Request::from_current_process([OsString::from("a.txt"), OsString::new()]);
        assert_eq!(request.args[0], request.cwd.join("a.txt").into_os_string());
        assert!(request.args[1].is_empty());
        assert!(request.cwd.is_absolute());
    }
}
