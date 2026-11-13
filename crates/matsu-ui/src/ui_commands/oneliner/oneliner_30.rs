//! `F_ONELINER_30`: runs the one-liner in slot 30 of `[OneLiner]` (ADR-0029). See [`crate::oneliner::flow`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Oneliner30;

impl UiCommand for Oneliner30 {
    const ID: &'static str = "oneliner_30";
    const SAKURA: &'static str = "F_ONELINER_30";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::oneliner::flow::run_slot(view, 30, window, cx)
    }
}

register_ui_command!(Oneliner30);
