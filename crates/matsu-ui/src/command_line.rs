// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/_main/CCommandLine.cpp (CheckCommandLine, ParseCommandLine, SetDocType)
//   sakura_core/basis/GrepInfo.h (the defaults of the Grep conditions)
//   sakura_core/basis/EditInfo.h (the defaults of the file's conditions)
//
// sakura_core/_main/CCommandLine.cpp:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2001, genta
//   Copyright (C) 2002, aroka CControlTrayより分離
//   Copyright (C) 2002, genta, Moca
//   Copyright (C) 2005, D.S.Koba, genta, susu
//   Copyright (C) 2006, ryoji
//   Copyright (C) 2007, ryoji
//   Copyright (C) 2018-2026, Sakura Editor Organization
// sakura_core/basis/GrepInfo.h, sakura_core/basis/EditInfo.h:
//   Copyright (C) 2018-2026, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (a command line as one
// UTF-16 string, `CNativeW`, fixed buffers) was rewritten in Rust on the arguments that the OS
// has already split, and the tests are new.
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

//! Sakura's command-line options, read from the arguments of a launch.
//!
//! `-X=` `-Y=` (the caret), `-VX=` `-VY=` (the top left of the view), `-SX=` `-SY=` `-WX=` `-WY=`
//! (the window), `-TYPE=`, `-CODE=`, `-R`, `-GREPMODE`, `-GREPDLG`, `-GKEY=`, `-GREPR=`, `-GFILE=`,
//! `-GFOLDER=`, `-GOPT=`, `-GCODE=`, `-M=`, `-MTYPE=`, `-PROF=`, `-@=` and the ones that MATSU Editor
//! accepts and ignores (`-NOWIN`, `-GROUP=`, `-DEBUGMODE`, `-PROFMGR`), with the files named
//! among them. The help is `help/sakura/res/HLP000109.html`.
//!
//! Sakura reads one string (`GetCommandLine`) and cuts it at the blanks. A launch of MATSU Editor has
//! the arguments as the OS split them (the shell, or the C runtime of Windows, took the quotes
//! away), so every argument is one token here. What `CCommandLine` does with a token is kept:
//!
//! * a token that starts with `-` (or with `"-`) is an option, until the option `--`; the
//!   names are not case sensitive; the value follows `=` or `:`; an option that is not known
//!   is dropped, and so is one that needs a value and has none;
//! * the quotation marks around a token, and around a value, are taken away; in the values of
//!   `-GKEY=` `-GREPR=` `-GFILE=` `-GFOLDER=` `-M=` and in a quoted file name, `""` is `"`;
//! * every other token is a file, and so is every token after `--`.
//!
//! Not done: Sakura looks for an existing file among the first tokens, glued with blanks, when the
//! first token does not start with `-` (a name with blanks that was not quoted). The OS has cut the
//! command line already, so MATSU Editor needs the quotes. Not done either:
//! `ParseKanjiCodeFromFileName` (the digits in the name of the program give the encoding), and
//! the message box for a file name that is too long or has characters that Windows does not allow
//! (the open fails with its own message).

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

/// The longest type name of `-TYPE=` (`MAX_DOCTYPE_LEN` in `sakura_core/config/maxdata.h`); a
/// longer name is cut (`CCommandLine::SetDocType`).
pub(crate) const MAX_DOC_TYPE_LEN: usize = 7;

/// `-GREPMODE` gives the document type `grepout` unless `-TYPE=` came first.
const GREP_TYPE: &str = "grepout";
/// `-DEBUGMODE` gives the document type `output` unless `-TYPE=` came first.
const DEBUG_TYPE: &str = "output";

