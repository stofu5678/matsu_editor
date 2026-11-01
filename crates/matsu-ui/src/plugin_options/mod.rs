//! The options of a plugin: the dialog that edits the `[Option]` items of `plugin.def` and keeps
//! the values in `plugins\<folder>.ini` (`CDlgPluginOption`, ADR-0026).
//!
//! * [`model`] is what the dialog holds, and how a value becomes the text of the list and of the file.
//! * [`dialog`] is the window.
//!
//! The dialog writes the file when the user presses OK. The host of the plugin reads the file
//! at each call (`DefApi`, as `Plugin.GetOption` does in Sakura), so a running plugin sees the new
//! values at its next call; Sakura does not ask for a restart either.

pub(crate) mod dialog;
pub(crate) mod model;

use gpui::App;
use matsu_config::plugins::SearchDirs;

/// Opens the dialog for the plugin in `folder` (the button 設定 of the page 「プラグイン」). Returns
/// `false` when the plugin has no `plugin.def` to read.
pub(crate) fn open(cx: &mut App, dirs: &SearchDirs, folder: &str) -> bool {
    let Some(def) = dirs.load_def(folder, matsu_config::plugins::DEFAULT_LANGUAGE) else {
        return false;
    };
    dialog::open(cx, &def, &dirs.option_file(folder));
    true
}

#[cfg(test)]
mod tests;
