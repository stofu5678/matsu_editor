//! Key macros: recording the commands that were run, saving and loading Sakura's `.mac` text
//! format, and playing a macro back through the [registry](crate::registry).
//!
//! Sakura: `F_RECKEYMACRO`, `F_SAVEKEYMACRO`, `F_LOADKEYMACRO` and `F_EXECKEYMACRO`
//! (`sakura_core/macro/CKeyMacroMgr.cpp`, `CMacro.cpp`, `CSMacroMgr.cpp`).
//!
//! # What the editor calls
//!
//! The UI owns one [`KeyMacroRecorder`] and one [`KeyMacro`] (the last recorded or loaded
//! one). It does not run any command by itself here; the commands for the four functions need
//! the UI state (the recorder, the file dialogs), so they are added with the UI.
//!
//! * `F_RECKEYMACRO` (start and stop): [`KeyMacroRecorder::start`], later
//!   [`KeyMacroRecorder::stop`], which returns the recorded [`KeyMacro`].
//! * After every command that the user ran (a key, a menu item), while
//!   [`KeyMacroRecorder::is_recording`]: [`KeyMacroRecorder::record`] with the function code
//!   and the arguments the command was run with. It keeps only what a key macro keeps
//!   (`CSMacroMgr::CanFuncIsKeyMacro`) and says whether it did.
//! * `F_SAVEKEYMACRO`: [`KeyMacro::save`] (or [`KeyMacro::to_bytes`]).
//! * `F_LOADKEYMACRO`: [`KeyMacro::load`] (or [`KeyMacro::from_bytes`]); an error carries the
//!   line and the column for the message.
//! * `F_EXECKEYMACRO`: [`KeyMacro::play`] on the focused document's `CommandContext`.
//!
//! The file is UTF-8 with a BOM, one call per line (`Char(97);`, `InsText('a\n');`), as
//! Sakura writes it (`CTextOutputStream` defaults to UTF-8 with BOM).
//!
//! # Differences from Sakura
//!
//! * A character is recorded as `Char(<code point>)`. Sakura records UTF-16 code units, so a
//!   character outside the BMP is two `Char` lines there and one here. A file with a surrogate
//!   pair in two `Char` lines is not joined when played; the second line fails as a bad
//!   argument.
//! * Sakura shows a message box for every error in a loaded file and goes on to the end of the
//!   file. Here the first error stops the load and is returned.
//! * A number must fit in 32 bits (Sakura reads it with `_wtoi`).

use std::fmt::{self, Write as _};
use std::fs;
use std::io;
use std::path::Path;

use crate::command::{Arg, CommandError};
use crate::context::CommandContext;
use crate::macro_names;
use crate::registry::Registry;
use crate::sakura::FUNCTIONS;
use crate::sakura::macro_functions::{ArgType, MacroFunction};

/// The first line of a saved file (`STR_ERR_DLGKEYMACMGR1` in `sakura_core/sakura_rc.rc`).
const FILE_HEADER: &str = "//キーボードマクロのファイル";

/// One recorded call: a function code and its arguments (`Arg::Int` and `Arg::Text` only; the
/// file format has no other kinds).
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub code: u16,
    pub args: Vec<Arg>,
}

/// Why a call cannot be kept in a key macro.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepError {
    /// The code has no macro name (it is not in Sakura's macro tables).
    NoMacroName(u16),
    /// More arguments than the function has.
    TooManyArguments { name: &'static str, max: usize },
    /// An argument of the wrong kind (a number where the function takes text, or the other way).
    WrongArgumentType { name: &'static str, position: usize },
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMacroName(code) => write!(f, "function code {code} has no macro name"),
            Self::TooManyArguments { name, max } => {
                write!(f, "{name} takes at most {max} argument(s)")
            }
            Self::WrongArgumentType { name, position } => {
                write!(f, "argument {position} of {name} has the wrong type")
            }
        }
    }
}

impl std::error::Error for StepError {}

/// Turns `Arg::Char` into the code point and checks the arguments against the macro table.
fn checked_step(row: &'static MacroFunction, args: &[Arg]) -> Result<Step, StepError> {
    if args.len() > row.args.len() {
        return Err(StepError::TooManyArguments {
            name: row.name,
            max: row.args.len(),
        });
    }
    let mut kept = Vec::with_capacity(args.len());
    for (i, (arg, expected)) in args.iter().zip(row.args).enumerate() {
        let arg = match (arg, expected) {
            (Arg::Int(n), ArgType::Int) => Arg::Int(*n),
            (Arg::Char(c), ArgType::Int) => Arg::Int(i64::from(u32::from(*c))),
            (Arg::Text(s), ArgType::Str) => Arg::Text(s.clone()),
            _ => {
                return Err(StepError::WrongArgumentType {
                    name: row.name,
                    position: i + 1,
                });
            }
        };
        kept.push(arg);
    }
    Ok(Step {
        code: row.code,
        args: kept,
    })
}

