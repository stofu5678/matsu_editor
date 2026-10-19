//! The 「参照...」 button of an edit box that holds a file name: the settings of 支援 (the word file
//! of a type, the external help files).
//!
//! Sakura: `CDlgOpenFile::SelectFile` with `resolvePath` (`sakura_core/dlg/CDlgOpenFile.cpp`):
//! the dialog starts where the box says, and a file under the folder of the settings file is
//! given back relative to it (`GetRelPath`).

use std::path::{Path, PathBuf};

use crate::common_settings::CommonSettingsDialog;
use crate::file_dialog::{self, Ask};

/// The path the file dialog gives, as the box takes it: relative to the folder of the settings
/// file when it is under it.
pub(crate) fn path_for_box(path: &Path, ini_dir: Option<&Path>) -> String {
    let shown = ini_dir
        .and_then(|dir| path.strip_prefix(dir).ok())
        .unwrap_or(path);
    shown.to_string_lossy().into_owned()
}

/// Chooses a file and puts it into the edit box `id`, starting where the box says (a relative
/// path starts at the settings file's folder, `CTextInputStream_AbsIni`).
pub(crate) fn browse_into(
    dialog: &mut CommonSettingsDialog,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<CommonSettingsDialog>,
    id: &'static str,
    title: &'static str,
) {
    let text = dialog.input_text(id, cx).unwrap_or_default();
    let ini_dir: Option<PathBuf> = crate::doc_types::with_user(cx, |user| {
        crate::settings::history_file(user).and_then(|path| path.parent().map(Path::to_owned))
    });
    let typed = Path::new(&text);
    let full = match (&ini_dir, typed.is_relative()) {
        (Some(dir), true) => dir.join(typed),
        _ => typed.to_owned(),
    };
    let start = if text.is_empty() {
        ini_dir.clone()
    } else {
        full.parent().map(Path::to_owned)
    };
    let name = full
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|_| !text.is_empty());
    let receiver = file_dialog::ask(
        window,
        cx,
        Ask::open(false)
            .start(start)
            .name(name)
            .titled(title, "選択"),
    );
    cx.spawn_in(window, async move |this, cx| {
        let chosen = receiver.await.ok().flatten();
        if let Some(path) = chosen.and_then(|paths| paths.into_iter().next()) {
            let text = path_for_box(&path, ini_dir.as_deref());
            this.update_in(cx, |this, window, cx| this.set_input(id, &text, window, cx))
                .ok();
        }
    })
    .detach();
}
