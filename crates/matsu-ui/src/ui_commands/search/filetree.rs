//! `F_FILETREE`: `Command_FUNCLIST( .., OUTLINE_FILETREE )` (`sakura_core/cmd/CViewCommander.cpp`) shows the
//! file tree in the outline pane (`sakura_core/outline/CDlgFuncList.cpp`). The argument is the action as for
//! `F_OUTLINE` (0 show, 1 show and read the settings again, 2 toggle); the menu and the key give none. Sakura has
//! no macro for it (the help says 「マクロからは利用できません」). The tree is in `crate::outline::file_tree`.

use crate::outline::{Requested, Show};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct FileTree;

impl UiCommand for FileTree {
    const ID: &'static str = "filetree";
    const SAKURA: &'static str = "F_FILETREE";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let show = match args.first() {
            Some(Arg::Int(n)) => Show::from_argument(*n),
            _ => Show::Normal,
        };
        view.run_outline(Requested::FileTree, show, window, cx)
    }
}

register_ui_command!(FileTree);
