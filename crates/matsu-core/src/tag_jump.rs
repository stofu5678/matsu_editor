// Ported in part from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/cmd/CViewCommander_TagJump.cpp (Command_TagJumpNoMessage: the Grep headings)
//   sakura_core/util/file.cpp (IsFilePath)
//   sakura_core/util/string_ex2.cpp (GetLineColumn, GetLineColumnPos)
//
// sakura_core/cmd/CViewCommander_TagJump.cpp:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2001, genta
//   Copyright (C) 2001, YAZAKI
//   Copyright (C) 2002, MIK
//   Copyright (C) 2003, MIK, genta, じゅうじ
//   Copyright (C) 2004, Moca, novice
//   Copyright (C) 2005, MIK
//   Copyright (C) 2006, genta
//   Copyright (C) 2007, ryoji, maru, Uchi
//   Copyright (C) 2008, MIK
//   Copyright (C) 2010, Moca
//   Copyright (C) 2018-2022, Sakura Editor Organization
// sakura_core/util/file.cpp:
//   Copyright (C) 2002, SUI
//   Copyright (C) 2003, MIK
//   Copyright (C) 2008, kobake
//   Copyright (C) 2018-2026, Sakura Editor Organization
// sakura_core/util/string_ex2.cpp:
//   Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (UTF-16 buffers, the
// current folder of the process and the file system calls) was rewritten in Rust on `char`s, with
// the folder of the document and an `is_file` closure from the caller, and the tests are new.
//
// zlib License
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

//! Finding the target of a tag jump in the text around the caret (`F_TAGJUMP`).
//!
//! Sakura: `CViewCommander::Command_TagJumpNoMessage` (`sakura_core/cmd/CViewCommander_TagJump.cpp`),
//! `IsFilePath` (`sakura_core/util/file.cpp`), `GetLineColumn` (`sakura_core/util/string_ex2.cpp`)
//! and `_IS_REL_PATH`. The help is `help/sakura/res/HLP000065.html`.
//!
//! The text of the line of the caret is read in the formats of Sakura's Grep output, and then in
//! the free form of compiler messages:
//!
//! * `path(line,col): text`, `path(line,col)  [UTF-8]: text`, `path(line): text` (one line of
//!   Grep output of the "normal" form, with the full path);
//! * `・path(line,col): text`, `・sub\path(line,col)  [SJIS]: text` (the path is relative; the lines
//!   above it, `■"folder"`, `◆"file"` and `◎"base"`, tell where);
//! * `・(  line,col   ): text` (the "per file" form; `■"file"` above it);
//! * anything else: the first word of the line that is the name of an existing file, followed by
//!   `(line,col)`, `:line`, ` line N` and so on (Borland, gcc, rustc, ...). Only the first
//!   number is the line; the number after a comma is the column.
//!
//! Nothing here opens a window or reads a file except through the `is_file` closure that the
//! caller gives, so every rule can be tested with a list of names.
//!
//! A line of a Grep of the text of a window that has no file names it by `:HWND:[id]name`
//! (`IsHWNDTag`, `CGrepAgent.cpp` `GetHwndTitle`): the target then has no path but the `window`
//! (the id the Grep wrote), for the caller to find the window by.
//!
//! Differences from Sakura: Sakura's code units are UTF-16, here the text is read in `char`s (the column that
//! is returned is counted in UTF-16 code units, as in Sakura, for the caller to convert).

use std::path::{Path, PathBuf};

/// Sakura's `MAX_PATH`, the limit of a path in a tag (`MAX_TAG_PATH`).
const MAX_TAG_PATH: usize = 260;

/// Characters that end a file name in free text, where a longer name can still follow
/// (`IsFilePath`).
const NAME_ENDINGS: &str = " \t(\")'`[]{};#!@&%$";

/// Where a tag jump goes. The line and the column count from 1 and are `None` when the text does
/// not say (then the caret is not moved, `Command_TagJumpNoMessage`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagTarget {
    /// The file, made absolute with the folder of the document when it was relative. Empty for a
    /// window ([`window`](Self::window)).
    pub path: PathBuf,
    /// The window named by a `:HWND:[id]` tag instead of a file: the id the Grep wrote
    /// (`TagJumpSub` with a `:HWND:[` prefix).
    pub window: Option<u64>,
    pub line: Option<usize>,
    /// In UTF-16 code units from the start of the line (Sakura's logical column).
    pub column: Option<usize>,
}

