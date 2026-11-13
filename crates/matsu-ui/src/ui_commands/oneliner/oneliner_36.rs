//! `F_ONELINER_36`: runs the one-liner in slot 36 of `[OneLiner]` (ADR-0029). See [`crate::oneliner::flow`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Oneliner36;

impl UiCommand for Oneliner36 {
    const ID: &'static str = "oneliner_36";
    const SAKURA: &'static str = "F_ONELINER_36";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::oneliner::flow::run_slot(view, 36, window, cx)
    }
}

register_ui_command!(Oneliner36);
