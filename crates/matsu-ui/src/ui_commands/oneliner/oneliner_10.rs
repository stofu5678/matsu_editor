//! `F_ONELINER_10`: runs the one-liner in slot 10 of `[OneLiner]` (ADR-0029). See [`crate::oneliner::flow`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Oneliner10;

impl UiCommand for Oneliner10 {
    const ID: &'static str = "oneliner_10";
    const SAKURA: &'static str = "F_ONELINER_10";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::oneliner::flow::run_slot(view, 10, window, cx)
    }
}

register_ui_command!(Oneliner10);