/// Finds the target of a tag jump from line `caret_line` (0-based).
///
/// `line_at(i)` is the text of line `i` without its line break, `None` past the end. `base_dir` is
/// the folder that relative paths are relative to: Sakura changes its current folder to the
/// folder of the document when it opens it (`CDocEditor`), so this is the folder of the document.
/// `is_file(path)` tells whether a file (not a folder) exists.
pub fn find_target(
    line_at: &dyn Fn(usize) -> Option<String>,
    caret_line: usize,
    base_dir: Option<&Path>,
    is_file: &dyn Fn(&Path) -> bool,
) -> Option<TagTarget> {
    let current: Vec<char> = line_at(caret_line)?.chars().collect();
    let fs = Fs { base_dir, is_file };
    let found = grep_format(line_at, caret_line, &current, &fs)
        .or_else(|| hwnd_line(&current))
        .or_else(|| free_form(&current, &fs));
    let (path, line, column) = found?;
    let chars: Vec<char> = path.chars().collect();
    if let Some((id, _)) = hwnd_tag(&chars) {
        return Some(TagTarget {
            path: PathBuf::new(),
            window: Some(id),
            line: positive(line),
            column: positive(column),
        });
    }
    Some(TagTarget {
        path: fs.resolve(&path),
        window: None,
        line: positive(line),
        column: positive(column),
    })
}

/// `IsHWNDTag`: `:HWND:[` and the id in hexadecimal digits, then `]`. Returns the id and the
/// length of the tag.
fn hwnd_tag(c: &[char]) -> Option<(u64, usize)> {
    const PREFIX: &str = ":HWND:[";
    if !starts_with(c, PREFIX) {
        return None;
    }
    let start = PREFIX.chars().count();
    let digits = c[start..]
        .iter()
        .take_while(|ch| ch.is_ascii_hexdigit())
        .count();
    if digits == 0 || digits > 16 || c.get(start + digits) != Some(&']') {
        return None;
    }
    let text: String = c[start..start + digits].iter().collect();
    let id = u64::from_str_radix(&text, 16).ok()?;
    Some((id, start + digits + 1))
}

/// The tag at the start of `c` as text, for [`find_target`] to read again.
fn hwnd_tag_text(c: &[char]) -> Option<String> {
    hwnd_tag(c).map(|(_, len)| c[..len].iter().collect())
}

fn positive(n: i64) -> Option<usize> {
    usize::try_from(n).ok().filter(|n| *n > 0)
}

/// The file system as the rules see it.
struct Fs<'a> {
    base_dir: Option<&'a Path>,
    is_file: &'a dyn Fn(&Path) -> bool,
}

impl Fs<'_> {
    fn resolve(&self, path: &str) -> PathBuf {
        match self.base_dir {
            // `join` keeps an absolute `path` as it is.
            Some(base) if is_relative(path) => base.join(path),
            _ => PathBuf::from(path),
        }
    }

    fn exists(&self, path: &str) -> bool {
        (self.is_file)(&self.resolve(path))
    }

    /// `IsFileExists2`: an absolute path of valid characters that is a file.
    fn exists_absolute(&self, path: &str) -> bool {
        path.chars().all(is_valid_filename_char) && !is_relative(path) && self.exists(path)
    }
}

/// `_IS_REL_PATH` (a drive and a backslash, or two backslashes, are absolute), and also the
/// absolute paths of the OS this runs on.
fn is_relative(path: &str) -> bool {
    let c: Vec<char> = path.chars().take(3).collect();
    let drive = c.len() == 3 && c[0].is_ascii_alphabetic() && c[1] == ':' && c[2] == '\\';
    let unc = c.len() >= 2 && c[0] == '\\' && c[1] == '\\';
    !(drive || unc || Path::new(path).is_absolute())
}

/// `WCODE::IsValidFilenameChar`.
fn is_valid_filename_char(c: char) -> bool {
    !"<>?\"|*".contains(c)
}

/// `AddLastYenPath`: the folder with a separator at the end. Sakura always uses a backslash;
/// here it is a backslash when the folder or the `name` that follows it is written with
/// backslashes or a drive letter, and the separator of the OS otherwise.
fn with_separator(dir: &str, name: &str) -> String {
    if dir.ends_with(['\\', '/']) {
        return dir.to_owned();
    }
    let separator = if is_windows_style(dir) || is_windows_style(name) {
        '\\'
    } else {
        std::path::MAIN_SEPARATOR
    };
    format!("{dir}{separator}")
}

/// A path written with backslashes or a drive letter.
fn is_windows_style(path: &str) -> bool {
    path.contains('\\') || path.chars().nth(1) == Some(':')
}

fn starts_with(c: &[char], prefix: &str) -> bool {
    let mut chars = c.iter();
    prefix.chars().all(|p| chars.next() == Some(&p))
}

fn find_seq(c: &[char], pattern: &str) -> Option<usize> {
    let pattern: Vec<char> = pattern.chars().collect();
    c.windows(pattern.len()).position(|w| w == pattern)
}

