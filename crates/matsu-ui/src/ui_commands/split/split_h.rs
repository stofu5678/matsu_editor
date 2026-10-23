//! `F_SPLIT_H`: `Command_SPLIT_H` (`sakura_core/cmd/CViewCommander_Window.cpp`) calls `CSplitterWnd::HSplitOnOff`
//! (`sakura_core/window/CSplitterWnd.cpp`): splits the window into left and right panes, or removes the split of columns. The rules are in `crate::panes::plan`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::panes::SplitKind;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct SplitH;

impl UiCommand for SplitH {
    const ID: &'static str = "split_h";
    const SAKURA: &'static str = "F_SPLIT_H";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.request_split(SplitKind::Horizontal, window, cx)
    }
}

register_ui_command!(SplitH);
