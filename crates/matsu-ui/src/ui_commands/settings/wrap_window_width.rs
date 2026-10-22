//! `F_WRAPWINDOWWIDTH`: `Command_WRAPWINDOWWIDTH`: toggles the wrap column between the window width, the largest column and the column of the type (`CEditView::GetWrapMode`).

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct WrapWindowWidth;

impl UiCommand for WrapWindowWidth {
    const ID: &'static str = "wrap_window_width";
    const SAKURA: &'static str = "F_WRAPWINDOWWIDTH";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.toggle_wrap_window_width(cx);
        Ok(())
    }
}

register_ui_command!(WrapWindowWidth);
