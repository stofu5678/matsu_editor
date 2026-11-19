//! 外部コマンド実行 (`F_EXECMD_DIALOG`, `F_EXECMD`): runs a command line, takes its output and puts
//! it in the window or in the output panel.
//!
//! Sakura: `CViewCommander::Command_EXECCOMMAND_DIALOG` and `Command_EXECCOMMAND`
//! (`sakura_core/cmd/CViewCommander_Macro.cpp`), `CDlgExec` (`sakura_core/dlg/CDlgExec.cpp`) and
//! `CEditView::ExecCmd` (`sakura_core/view/CEditView_ExecCmd.cpp`).
//!
//! * [`dialog`]: the dialog (`IDD_EXEC`) with the histories of the commands and the folders.
//! * [`flow`]: what `ExecCmd` does: expanding `$F`, starting the process, sending the selection
//!   or the document to its standard input, reading what it writes while the window stays
//!   usable, and putting that into the document (one undo step, the inserted text selected when
//!   something was selected) or into the output panel of macros.
//! * [`running`]: the box with 中断 that stands for Sakura's 「コマンド実行中・・・」 dialog.
//! * [`ExecPrefs`]: what Sakura keeps in the shared data (the commands and folders that were run,
//!   the choices of the dialog), saved to `[Cmd]` and `[Other]` of the settings file.
//!
//! The process is `matsu_platform::exec`; the rules without a window are `matsu_core::exec`.

use std::path::{Path, PathBuf};

use gpui::{App, Global};
use matsu_config::Profile;
use matsu_config::history::update_profile;
use matsu_config::settings::{EditableList as _, ExecHistory};
use matsu_core::exec::ExecFlags;

pub(crate) mod dialog;
pub(crate) mod flow;
pub(crate) mod running;
#[cfg(test)]
mod tests;

/// The commands and folders that were run and the choices of the dialog, shared by the windows of
/// the application (`m_sHistory.m_aCommands`, `m_aCurDirs`, `m_nExecFlgOpt`).
#[derive(Default)]
pub struct ExecPrefs {
    history: ExecHistory,
    /// The settings file to save to; nothing is saved without one (the tests).
    store: Option<PathBuf>,
}

impl Global for ExecPrefs {}

impl ExecPrefs {
    /// What the file the application read has, to be saved to `store`.
    pub fn load(settings: Option<&Path>, store: Option<PathBuf>) -> Self {
        let history = settings
            .and_then(|path| Profile::load(path).ok())
            .map(|profile| ExecHistory::from_profile(&profile))
            .unwrap_or_default();
        Self { history, store }
    }

    /// The choices the dialog starts with (`m_nExecFlgOpt`).
    pub fn flags(&self) -> ExecFlags {
        self.history.flags.map_or(ExecFlags::DEFAULT, ExecFlags)
    }

    /// Changes the history and saves it, under the lock that every writer of the file takes
    /// (`matsu_config::history::update_profile`): the file is read again and merged first, so what
    /// another window saved at the same moment is not lost. A history that cannot be saved is
    /// kept in memory and is not worth stopping a command for.
    fn change(&mut self, change: impl Fn(&mut ExecHistory)) {
        change(&mut self.history);
        let Some(path) = self.store.clone() else {
            return;
        };
        let local = self.history.clone();
        let mut merged = None;
        let saved = update_profile(&path, |profile| {
            let mut history = local.clone();
            history.merge_from(&ExecHistory::from_profile(profile));
            change(&mut history);
            // nothing is written when the file already has these
            let before = profile.to_bytes();
            let written = history.write_to(profile).is_ok() && profile.to_bytes() != before;
            merged = Some(history);
            Ok(written)
        });
        if saved.is_ok()
            && let Some(history) = merged
        {
            self.history = history;
        }
    }
}

impl ExecPrefs {
    /// Changes the lists by hand (the history management dialog), here and in the file. `change`
    /// is applied to our lists (so strings only we have stay, and can be edited and deleted) and,
    /// under the lock, to the lists the file has now, which are written. Nothing is merged, so a
    /// deleted string does not come back from either side.
    fn edit(&mut self, removes: bool, change: impl Fn(&mut ExecHistory)) {
        let before = self.history.clone();
        change(&mut self.history);
        // The file takes the change only when our own lists did (see `SearchHistory::edit`).
        if self.history == before {
            return;
        }
        let Some(path) = self.store.clone() else {
            return;
        };
        let _ = update_profile(&path, |profile| {
            let mut history = ExecHistory::from_profile(profile);
            change(&mut history);
            // A string only we had, edited into one the file can hold: the file did not have the
            // original, so the edit cannot be replayed; the result goes in front.
            for (was, now, saved) in [
                (
                    &before.commands,
                    &self.history.commands,
                    &mut history.commands,
                ),
                (
                    &before.cur_dirs,
                    &self.history.cur_dirs,
                    &mut history.cur_dirs,
                ),
            ] {
                for text in now.items().iter().rev() {
                    if !was.items().contains(text) && !saved.items().contains(text) {
                        saved.add(text);
                    }
                }
                // What the user removed here goes from the file too, even when the replay could
                // not do it (an edit into a string the file has already).
                for text in was.items().iter().filter(|_| removes) {
                    if !now.items().contains(text)
                        && let Some(at) = saved.items().iter().position(|t| t == text)
                    {
                        saved.delete_at(at);
                    }
                }
            }
            let before = profile.to_bytes();
            let written = history.write_to(profile).is_ok() && profile.to_bytes() != before;
            Ok(written)
        });
    }
}

/// Changes the commands and folders by hand ([`ExecPrefs::edit`]).
#[cfg(test)]
pub(crate) fn edit_history(cx: &mut App, change: impl Fn(&mut ExecHistory)) {
    edit_history_with(cx, true, change);
}

/// `edit_history`; `removes` is false for an addition, whose pushing out of a full list is the
/// capacity rule of each list, not a removal by the user.
pub(crate) fn edit_history_with(cx: &mut App, removes: bool, change: impl Fn(&mut ExecHistory)) {
    cx.default_global::<ExecPrefs>().edit(removes, change);
}

/// Makes the history of the application. Called once at startup.
pub fn install(cx: &mut App, prefs: ExecPrefs) {
    cx.set_global(prefs);
}

/// The history as it is now.
pub(crate) fn history(cx: &App) -> ExecHistory {
    cx.try_global::<ExecPrefs>()
        .map(|p| p.history.clone())
        .unwrap_or_default()
}

/// The choices the dialog starts with.
pub(crate) fn flags(cx: &App) -> ExecFlags {
    cx.try_global::<ExecPrefs>()
        .map_or(ExecFlags::DEFAULT, ExecPrefs::flags)
}

/// What the dialog's 実行 keeps (`GetData` for the choices, `AddToCmdArr` and
/// `CRecentCurDir::AppendItem` for the lists).
pub(crate) fn remember(cx: &mut App, command: &str, cur_dir: &str, flags: ExecFlags) {
    let command = command.to_owned();
    let cur_dir = cur_dir.to_owned();
    cx.default_global::<ExecPrefs>().change(move |history| {
        history.flags = Some(flags.0);
        history.add_command(&command);
        history.add_cur_dir(&cur_dir);
    });
}
