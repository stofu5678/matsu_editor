// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/env/CProfile.cpp, sakura_core/env/CProfile.h
//   sakura_core/io/CTextStream.cpp (reading and writing of lines and the encoding)
//   src/test/cpp/tests1/test-cprofile.cpp (test cases)
//
// Original copyright notices of those files:
//   Copyright (C) 2003-2006, D.S.Koba
//   Copyright (C) 2004, MIK, genta
//   Copyright (C) 2006, ryoji
//   Copyright (C) 2008, kobake
//   Copyright (C) 2009, ryoji
//   Copyright (C) 2018-2026, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// zlib License
//
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! The profile (INI-like) file format of Sakura Editor.
//!
//! Sakura reads `sakura.ini` into `std::map`s and writes it back sorted and without comments. MATSU Editor must
//! not lose anything the user wrote (ADR-0010), so [`Profile`] keeps every line of the file, in order, and
//! writes the original bytes of every line that was not changed. How Sakura itself would write the same data is
//! available through [`Profile::to_sakura_bytes`].
//!
//! # Format rules (all from `sakura_core/env/CProfile.cpp`, `CProfile::_ReadOneline`)
//!
//! - Lines end with CRLF, LF or CR (`sakura_core/io/CTextStream.cpp`, `CTextInputStream::ReadLineW`).
//! - An empty line is skipped. A line that starts with `;` or `//` is a comment.
//! - `[name]` starts a section when the name is not empty and has no `=` in it.
//! - Lines before the first section are ignored.
//! - `key=value` is an entry: the key is everything before the first `=` (it must not be empty), the value is
//!   everything after it. Nothing is trimmed.
//! - A line that is none of these is ignored.
//! - When a key is repeated in a section, the first one wins. When a section name is repeated, lookups use the
//!   first section with that name.
//! - A file with a UTF-8 BOM is UTF-8. A file without one is Shift_JIS (CP932)
//!   (`CTextInputStream::CTextInputStream`, `ReadLineW`).
//! - Sakura writes UTF-8 with a BOM and CRLF line ends (`CTextOutputStream`, `CProfile::_WriteFile`).
//!   MATSU Editor writes the line ends of the OS instead where Sakura says CRLF (CRLF on Windows, LF on
//!   macOS and Linux; the user decided so on 2026-10-15, ADR-0010): in a new file, in the Sakura
//!   format, and for lines added to a file that has no line end yet. A file that exists keeps the line
//!   ends it has.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;
use std::io;
use std::path::Path;

/// The first comment line of MATSU Editor's `matsu.ini`. Sakura writes the same with `sakura.ini`
/// (`CShareData_IO::ShareData_IO_2`); the format is unchanged. A file that is read and written back with
/// [`Profile::save`] keeps whatever comment it has.
pub const MATSU_HEADER_COMMENT: &str = " matsu.ini テキストエディタ設定ファイル";

const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// Text encoding of a profile file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileEncoding {
    /// UTF-8. Sakura writes it with a BOM.
    Utf8,
    /// Shift_JIS (CP932). Sakura reads a file without a BOM as this.
    ShiftJis,
}

/// How a line ends in the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    /// `\r\n`
    CrLf,
    /// `\n`
    Lf,
    /// `\r` (Sakura reads it as a line end, and its own test covers it).
    Cr,
    /// The last line of a file that does not end with a line end.
    None,
}

impl LineEnding {
    /// The line end of new files: CRLF on Windows, LF on macOS and Linux. (`matsu-core` has the same
    /// rule for documents, `Eol::platform_default`; this crate does not depend on it.)
    pub fn platform_default() -> LineEnding {
        if cfg!(windows) {
            LineEnding::CrLf
        } else {
            LineEnding::Lf
        }
    }

    fn bytes(self) -> &'static [u8] {
        match self {
            LineEnding::CrLf => b"\r\n",
            LineEnding::Lf => b"\n",
            LineEnding::Cr => b"\r",
            LineEnding::None => b"",
        }
    }
}

