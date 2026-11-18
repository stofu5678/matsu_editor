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
    // `CDlgPluginOption::SetData`: the ReadMe button is for a plugin that has a file
    let readme = dirs.readme_file(folder);
    dialog::open(cx, &def, &dirs.option_file(folder), readme);
    true
}

/// A new window that reads `file` in view mode (`-R`), the encoding detected (`-CODE=99`): what
/// `CPropPlugin::BrowseReadMe` starts. The window is opened inside this application, so that the
/// profile in use (`-PROF=`) is the same and the tab setting does not turn it into a tab.
pub(crate) fn open_readme_window(cx: &mut App, file: &std::path::Path) -> gpui::Result<()> {
    use crate::document::SharedDocument;
    use crate::file_ops::{view_mode_request, windows};
    view_mode_request::request(cx, file);
    let load = windows::LoadRequest {
        path: file.to_owned(),
        encoding: None,
    };
    match windows::open_editor_window(cx, SharedDocument::empty(), Some(load)) {
        Ok(handle) => {
            windows::activate(cx, handle);
            Ok(())
        }
        Err(error) => {
            view_mode_request::take(cx, file);
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests;