/// The conditions of `-GREPMODE` and `-GREPDLG` (`GrepInfo`), with the values it starts with
/// (`sakura_core/basis/GrepInfo.h`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GrepArgs {
    /// `-GKEY=`: the condition.
    pub key: String,
    /// `-GREPR=`: the new text of a Grep 置換 (`bGrepReplace` is `Some`).
    pub replace: Option<String>,
    /// `-GFILE=`: the file patterns, with the excluded ones (`;#folder`, `;!file`) packed in.
    pub file: String,
    /// `-GFOLDER=`.
    pub folder: String,
    /// `-GOPT=L`: 英大文字と小文字を区別.
    pub match_case: bool,
    /// `-GOPT=R`: 正規表現.
    pub regex: bool,
    /// `-GOPT=W`: 単語単位.
    pub whole_word: bool,
    /// `-GOPT=X`: do not move the current folder after the search.
    pub keep_current_folder: bool,
    /// `-GOPT=U`: write the result to the standard output.
    pub stdout: bool,
    /// `-GOPT=H` turns it off: the heading and the count.
    pub header: bool,
    /// `-GOPT=S`: サブフォルダーからも検索.
    pub sub_folder: bool,
    /// `-GCODE=` and `-GOPT=K`: the number of the encoding (`CODE_SJIS`, 0, until one is given;
    /// 99 is the detection).
    pub charset: i32,
    /// `-GOPT=1|2|3`: 出力形式 (1 is the normal one).
    pub output_style: i32,
    /// `-GOPT=P|N`: 結果出力 (0 is 該当部分, 1 該当行, 2 否該当行).
    pub output_line_type: i32,
    /// `-GOPT=F`: ファイル毎最初のみ.
    pub file_only: bool,
    /// `-GOPT=B`: ベースフォルダー表示.
    pub base_folder: bool,
    /// `-GOPT=D`: フォルダー毎に表示.
    pub separate_folder: bool,
    /// `-GOPT=C`: クリップボードから貼り付け (Grep 置換).
    pub paste: bool,
    /// `-GOPT=O`: バックアップ作成 (Grep 置換).
    pub backup: bool,
    /// `-GOPT=E`: 除外ファイルを正規表現で指定.
    pub except_file_regex: bool,
}

impl Default for GrepArgs {
    fn default() -> Self {
        // GrepInfo.h
        Self {
            key: String::new(),
            replace: None,
            file: String::new(),
            folder: String::new(),
            match_case: false,
            regex: false,
            whole_word: false,
            keep_current_folder: false,
            stdout: false,
            header: true,
            sub_folder: false,
            charset: 0,
            output_style: 1,
            output_line_type: 0,
            file_only: false,
            base_folder: false,
            separate_folder: false,
            paste: false,
            backup: false,
            except_file_regex: false,
        }
    }
}

/// What a command line says. `None` and `false` are what Sakura has before it reads anything
/// (`CCommandLine::CCommandLine`, `EditInfo::EditInfo`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CommandLine {
    /// The files, in the order of the command line; relative ones are in the folder of the launch.
    pub files: Vec<PathBuf>,
    /// `-X=`: the column of the caret, from 0 (the value on the command line counts from 1), in
    /// UTF-16 code units. A value below 1 is no value.
    pub caret_x: Option<usize>,
    /// `-Y=`: the line of the caret, from 0.
    pub caret_y: Option<usize>,
    /// `-VX=`: the column at the left of the view, from 0.
    pub view_left: Option<usize>,
    /// `-VY=`: the row at the top of the view, from 0.
    pub view_top: Option<usize>,
    /// `-SX=`: the width of the window, in pixels (Sakura subtracts 1 from the value; a result
    /// below 0 is no value).
    pub window_width: Option<i32>,
    /// `-SY=`: the height of the window.
    pub window_height: Option<i32>,
    /// `-WX=`: the x coordinate of the left of the window, as it is. A value is `CW_USEDEFAULT`
    /// (`i32::MIN`) when none is given; that is `None` here.
    pub window_x: Option<i32>,
    /// `-WY=`: the y coordinate of the top of the window.
    pub window_y: Option<i32>,
    /// `-TYPE=`: the extension that chooses the document type, at most [`MAX_DOC_TYPE_LEN`] long.
    pub doc_type: Option<String>,
    /// `-CODE=`: Sakura's number of the encoding, as given (it may be none that exists).
    pub code: Option<i32>,
    /// `-R`.
    pub view_mode: bool,
    /// `-NOWIN`.
    pub no_window: bool,
    /// `-GREPMODE`.
    pub grep_mode: bool,
    /// `-GREPDLG`.
    pub grep_dialog: bool,
    /// `-DEBUGMODE`.
    pub debug_mode: bool,
    /// `-PROFMGR`.
    pub profile_manager: bool,
    /// `-GROUP=`: -1 when it was not given.
    pub group: Option<i32>,
    /// `-M=`: the macro to run after the start.
    pub macro_: String,
    /// `-MTYPE=`: what `-M=` is (the extension of a language; empty or `file` for a file).
    pub macro_type: String,
    /// `-PROF=`: the profile; empty selects the default one.
    pub profile: Option<String>,
    /// The conditions of the Grep.
    pub grep: GrepArgs,
}