/// A list of calls: what a recording produced, or what a file contains.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeyMacro {
    steps: Vec<Step>,
}

impl KeyMacro {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Adds a call. The code needs a macro name, and the arguments must fit that function.
    /// (Unlike [`KeyMacroRecorder::record`], this does not ask whether a key macro records the
    /// function: Sakura's loader accepts any function that has a name.)
    pub fn push(&mut self, code: u16, args: &[Arg]) -> Result<(), StepError> {
        let row = macro_names::by_code(code).ok_or(StepError::NoMacroName(code))?;
        self.steps.push(checked_step(row, args)?);
        Ok(())
    }

    /// Runs the calls one after the other through the registry; each is one undo step, as when
    /// the user ran them. Stops at the first one that fails.
    ///
    /// Sakura: `CKeyMacroMgr::ExecKeyMacro`, which stops at the first call that returns false.
    pub fn play(&self, registry: &Registry, cx: &mut CommandContext<'_>) -> Result<(), PlayError> {
        for (index, step) in self.steps.iter().enumerate() {
            registry
                .run_code(step.code, cx, &step.args)
                .map_err(|error| PlayError { index, error })?;
        }
        Ok(())
    }

    // ---- the file format ----

    /// The text of the file: a header comment, then one line per call, with CR LF.
    ///
    /// Sakura: `CKeyMacroMgr::SaveKeyMacro` and `CMacro::Save`. Each line is
    /// `Name(arg, 'text');` and a tab-separated comment with the label of the function.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(FILE_HEADER);
        out.push_str("\r\n");
        for step in &self.steps {
            let Some(row) = macro_names::by_code(step.code) else {
                continue;
            };
            out.push_str(row.name);
            out.push('(');
            for (i, arg) in step.args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                match arg {
                    Arg::Int(n) => out.push_str(&n.to_string()),
                    Arg::Text(s) => {
                        out.push('\'');
                        escape_into(&mut out, s);
                        out.push('\'');
                    }
                    Arg::Char(c) => out.push_str(&u32::from(*c).to_string()),
                }
            }
            out.push_str(");\t// ");
            out.push_str(label(step.code));
            out.push_str("\r\n");
        }
        out
    }

    /// Reads the text of a file.
    ///
    /// Sakura: `CKeyMacroMgr::LoadKeyMacro`. A line is blank, a `//` comment, or
    /// `Name(arg, ...)`, where `Name` may start with `S_`, and an argument is a number or a text
    /// in `'` or `"`. What follows the closing `)` is ignored.
    pub fn from_text(text: &str) -> Result<Self, ParseError> {
        let mut steps = Vec::new();
        for (index, line) in text.trim_start_matches('\u{feff}').lines().enumerate() {
            if let Some(step) = parse_line(line).map_err(|(column, kind)| ParseError {
                line: index + 1,
                column,
                kind,
            })? {
                steps.push(step);
            }
        }
        Ok(Self { steps })
    }

    /// The bytes of the file: UTF-8 with a BOM.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(self.to_text().as_bytes());
        bytes
    }

    /// Reads the bytes of a file (UTF-8, with or without a BOM).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, LoadError> {
        let text = std::str::from_utf8(bytes).map_err(|_| LoadError::NotUtf8)?;
        Ok(Self::from_text(text)?)
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        fs::write(path, self.to_bytes())
    }

    pub fn load(path: &Path) -> Result<Self, LoadError> {
        Self::from_bytes(&fs::read(path)?)
    }
}

/// The Japanese label that follows a saved call as a comment (`LS(nFuncID)` in `CMacro::Save`).
fn label(code: u16) -> &'static str {
    FUNCTIONS
        .iter()
        .find(|f| f.code == code)
        .and_then(|f| f.label)
        .unwrap_or("")
}

/// Writes `text` the way `CMacro::Save` does: `\`, `'`, CR, LF and tab get a backslash, and
/// the other control characters (and DEL, NEL, LS and PS) become `\uXXXX`.
fn escape_into(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\0'..='\u{1f}' | '\u{7f}' | '\u{85}' | '\u{2028}' | '\u{2029}' => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
}

// ---- reading ----

