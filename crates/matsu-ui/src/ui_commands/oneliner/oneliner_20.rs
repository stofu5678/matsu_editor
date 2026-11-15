//! `F_ONELINER_20`: runs the one-liner in slot 20 of `[OneLiner]` (ADR-0029). See [`crate::oneliner::flow`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Oneliner20;

impl UiCommand for Oneliner20 {
    const ID: &'static str = "oneliner_20";
    const SAKURA: &'static str = "F_ONELINER_20";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::oneliner::flow::run_slot(view, 20, window, cx)
    }
}

register_ui_command!(Oneliner20);