/// The character at `i`, or NUL outside the line (Sakura reads the terminator there).
fn at(c: &[char], i: isize) -> char {
    usize::try_from(i)
        .ok()
        .and_then(|i| c.get(i))
        .copied()
        .unwrap_or('\0')
}

/// `GetQuoteFilePath`: the text up to the next `"`.
fn quoted(c: &[char]) -> Option<String> {
    let end = c.iter().position(|ch| *ch == '"')?;
    if end == 0 || end >= MAX_TAG_PATH {
        return None;
    }
    Some(c[..end].iter().collect())
}

/// `GetLineColumn`: the first number is the line; a number after a comma that follows it is the
/// column. 0 where there is none.
fn line_column(c: &[char]) -> (i64, i64) {
    fn number(c: &[char], mut i: usize) -> (i64, usize) {
        let mut value: i64 = 0;
        let mut digits = 0;
        // The buffer of Sakura holds 31 digits.
        while i < c.len() && c[i].is_ascii_digit() && digits < 31 {
            value = value
                .saturating_mul(10)
                .saturating_add(i64::from(c[i].to_digit(10).unwrap_or(0)));
            digits += 1;
            i += 1;
        }
        // `_wtoi` gives an int.
        (value.min(i64::from(i32::MAX)), i)
    }
    let Some(start) = c.iter().position(char::is_ascii_digit) else {
        return (0, 0);
    };
    let (line, mut i) = number(c, start);
    let mut column = 0;
    if i < c.len() && c[i] == ',' {
        i += 1;
        column = number(c, i).0;
    }
    (line, column)
}

/// `GetLineColumnPos`: the index of the `(` of `name(line,col): text` or
/// `name(line,col)  [enc]: text`, or 0.
fn line_column_pos(c: &[char]) -> usize {
    let mut tag_end = find_seq(c, "): ").map(|i| i as isize);
    if tag_end.is_none()
        && let Some(close) = find_seq(c, "]: ")
    {
        let mut end = close as isize - 1;
        while 1 < end {
            if at(c, end) == '[' {
                end -= 1;
                break;
            }
            end -= 1;
        }
        while 1 < end && at(c, end) == ' ' {
            end -= 1;
        }
        tag_end = (at(c, end) == ')').then_some(end);
    }
    if let Some(tag_end) = tag_end {
        let mut i = tag_end - 1;
        while 1 < i && at(c, i).is_ascii_digit() {
            i -= 1;
        }
        if 1 < i && at(c, i) == ',' {
            i -= 1;
        }
        while 1 < i && at(c, i).is_ascii_digit() {
            i -= 1;
        }
        if 1 < i && at(c, i) == '(' {
            return i as usize;
        }
    }
    0
}

/// `IsFilePath`: the longest start of `c` (after spaces, tabs and quotes, and `#include <` or
/// `#include "`) that is the name of an existing file. Returns where it starts and its length.
fn file_path_in(c: &[char], fs: &Fs) -> Option<(usize, usize)> {
    let len = c.len();
    let mut i = 0;
    while i < len && matches!(c[i], ' ' | '\t' | '"') {
        i += 1;
    }
    if i < len && c[i] == '#' {
        while i < len {
            if matches!(c[i], '<' | '"') {
                i += 1;
                break;
            }
            i += 1;
        }
    }
    if i >= len {
        return None;
    }
    let begin = i;
    let mut name = String::new();
    let mut name_chars = 0;
    let mut longest = 0;
    while i < len && name_chars + 1 < MAX_TAG_PATH {
        let ch = c[i];
        if is_line_delimiter(ch) || ch == '\0' {
            break;
        }
        if NAME_ENDINGS.contains(ch) && !name.is_empty() && fs.exists(&name) {
            longest = name_chars;
        }
        // The colon after a drive letter belongs to the name; any later one ends it (gcc).
        if name_chars > 1 && ch == ':' {
            break;
        }
        if !is_valid_filename_char(ch) {
            break;
        }
        name.push(ch);
        name_chars += 1;
        i += 1;
    }
    if !name.is_empty() && fs.exists(&name) {
        longest = name_chars;
    }
    (longest != 0).then_some((begin, longest))
}

/// `WCODE::IsLineDelimiterExt`.
fn is_line_delimiter(c: char) -> bool {
    matches!(c, '\r' | '\n' | '\u{85}' | '\u{2028}' | '\u{2029}')
}

/// Which of the lines above the caret line tells the folder (the search mode of Sakura's heading search).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Search {
    /// A full path is expected (the per-file form, `■"path"`).
    FilePath,
    /// A file name is known; a folder is expected (`■"folder"`).
    SubPath,
    /// A path relative to the base folder is known; `◎"base"` is expected.
    Root,
}

