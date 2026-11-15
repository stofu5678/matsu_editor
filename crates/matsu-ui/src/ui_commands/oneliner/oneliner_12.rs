//! `F_ONELINER_12`: runs the one-liner in slot 12 of `[OneLiner]` (ADR-0029). See [`crate::oneliner::flow`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Oneliner12;

impl UiCommand for Oneliner12 {
    const ID: &'static str = "oneliner_12";
    const SAKURA: &'static str = "F_ONELINER_12";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::oneliner::flow::run_slot(view, 12, window, cx)
    }
}

register_ui_command!(Oneliner12);
