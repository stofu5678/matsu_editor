//! Handing the file of the window to the OS: the file manager, the default program, a shell in
//! the folder (`F_OPEN_FOLDER_IN_EXPLORER`, `F_BROWSE`, `F_OPEN_COMMAND_PROMPT` and the others).
//!
//! Sakura beeps when the document has no file (`IsValidPath`) or the OS refuses. Here a message
//! goes to the status bar instead, and the command ends without an error, so that a macro goes on
//! as it does in Sakura.

use std::path::{Path, PathBuf};

use gpui::Context;
use matsu_platform::shell::{Shell, ShellError, open_shell};

use crate::view::EditorView;

/// The folder of a file: where a shell opens. A file name without a folder is in the current one.
pub(crate) fn folder_of(path: &Path) -> PathBuf {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_owned(),
        _ => PathBuf::from("."),
    }
}

/// The full path of the file of the window. `None` after a notice if the document has none.
pub(crate) fn document_file(
    view: &mut EditorView,
    cx: &mut Context<EditorView>,
) -> Option<PathBuf> {
    let path = view.document().read(cx).path().map(Path::to_owned);
    match path {
        Some(path) => Some(std::path::absolute(&path).unwrap_or(path)),
        None => {
            view.notice = Some("ファイルがないので使えない".to_owned());
            cx.notify();
            None
        }
    }
}

/// Opens `shell` in the folder of the file (`admin` asks for an elevated one).
pub(crate) fn open_shell_here(
    view: &mut EditorView,
    shell: Shell,
    admin: bool,
    cx: &mut Context<EditorView>,
) {
    let Some(path) = document_file(view, cx) else {
        return;
    };
    if let Err(error) = open_shell(shell, admin, &folder_of(&path)) {
        view.notice = Some(shell_error_text(&error));
        cx.notify();
    }
}

/// What the status bar says when no shell opened.
pub(crate) fn shell_error_text(error: &ShellError) -> String {
    match error {
        ShellError::Unavailable(what) => format!("{what} はこの環境では使えない"),
        ShellError::Failed(why) => format!("開けなかった: {why}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_folder_of_a_file() {
        assert_eq!(folder_of(Path::new("/a/b/c.txt")), PathBuf::from("/a/b"));
        assert_eq!(folder_of(Path::new("c.txt")), PathBuf::from("."));
        assert_eq!(folder_of(Path::new("/c.txt")), PathBuf::from("/"));
    }

    #[test]
    fn errors_are_told_in_japanese() {
        assert_eq!(
            shell_error_text(&ShellError::Unavailable("PowerShell (pwsh)")),
            "PowerShell (pwsh) はこの環境では使えない"
        );
        assert!(shell_error_text(&ShellError::Failed("x".into())).contains('x'));
    }
}
