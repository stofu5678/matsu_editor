//! The settings of the shell tab (ADR-0028, decision 7): the section `[Terminal]` of `matsu.ini`.
//!
//! Sakura has no terminal, so the section is MATSU's own (ADR-0024, decision 7). The file is in the format of
//! [`Profile`], so lines this program does not know are kept when it is written back.
//!
//! ```text
//! [Terminal]
//! Shell=cmd.exe /K title "my shell"
//! ScrollbackLines=5000
//! OptionAsMeta=0
//! ```
//!
//! * `Shell` is one command line: the program, then its arguments, as they would be typed. Empty (or absent) is
//!   the default shell. This crate does not parse it (`matsu_terminal::ShellCommand::from_command_line` does).
//!   It is one string on every OS so that the Windows command line reaches the program as it is written
//!   (cmd.exe reads its command line by its own rules).
//! * `ScrollbackLines` is how many lines a tab keeps above the screen, `0` to [`MAX_SCROLLBACK_LINES`].
//!   Absent or not a number (or negative) is [`DEFAULT_SCROLLBACK_LINES`]; a number above the limit is the
//!   limit.
//! * `OptionAsMeta` is read on every OS but acts on macOS only: Option sends ESC and the key instead of typing
//!   the character it gives. Absent or not a number is off.
//!
//! The settings page writes the keys (the dialog's own writer, which keeps every other line of the file).
//!
//! A value that cannot be used never makes the file or the tab fail: it is the default, as in the other
//! sections of MATSU's own.

use crate::profile::Profile;

/// The section of the settings file.
pub const SECTION: &str = "Terminal";
/// The key of the shell's command line.
pub const KEY_SHELL: &str = "Shell";
/// The key of the scrollback length.
pub const KEY_SCROLLBACK_LINES: &str = "ScrollbackLines";
/// The key of "Option as Meta".
pub const KEY_OPTION_AS_META: &str = "OptionAsMeta";

/// The scrollback length when the file has none (ADR-0028, decision 7: 5000 lines, about 14 MB at 120 columns).
pub const DEFAULT_SCROLLBACK_LINES: usize = 5000;
/// The longest scrollback. A cell of the screen takes about 24 bytes, so a tab 120 columns wide takes about
/// 144 MB of memory at this length, per tab: more than that is not what a shell tab is for (the output of a
/// build or of an AI tool is read once, then found in the tool's own log). `alacritty_terminal` itself stops at
/// 100000.
pub const MAX_SCROLLBACK_LINES: usize = 50_000;

/// What `[Terminal]` says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalSettings {
    /// `Shell`: the command line of the shell; empty is the default shell.
    pub shell: String,
    /// `ScrollbackLines`.
    pub scrollback_lines: usize,
    /// `OptionAsMeta`.
    pub option_as_meta: bool,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            shell: String::new(),
            scrollback_lines: DEFAULT_SCROLLBACK_LINES,
            option_as_meta: false,
        }
    }
}

impl TerminalSettings {
    /// The settings as the values of the keys are: a number that is above the limit is the limit, one that is
    /// negative is the default. (The settings page gives the numbers it has.)
    pub fn from_values(shell: &str, scrollback_lines: i64, option_as_meta: bool) -> Self {
        Self {
            shell: shell.trim().to_owned(),
            scrollback_lines: match usize::try_from(scrollback_lines) {
                Ok(lines) => lines.min(MAX_SCROLLBACK_LINES),
                Err(_) => DEFAULT_SCROLLBACK_LINES,
            },
            option_as_meta,
        }
    }

    /// Reads the section; what is missing or cannot be used is the default.
    pub fn from_profile(profile: &Profile) -> Self {
        let default = Self::default();
        Self::from_values(
            profile.get(SECTION, KEY_SHELL).unwrap_or(""),
            profile
                .get_i32(SECTION, KEY_SCROLLBACK_LINES)
                .map_or(DEFAULT_SCROLLBACK_LINES as i64, i64::from),
            profile
                .get_bool(SECTION, KEY_OPTION_AS_META)
                .unwrap_or(default.option_as_meta),
        )
    }
}

#[cfg(test)]
mod tests;
