//! `F_SPLIT_VH`: `Command_SPLIT_VH` (`sakura_core/cmd/CViewCommander_Window.cpp`) calls `CSplitterWnd::VHSplitOnOff`
//! (`sakura_core/window/CSplitterWnd.cpp`): splits the window into four panes, or removes the split. The rules are in `crate::panes::plan`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::panes::SplitKind;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct SplitVh;

impl UiCommand for SplitVh {
    const ID: &'static str = "split_vh";
    const SAKURA: &'static str = "F_SPLIT_VH";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.request_split(SplitKind::Both, window, cx)
    }
}

register_ui_command!(SplitVh);
