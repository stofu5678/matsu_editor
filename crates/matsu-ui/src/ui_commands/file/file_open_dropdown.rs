//! `F_FILEOPEN_DROPDOWN`: opens files; the function of the toolbar's 「開く」 button with the arrow.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) calls
//! `Command_FILEOPEN((const WCHAR*)lparam1)`, so it is [`F_FILEOPEN`](super::file_open) with
//! the path only (no encoding): the open dialog without a path. `CMenuDrawer` gives the button
//! the drop-down style (`sakura_core/uiparts/CMenuDrawer.cpp`). `CSMacroMgr.cpp` has no macro
//! name for it (only `FileOpen`, which is `F_FILEOPEN2`), so a script cannot call it; a key, a
//! menu or the toolbar can. The toolbar's click runs `F_FILEOPEN` here (`Button::click_code`),
//! which is the same.

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use super::file_open::FileOpen;

#[derive(Default)]
pub struct FileOpenDropdown;

impl UiCommand for FileOpenDropdown {
    const ID: &'static str = "file_open_dropdown";
    const SAKURA: &'static str = "F_FILEOPEN_DROPDOWN";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        // only `lparam1`, the path, is read
        FileOpen.run(view, &args[..args.len().min(1)], window, cx)
    }
}

register_ui_command!(FileOpenDropdown);