/// An error from changing a profile: the value could not be read back as the same entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    /// The section name is empty, or has `=`, a line break, or a name that is not written as `[name]`.
    InvalidSection(String),
    /// The key is empty, has `=` or a line break, or starts with `;` or `//` (it would read as a comment).
    InvalidKey(String),
    /// The value has a line break.
    InvalidValue(String),
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProfileError::InvalidSection(s) => write!(f, "invalid section name: {s:?}"),
            ProfileError::InvalidKey(s) => write!(f, "invalid key: {s:?}"),
            ProfileError::InvalidValue(s) => write!(f, "invalid value: {s:?}"),
        }
    }
}

impl std::error::Error for ProfileError {}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LineKind {
    /// An empty line.
    Blank,
    /// A line that starts with `;` or `//`.
    Comment(String),
    Section(String),
    Entry {
        key: String,
        value: String,
    },
    /// A line Sakura skips (before the first section, or not an entry). Kept as it is.
    Ignored(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Line {
    kind: LineKind,
    eol: LineEnding,
    /// The bytes of the line as read (without the line end). `None` for a line that was added or changed.
    raw: Option<Vec<u8>>,
}

impl Line {
    fn new(kind: LineKind, eol: LineEnding) -> Self {
        Line {
            kind,
            eol,
            raw: None,
        }
    }

    fn text(&self) -> Cow<'_, str> {
        match &self.kind {
            LineKind::Blank => Cow::Borrowed(""),
            LineKind::Comment(t) | LineKind::Ignored(t) => Cow::Borrowed(t),
            LineKind::Section(name) => Cow::Owned(format!("[{name}]")),
            LineKind::Entry { key, value } => Cow::Owned(format!("{key}={value}")),
        }
    }
}

/// A profile file, kept line by line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    encoding: FileEncoding,
    bom: bool,
    lines: Vec<Line>,
}

impl Default for Profile {
    fn default() -> Self {
        Self::new()
    }
}

/// Classifies one line the way `CProfile::_ReadOneline` does. `in_section` is whether a section header has
/// been seen before this line.
fn classify(text: &str, in_section: bool) -> LineKind {
    // Skip empty lines (CProfile.cpp, `_ReadOneline`).
    if text.is_empty() {
        return LineKind::Blank;
    }
    // Comment lines.
    if text.starts_with(';') || text.starts_with("//") {
        return LineKind::Comment(text.to_owned());
    }
    // Section: `2 < line.length()` means the name is not empty.
    if text.len() > 2
        && text.starts_with('[')
        && text.ends_with(']')
        && !text[1..text.len() - 1].contains('=')
    {
        return LineKind::Section(text[1..text.len() - 1].to_owned());
    }
    // Entries before the first section are ignored.
    if !in_section {
        return LineKind::Ignored(text.to_owned());
    }
    // Entry: `=` must not be the first character.
    match text.find('=') {
        Some(pos) if pos > 0 => LineKind::Entry {
            key: text[..pos].to_owned(),
            value: text[pos + 1..].to_owned(),
        },
        _ => LineKind::Ignored(text.to_owned()),
    }
}

/// Splits bytes into lines at CRLF, LF and CR (`CTextInputStream::ReadLineW`).
fn split_lines(bytes: &[u8]) -> Vec<(&[u8], LineEnding)> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\r' if bytes.get(i + 1) == Some(&b'\n') => {
                lines.push((&bytes[start..i], LineEnding::CrLf));
                i += 2;
                start = i;
            }
            b'\r' => {
                lines.push((&bytes[start..i], LineEnding::Cr));
                i += 1;
                start = i;
            }
            b'\n' => {
                lines.push((&bytes[start..i], LineEnding::Lf));
                i += 1;
                start = i;
            }
            _ => i += 1,
        }
    }
    if start < bytes.len() {
        lines.push((&bytes[start..], LineEnding::None));
    }
    lines
}

fn decode(bytes: &[u8], encoding: FileEncoding) -> String {
    match encoding {
        FileEncoding::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
        FileEncoding::ShiftJis => encoding_rs::SHIFT_JIS
            .decode_without_bom_handling(bytes)
            .0
            .into_owned(),
    }
}

