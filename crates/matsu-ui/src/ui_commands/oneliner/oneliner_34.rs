//! `F_ONELINER_34`: runs the one-liner in slot 34 of `[OneLiner]` (ADR-0029). See [`crate::oneliner::flow`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Oneliner34;

impl UiCommand for Oneliner34 {
    const ID: &'static str = "oneliner_34";
    const SAKURA: &'static str = "F_ONELINER_34";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::oneliner::flow::run_slot(view, 34, window, cx)
    }
}

register_ui_command!(Oneliner34);
