//! Saving decoded bytes to a file the user chooses: what `F_BASE64DECODE` and `F_UUDECODE` do
//! after the decoding (`Command_BASE64DECODE` and `Command_UUDECODE` in
//! `sakura_core/cmd/CViewCommander_Convert.cpp`).
//!
//! The bytes do not become the document: they are written as they are to the path of the save
//! dialog, and the document, its name and its modified state do not change.

use std::path::PathBuf;

use gpui::{AsyncWindowContext, Context, PromptLevel, Window};
use ume_core::cursor::SelectionMode;

use super::ask::tell;
use super::save::choose_path;
use super::start;
use crate::view::EditorView;

/// The selected text, with its line endings as they are (`GetSelectedDataSimple`). `None` without
/// a selection and for a rectangular selection, which is not supported yet (the commands do
/// nothing then).
pub(crate) fn selected_text(view: &EditorView, cx: &gpui::App) -> Option<String> {
    if view.cursor().mode() == SelectionMode::Rectangular {
        return None;
    }
    let (from, to) = view.cursor().selection()?;
    view.document().read(cx).doc().text_between(from, to).ok()
}

/// `STR_ERR_CEDITVIEW_CMD14` and `STR_ERR_CEDITVIEW_CMD16`.
fn write_error_message(path: &std::path::Path) -> String {
    format!("ファイルの書き込みに失敗しました。\n\n{}", path.display())
}

/// Asks for a path with the save dialog (suggesting `name`) and writes `data` to it. Cancelling
/// the dialog does nothing. A failed write shows an error.
pub(crate) fn save_bytes_as(
    view: &EditorView,
    name: String,
    data: Vec<u8>,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) {
    let current = view
        .document()
        .read(cx)
        .path()
        .map(std::path::Path::to_path_buf);
    start(
        window,
        cx,
        async move |_view, cx: &mut AsyncWindowContext| {
            let Some(path) = choose_path(cx, current.as_deref(), Some(&name)).await else {
                return;
            };
            write(cx, path, data).await;
        },
    );
}

/// Writes `data` to `path`. An existing file is replaced only after the new bytes are all on
/// disk (a temporary file next to it, then a rename), so a failed write leaves it as it was
/// (ADR-0005). Sakura truncates the file first (`CBinaryOutputStream`).
fn write_bytes(path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
    if !path.exists() {
        return std::fs::write(path, data);
    }
    let mut temp = path.as_os_str().to_owned();
    temp.push(".ume-tmp");
    let temp = PathBuf::from(temp);
    let result = std::fs::write(&temp, data).and_then(|()| std::fs::rename(&temp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

async fn write(cx: &mut AsyncWindowContext, path: PathBuf, data: Vec<u8>) {
    if write_bytes(&path, &data).is_err() {
        let message = write_error_message(&path);
        tell(cx, PromptLevel::Critical, &message, None).await;
    }
}
