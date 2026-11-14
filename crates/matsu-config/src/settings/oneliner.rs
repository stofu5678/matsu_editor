//! The registered one-liners (ADR-0029, decision 4): the section `[OneLiner]` of `matsu.ini`.
//!
//! A one-liner is a name and one shell command line. It processes the selection (or the whole document) and
//! its result goes where `Output` says. Sakura has nothing like it, so the section is MATSU's own
//! (ADR-0024, decision 7), numbered like the registered macros (`[Macro]`, [`super::macros`]) so that keys,
//! menus and the toolbar can call one-liner `n` as the function `F_ONELINER_0 + n`.
//!
//! ```text
//! [OneLiner]
//! Name[000]=重複を除いて並べ替え
//! Command[000]=sort -u
//! Output[000]=0
//! AI[001]=1
//! AskBeforeSend[001]=0
//! Recent[00]=tr a-z A-Z
//! ```
//!
//! * `Name[nnn]`, `Command[nnn]`, `Output[nnn]` for `nnn` in `000..=049` ([`MAX_ONELINER`]). A slot whose
//!   `Command` is empty, only spaces or absent is not registered: it reads as an empty slot, and writing it removes its keys.
//! * `Output`: 0 replace, 1 next line, 2 output window ([`OutputTo`]). Absent, not a number or any other number
//!   reads as 0.
//! * `AI[nnn]` (ADR-0030, decision 7): 1 marks a one-liner that sends the text to a generative AI (it gets a
//!   confirmation window and a log tab). 0 is the default; absent, not a number or any other number reads as 0.
//! * `AskBeforeSend[nnn]`: 0 sends an AI one-liner without the confirmation window ("次から聞かない"). 1 is the
//!   default; absent, not a number or any other number reads as 1. Both keys are written only when they differ
//!   from the default (or from what the file has), so a file that never used them does not get them.
//! * `Recent[00]`..`Recent[19]` ([`MAX_ONELINER_RECENT`]): the history of the input dialog, most recent first.
//! * Keys of the section that this program does not know stay in the file when it is written back
//!   ([`Profile`] keeps every line it does not set).
//!
//! # One line, and the limits
//!
//! A command is one line: the file is read line by line, so a value with a line break cannot be stored.
//! * The writer ([`OneLiners::store`]) refuses it with [`OneLinerError::LineBreak`] before it changes anything.
//! * The reader never sees one. A hand-edited file whose command was broken over two lines has the first line
//!   as the command (and the rest is a line that is not an entry, which [`Profile`] keeps and ignores).
//!
//! A name has at most 63 UTF-16 units ([`NAME_MAX_UNITS`], the same as `MACRONAME_MAX` of the macros: it is a
//! menu and toolbar label). A command is shorter than 1024 UTF-16 units ([`COMMAND_MAX_UNITS`], `MAX_CMDLEN`
//! of 外部コマンド実行, the same kind of text, see [`super::exec_history`]).
//! * The writer refuses longer ones with [`OneLinerError::TooLong`].
//! * The reader cuts a long name to the limit (a label is harmless cut), but never cuts a command: running
//!   half of a command line could do something else, so a slot with a too long command is not registered.
//!   A history line that is too long is dropped.

use std::fmt;

use crate::profile::{Profile, ProfileError};

/// The section of the file.
pub const SECTION: &str = "OneLiner";
/// How many one-liners can be registered.
pub const MAX_ONELINER: usize = 50;
/// How many lines of the input dialog's history are kept.
pub const MAX_ONELINER_RECENT: usize = 20;
/// A name has fewer UTF-16 units than this (`MACRONAME_MAX` of the macros is 64 with the end of the string).
pub const NAME_MAX_UNITS: usize = 64;
/// A command (and a history line) has fewer UTF-16 units than this (`MAX_CMDLEN`).
pub const COMMAND_MAX_UNITS: usize = 1024;

/// `F_ONELINER_0`: the function of one-liner 0 (MATSU-only, `xtask/src/matsu_extras.rs`). The functions of
/// the 50 are in a row.
pub const F_ONELINER_0: u16 = 32711;
/// `F_ONELINER_LIST`: in a menu, the registered one-liners.
pub const F_ONELINER_LIST: u16 = 32761;
/// `F_ONELINER_DIALOG`: 「ワンライナーを入力して実行」.
pub const F_ONELINER_DIALOG: u16 = 32762;
/// `F_ONELINER_SETTINGS`: 「ワンライナーの設定」.
pub const F_ONELINER_SETTINGS: u16 = 32763;
/// `F_AI_LOG_SHOW`: 「AI ログを表示」, the log of what was sent to a generative AI (ADR-0030). It is the last item of
/// what `F_ONELINER_LIST` expands to.
pub const F_AI_LOG_SHOW: u16 = 32764;

