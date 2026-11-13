//! `F_ONELINER_4`: runs the one-liner in slot 4 of `[OneLiner]` (ADR-0029). See [`crate::oneliner::flow`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Oneliner4;

impl UiCommand for Oneliner4 {
    const ID: &'static str = "oneliner_4";
    const SAKURA: &'static str = "F_ONELINER_4";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::oneliner::flow::run_slot(view, 4, window, cx)
    }
}

register_ui_command!(Oneliner4);
