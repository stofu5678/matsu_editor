//! What the input dialog changes in `[OneLiner]` of the settings file (ADR-0029): the history of the lines that were
//! run, and a line registered in the first free slot. The settings page changes the registered ones otherwise.
//!
//! The file is changed under the lock that every writer of it takes (`crate::history::write_settings`): the section
//! is read again from the file and changed there, so what another window saved at the same moment is not lost. The
//! settings the windows hold become what the file has then (or, without a file on disk, what the change makes of
//! them, for this session).

use std::fmt;

use gpui::{App, BorrowAppContext as _};
use matsu_config::settings::oneliner::{
    MAX_ONELINER, OneLiner, OneLinerError, OneLiners, OutputTo,
};

use crate::settings::UserSettings;

/// Why a line was not registered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RegisterError {
    /// All 50 slots have a command.
    Full,
    /// The command is empty.
    EmptyCommand,
    /// The name or the command cannot be kept (a line break, too long).
    Invalid(OneLinerError),
}

impl fmt::Display for RegisterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => write!(
                f,
                "登録できるワンライナー ({MAX_ONELINER} 個) がすべて使われています。設定で空きを作ってください。"
            ),
            Self::EmptyCommand => f.write_str("ワンライナーが空です。"),
            Self::Invalid(OneLinerError::LineBreak { .. }) => {
                f.write_str("名前とワンライナーは 1 行で書いてください。")
            }
            Self::Invalid(OneLinerError::TooLong { key }) => {
                if key.starts_with("Name") {
                    f.write_str("名前が長すぎます。")
                } else {
                    f.write_str("ワンライナーが長すぎます。")
                }
            }
            Self::Invalid(OneLinerError::Profile(e)) => write!(f, "登録できません: {e}"),
        }
    }
}

/// Changes the table with `change` and keeps it. `change` returns whether it changed anything; it runs on the
/// table the file has (so it can run twice: on the file's and, without a file, on the windows'), and the table the
/// windows get is the one it last made. Returns that table, `None` when nothing changed.
fn update(
    cx: &mut App,
    mut change: impl FnMut(&mut OneLiners) -> Result<bool, OneLinerError>,
) -> Result<Option<OneLiners>, OneLinerError> {
    let mut made: Option<OneLiners> = None;
    let mut problem: Option<OneLinerError> = None;
    crate::history::write_settings(cx, |profile| {
        let mut table = OneLiners::from_profile(profile);
        match change(&mut table) {
            Ok(true) => {}
            Ok(false) => return Ok(false),
            Err(e) => {
                problem = Some(e);
                return Ok(false);
            }
        }
        match table.store(profile) {
            Ok(changed) => {
                made = Some(table);
                Ok(changed)
            }
            Err(OneLinerError::Profile(e)) => Err(e),
            Err(e) => {
                problem = Some(e);
                Ok(false)
            }
        }
    });
    if let Some(e) = problem {
        return Err(e);
    }
    let table = match made {
        Some(table) => table,
        // no file to write: the change is for this session
        None => {
            let mut table = super::table(cx);
            if !change(&mut table)? {
                return Ok(None);
            }
            table
        }
    };
    if cx.try_global::<UserSettings>().is_none() {
        cx.set_global(UserSettings::default());
    }
    cx.update_global::<UserSettings, _>(|user, _| user.set_oneliners(table.clone()));
    Ok(Some(table))
}

/// Puts `line` first in the history of the input dialog (running it does that). A line that cannot be kept (too
/// long) is not.
pub(crate) fn remember(cx: &mut App, line: &str) {
    let line = line.to_owned();
    // a history that cannot be written is kept in memory and is not worth stopping a run for
    let _ = update(cx, move |table| Ok(table.add_recent(&line)));
}

/// 「登録...」: registers `command` under `name` in the first slot that has no command, for the result `output`
/// asks for. Returns the number of the slot. The menus are made again, since they list the registered ones.
pub(crate) fn register(
    cx: &mut App,
    name: &str,
    command: &str,
    output: OutputTo,
) -> Result<usize, RegisterError> {
    let (name, command) = (name.trim().to_owned(), command.trim().to_owned());
    if command.is_empty() {
        return Err(RegisterError::EmptyCommand);
    }
    let mut slot = None;
    let result = update(cx, |table| {
        let Some(index) = table.slots.iter().position(|one| !one.is_registered()) else {
            slot = None;
            return Ok(false);
        };
        let before = table.slots[index].clone();
        table.slots[index] = OneLiner {
            name: name.clone(),
            command: command.clone(),
            output,
        };
        slot = Some(index);
        // checks the values before anything is written; a table with a value that cannot be kept is not used
        let checked = OneLiners {
            slots: vec![table.slots[index].clone()],
            recent: Vec::new(),
        }
        .store(&mut matsu_config::Profile::new());
        if let Err(e) = checked {
            table.slots[index] = before;
            return Err(e);
        }
        Ok(true)
    });
    match result {
        Ok(Some(_)) => match slot {
            Some(index) => {
                crate::app::refresh_menus(cx);
                Ok(index)
            }
            None => Err(RegisterError::Full),
        },
        Ok(None) => Err(RegisterError::Full),
        Err(e) => Err(RegisterError::Invalid(e)),
    }
}
