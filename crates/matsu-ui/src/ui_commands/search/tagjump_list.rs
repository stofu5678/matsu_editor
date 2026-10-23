//! `F_TAGJUMP_LIST`: shows the tag jump list dialog (タグジャンプ一覧).
//!
//! Sakura has no command for this function: it is the number of the help topic of the dialog
//! (`FuncID_To_HelpContextID`, `sakura_core/func/Funccode.cpp`, and the help button of
//! `CDlgTagJumpList`, `sakura_core/dlg/CDlgTagJumpList.cpp`). Like the `F_OPTION_*` of the
//! settings pages, it opens the dialog it names (ADR-0018, type B: the nearest form). The
//! dialog is the one `CDlgTagJumpList` shows for `Command_TagJumpByTagsFileKeyword`, so this runs
//! the same flow as `F_TAGJUMP_KEYWORD` (`tagjump_keyword`), with the same optional keyword.

use crate::file_ops::start;
use crate::tags::keyword_tag_jump;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct TagjumpList;

impl UiCommand for TagjumpList {
    const ID: &'static str = "tagjump_list";
    const SAKURA: &'static str = "F_TAGJUMP_LIST";

    fn run(
        &self,
        _view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let keyword = match args.first() {
            Some(Arg::Text(text)) => Some(text.clone()),
            _ => None,
        };
        start(window, cx, async move |view, cx| {
            keyword_tag_jump(&view, cx, keyword).await;
        });
        Ok(())
    }
}

register_ui_command!(TagjumpList);