/// Reads the arguments. `-@=file` is not read (see [`parse_with`]).
#[cfg(test)]
pub(crate) fn parse(args: &[OsString]) -> CommandLine {
    parse_with(args, &mut |_| None)
}

/// Reads the arguments. A response file (`-@=file`, the last one counts) is read with `read`,
/// which gives the text of the file, or `None` if it cannot be read; its text is cut at the
/// blanks (`my_strtok` with a blank) and read as more arguments of the same launch, after the
/// real ones (`ParseCommandLine` calls itself with `bResponse` off).
pub(crate) fn parse_with(
    args: &[OsString],
    read: &mut dyn FnMut(&Path) -> Option<String>,
) -> CommandLine {
    let mut line = CommandLine::default();
    let mut options_end = false;
    let response = line.parse_tokens(args, &mut options_end);
    if let Some(path) = response
        && let Some(text) = read(Path::new(&path))
    {
        let tokens: Vec<OsString> = text
            .split(' ')
            .filter(|t| !t.is_empty())
            .map(OsString::from)
            .collect();
        // The nested parse starts again with the options on, and has no response file.
        let mut options_end = false;
        line.parse_tokens(&tokens, &mut options_end);
    }
    line
}

/// What an option is (the `CMDLINEOPT_*` values).
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Opt {
    At,
    X,
    Y,
    Vx,
    Vy,
    Sx,
    Sy,
    Wx,
    Wy,
    Code,
    Type,
    Gkey,
    Grepr,
    Gfile,
    Gfolder,
    Gopt,
    Gcode,
    Group,
    M,
    Mtype,
    Prof,
    R,
    NoMoreOpt,
    NoWin,
    GrepMode,
    GrepDlg,
    DebugMode,
    ProfMgr,
}

/// The options without a value (`_COptWoA`).
const WITHOUT_VALUE: &[(&str, Opt)] = &[
    ("R", Opt::R),
    ("-", Opt::NoMoreOpt),
    ("NOWIN", Opt::NoWin),
    ("GREPMODE", Opt::GrepMode),
    ("GREPDLG", Opt::GrepDlg),
    ("DEBUGMODE", Opt::DebugMode),
    ("PROFMGR", Opt::ProfMgr),
];

/// The options with a value (`_COptWithA`), and whether the value may be empty (`bLen0`). The
/// order matters as in Sakura, where the first name that fits wins.
const WITH_VALUE: &[(&str, Opt, bool)] = &[
    ("@", Opt::At, false),
    ("X", Opt::X, false),
    ("Y", Opt::Y, false),
    ("VX", Opt::Vx, false),
    ("VY", Opt::Vy, false),
    ("SX", Opt::Sx, false),
    ("SY", Opt::Sy, false),
    ("WX", Opt::Wx, false),
    ("WY", Opt::Wy, false),
    ("CODE", Opt::Code, false),
    ("TYPE", Opt::Type, false),
    ("GKEY", Opt::Gkey, false),
    ("GREPR", Opt::Grepr, true),
    ("GFILE", Opt::Gfile, false),
    ("GFOLDER", Opt::Gfolder, false),
    ("GOPT", Opt::Gopt, false),
    ("GCODE", Opt::Gcode, false),
    ("GROUP", Opt::Group, false),
    ("M", Opt::M, false),
    ("MTYPE", Opt::Mtype, false),
    ("PROF", Opt::Prof, true),
];

/// `CheckCommandLine`: the option that `text` (without its `-`) is, and its value.
fn check_option(text: &str) -> Option<(Opt, &str)> {
    let bytes = text.as_bytes();
    for &(name, opt, may_be_empty) in WITH_VALUE {
        let n = name.len();
        if bytes.len() > n
            && matches!(bytes[n], b'=' | b':')
            && bytes[..n].eq_ignore_ascii_case(name.as_bytes())
        {
            // `=` and `:` are one byte, so n + 1 is a boundary of chars.
            let mut value = &text[n + 1..];
            if let Some(rest) = value.strip_prefix('"') {
                value = rest;
                if let Some(rest) = value.strip_suffix('"') {
                    value = rest;
                }
            }
            // 2010.06.12 syat: an option without a value is not an option
            if value.is_empty() && !may_be_empty {
                return None;
            }
            return Some((opt, value));
        }
    }
    WITHOUT_VALUE
        .iter()
        .find(|(name, _)| text.eq_ignore_ascii_case(name))
        .map(|&(_, opt)| (opt, ""))
}

