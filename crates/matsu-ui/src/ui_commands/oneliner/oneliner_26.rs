//! `F_ONELINER_26`: runs the one-liner in slot 26 of `[OneLiner]` (ADR-0029). See [`crate::oneliner::flow`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Oneliner26;

impl UiCommand for Oneliner26 {
    const ID: &'static str = "oneliner_26";
    const SAKURA: &'static str = "F_ONELINER_26";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::oneliner::flow::run_slot(view, 26, window, cx)
    }
}

register_ui_command!(Oneliner26);