type Found = (String, i64, i64);

/// The forms of Grep output: the part of `Command_TagJumpNoMessage` before the free form.
fn grep_format(
    line_at: &dyn Fn(usize) -> Option<String>,
    caret_line: usize,
    line: &[char],
    fs: &Fs,
) -> Option<Found> {
    let mut file = String::new();
    let mut at_line = 0;
    let mut at_column = 0;
    let mut mode;
    if starts_with(line, "■\"") {
        // `■":HWND:[id]name"`: the heading of a window without a file (the only line of the
        // result of a search of the names).
        if let Some(tag) = hwnd_tag_text(&line[2..]) {
            return Some((tag, 0, 0));
        }
        // `■"C:\folder\file.ext"` (the per-file form): a full path, with the position that
        // follows it (there is none on this line in Sakura's output, but a hand-written one may).
        if let Some((begin, len)) = file_path_in(&line[2..], fs)
            && !is_relative(&line[2..].iter().collect::<String>())
        {
            let path: String = line[2 + begin..2 + begin + len].iter().collect();
            let (l, c) = line_column(&line[2 + len..]);
            return Some((path, l, c));
        }
        file = quoted(&line[2..])?;
        mode = Search::Root;
    } else if starts_with(line, "◆\"") {
        file = quoted(&line[2..])?;
        if let Some(tag) = hwnd_tag_text(&line[2..]) {
            return Some((tag, 0, 0));
        }
        mode = Search::SubPath;
    } else if starts_with(line, "・") {
        match line.get(1) {
            Some('"') => {
                file = quoted(&line[2..])?;
                if let Some(tag) = hwnd_tag_text(&line[2..]) {
                    return Some((tag, 0, 0));
                }
                mode = Search::SubPath;
            }
            Some('(') => {
                (at_line, at_column) = line_column(&line[1..]);
                mode = Search::FilePath;
            }
            _ => {
                let open = line_column_pos(line);
                if 1 < open && line[open] == '(' && open - 1 < MAX_TAG_PATH {
                    let (l, c) = line_column(&line[open + 1..]);
                    // `・:HWND:[id](無題)2(123,45): str`
                    if let Some(tag) = hwnd_tag_text(&line[1..]) {
                        return Some((tag, l, c));
                    }
                    file = line[1..open].iter().collect();
                    (at_line, at_column) = (l, c);
                    mode = Search::SubPath;
                } else {
                    return None;
                }
            }
        }
    } else {
        return None;
    }

    // Upwards: the folder or file lines of the heading (`・` → `◆` → `■` → `◎`).
    for y in (0..caret_line).rev() {
        let above: Vec<char> = line_at(y)?.chars().collect();
        if starts_with(&above, "・") {
            continue;
        } else if starts_with(&above, "◆\"") {
            if mode != Search::FilePath {
                continue;
            }
            // The file name of the folder-by-folder form.
            if let Some(tag) = hwnd_tag_text(&above[2..]) {
                return Some((tag, at_line, at_column));
            }
            file = quoted(&above[2..])?;
            mode = Search::SubPath;
        } else if above == ['■'] {
            // The root folder is the base folder of the lines below it.
            if mode == Search::Root {
                continue;
            }
            mode = Search::Root;
        } else if starts_with(&above, "■\"") {
            if mode == Search::Root {
                continue;
            }
            if let Some(tag) = hwnd_tag_text(&above[2..]) {
                return Some((tag, at_line, at_column));
            }
            if let Some((begin, len)) = file_path_in(&above[2..], fs)
                && !is_relative(&above[2..].iter().collect::<String>())
            {
                let path = above[2 + begin..2 + begin + len].iter().collect();
                return Some((path, at_line, at_column));
            }
            let mut path = quoted(&above[2..])?;
            if !file.is_empty() {
                path = with_separator(&path, &file);
            }
            path.push_str(&file);
            if path.chars().count() >= MAX_TAG_PATH {
                return None;
            }
            if fs.exists_absolute(&path) {
                return Some((path, at_line, at_column));
            }
            // A relative folder: the base folder (`◎`) is above.
            file = path;
            mode = Search::Root;
        } else if starts_with(&above, "◎\"") {
            let root = quoted(&above[2..])?;
            // The relative part was joined before the style of the base folder was known.
            if is_windows_style(&root) {
                file = file.replace('/', "\\");
            }
            let mut path = with_separator(&root, &file);
            path.push_str(&file);
            if path.chars().count() >= MAX_TAG_PATH || !fs.exists_absolute(&path) {
                return None;
            }
            return Some((path, at_line, at_column));
        } else {
            return None;
        }
    }
    None
}