/// What is wrong with a line of a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// No function has this name (`STR_ERR_DLGKEYMACMGR8`).
    UnknownFunction(String),
    /// More arguments than the function takes (`STR_ERR_DLGKEYMACMGR3`).
    TooManyArguments,
    /// A text where the function takes a number, or the other way round
    /// (`STR_ERR_DLGKEYMACMGR4` and `6`). `position` counts from 1.
    WrongArgumentType { name: String, position: usize },
    /// A text has no closing quote (`STR_ERR_DLGKEYMACMGR5`).
    UnterminatedText { name: String, position: usize },
    /// Something that is not an argument (`STR_ERR_DLGKEYMACMGR7`).
    Syntax,
    /// A number that does not fit in 32 bits.
    NumberOutOfRange,
}

/// A mistake in a file, with where it is. Both numbers count from 1; `column` is `None` where
/// Sakura names only the line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub column: Option<usize>,
    pub kind: ParseErrorKind,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}", self.line)?;
        if let Some(column) = self.column {
            write!(f, ", column {column}")?;
        }
        match &self.kind {
            ParseErrorKind::UnknownFunction(name) => write!(f, ": {name:?} is not a function"),
            ParseErrorKind::TooManyArguments => write!(f, ": too many arguments"),
            ParseErrorKind::WrongArgumentType { name, position } => {
                write!(f, ": argument {position} of {name} has the wrong type")
            }
            ParseErrorKind::UnterminatedText { name, position } => {
                write!(f, ": argument {position} of {name} has no closing quote")
            }
            ParseErrorKind::Syntax => write!(f, ": syntax error"),
            ParseErrorKind::NumberOutOfRange => write!(f, ": the number is out of range"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Why a file could not be loaded.
#[derive(Debug)]
pub enum LoadError {
    Io(io::Error),
    NotUtf8,
    Parse(ParseError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::NotUtf8 => write!(f, "the file is not UTF-8"),
            Self::Parse(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Parse(e) => Some(e),
            Self::NotUtf8 => None,
        }
    }
}

impl From<io::Error> for LoadError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<ParseError> for LoadError {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}

type LineError = (Option<usize>, ParseErrorKind);

fn is_blank(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// Reads one line. `Ok(None)` for a blank line or a comment. Columns count from 1.
///
/// This follows `CKeyMacroMgr::LoadKeyMacro` step by step, including how it skips to the next
/// `,` or `)` after an argument.
fn parse_line(line: &str) -> Result<Option<Step>, LineError> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = chars
        .iter()
        .position(|&c| !is_blank(c))
        .unwrap_or(chars.len());
    if i == chars.len() || (chars[i] == '/' && chars.get(i + 1) == Some(&'/')) {
        return Ok(None);
    }
    // The name ends at the first `(`.
    // A line without one has no function name (Sakura looks for an empty name).
    let name_end = chars[i..].iter().position(|&c| c == '(').map(|p| i + p);
    let name: String = chars[i..name_end.unwrap_or(chars.len())].iter().collect();
    let row = name_end
        .and_then(|_| macro_names::by_name(&name))
        .ok_or_else(|| (None, ParseErrorKind::UnknownFunction(name.clone())))?;
    i = name_end.map_or(chars.len(), |p| p + 1);

    // `CKeyMacroMgr` takes four arguments, or all of the extra ones of Grep and GrepReplace.
    let max = row.args.len().max(4);
    let mut args: Vec<Arg> = Vec::new();
    while i < chars.len() {
        if args.len() >= max {
            return Err((Some(i + 1), ParseErrorKind::TooManyArguments));
        }
        let position = args.len() + 1;
        let expected = row.args.get(args.len()).copied();
        while i < chars.len() && is_blank(chars[i]) {
            i += 1;
        }
        match chars.get(i).copied() {
            Some(quote @ ('\'' | '"')) => {
                // A function takes up to four arguments in the file even if it declares fewer
                // (the extra ones are accepted of any kind).
                if expected == Some(ArgType::Int) {
                    return Err((
                        Some(i + 1),
                        ParseErrorKind::WrongArgumentType {
                            name: row.name.to_owned(),
                            position,
                        },
                    ));
                }
                let start = i + 1;
                let mut j = start;
                loop {
                    match chars.get(j) {
                        None => {
                            return Err((
                                None,
                                ParseErrorKind::UnterminatedText {
                                    name: row.name.to_owned(),
                                    position,
                                },
                            ));
                        }
                        Some('\\') => j += 2,
                        Some(&c) if c == quote => break,
                        Some(_) => j += 1,
                    }
                }
                args.push(Arg::Text(unescape(&chars[start..j])));
                i = j;
            }
            Some(c) if c.is_ascii_digit() || c == '-' => {
                if expected == Some(ArgType::Str) {
                    return Err((
                        Some(i + 1),
                        ParseErrorKind::WrongArgumentType {
                            name: row.name.to_owned(),
                            position,
                        },
                    ));
                }
                let start = i;
                i += 1;
                while chars.get(i).is_some_and(char::is_ascii_digit) {
                    i += 1;
                }
                let digits: String = chars[start..i].iter().collect();
                let number = digits
                    .parse::<i32>()
                    .map_err(|_| (Some(start + 1), number_error(&digits)))?;
                args.push(Arg::Int(i64::from(number)));
                // `i` is now after the last digit; Sakura points at the last digit. The scan
                // below starts at the same place in both.
                i -= 1;
            }
            Some(')') => break,
            _ => return Err((Some(i + 1), ParseErrorKind::Syntax)),
        }
        // Skip to the `,` or `)` that ends the argument.
        let Some(end) = chars[i..].iter().position(|&c| c == ')' || c == ',') else {
            break;
        };
        i += end + 1;
        if chars[i - 1] == ')' {
            break;
        }
    }
    Ok(Some(Step {
        code: row.code,
        args,
    }))
}

/// `-` alone is not a number; anything else that fails to parse is too big.
fn number_error(digits: &str) -> ParseErrorKind {
    if digits == "-" {
        ParseErrorKind::Syntax
    } else {
        ParseErrorKind::NumberOutOfRange
    }
}

/// Undoes the escapes of `CMacro::Save` (and of `CKeyMacroMgr::LoadKeyMacro`): `\\`, `\'`,
/// `\"`, `\r`, `\n`, `\t` and `\uXXXX` (1 to 4 hex digits; none drops the `\u`). Any other
/// backslash stays. `\u` units are UTF-16, so a surrogate pair gives one character.
fn unescape(chars: &[char]) -> String {
    let mut units: Vec<u16> = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c != '\\' {
            let mut buf = [0u16; 2];
            units.extend_from_slice(c.encode_utf16(&mut buf));
            i += 1;
            continue;
        }
        match chars.get(i + 1) {
            Some('\\') => units.push(u16::from(b'\\')),
            Some('\'') => units.push(u16::from(b'\'')),
            Some('"') => units.push(u16::from(b'"')),
            Some('r') => units.push(13),
            Some('n') => units.push(10),
            Some('t') => units.push(9),
            Some('u') => {
                let digits: String = chars[i + 2..]
                    .iter()
                    .take(4)
                    .take_while(|c| c.is_ascii_hexdigit())
                    .collect();
                if let Ok(unit) = u16::from_str_radix(&digits, 16) {
                    units.push(unit);
                }
                i += 2 + digits.len();
                continue;
            }
            _ => {
                units.push(u16::from(b'\\'));
                i += 1;
                continue;
            }
        }
        i += 2;
    }
    String::from_utf16_lossy(&units)
}

// ---- playing ----

/// A call failed while a key macro was played.
#[derive(Debug)]
pub struct PlayError {
    /// The index of the call in [`KeyMacro::steps`].
    pub index: usize,
    pub error: CommandError,
}

impl fmt::Display for PlayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "step {}: {}", self.index + 1, self.error)
    }
}

