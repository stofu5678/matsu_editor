//! `F_VIEWMODE`: switches view mode on and off. In view mode every edit is refused.
//!
//! Sakura: `Command_VIEWMODE` (`sakura_core/cmd/CViewCommander_File.cpp`) flips
//! `CAppMode::m_bViewMode`, which belongs to the window. The title shows `(ビューモード)`
//! (`STR_CAPTION_ACTIVE_VIEW`), saving to the file itself is refused (`CSaveAgent::OnCheckSave`),
//! and the functions of `EIsModificationForbidden` in `sakura_core/doc/CEditDoc.cpp` (typing,
//! deleting, cut, paste, undo, redo, case conversion, replace, ...) are not available. Here the
//! command context refuses every edit (`CommandContext::with_view_mode`).
//!
//! Not done: Sakura also releases the exclusive lock of the file when the mode goes on and takes
//! it again when it goes off (`DoFileUnlock`, `DoFileLock`); this program does not lock files.

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct ViewMode;

impl UiCommand for ViewMode {
    const ID: &'static str = "view_mode";
    const SAKURA: &'static str = "F_VIEWMODE";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        // An IME composition is text in the document; end it before the edits are refused.
        view.interrupt_composition(cx);
        view.document().update(cx, |shared, cx| {
            let on = !shared.view_mode();
            shared.set_view_mode(on, cx);
        });
        Ok(())
    }
}

register_ui_command!(ViewMode);
