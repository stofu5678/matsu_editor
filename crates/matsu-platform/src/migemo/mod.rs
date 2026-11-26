//! C/Migemo (koron/cmigemo, MIT): the library that turns romaji into a regular expression that
//! matches the same word written in kana and kanji, for the incremental search of Migemo.
//!
//! Sakura: `sakura_core/extmodule/CMigemo.cpp` and `CMigemo.h`. The user puts the library and the
//! dictionary there (the decision of 2026-11-25); this crate loads the library at run time and
//! binds the eight functions Sakura binds (`migemo_open`, `migemo_close`, `migemo_query`,
//! `migemo_release`, `migemo_setproc_char2int`, `migemo_setproc_int2char`, `migemo_load`,
//! `migemo_is_enable`).
//!
//! # Where the library is
//!
//! Sakura asks `GetInidirOrExedir` for the configured name (a relative name is looked up in the
//! folder of the settings file, then in the folder of the program), uses it if the file is there,
//! and otherwise hands the bare name `migemo.dll` to `LoadLibrary`, which searches the folder of
//! the program, the system folders, the current folder and `PATH`. This module keeps the first
//! half and makes the second half safe ([`library_path`]): the library is only ever loaded from a
//! full path, and a bare name is looked up in the two folders only. The current folder and `PATH`
//! are never searched, so that a file placed in a folder the user happened to start MATSU Editor
//! in is not run.
//!
//! # The dictionary
//!
//! Sakura looks in the folder of the settings (a relative name) for `utf-8/migemo-dict` and then
//! `cp932/migemo-dict`. Only the UTF-8 dictionary is used here ([`dictionary_dir`]): the query
//! and the pattern then are UTF-8, and no conversion from the code page is needed.

mod native;
#[cfg(unix)]
mod sys_unix;
#[cfg(windows)]
mod sys_windows;

use std::path::{Path, PathBuf};

pub use native::{Migemo, OpenError};

/// The file name the library has on this OS (`migemo.dll` in Sakura, `GetDllNameImp`).
pub const fn library_file_name() -> &'static str {
    if cfg!(windows) {
        "migemo.dll"
    } else if cfg!(target_os = "macos") {
        "libmigemo.dylib"
    } else {
        "libmigemo.so"
    }
}

/// The full path of the library to load, or `None` when there is no folder to look in.
///
/// * An absolute `configured` path is used as it is.
/// * A relative one is looked up in `ini_dir`, then in `exe_dir` (`GetInidirOrExedir`).
/// * An empty one, or one whose file is not there, stands for [`library_file_name`] in `exe_dir`,
///   then in `ini_dir` (Sakura's default is the bare name, which the OS looks up in the folder of
///   the program first).
/// * If no file is there, the place where it was expected is given, so that the load fails with
///   the message of a missing library.
///
/// `exists` tells whether a file is there. The result is always absolute: a folder that is not
/// absolute is not looked in.
pub fn library_path(
    configured: &str,
    ini_dir: Option<&Path>,
    exe_dir: Option<&Path>,
    exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let configured = configured.trim();
    let given = Path::new(configured);
    let ini_dir = ini_dir.filter(|dir| dir.is_absolute());
    let exe_dir = exe_dir.filter(|dir| dir.is_absolute());
    if !configured.is_empty() {
        if given.is_absolute() {
            if exists(given) {
                return Some(given.to_owned());
            }
        } else if let Some(found) = [ini_dir, exe_dir]
            .into_iter()
            .flatten()
            .map(|dir| dir.join(given))
            .find(|path| exists(path))
        {
            return Some(found);
        }
    }
    // the default name: the folder of the program first, as the OS does
    let default = library_file_name();
    if let Some(found) = [exe_dir, ini_dir]
        .into_iter()
        .flatten()
        .map(|dir| dir.join(default))
        .find(|path| exists(path))
    {
        return Some(found);
    }
    // nothing is there
    if configured.is_empty() {
        return ini_dir.or(exe_dir).map(|dir| dir.join(default));
    }
    if given.is_absolute() {
        return Some(given.to_owned());
    }
    ini_dir.or(exe_dir).map(|dir| dir.join(given))
}

/// The folder that holds `migemo-dict` and the four conversion tables, or `None` when the folder
/// configured (`CMigemo::migemo_load_all`) has no UTF-8 dictionary.
///
/// An empty `configured` is `dict` under `ini_dir`, a relative one is under `ini_dir`, an
/// absolute one is as it is. `exists` tells whether a file is there.
pub fn dictionary_dir(
    configured: &str,
    ini_dir: Option<&Path>,
    exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let configured = configured.trim();
    let base = if configured.is_empty() {
        ini_dir?.join("dict")
    } else if Path::new(configured).is_absolute() {
        PathBuf::from(configured)
    } else {
        ini_dir?.join(configured)
    };
    let utf8 = base.join("utf-8");
    exists(&utf8.join("migemo-dict")).then_some(utf8)
}

/// What the regular expression engine takes for syntax: the characters that the library writes
/// with a backslash. This is the set of `pcre_int2char` in `CMigemo.cpp` (and of
/// `default_int2char` of C/Migemo's `rxgen.c`).
const ESCAPED: &[u8] = b"\\.*^$/[]|()+?{}:-&";

/// `migemo_setproc_char2int`: the code point at the start of `bytes` and its length in bytes. A
/// byte that does not start a well-formed sequence stands for itself with the length 1.
pub fn decode_char(bytes: &[u8]) -> Option<(u32, usize)> {
    let lead = *bytes.first()?;
    let (len, bits) = match lead {
        0x00..=0x7F => return Some((u32::from(lead), 1)),
        0xC0..=0xDF => (2, u32::from(lead & 0x1F)),
        0xE0..=0xEF => (3, u32::from(lead & 0x0F)),
        0xF0..=0xF7 => (4, u32::from(lead & 0x07)),
        _ => return Some((u32::from(lead), 1)),
    };
    let mut code = bits;
    for i in 1..len {
        match bytes.get(i) {
            Some(b) if b & 0xC0 == 0x80 => code = (code << 6) | u32::from(b & 0x3F),
            _ => return Some((u32::from(lead), 1)),
        }
    }
    Some((code, len))
}

/// `migemo_setproc_int2char`: writes the UTF-8 of `code` into `out` and returns its length; a
/// character that is syntax in a regular expression gets a backslash first.
pub fn encode_char(code: u32, out: &mut [u8; 8]) -> usize {
    if code < 0x80 {
        // the range was checked: it is a byte
        let byte = (code & 0x7F) as u8;
        if ESCAPED.contains(&byte) {
            out[0] = b'\\';
            out[1] = byte;
            return 2;
        }
        out[0] = byte;
        return 1;
    }
    // Written by hand so that a code point that `char` refuses (a surrogate) is still written,
    // as the library gave it.
    if code < 0x800 {
        out[0] = 0xC0 | ((code >> 6) & 0x1F) as u8;
        out[1] = 0x80 | (code & 0x3F) as u8;
        2
    } else if code < 0x1_0000 {
        out[0] = 0xE0 | ((code >> 12) & 0x0F) as u8;
        out[1] = 0x80 | ((code >> 6) & 0x3F) as u8;
        out[2] = 0x80 | (code & 0x3F) as u8;
        3
    } else {
        out[0] = 0xF0 | ((code >> 18) & 0x07) as u8;
        out[1] = 0x80 | ((code >> 12) & 0x3F) as u8;
        out[2] = 0x80 | ((code >> 6) & 0x3F) as u8;
        out[3] = 0x80 | (code & 0x3F) as u8;
        4
    }
}

#[cfg(test)]
mod tests;