/// Encodes text into Shift_JIS. `None` if some character cannot be written back as the same character.
fn encode_sjis(text: &str) -> Option<Cow<'_, [u8]>> {
    let (bytes, _, had_errors) = encoding_rs::SHIFT_JIS.encode(text);
    // The WHATWG encoder also maps some characters to bytes of another character without an error
    // (U+00A5 and U+203E to `\` and `~`, U+2212 to the byte of U+FF0D), so check that it reads back.
    (!had_errors && decode(&bytes, FileEncoding::ShiftJis) == text).then_some(bytes)
}

fn utf16_cmp(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

fn validate(section: &str, key: &str, value: &str) -> Result<(), ProfileError> {
    let has_break = |s: &str| s.contains(['\r', '\n']);
    if section.is_empty() || section.contains('=') || has_break(section) {
        return Err(ProfileError::InvalidSection(section.to_owned()));
    }
    if key.is_empty()
        || key.contains('=')
        || has_break(key)
        || key.starts_with(';')
        || key.starts_with("//")
    {
        return Err(ProfileError::InvalidKey(key.to_owned()));
    }
    if has_break(value) {
        return Err(ProfileError::InvalidValue(value.to_owned()));
    }
    Ok(())
}

impl Profile {
    /// An empty profile that will be written the way Sakura writes it (UTF-8 with a BOM, and the line ends of
    /// the OS: [`LineEnding::platform_default`]).
    pub fn new() -> Self {
        Profile {
            encoding: FileEncoding::Utf8,
            bom: true,
            lines: Vec::new(),
        }
    }

    /// Reads a profile from the bytes of a file. Every byte sequence is accepted; nothing is lost.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let (encoding, bom, body) = match bytes.strip_prefix(&UTF8_BOM) {
            Some(rest) => (FileEncoding::Utf8, true, rest),
            None => (FileEncoding::ShiftJis, false, bytes),
        };
        let mut in_section = false;
        let lines = split_lines(body)
            .into_iter()
            .map(|(raw, eol)| {
                let kind = classify(&decode(raw, encoding), in_section);
                in_section |= matches!(kind, LineKind::Section(_));
                Line {
                    kind,
                    eol,
                    raw: Some(raw.to_vec()),
                }
            })
            .collect();
        Profile {
            encoding,
            bom,
            lines,
        }
    }

    /// Reads a profile file.
    pub fn load(path: &Path) -> io::Result<Self> {
        Ok(Self::from_bytes(&std::fs::read(path)?))
    }

    /// The encoding the file had when it was read.
    pub fn encoding(&self) -> FileEncoding {
        self.encoding
    }

    /// Whether the file had a UTF-8 BOM.
    pub fn has_bom(&self) -> bool {
        self.bom
    }

    /// The line end used for added lines: the first line end in the file, or the one of the OS.
    fn default_eol(&self) -> LineEnding {
        self.lines
            .iter()
            .map(|l| l.eol)
            .find(|e| *e != LineEnding::None)
            .unwrap_or_else(LineEnding::platform_default)
    }

    /// Index of the first section header with this name.
    fn find_section(&self, name: &str) -> Option<usize> {
        self.lines
            .iter()
            .position(|l| matches!(&l.kind, LineKind::Section(n) if n == name))
    }

    /// The lines after a header up to the next header.
    fn section_range(&self, header: usize) -> std::ops::Range<usize> {
        let end = self.lines[header + 1..]
            .iter()
            .position(|l| matches!(l.kind, LineKind::Section(_)))
            .map_or(self.lines.len(), |p| header + 1 + p);
        header + 1..end
    }

    /// Names of all sections, in file order (a repeated name appears again).
    pub fn section_names(&self) -> impl Iterator<Item = &str> {
        self.lines.iter().filter_map(|l| match &l.kind {
            LineKind::Section(n) => Some(n.as_str()),
            _ => None,
        })
    }

    /// The entries of the first section with this name, in file order, including repeated keys.
    pub fn entries(&self, section: &str) -> impl Iterator<Item = (&str, &str)> {
        let range = self
            .find_section(section)
            .map_or(0..0, |h| self.section_range(h));
        self.lines[range].iter().filter_map(|l| match &l.kind {
            LineKind::Entry { key, value } => Some((key.as_str(), value.as_str())),
            _ => None,
        })
    }

    /// The value of a key in the first section with this name. If the key is repeated, the first is used
    /// (`CProfile::GetProfileData`, `try_emplace` in `_ReadOneline`).
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.entries(section)
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
    }

    /// Sets a value, as `CProfile::SetProfileData` does: the section is added if it is missing, and the entry is
    /// replaced or added. An existing entry keeps its place and an added one goes after the last entry of its
    /// section. Setting the value an entry already has leaves its bytes untouched.
    pub fn set(&mut self, section: &str, key: &str, value: &str) -> Result<(), ProfileError> {
        validate(section, key, value)?;
        let eol = self.default_eol();
        let Some(header) = self.find_section(section) else {
            // Add the section at the end, after a blank line.
            if let Some(last) = self.lines.last_mut() {
                if last.eol == LineEnding::None {
                    last.eol = eol;
                }
                if last.kind != LineKind::Blank {
                    self.lines.push(Line::new(LineKind::Blank, eol));
                }
            }
            self.lines
                .push(Line::new(LineKind::Section(section.to_owned()), eol));
            self.lines.push(Line::new(
                LineKind::Entry {
                    key: key.to_owned(),
                    value: value.to_owned(),
                },
                eol,
            ));
            return Ok(());
        };
        let range = self.section_range(header);
        let mut last_entry = None;
        for i in range {
            let line = &mut self.lines[i];
            if let LineKind::Entry { key: k, value: v } = &mut line.kind {
                if k == key {
                    if v != value {
                        *v = value.to_owned();
                        line.raw = None;
                    }
                    return Ok(());
                }
                last_entry = Some(i);
            }
        }
        let at = last_entry.unwrap_or(header) + 1;
        if let Some(prev) = self.lines.get_mut(at - 1)
            && prev.eol == LineEnding::None
        {
            prev.eol = eol;
        }
        self.lines.insert(
            at,
            Line::new(
                LineKind::Entry {
                    key: key.to_owned(),
                    value: value.to_owned(),
                },
                eol,
            ),
        );
        Ok(())
    }

    /// Removes every entry with this key from the first section with this name. Returns whether there was one.
    /// The other lines stay as they were (a rewritten list drops the entries it no longer has this way).
    pub fn remove(&mut self, section: &str, key: &str) -> bool {
        let Some(header) = self.find_section(section) else {
            return false;
        };
        let range = self.section_range(header);
        let before = self.lines.len();
        let mut at = range.start;
        let mut end = range.end;
        while at < end {
            if matches!(&self.lines[at].kind, LineKind::Entry { key: k, .. } if k == key) {
                self.lines.remove(at);
                end -= 1;
            } else {
                at += 1;
            }
        }
        self.lines.len() != before
    }

    /// Removes the first section with this name, with its header and entries. Returns whether there was one.
    pub fn remove_section(&mut self, section: &str) -> bool {
        let Some(header) = self.find_section(section) else {
            return false;
        };
        let range = self.section_range(header);
        self.lines.drain(header..range.end);
        true
    }

    /// The bytes of the file with every unchanged line exactly as it was read.
    ///
    /// A file read as Shift_JIS is written as Shift_JIS. If a changed line has a character Shift_JIS cannot
    /// hold, the whole file is written as UTF-8 with a BOM, which is what Sakura always writes. In that case
    /// bytes of unchanged lines that were not valid Shift_JIS are replaced by U+FFFD.
    pub fn to_bytes(&self) -> Vec<u8> {
        if let Some(bytes) = self.emit(self.encoding, self.bom) {
            return bytes;
        }
        self.emit(FileEncoding::Utf8, true)
            .expect("UTF-8 can always be written")
    }

    fn emit(&self, encoding: FileEncoding, bom: bool) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        if bom {
            out.extend_from_slice(&UTF8_BOM);
        }
        for line in &self.lines {
            match (&line.raw, encoding == self.encoding) {
                (Some(raw), true) => out.extend_from_slice(raw),
                _ => {
                    let text = line.text();
                    match encoding {
                        FileEncoding::Utf8 => out.extend_from_slice(text.as_bytes()),
                        FileEncoding::ShiftJis => out.extend_from_slice(&encode_sjis(&text)?),
                    }
                }
            }
            out.extend_from_slice(line.eol.bytes());
        }
        Some(out)
    }

    /// The bytes Sakura would write for this data (`CProfile::WriteProfile`, `_WriteFile`): UTF-8 with a BOM,
    /// the line ends of the OS (Sakura: CRLF), an optional first comment line and a blank line, then every section as `[name]`, its entries
    /// sorted by key and a blank line. Comments, blank lines, ignored lines and repeated keys are dropped.
    ///
    /// Sakura passes `" sakura.ini テキストエディタ設定ファイル"` as the comment (`CShareData_IO::ShareData_IO_2`);
    /// MATSU Editor passes [`MATSU_HEADER_COMMENT`]. Only the name differs; the format is the same.
    pub fn to_sakura_bytes(&self, comment: Option<&str>) -> Vec<u8> {
        let eol = std::str::from_utf8(LineEnding::platform_default().bytes()).expect("ASCII");
        let mut text = String::new();
        if let Some(comment) = comment {
            text.push(';');
            text.push_str(comment);
            text.push_str(eol);
            text.push_str(eol);
        }
        let mut i = 0;
        while i < self.lines.len() {
            if let LineKind::Section(name) = &self.lines[i].kind {
                text.push_str(&format!("[{name}]{eol}"));
                // `std::map` keeps the first of a repeated key and orders keys by UTF-16 code units.
                let mut entries: Vec<(&str, &str)> = Vec::new();
                for (k, v) in self.entries_at(i) {
                    if !entries.iter().any(|(ek, _)| *ek == k) {
                        entries.push((k, v));
                    }
                }
                entries.sort_by(|a, b| utf16_cmp(a.0, b.0));
                for (k, v) in entries {
                    text.push_str(&format!("{k}={v}{eol}"));
                }
                text.push_str(eol);
            }
            i += 1;
        }
        let mut out = UTF8_BOM.to_vec();
        out.extend_from_slice(text.as_bytes());
        out
    }

    fn entries_at(&self, header: usize) -> impl Iterator<Item = (&str, &str)> {
        self.lines[self.section_range(header)]
            .iter()
            .filter_map(|l| match &l.kind {
                LineKind::Entry { key, value } => Some((key.as_str(), value.as_str())),
                _ => None,
            })
    }

    /// Writes the file like [`Profile::to_bytes`]. Like Sakura, it writes a temporary file next to the target
    /// and replaces the target with it, so a crash does not leave a half-written file.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        write_replacing(path, &self.to_bytes())
    }

    /// Writes the file the way Sakura does ([`Profile::to_sakura_bytes`]).
    pub fn save_sakura_style(&self, path: &Path, comment: Option<&str>) -> io::Result<()> {
        write_replacing(path, &self.to_sakura_bytes(comment))
    }
}

