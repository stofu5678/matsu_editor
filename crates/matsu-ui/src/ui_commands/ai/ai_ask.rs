//! `F_AI_ASK` (「指示をつけて送る...」): opens the window of mode b (ADR-0035, 4), where the person types the
//! instruction and picks the destination; 「送る」 runs the destination on the instruction and the target text. See
//! [`crate::ai_ask`] for the window and [`crate::oneliner::flow`] for the run. MATSU Editor's own function; Sakura has
//! nothing like it.
//!
//! A macro calls it as `AiAsk()` and gets the same window: no argument gives the instruction, so it is always typed by
//! a person (ADR-0035, 1).

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct AiAsk;

impl UiCommand for AiAsk {
    const ID: &'static str = "ai_ask";
    const SAKURA: &'static str = "F_AI_ASK";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::oneliner::flow::run_ask(view, window, cx)
    }
}

register_ui_command!(AiAsk);
