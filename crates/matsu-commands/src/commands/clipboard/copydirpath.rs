//! `F_COPYDIRPATH`. Sakura: `CViewCommander::Command_COPYDIRPATH` in
//! `sakura_core/cmd/CViewCommander_Clipboard.cpp`. Copies the folder of the document, without a
//! trailing backslash (so the root of a drive is `C:`). A document without a file only beeps.

use crate::clipboard::ClipKind;
use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CopyDirPath;

impl Command for CopyDirPath {
    const ID: &'static str = "copydirpath";
    const SAKURA: &'static str = "F_COPYDIRPATH";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        let dir = cx
            .path()
            .and_then(|path| path.parent())
            .map(|dir| dir.to_string_lossy().into_owned())
            .filter(|dir| !dir.is_empty());
        let Some(mut dir) = dir else {
            cx.beep();
            return Ok(());
        };
        if dir.ends_with('\\') {
            dir.pop();
        }
        cx.clipboard_set(dir, ClipKind::Normal)?;
        Ok(())
    }
}

register_command!(CopyDirPath);

#[cfg(test)]
mod tests {
    use crate::clipboard::ClipKind;
    use crate::testing::Editor;

    #[test]
    fn copies_the_folder_of_the_file() {
        let mut e = Editor::new("ab|c").with_path("/home/user/doc/a.txt");
        e.run("copydirpath");
        let clip = e.clipboard().expect("a copy");
        assert_eq!(clip.text, "/home/user/doc");
        assert_eq!(clip.kind, ClipKind::Normal);
    }

    #[test]
    fn a_document_without_a_file_beeps() {
        let mut e = Editor::new("ab|c");
        e.run("copydirpath");
        assert!(e.changes().beep);
        assert!(e.clipboard().is_none());
    }

    #[cfg(windows)]
    #[test]
    fn the_root_of_a_drive_has_no_backslash() {
        let mut e = Editor::new("ab|c").with_path("C:\\a.txt");
        e.run("copydirpath");
        assert_eq!(e.clipboard().map(|c| c.text.as_str()), Some("C:"));
    }
}
