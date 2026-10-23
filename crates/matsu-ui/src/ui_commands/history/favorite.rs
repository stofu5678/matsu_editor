//! `F_FAVORITE`: shows the window that manages the history (履歴の管理...).
//!
//! Sakura: `CControlTray::OnCommand` shows `CDlgFavorite` (`sakura_core/_main/CControlTray.cpp`,
//! `sakura_core/dlg/CDlgFavorite.cpp`). The window is `crate::history::dialog`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::history::dialog;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Favorite;

impl UiCommand for Favorite {
    const ID: &'static str = "favorite";
    const SAKURA: &'static str = "F_FAVORITE";

    fn run(
        &self,
        _: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        dialog::open(cx);
        Ok(())
    }
}

register_ui_command!(Favorite);
