//! `F_AI_PROMPT_35`: sends the target text to destination 35 of `[AI]` as the prompt (ADR-0035, mode a). See
//! [`crate::ai_send`]. MATSU Editor's own function; Sakura has nothing like it.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct AiPrompt35;

impl UiCommand for AiPrompt35 {
    const ID: &'static str = "ai_prompt_35";
    const SAKURA: &'static str = "F_AI_PROMPT_35";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::ai_send::run_prompt(view, 35, window, cx)
    }
}

register_ui_command!(AiPrompt35);