/// `AtoiOptionInt`: the number at the start of a value, which may start with a quotation mark.
fn atoi_option(value: &str) -> i32 {
    atoi(value.strip_prefix(['"', '\'']).unwrap_or(value))
}

/// C's `atoi` (`_wtoi`): blanks, a sign, digits; anything else ends the number, and no digit is 0.
/// A number too big for `i32` stops at its limit.
fn atoi(text: &str) -> i32 {
    let text = text.trim_start_matches([' ', '\t', '\n', '\x0b', '\x0c', '\r']);
    let (negative, digits) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let mut value: i64 = 0;
    for b in digits.bytes().take_while(u8::is_ascii_digit) {
        value = (value * 10 + i64::from(b - b'0')).min(i64::from(i32::MAX) + 1);
    }
    let value = if negative { -value } else { value };
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// A number on the command line counts from 1; the result counts from 0 and is none below 0.
fn from_one(value: i32) -> Option<i32> {
    value.checked_sub(1).filter(|v| *v >= 0)
}

/// `Replace( L"\"\"", L"\"" )`: two quotation marks are one.
fn unescape_quotes(value: &str) -> String {
    value.replace("\"\"", "\"")
}

impl CommandLine {
    /// Reads `tokens` into this. Returns the response file that was named, if any.
    fn parse_tokens(&mut self, tokens: &[OsString], options_end: &mut bool) -> Option<OsString> {
        let mut response = None;
        for token in tokens {
            let option = match token.to_str() {
                Some(text)
                    if !*options_end && (text.starts_with('-') || text.starts_with("\"-")) =>
                {
                    Some(text)
                }
                _ => None,
            };
            let Some(text) = option else {
                if let Some(file) = file_of(token) {
                    self.files.push(file);
                }
                continue;
            };
            // `"-X=1"`: Visual Studio put quotes around each argument (2007.09.09 genta).
            let mut text = text;
            if let Some(rest) = text.strip_prefix('"') {
                text = rest.strip_suffix('"').unwrap_or(rest);
            }
            // the `-`
            let Some((opt, value)) = check_option(&text[1..]) else {
                continue;
            };
            match opt {
                Opt::At => response = Some(OsString::from(value)),
                Opt::X => self.caret_x = from_one(atoi_option(value)).map(|v| v as usize),
                Opt::Y => self.caret_y = from_one(atoi_option(value)).map(|v| v as usize),
                Opt::Vx => self.view_left = from_one(atoi_option(value)).map(|v| v as usize),
                Opt::Vy => self.view_top = from_one(atoi_option(value)).map(|v| v as usize),
                Opt::Sx => self.window_width = from_one(atoi_option(value)),
                Opt::Sy => self.window_height = from_one(atoi_option(value)),
                Opt::Wx => self.window_x = Some(atoi_option(value)).filter(|v| *v != i32::MIN),
                Opt::Wy => self.window_y = Some(atoi_option(value)).filter(|v| *v != i32::MIN),
                Opt::Type => self.set_doc_type(value),
                Opt::Code => self.code = Some(atoi_option(value)),
                Opt::R => self.view_mode = true,
                Opt::NoWin => self.no_window = true,
                Opt::GrepMode => {
                    self.grep_mode = true;
                    if self.doc_type.is_none() {
                        self.set_doc_type(GREP_TYPE);
                    }
                }
                Opt::GrepDlg => self.grep_dialog = true,
                Opt::Gkey => self.grep.key = unescape_quotes(value),
                Opt::Grepr => self.grep.replace = Some(unescape_quotes(value)),
                Opt::Gfile => self.grep.file = unescape_quotes(value),
                Opt::Gfolder => self.grep.folder = unescape_quotes(value),
                Opt::Gopt => self.grep.read_options(value),
                Opt::Gcode => self.grep.charset = atoi_option(value),
                Opt::Group => self.group = Some(atoi_option(value)),
                Opt::DebugMode => {
                    self.debug_mode = true;
                    if self.doc_type.is_none() {
                        self.set_doc_type(DEBUG_TYPE);
                    }
                }
                Opt::NoMoreOpt => *options_end = true,
                Opt::M => self.macro_ = unescape_quotes(value),
                Opt::Mtype => self.macro_type = value.to_owned(),
                Opt::Prof => self.profile = Some(value.to_owned()),
                Opt::ProfMgr => self.profile_manager = true,
            }
        }
        response
    }

    /// `SetDocType`: the name is cut at [`MAX_DOC_TYPE_LEN`].
    fn set_doc_type(&mut self, name: &str) {
        self.doc_type = Some(name.chars().take(MAX_DOC_TYPE_LEN).collect());
    }
}

impl GrepArgs {
    /// The letters of `-GOPT=` (capitals only). Letters that mean nothing are skipped.
    fn read_options(&mut self, letters: &str) {
        for letter in letters.chars() {
            match letter {
                'X' => self.keep_current_folder = true,
                'U' => self.stdout = true,
                'H' => self.header = false,
                'S' => self.sub_folder = true,
                'L' => self.match_case = true,
                'R' => self.regex = true,
                // 2002/09/21 Moca: kept for old command lines
                'K' => self.charset = matsu_core::encoding::AUTODETECT_CODE,
                'P' => self.output_line_type = 1,
                'N' => self.output_line_type = 2,
                'W' => self.whole_word = true,
                '1' => self.output_style = 1,
                '2' => self.output_style = 2,
                '3' => self.output_style = 3,
                'F' => self.file_only = true,
                'B' => self.base_folder = true,
                'D' => self.separate_folder = true,
                'C' => self.paste = true,
                'O' => self.backup = true,
                'E' => self.except_file_regex = true,
                _ => {}
            }
        }
    }
}

/// The file that a token that is no option names, or `None` for an empty name.
fn file_of(token: &OsStr) -> Option<PathBuf> {
    let Some(text) = token.to_str() else {
        // not text: no quotes or URL to read
        return Some(PathBuf::from(token));
    };
    let name = match text.strip_prefix('"') {
        // 2005 genta: the closing mark may be missing; one mark alone is an empty name
        Some(rest) => unescape_quotes(rest.strip_suffix('"').unwrap_or(rest)),
        None => text.to_owned(),
    };
    if name.is_empty() {
        return None;
    }
    // Nov. 11, 2005 susu: a URL of the browser or the file manager
    if name.starts_with("file:///") {
        return crate::launch::path_of_file_url(&name).map(windows_drive_path);
    }
    Some(PathBuf::from(name))
}

/// `/C:/work/a.txt` of a URL is `C:/work/a.txt` on Windows.
fn windows_drive_path(path: PathBuf) -> PathBuf {
    if cfg!(windows)
        && let Some(text) = path.to_str()
        && let Some(rest) = text.strip_prefix('/')
        && rest.as_bytes().get(1) == Some(&b':')
    {
        return PathBuf::from(rest);
    }
    path
}

/// The profile name as a folder name: none when it would leave the settings folder (a separator,
/// a drive, `.` or `..`) or is too long (`CDlgProfileMgr::m_ProfileName` is `_MAX_PATH`).
pub(crate) fn valid_profile_name(name: &str) -> Option<&str> {
    let bad = name.is_empty()
        || name == "."
        || name == ".."
        || name.chars().count() >= 260
        || name
            .chars()
            .any(|c| c.is_control() || "/\\:*?\"<>|".contains(c));
    (!bad).then_some(name)
}

/// The text of a response file (`-@=`): UTF-8 with a BOM, else Shift_JIS (the help says "SJIS か
/// UTF-8 BOM付き"). Sakura reads the lines and glues them without anything between
/// (`responseData += input.ReadLineW()`).
pub(crate) fn response_text(bytes: &[u8]) -> String {
    use matsu_core::encoding::Encoding;
    let mut text = String::new();
    match bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        Some(utf8) => text.push_str(&String::from_utf8_lossy(utf8)),
        None => Encoding::ShiftJis.codec().decode(bytes, &mut text),
    }
    text.lines().collect()
}

#[cfg(test)]
mod tests;
