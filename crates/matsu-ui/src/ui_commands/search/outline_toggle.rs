//! `F_OUTLINE_TOGGLE`: `Command_FUNCLIST( SHOW_TOGGLE )` shows the outline pane, or closes it when it shows the same kind
//! of outline (`sakura_core/cmd/CViewCommander_Outline.cpp`).

use crate::outline::{Requested, Show};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct OutlineToggle;

impl UiCommand for OutlineToggle {
    const ID: &'static str = "outline_toggle";
    const SAKURA: &'static str = "F_OUTLINE_TOGGLE";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let _ = args;
        view.run_outline(Requested::Default, Show::Toggle, window, cx)
    }
}

register_ui_command!(OutlineToggle);
