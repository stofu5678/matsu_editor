//! `F_SPLIT_V`: `Command_SPLIT_V` (`sakura_core/cmd/CViewCommander_Window.cpp`) calls `CSplitterWnd::VSplitOnOff`
//! (`sakura_core/window/CSplitterWnd.cpp`): splits the window into upper and lower panes, or removes the split of rows. The rules are in `crate::panes::plan`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::panes::SplitKind;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct SplitV;

impl UiCommand for SplitV {
    const ID: &'static str = "split_v";
    const SAKURA: &'static str = "F_SPLIT_V";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.request_split(SplitKind::Vertical, window, cx)
    }
}

register_ui_command!(SplitV);