/// A hit of the text of a window that has no file: `:HWND:[id]name(line,col): text`.
fn hwnd_line(line: &[char]) -> Option<Found> {
    let tag = hwnd_tag_text(line)?;
    let open = line_column_pos(line);
    if open == 0 {
        // The list of the names (an empty pattern) has no position: the window is the target.
        // (Sakura needs the position; a list of windows is of no other use.)
        return Some((tag, 0, 0));
    }
    let (l, c) = line_column(&line[open + 1..]);
    Some((tag, l, c))
}

/// The free form (compiler messages, `#include`): the first word that names a file.
fn free_form(line: &[char], fs: &Fs) -> Option<Found> {
    let mut p = 0;
    while p < line.len() {
        while p < line.len() && (matches!(line[p], ' ' | '\t') || is_line_delimiter(line[p])) {
            p += 1;
        }
        if p >= line.len() {
            break;
        }
        if let Some((begin, len)) = file_path_in(&line[p..], fs) {
            let path = line[p + begin..p + begin + len].iter().collect();
            let (l, c) = line_column(&line[p + begin + len..]);
            return Some((path, l, c));
        }
        while p < line.len() && !matches!(line[p], ' ' | '\t') {
            p += 1;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// Text lines and the files that exist.
    struct World {
        lines: Vec<String>,
        files: HashSet<PathBuf>,
        base: Option<PathBuf>,
    }

    fn world(lines: &[&str], files: &[&str]) -> World {
        World {
            lines: lines.iter().map(|s| (*s).to_owned()).collect(),
            files: files.iter().map(PathBuf::from).collect(),
            base: None,
        }
    }

    impl World {
        fn jump(&self, line: usize) -> Option<TagTarget> {
            find_target(
                &|i| self.lines.get(i).cloned(),
                line,
                self.base.as_deref(),
                &|p| self.files.contains(p),
            )
        }
    }

    fn target(path: &str, line: Option<usize>, column: Option<usize>) -> Option<TagTarget> {
        Some(TagTarget {
            path: PathBuf::from(path),
            window: None,
            line,
            column,
        })
    }

    #[test]
    fn the_numbers_of_a_position() {
        let n = |s: &str| line_column(&s.chars().collect::<Vec<_>>());
        assert_eq!(n("5395,11): str"), (5395, 11));
        assert_eq!(n("(  5395,11   ): str"), (5395, 11));
        assert_eq!(n("12: error"), (12, 0));
        assert_eq!(n("12:3: error"), (12, 0));
        assert_eq!(n("no numbers"), (0, 0));
        assert_eq!(n("7,"), (7, 0));
        assert_eq!(n("99999999999999999999,1"), (i64::from(i32::MAX), 1));
    }

    #[test]
    fn the_position_of_the_open_paren() {
        let p = |s: &str| line_column_pos(&s.chars().collect::<Vec<_>>());
        assert_eq!(p("・a.txt(12,3): x"), 6);
        assert_eq!(p("・a.txt(12): x"), 6);
        assert_eq!(p("・a.txt(12,3)  [UTF-8]: x"), 6);
        assert_eq!(p("・a.txt(12,3) [UTF-8]: x"), 6);
        assert_eq!(p("・a.txt: x"), 0);
        assert_eq!(p("・(12,3): x"), 0);
        assert_eq!(p("): "), 0);
    }

    // ---- the normal form, with the full path ----

    #[test]
    fn a_full_path_with_line_and_column() {
        let w = world(
            &[r"C:\RootFolder\SubFolders\FileName.ext(5395,11): str"],
            &[r"C:\RootFolder\SubFolders\FileName.ext"],
        );
        assert_eq!(
            w.jump(0),
            target(
                r"C:\RootFolder\SubFolders\FileName.ext",
                Some(5395),
                Some(11)
            )
        );
    }

    #[test]
    fn a_full_path_with_an_encoding_name() {
        let w = world(
            &[r"C:\Root\FileName.ext(150,6)  [UTF-8]: str"],
            &[r"C:\Root\FileName.ext"],
        );
        assert_eq!(
            w.jump(0),
            target(r"C:\Root\FileName.ext", Some(150), Some(6))
        );
    }

    #[test]
    fn a_line_without_a_column_keeps_the_column_unset() {
        let w = world(&["/p/a.txt(7): x"], &["/p/a.txt"]);
        assert_eq!(w.jump(0), target("/p/a.txt", Some(7), None));
    }

    #[test]
    fn a_file_name_without_a_position_does_not_move_the_caret() {
        let w = world(&["see /p/a.txt now"], &["/p/a.txt"]);
        assert_eq!(w.jump(0), target("/p/a.txt", None, None));
    }

    #[test]
    fn crlf_at_the_end_of_the_line_is_not_part_of_the_name() {
        let w = world(&["/p/a.txt(3,1)\r"], &["/p/a.txt"]);
        assert_eq!(w.jump(0), target("/p/a.txt", Some(3), Some(1)));
        let w = world(&["/p/a.txt\r"], &["/p/a.txt"]);
        assert_eq!(w.jump(0), target("/p/a.txt", None, None));
    }

    // ---- compiler messages ----

    #[test]
    fn gcc_style_messages() {
        let w = world(&["/p/src/main.c:12:5: error: x"], &["/p/src/main.c"]);
        // the column after a colon is not taken (only a comma separates it)
        assert_eq!(w.jump(0), target("/p/src/main.c", Some(12), None));
    }

    #[test]
    fn borland_style_messages_with_a_leading_tool_name() {
        let w = world(
            &[r"Error E2141 C:\work\a.cpp 21: Declaration syntax error"],
            &[r"C:\work\a.cpp"],
        );
        assert_eq!(w.jump(0), target(r"C:\work\a.cpp", Some(21), None));
    }

    #[test]
    fn a_name_with_spaces_takes_the_longest_existing_one() {
        let w = world(
            &["/p/my notes/a b.txt(4,2): x"],
            &["/p/my notes/a b.txt", "/p/my"],
        );
        assert_eq!(w.jump(0), target("/p/my notes/a b.txt", Some(4), Some(2)));
    }

    #[test]
    fn a_japanese_path() {
        let w = world(
            &["/p/資料/設計書.txt(10,3): 検索語"],
            &["/p/資料/設計書.txt"],
        );
        assert_eq!(w.jump(0), target("/p/資料/設計書.txt", Some(10), Some(3)));
    }

    #[test]
    fn a_quoted_name() {
        let w = world(&["\"/p/a.txt\"(5,1): x"], &["/p/a.txt"]);
        assert_eq!(w.jump(0), target("/p/a.txt", Some(5), Some(1)));
    }

    #[test]
    fn an_include_line() {
        let mut w = world(&["#include <stdio.h>"], &["/p/stdio.h"]);
        w.base = Some(PathBuf::from("/p"));
        assert_eq!(w.jump(0), target("/p/stdio.h", None, None));
        let mut w = world(&["#include \"util.h\""], &["/p/util.h"]);
        w.base = Some(PathBuf::from("/p"));
        assert_eq!(w.jump(0), target("/p/util.h", None, None));
    }

    #[test]
    fn a_relative_name_is_relative_to_the_folder_of_the_document() {
        let mut w = world(&["src/a.rs(3,4): x"], &["/proj/src/a.rs"]);
        w.base = Some(PathBuf::from("/proj"));
        assert_eq!(w.jump(0), target("/proj/src/a.rs", Some(3), Some(4)));
    }

    #[test]
    fn the_words_of_the_line_are_tried_in_order() {
        let w = world(&["working ... /p/a.txt(2,2)"], &["/p/a.txt"]);
        assert_eq!(w.jump(0), target("/p/a.txt", Some(2), Some(2)));
    }

    #[test]
    fn a_line_without_a_file_has_no_target() {
        let w = world(&["nothing to see here (1,2)"], &["/p/a.txt"]);
        assert_eq!(w.jump(0), None);
        assert_eq!(w.jump(5), None);
        let w = world(&[""], &[]);
        assert_eq!(w.jump(0), None);
    }

    #[test]
    fn a_folder_is_not_a_file() {
        // `is_file` is false for a folder
        let w = world(&["/p/dir(3,1): x"], &[]);
        assert_eq!(w.jump(0), None);
    }

    #[test]
    fn characters_that_cannot_be_in_a_name_end_it() {
        let w = world(&["/p/a.txt|b(1,1)"], &["/p/a.txt"]);
        assert_eq!(w.jump(0), target("/p/a.txt", Some(1), Some(1)));
    }

    // ---- Grep output with headings ----

    #[test]
    fn folder_headings_by_folder() {
        let w = world(
            &[
                r#"◎"C:\Root""#,
                "■",
                r#"・FileName.ext(5395,11): str"#,
                r#"■"SubFolders""#,
                r#"・FileName2.ext(5395,11): str"#,
                r#"・FileName3.ext(123,12): str"#,
            ],
            &[
                r"C:\Root\FileName.ext",
                r"C:\Root\SubFolders\FileName2.ext",
                r"C:\Root\SubFolders\FileName3.ext",
            ],
        );
        assert_eq!(
            w.jump(2),
            target(r"C:\Root\FileName.ext", Some(5395), Some(11))
        );
        assert_eq!(
            w.jump(4),
            target(r"C:\Root\SubFolders\FileName2.ext", Some(5395), Some(11))
        );
        assert_eq!(
            w.jump(5),
            target(r"C:\Root\SubFolders\FileName3.ext", Some(123), Some(12))
        );
    }

    #[test]
    fn the_base_folder_form() {
        let w = world(
            &[
                r#"■"C:\Root""#,
                r#"・SubFolders\FileName2.ext(5395,11): str"#,
                r#"・SubFolders\FileName2.ext(5396,11): str"#,
            ],
            &[r"C:\Root\SubFolders\FileName2.ext"],
        );
        assert_eq!(
            w.jump(2),
            target(r"C:\Root\SubFolders\FileName2.ext", Some(5396), Some(11))
        );
    }

    #[test]
    fn the_per_file_form() {
        let w = world(
            &[
                r#"■"C:\Root\FileName.ext""#,
                "・(  5395,11   ): str",
                r#"■"C:\Root\Sub\FileName2.ext""#,
                "・(  5395,11   ): str",
                "・(   123,12   ): str",
            ],
            &[r"C:\Root\FileName.ext", r"C:\Root\Sub\FileName2.ext"],
        );
        assert_eq!(
            w.jump(1),
            target(r"C:\Root\FileName.ext", Some(5395), Some(11))
        );
        assert_eq!(
            w.jump(4),
            target(r"C:\Root\Sub\FileName2.ext", Some(123), Some(12))
        );
    }

    #[test]
    fn the_per_file_form_with_an_encoding_name() {
        let w = world(
            &[
                r#"■"C:\Root\FileName.ext"  [UTF-8]"#,
                "・(   150,6    ): str",
            ],
            &[r"C:\Root\FileName.ext"],
        );
        assert_eq!(
            w.jump(1),
            target(r"C:\Root\FileName.ext", Some(150), Some(6))
        );
    }

    #[test]
    fn the_per_file_form_with_a_base_folder() {
        let w = world(
            &[
                r#"◎"C:\Root""#,
                r#"■"FileName.ext""#,
                "・(  5395,11   ): str",
                r#"■"SubFolders\FileName2.ext""#,
                "・(  5396,11   ): str",
            ],
            &[r"C:\Root\FileName.ext", r"C:\Root\SubFolders\FileName2.ext"],
        );
        assert_eq!(
            w.jump(2),
            target(r"C:\Root\FileName.ext", Some(5395), Some(11))
        );
        assert_eq!(
            w.jump(4),
            target(r"C:\Root\SubFolders\FileName2.ext", Some(5396), Some(11))
        );
    }

    #[test]
    fn the_per_file_form_with_folders_and_files() {
        let w = world(
            &[
                r#"◎"C:\Root""#,
                "■",
                r#"◆"FileName.ext""#,
                "・(  5395,11   ): str",
                r#"■"SubFolders""#,
                r#"◆"FileName2.ext""#,
                "・(  5395,11   ): str",
                "・(  5396,11   ): str",
                r#"◆"FileName3.ext""#,
                "・(   123,12   ): str",
            ],
            &[
                r"C:\Root\FileName.ext",
                r"C:\Root\SubFolders\FileName2.ext",
                r"C:\Root\SubFolders\FileName3.ext",
            ],
        );
        assert_eq!(
            w.jump(3),
            target(r"C:\Root\FileName.ext", Some(5395), Some(11))
        );
        assert_eq!(
            w.jump(7),
            target(r"C:\Root\SubFolders\FileName2.ext", Some(5396), Some(11))
        );
        assert_eq!(
            w.jump(9),
            target(r"C:\Root\SubFolders\FileName3.ext", Some(123), Some(12))
        );
    }

    #[test]
    fn the_per_file_form_with_full_folders() {
        let w = world(
            &[
                r#"■"C:\Root\SubFolders""#,
                r#"◆"FileName2.ext""#,
                "・(  5395,11   ): str",
            ],
            &[r"C:\Root\SubFolders\FileName2.ext"],
        );
        assert_eq!(
            w.jump(2),
            target(r"C:\Root\SubFolders\FileName2.ext", Some(5395), Some(11))
        );
    }

    // `/root` is not an absolute path on Windows.
    #[cfg(unix)]
    #[test]
    fn a_heading_can_be_jumped_from_too() {
        let w = world(&[r#"■"/p/a.txt""#], &["/p/a.txt"]);
        assert_eq!(w.jump(0), target("/p/a.txt", None, None));
    }

    // `/root` is not an absolute path on Windows.
    #[cfg(unix)]
    #[test]
    fn unix_style_headings_use_slashes() {
        let w = world(
            &[r#"■"/root""#, r#"・sub/a.txt(2,3): x"#],
            &["/root/sub/a.txt"],
        );
        assert_eq!(w.jump(1), target("/root/sub/a.txt", Some(2), Some(3)));
    }

    #[test]
    fn a_missing_file_gives_no_target() {
        let w = world(&[r#"■"/root""#, r#"・gone.txt(2,3): x"#], &[]);
        assert_eq!(w.jump(1), None);
    }

    #[test]
    fn a_broken_heading_gives_no_target() {
        // a line that is not part of the heading stops the search upwards
        let w = world(
            &[r#"■"/root""#, "something else", r#"・a.txt(2,3): x"#],
            &["/root/a.txt"],
        );
        assert_eq!(w.jump(2), None);
    }

    // `/root` is not an absolute path on Windows.
    #[cfg(unix)]
    #[test]
    fn a_relative_heading_needs_a_base_folder() {
        let w = world(
            &[r#"◎"/root""#, r#"■"sub""#, r#"・a.txt(2,3): x"#],
            &["/root/sub/a.txt"],
        );
        assert_eq!(w.jump(2), target("/root/sub/a.txt", Some(2), Some(3)));
        let w = world(&[r#"■"sub""#, r#"・a.txt(2,3): x"#], &["sub/a.txt"]);
        assert_eq!(w.jump(1), None);
    }

    #[test]
    fn long_quoted_names_are_refused() {
        let long = "a".repeat(300);
        let w = world(&["◎\"/r\"", &format!("・{long}(1,1): x")], &[]);
        assert_eq!(w.jump(1), None);
        assert_eq!(
            quoted(&format!("{long}\"").chars().collect::<Vec<_>>()),
            None
        );
        assert_eq!(quoted(&"\"".chars().collect::<Vec<_>>()), None);
    }

    // ---- the windows without a file: `:HWND:[id]name` ----

    fn window(id: u64, line: Option<usize>, column: Option<usize>) -> Option<TagTarget> {
        Some(TagTarget {
            path: PathBuf::new(),
            window: Some(id),
            line,
            column,
        })
    }

    #[test]
    fn a_hit_of_a_window_without_a_file_names_the_window() {
        // `:HWND:[01234567](無題)2(123,45): str`
        let w = world(&[":HWND:[00000000000000ab](無題)2(123,45): str"], &[]);
        assert_eq!(w.jump(0), window(0xab, Some(123), Some(45)));
        // with the encoding in front of the colon
        let w = world(&[":HWND:[1f](無題)(7,2)  [UTF-8]: str"], &[]);
        assert_eq!(w.jump(0), window(0x1f, Some(7), Some(2)));
    }

    #[test]
    fn the_relative_form_names_the_window_too() {
        let w = world(&["・:HWND:[0000000000000010](Grep)(3,4): str"], &[]);
        assert_eq!(w.jump(0), window(0x10, Some(3), Some(4)));
    }

    #[test]
    fn the_per_file_forms_name_the_window() {
        // `■":HWND:[id]name"` and `・(line,col)`
        let w = world(&[r#"■":HWND:[2a](無題)1""#, "・(   12,3    ): str"], &[]);
        assert_eq!(w.jump(1), window(0x2a, Some(12), Some(3)));
        // `◆":HWND:[id]name"` below a folder line
        let w = world(
            &[
                r#"◎"/r""#,
                "■",
                r#"◆":HWND:[2a](無題)1""#,
                "・(   5,6    ): str",
            ],
            &[],
        );
        assert_eq!(w.jump(3), window(0x2a, Some(5), Some(6)));
    }

    #[test]
    fn the_heading_of_a_window_names_the_window() {
        // the per-file form without folders: `■":HWND:[id]name"  [enc]` is the line of a hit
        // and, for a search of the names, the whole result
        let w = world(&[r#"■":HWND:[2a](無題)1"  [UTF-8]"#], &[]);
        assert_eq!(w.jump(0), window(0x2a, None, None));
        let w = world(&[r#"■":HWND:[2a](無題)1""#, "・(   5,6    ): str"], &[]);
        assert_eq!(w.jump(0), window(0x2a, None, None));
        assert_eq!(w.jump(1), window(0x2a, Some(5), Some(6)));
    }

    #[test]
    fn a_name_alone_names_the_window_without_a_place() {
        let w = world(&[":HWND:[2a](無題)1  [UTF-8]"], &[]);
        assert_eq!(w.jump(0), window(0x2a, None, None));
    }

    #[test]
    fn a_broken_tag_is_not_a_window() {
        for line in [
            ":HWND:[](無題)(1,2): x",
            ":HWND:[xyz](無題)(1,2): x",
            ":HWND:[11111111111111111](無題)(1,2): x",
            ":HWND:[12 (無題)(1,2): x",
        ] {
            let w = world(&[line], &[]);
            assert_eq!(w.jump(0), None, "{line}");
        }
    }
}
