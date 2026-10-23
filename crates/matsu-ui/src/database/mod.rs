//! The database functions (ADR-0023): `F_PLSQL_COMPILE_ON_SQLPLUS` (SQL*Plusで実行, F10) and
//! `F_ACTIVATE_SQLPLUS` (SQL*Plusをアクティブ表示).
//!
//! Sakura finds a running Windows SQL*Plus window and pastes `@file` into it
//! (`Command_PLSQL_COMPILE_ON_SQLPLUS`, `sakura_core/cmd/CViewCommander_File.cpp`). This program
//! runs the command line client of the connection in `databases.ini`
//! ([`matsu_config::databases`]) through 外部コマンド実行 (`crate::exec`) and shows its output in the
//! output panel.
//!
//! * [`command`]: the template of the connection becomes a command line, with the file quoted for
//!   the shell of the OS.
//! * [`run`]: the flow: the connection, the question about saving, the run.
//!
//! The connections are read from the file at every run, so a change of the file is used at once.

use std::path::PathBuf;

use gpui::{App, Global};

pub(crate) mod command;
pub(crate) mod run;
#[cfg(test)]
mod tests;

/// Where `databases.ini` is. `None` when this program has no place for settings (the tests that
/// do not need one): the functions then say there is no connection.
#[derive(Default)]
pub struct DatabasePrefs {
    path: Option<PathBuf>,
}

impl Global for DatabasePrefs {}

impl DatabasePrefs {
    /// The file beside the settings file `settings`.
    pub fn beside(settings: Option<PathBuf>) -> Self {
        Self {
            path: settings.map(|p| matsu_config::databases::path_beside(&p)),
        }
    }

    /// The file itself.
    pub fn at(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }
}

/// Makes the place of the file known. Called once at startup.
pub fn install(cx: &mut App, prefs: DatabasePrefs) {
    cx.set_global(prefs);
}

/// The file of the connections.
pub(crate) fn file_path(cx: &App) -> Option<PathBuf> {
    cx.try_global::<DatabasePrefs>()
        .and_then(|p| p.path.clone())
}
