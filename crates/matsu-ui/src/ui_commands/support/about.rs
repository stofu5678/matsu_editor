//! `F_ABOUT`: the version information (バージョン情報), a dialog with the name, the version, the
//! license and the repository.
//!
//! Sakura: `CViewCommander::Command_ABOUT` (`sakura_core/cmd/CViewCommander_Support.cpp`) opens
//! `CDlgAbout`. See [`crate::support::about`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct About;

impl UiCommand for About {
    const ID: &'static str = "about";
    const SAKURA: &'static str = "F_ABOUT";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.open_about(window, cx);
        Ok(())
    }
}

register_ui_command!(About);