/// `CProfile::WriteProfile`: create the folder, write to another file, then replace.
///
/// The other file has a name of its own for each call (`GetTempFilePath` does the same in Sakura), so two saves
/// at the same time do not write into the same file.
fn write_replacing(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);

    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut attempt = 0;
    let (tmp, mut file) = loop {
        let mut name = path.as_os_str().to_owned();
        name.push(format!(
            ".{}-{}.matsu-tmp",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let tmp = std::path::PathBuf::from(name);
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        // Private from the first byte: the file may be made more open below, to match the file it replaces.
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        match options.open(&tmp) {
            Ok(file) => break (tmp, file),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists && attempt < 100 => attempt += 1,
            Err(e) => return Err(e),
        }
    };
    let result = file
        .write_all(bytes)
        .and_then(|()| crate::sync::sync_file(&file))
        .and_then(|()| {
            // Keep the permissions of the file that is replaced (a user may have made it private).
            match std::fs::metadata(path) {
                Ok(meta) => file.set_permissions(meta.permissions()),
                Err(_) => Ok(()),
            }
        })
        .and_then(|()| {
            drop(file);
            std::fs::rename(&tmp, path)
        });
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A profile from UTF-8 text with a BOM (without one, the bytes are read as Shift_JIS).
    /// Text written with CRLF (as Sakura writes it) in the line ends of this OS.
    fn os(s: &str) -> String {
        s.replace("\r\n", if cfg!(windows) { "\r\n" } else { "\n" })
    }

    fn parse(s: &str) -> Profile {
        let mut bytes = UTF8_BOM.to_vec();
        bytes.extend_from_slice(s.as_bytes());
        Profile::from_bytes(&bytes)
    }

    /// The bytes of a profile as text, without the BOM.
    fn text(p: &Profile) -> String {
        String::from_utf8(p.to_bytes())
            .unwrap()
            .trim_start_matches('\u{feff}')
            .to_owned()
    }

    // sakura_core/env/CProfile.cpp `_ReadOneline`
    #[test]
    fn reads_sections_and_entries() {
        let p = parse("; c\r\n// c\r\n[A]\r\nk=v\r\nk2=a=b\r\n=x\r\n\r\n[B]\r\nk=w\r\n");
        assert_eq!(p.section_names().collect::<Vec<_>>(), ["A", "B"]);
        assert_eq!(p.get("A", "k"), Some("v"));
        assert_eq!(p.get("A", "k2"), Some("a=b"));
        assert_eq!(p.get("B", "k"), Some("w"));
        assert_eq!(p.get("A", ""), None);
        assert_eq!(p.get("C", "k"), None);
    }

    #[test]
    fn entries_before_first_section_are_ignored() {
        let p = parse("k=v\r\n[A]\r\nk2=v2\r\n");
        assert_eq!(p.get("A", "k"), None);
        assert_eq!(p.entries("A").count(), 1);
    }

    #[test]
    fn empty_section_name_and_section_with_equals_are_not_sections() {
        // `2 < line.length()` and `npos == sectionName.find(L'=')`.
        let p = parse("[]\r\n[A]\r\n[x=y]\r\nk=v\r\n");
        assert_eq!(p.section_names().collect::<Vec<_>>(), ["A"]);
        // "[x=y]" is an entry whose key is "[x".
        assert_eq!(p.get("A", "[x"), Some("y]"));
    }

    #[test]
    fn nothing_is_trimmed() {
        let p = parse("[A]\r\n k = v \r\n [B]\r\n");
        assert_eq!(p.get("A", " k "), Some(" v "));
        assert_eq!(p.section_names().collect::<Vec<_>>(), ["A"]);
    }

    #[test]
    fn first_of_repeated_keys_and_sections_wins() {
        let p = parse("[A]\r\nk=1\r\nk=2\r\n[B]\r\n[A]\r\nk=3\r\nz=4\r\n");
        assert_eq!(p.get("A", "k"), Some("1"));
        assert_eq!(p.get("A", "z"), None);
    }

    #[test]
    fn bom_decides_the_encoding() {
        let mut bytes = UTF8_BOM.to_vec();
        bytes.extend_from_slice("[あ]\nk=い\n".as_bytes());
        let p = Profile::from_bytes(&bytes);
        assert_eq!(p.encoding(), FileEncoding::Utf8);
        assert!(p.has_bom());
        assert_eq!(p.get("あ", "k"), Some("い"));

        let (sjis, _, _) = encoding_rs::SHIFT_JIS.encode("[あ]\nk=い\n");
        let p = Profile::from_bytes(&sjis);
        assert_eq!(p.encoding(), FileEncoding::ShiftJis);
        assert!(!p.has_bom());
        assert_eq!(p.get("あ", "k"), Some("い"));
    }

    // src/test/cpp/tests1/test-cprofile.cpp, ReadProfile_LineTerminatorCr
    #[test]
    fn cr_is_a_line_end() {
        let p = parse("; test\r[test]\rtest=1\r");
        assert_eq!(p.get("test", "test"), Some("1"));
    }

    // test-cprofile.cpp, ReadProfile_ExpandLineBuffer: a line longer than 4096 characters.
    #[test]
    fn long_lines_are_read() {
        let long = format!("too_long_item={}", "x".repeat(4083));
        let p = parse(&format!("; test\r[test]\rtest=1\r{long}\r"));
        assert_eq!(p.get("test", "test"), Some("1"));
        assert_eq!(p.get("test", "too_long_item").map(str::len), Some(4083));
    }

    #[test]
    fn every_byte_sequence_round_trips() {
        let cases: [&[u8]; 8] = [
            b"",
            b"\r\n",
            b"[A]",
            b"[A]\rk=v\n\r\n\n",
            b"\xEF\xBB\xBF",
            b"\xEF\xBB\xBF[A]\r\nk=\xFF\xFE\r\n",
            b"[A]\r\nk=\x83\x41\r\n; \x82\xA0",
            b"junk\r\n  \r\n[A]\r\n =\r\n",
        ];
        for bytes in cases {
            assert_eq!(Profile::from_bytes(bytes).to_bytes(), bytes, "{bytes:?}");
        }
    }

    #[test]
    fn set_replaces_in_place_and_keeps_other_lines() {
        let src = "; top\r\n[A]\r\na=1\r\n; note\r\nb=2\r\n\r\n[B]\r\nc=3\r\n";
        let mut p = parse(src);
        p.set("A", "a", "9").unwrap();
        assert_eq!(
            text(&p),
            "; top\r\n[A]\r\na=9\r\n; note\r\nb=2\r\n\r\n[B]\r\nc=3\r\n"
        );
    }

    #[test]
    fn set_adds_after_the_last_entry_of_the_section() {
        let mut p = parse("[A]\na=1\n; note\n\n[B]\nc=3\n");
        p.set("A", "z", "26").unwrap();
        assert_eq!(text(&p), "[A]\na=1\nz=26\n; note\n\n[B]\nc=3\n");
    }

    #[test]
    fn remove_drops_the_entry_and_keeps_the_rest() {
        let mut p = Profile::from_bytes(b"[A]\na=1\nb=2\na=3\n[B]\na=9\n");
        assert!(p.remove("A", "a"));
        assert!(!p.remove("A", "a"));
        assert!(!p.remove("C", "a"));
        assert_eq!(p.to_bytes(), b"[A]\nb=2\n[B]\na=9\n");
    }

    #[test]
    fn set_adds_a_section_at_the_end() {
        let mut p = parse("[A]\r\na=1");
        p.set("New", "k", "v").unwrap();
        assert_eq!(text(&p), "[A]\r\na=1\r\n\r\n[New]\r\nk=v\r\n");
        let mut p = Profile::new();
        p.set("S", "k", "あ").unwrap();
        let mut want = UTF8_BOM.to_vec();
        want.extend_from_slice(os("[S]\r\nk=あ\r\n").as_bytes());
        assert_eq!(p.to_bytes(), want);
        // A file that exists keeps its own line ends, whatever the OS.
        let mut p = parse("[A]\r\na=1\r\n");
        p.set("A", "b", "2").unwrap();
        assert_eq!(text(&p), "[A]\r\na=1\r\nb=2\r\n");
        let mut p = parse("[A]\na=1\n");
        p.set("A", "b", "2").unwrap();
        assert_eq!(text(&p), "[A]\na=1\nb=2\n");
    }

    #[test]
    fn setting_the_same_value_keeps_the_bytes() {
        let src = b"[A]\r\n k=1\r\nk=1\r\n";
        let mut p = Profile::from_bytes(src);
        p.set("A", "k", "1").unwrap();
        assert_eq!(p.to_bytes(), src);
    }

    #[test]
    fn set_rejects_what_cannot_be_read_back() {
        let mut p = Profile::new();
        assert!(p.set("", "k", "v").is_err());
        assert!(p.set("a=b", "k", "v").is_err());
        assert!(p.set("A", "", "v").is_err());
        assert!(p.set("A", "k=1", "v").is_err());
        assert!(p.set("A", ";k", "v").is_err());
        assert!(p.set("A", "//k", "v").is_err());
        assert!(p.set("A", "k", "a\nb").is_err());
        assert!(p.set("A", "k", "a=b").is_ok());
    }

    #[test]
    fn shift_jis_file_stays_shift_jis() {
        let (src, _, _) = encoding_rs::SHIFT_JIS.encode("[基本]\r\nszTypeName=基本\r\n");
        let mut p = Profile::from_bytes(&src);
        p.set("基本", "x", "あ").unwrap();
        let (want, _, _) = encoding_rs::SHIFT_JIS.encode("[基本]\r\nszTypeName=基本\r\nx=あ\r\n");
        assert_eq!(p.to_bytes(), want.as_ref());
    }

    #[test]
    fn characters_that_do_not_read_back_move_the_file_to_utf8() {
        // U+2212 encodes to the Shift_JIS byte of U+FF0D without an error; U+00A5 becomes `\`.
        for text in ["\u{2212}", "\u{a5}", "\u{203e}"] {
            let (src, _, _) = encoding_rs::SHIFT_JIS.encode("[A]\r\nk=あ\r\n");
            let mut p = Profile::from_bytes(&src);
            p.set("A", "e", text).unwrap();
            let out = p.to_bytes();
            assert!(out.starts_with(&UTF8_BOM), "{text:?}");
            assert_eq!(Profile::from_bytes(&out).get("A", "e"), Some(text));
        }
    }

    #[test]
    fn shift_jis_file_moves_to_utf8_when_a_character_does_not_fit() {
        let (src, _, _) = encoding_rs::SHIFT_JIS.encode("[A]\r\nk=あ\r\n");
        let mut p = Profile::from_bytes(&src);
        p.set("A", "e", "😀").unwrap();
        let mut want = UTF8_BOM.to_vec();
        want.extend_from_slice("[A]\r\nk=あ\r\ne=😀\r\n".as_bytes());
        assert_eq!(p.to_bytes(), want);
    }

    // CProfile::WriteProfile + _WriteFile: BOM, CRLF, comment and blank line, sorted keys, blank after sections.
    #[test]
    fn writes_the_way_sakura_does() {
        let p = parse("; gone\n[B]\nz=1\nA=2\nz=3\n\n[A]\nk=あ\n");
        let out = p.to_sakura_bytes(Some(" sakura.ini テキストエディタ設定ファイル"));
        let want = "\u{feff}; sakura.ini テキストエディタ設定ファイル\r\n\r\n[B]\r\nA=2\r\nz=1\r\n\r\n[A]\r\nk=あ\r\n\r\n";
        assert_eq!(String::from_utf8(out).unwrap(), os(want));
        assert_eq!(
            parse("[A]\nk=1\n").to_sakura_bytes(None),
            os("\u{feff}[A]\r\nk=1\r\n\r\n").into_bytes()
        );
    }

    #[test]
    fn matsu_header_comment_differs_from_sakuras_only_in_the_name() {
        let out = parse("[A]\nk=1\n").to_sakura_bytes(Some(MATSU_HEADER_COMMENT));
        let want = os("\u{feff}; matsu.ini テキストエディタ設定ファイル\r\n\r\n[A]");
        assert!(out.starts_with(want.as_bytes()));
    }

    #[test]
    fn sakura_style_orders_keys_by_utf16_units() {
        // U+FF21 (one unit 0xFF21) sorts after U+1F600 (units 0xD83D 0xDE00) by UTF-16, before it by code point.
        let p = parse("[A]\nＡ=1\n😀=2\nb=3\n");
        let text = String::from_utf8(p.to_sakura_bytes(None)).unwrap();
        assert_eq!(text, os("\u{feff}[A]\r\nb=3\r\n😀=2\r\nＡ=1\r\n\r\n"));
    }

    #[test]
    fn sakura_style_output_reads_back_the_same() {
        let p = parse("[A]\nk=あ\nj=い\n[B]\nz=\n");
        let q = Profile::from_bytes(&p.to_sakura_bytes(Some(" x")));
        for s in ["A", "B"] {
            let mut a: Vec<_> = p.entries(s).collect();
            a.sort();
            let mut b: Vec<_> = q.entries(s).collect();
            b.sort();
            assert_eq!(a, b);
        }
    }

    #[test]
    fn save_creates_folders_and_replaces_the_file() {
        let dir = std::env::temp_dir().join(format!("matsu-config-test-{}", std::process::id()));
        let path = dir.join("sub").join("sakura.ini");
        let mut p = Profile::new();
        p.set("A", "k", "1").unwrap();
        p.save(&path).unwrap();
        p.set("A", "k", "2").unwrap();
        p.save(&path).unwrap();
        let back = Profile::load(&path).unwrap();
        assert_eq!(back.get("A", "k"), Some("2"));
        // No temporary file is left behind.
        let names: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["sakura.ini"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