/// The one-liner that the function `code` stands for (`F_ONELINER_0 <= code < F_ONELINER_0 + MAX_ONELINER`).
pub fn index_of_function(code: u16) -> Option<usize> {
    let n = usize::from(code.checked_sub(F_ONELINER_0)?);
    (n < MAX_ONELINER).then_some(n)
}

/// The function that runs one-liner `index`.
pub fn function_of_index(index: usize) -> Option<u16> {
    let n = u16::try_from(index).ok().filter(|_| index < MAX_ONELINER)?;
    Some(F_ONELINER_0 + n)
}

/// Where the result of a one-liner goes (`Output`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputTo {
    /// 0: replaces the selection (or the whole document).
    #[default]
    Replace,
    /// 1: goes into the lines after the input.
    NextLine,
    /// 2: goes to the output window; the document stays.
    OutputWindow,
}

impl OutputTo {
    /// The value of `Output`; any other number is [`OutputTo::Replace`].
    pub fn from_number(n: i32) -> Self {
        match n {
            1 => Self::NextLine,
            2 => Self::OutputWindow,
            _ => Self::Replace,
        }
    }

    /// The value written to `Output`.
    pub fn number(self) -> i32 {
        match self {
            Self::Replace => 0,
            Self::NextLine => 1,
            Self::OutputWindow => 2,
        }
    }
}

/// One slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OneLiner {
    /// The name shown in menus and on the toolbar.
    pub name: String,
    /// The shell command line: one line.
    pub command: String,
    pub output: OutputTo,
    /// `AI`: the text goes to a generative AI (ADR-0030).
    pub ai: bool,
    /// `AskBeforeSend`: an AI one-liner shows the confirmation window first. Only meant with [`ai`](Self::ai).
    pub ask_before_send: bool,
}

impl Default for OneLiner {
    fn default() -> Self {
        Self {
            name: String::new(),
            command: String::new(),
            output: OutputTo::default(),
            ai: false,
            ask_before_send: true,
        }
    }
}

impl OneLiner {
    /// A one-liner with a command can run.
    pub fn is_registered(&self) -> bool {
        !self.command.trim().is_empty()
    }

    /// The name, or the command when there is no name.
    pub fn title(&self) -> &str {
        if self.name.is_empty() {
            &self.command
        } else {
            &self.name
        }
    }
}

/// Why [`OneLiners::store`] did not write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OneLinerError {
    /// The value of `key` has a line break.
    LineBreak { key: String },
    /// The value of `key` is longer than its limit ([`NAME_MAX_UNITS`] or [`COMMAND_MAX_UNITS`]).
    TooLong { key: String },
    /// The file refused the key or the value.
    Profile(ProfileError),
}

impl fmt::Display for OneLinerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LineBreak { key } => write!(f, "{key} has a line break"),
            Self::TooLong { key } => write!(f, "{key} is too long"),
            Self::Profile(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for OneLinerError {}

impl From<ProfileError> for OneLinerError {
    fn from(e: ProfileError) -> Self {
        Self::Profile(e)
    }
}

/// The 50 slots and the history of the input dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OneLiners {
    pub slots: Vec<OneLiner>,
    /// The history, most recent first, at most [`MAX_ONELINER_RECENT`], none twice, none empty.
    pub recent: Vec<String>,
}

impl Default for OneLiners {
    fn default() -> Self {
        Self {
            slots: vec![OneLiner::default(); MAX_ONELINER],
            recent: Vec::new(),
        }
    }
}

/// `Name[003]`: the key of slot `index`.
fn key(prefix: &str, index: usize) -> String {
    format!("{prefix}[{index:03}]")
}

/// `Recent[03]`.
fn recent_key(index: usize) -> String {
    format!("Recent[{index:02}]")
}

fn units(text: &str) -> usize {
    text.encode_utf16().count()
}

fn has_break(text: &str) -> bool {
    text.contains(['\r', '\n'])
}

/// `text` cut to fewer than [`NAME_MAX_UNITS`] UTF-16 units, at a character.
fn cut_name(text: &str) -> String {
    let mut out = String::new();
    let mut used = 0;
    for c in text.chars() {
        used += c.len_utf16();
        if used >= NAME_MAX_UNITS {
            break;
        }
        out.push(c);
    }
    out
}

fn fits_recent(line: &str) -> bool {
    !line.is_empty() && !has_break(line) && units(line) < COMMAND_MAX_UNITS
}

/// A 0/1 key; anything else (absent, not a number, another number) is `default`.
fn flag(profile: &Profile, name: &str, default: bool) -> bool {
    match profile.get_i32(SECTION, name) {
        Some(0) => false,
        Some(1) => true,
        _ => default,
    }
}

fn check(key: &str, value: &str, limit: usize) -> Result<(), OneLinerError> {
    if has_break(value) {
        return Err(OneLinerError::LineBreak {
            key: key.to_owned(),
        });
    }
    if units(value) >= limit {
        return Err(OneLinerError::TooLong {
            key: key.to_owned(),
        });
    }
    Ok(())
}