impl std::error::Error for PlayError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

// ---- recording ----

/// Keeps the commands that run while a key macro is recorded.
///
/// Sakura: `CSMacroMgr::Append` is called by `CEditView::HandleCommand` for each command while
/// recording, and `F_RECKEYMACRO` toggles it (`CEditView::Command_RECKEYMACRO` clears the
/// buffer when it starts). See the module documentation for how the editor uses it.
#[derive(Debug, Default)]
pub struct KeyMacroRecorder {
    recording: bool,
    steps: KeyMacro,
}

impl KeyMacroRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// Starts recording. What was recorded before is dropped.
    pub fn start(&mut self) {
        self.recording = true;
        self.steps = KeyMacro::new();
    }

    /// Stops recording and returns what was recorded (empty if it was not recording).
    pub fn stop(&mut self) -> KeyMacro {
        self.recording = false;
        std::mem::take(&mut self.steps)
    }

    /// Records a command that was run, if a key macro keeps it. Returns whether it did.
    ///
    /// The call is not kept when the recorder is not recording, when Sakura's key macro does
    /// not record the function (`CSMacroMgr::CanFuncIsKeyMacro`: the file and window commands,
    /// for example), or when the arguments do not fit the function. `Arg::Char` is recorded as
    /// its code point.
    pub fn record(&mut self, code: u16, args: &[Arg]) -> bool {
        if !self.recording {
            return false;
        }
        match macro_names::by_code(code) {
            Some(row) if row.key_macro => self.steps.push(code, args).is_ok(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests;
