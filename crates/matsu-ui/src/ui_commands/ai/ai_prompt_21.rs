//! `F_AI_PROMPT_21`: sends the target text to destination 21 of `[AI]` as the prompt (ADR-0035, mode a). See
//! [`crate::ai_send`]. MATSU Editor's own function; Sakura has nothing like it.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct AiPrompt21;

impl UiCommand for AiPrompt21 {
    const ID: &'static str = "ai_prompt_21";
    const SAKURA: &'static str = "F_AI_PROMPT_21";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::ai_send::run_prompt(view, 21, window, cx)
    }
}

register_ui_command!(AiPrompt21);
