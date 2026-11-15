//! `F_ONELINER_17`: runs the one-liner in slot 17 of `[OneLiner]` (ADR-0029). See [`crate::oneliner::flow`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Oneliner17;

impl UiCommand for Oneliner17 {
    const ID: &'static str = "oneliner_17";
    const SAKURA: &'static str = "F_ONELINER_17";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::oneliner::flow::run_slot(view, 17, window, cx)
    }
}

register_ui_command!(Oneliner17);
