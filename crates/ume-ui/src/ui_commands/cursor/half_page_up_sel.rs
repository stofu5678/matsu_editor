//! `F_HalfPageUp_Sel`: `Command_HalfPageUp` with the selection extended.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`; the rules are in `view/paging.rs`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct HalfPageUpSel;

impl UiCommand for HalfPageUpSel {
    const ID: &'static str = "half_page_up_sel";
    const SAKURA: &'static str = "F_HalfPageUp_Sel";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.half_page_by(-1, true, cx)
    }
}

register_ui_command!(HalfPageUpSel);
