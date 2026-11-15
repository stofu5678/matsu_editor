//! `F_ONELINER_27`: runs the one-liner in slot 27 of `[OneLiner]` (ADR-0029). See [`crate::oneliner::flow`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Oneliner27;

impl UiCommand for Oneliner27 {
    const ID: &'static str = "oneliner_27";
    const SAKURA: &'static str = "F_ONELINER_27";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::oneliner::flow::run_slot(view, 27, window, cx)
    }
}

register_ui_command!(Oneliner27);