impl OneLiners {
    /// Reads the section; what is missing or cannot be used is an empty slot (see the module).
    pub fn from_profile(profile: &Profile) -> Self {
        let mut table = Self::default();
        for (i, slot) in table.slots.iter_mut().enumerate() {
            let Some(command) = profile.get(SECTION, &key("Command", i)) else {
                continue;
            };
            if command.trim().is_empty()
                || has_break(command)
                || units(command) >= COMMAND_MAX_UNITS
            {
                continue;
            }
            slot.command = command.to_owned();
            slot.name = cut_name(profile.get(SECTION, &key("Name", i)).unwrap_or("").trim());
            slot.output = OutputTo::from_number(
                profile
                    .get_i32(SECTION, &key("Output", i))
                    .unwrap_or_default(),
            );
            slot.ai = flag(profile, &key("AI", i), false);
            slot.ask_before_send = flag(profile, &key("AskBeforeSend", i), true);
        }
        // The file is in order, most recent first.
        for i in 0..MAX_ONELINER_RECENT {
            let Some(line) = profile.get(SECTION, &recent_key(i)) else {
                continue;
            };
            if fits_recent(line) && !table.recent.iter().any(|r| r == line) {
                table.recent.push(line.to_owned());
            }
        }
        table
    }

    /// The one-liner `index`, if it is in the table.
    pub fn get(&self, index: usize) -> Option<&OneLiner> {
        self.slots.get(index)
    }

    /// The one-liner that the function `code` runs.
    pub fn of_function(&self, code: u16) -> Option<&OneLiner> {
        self.get(index_of_function(code)?)
    }

    /// Puts `line` first in the history; one that is already there moves to the front, and when the list is
    /// full the oldest goes. An empty line, one with a line break, or one that is too long is not kept.
    /// Returns whether the history changed.
    pub fn add_recent(&mut self, line: &str) -> bool {
        if !fits_recent(line) {
            return false;
        }
        if let Some(i) = self.recent.iter().position(|r| r == line) {
            if i == 0 {
                return false;
            }
            self.recent.remove(i);
        }
        self.recent.insert(0, line.to_owned());
        self.recent.truncate(MAX_ONELINER_RECENT);
        true
    }

    /// Checks every value before anything is written.
    fn validate(&self) -> Result<(), OneLinerError> {
        for (i, slot) in self.slots.iter().take(MAX_ONELINER).enumerate() {
            if slot.is_registered() {
                check(&key("Name", i), &slot.name, NAME_MAX_UNITS)?;
                check(&key("Command", i), &slot.command, COMMAND_MAX_UNITS)?;
            }
        }
        for (i, line) in self.recent.iter().take(MAX_ONELINER_RECENT).enumerate() {
            check(&recent_key(i), line, COMMAND_MAX_UNITS)?;
        }
        Ok(())
    }

    /// Writes the section. A slot with no command is not written (its keys are removed), the history is
    /// written as it is and the keys after it are removed. A value with a line break or over its limit is
    /// refused before the file is touched. Returns whether the file changed; a value the file already has
    /// is left as written.
    pub fn store(&self, profile: &mut Profile) -> Result<bool, OneLinerError> {
        self.validate()?;
        let mut changed = false;
        let mut set =
            |profile: &mut Profile, name: &str, value: &str| -> Result<(), ProfileError> {
                if profile.get(SECTION, name) != Some(value) {
                    profile.set(SECTION, name, value)?;
                    changed = true;
                }
                Ok(())
            };
        let mut gone = Vec::new();
        for i in 0..MAX_ONELINER {
            match self.slots.get(i).filter(|s| s.is_registered()) {
                Some(slot) => {
                    set(profile, &key("Name", i), &slot.name)?;
                    set(profile, &key("Command", i), &slot.command)?;
                    set(
                        profile,
                        &key("Output", i),
                        &slot.output.number().to_string(),
                    )?;
                    for (name, value, default) in [
                        ("AI", slot.ai, false),
                        ("AskBeforeSend", slot.ask_before_send, true),
                    ] {
                        let name = key(name, i);
                        if flag(profile, &name, default) == value {
                            continue;
                        }
                        if value == default {
                            gone.push(name);
                        } else {
                            set(profile, &name, if value { "1" } else { "0" })?;
                        }
                    }
                }
                None => gone.extend(
                    ["Name", "Command", "Output", "AI", "AskBeforeSend"].map(|p| key(p, i)),
                ),
            }
        }
        for i in 0..MAX_ONELINER_RECENT {
            match self.recent.get(i) {
                Some(line) => set(profile, &recent_key(i), line)?,
                None => gone.push(recent_key(i)),
            }
        }
        for name in gone {
            changed |= profile.remove(SECTION, &name);
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod tests;
