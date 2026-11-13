//! `F_ONELINER_39`: runs the one-liner in slot 39 of `[OneLiner]` (ADR-0029). See [`crate::oneliner::flow`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Oneliner39;

impl UiCommand for Oneliner39 {
    const ID: &'static str = "oneliner_39";
    const SAKURA: &'static str = "F_ONELINER_39";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::oneliner::flow::run_slot(view, 39, window, cx)
    }
}

register_ui_command!(Oneliner39);
