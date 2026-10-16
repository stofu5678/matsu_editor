//! `F_TYPE_LIST`: shows the list of document types (タイプ別設定一覧...), to apply one to this document.
//!
//! Sakura: `Command_TYPE_LIST` (`sakura_core/cmd/CViewCommander_Settings.cpp`), with the dialog
//! `CDlgTypeList` (`sakura_core/typeprop/CDlgTypeList.cpp`). The dialog is `crate::doc_types::list`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::doc_types::list;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct TypeList;

impl UiCommand for TypeList {
    const ID: &'static str = "type_list";
    const SAKURA: &'static str = "F_TYPE_LIST";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        list::open(view, window, cx);
        Ok(())
    }
}

register_ui_command!(TypeList);
