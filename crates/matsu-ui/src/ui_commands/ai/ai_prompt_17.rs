//! `F_AI_PROMPT_17`: sends the target text to destination 17 of `[AI]` as the prompt (ADR-0035, mode a). See
//! [`crate::ai_send`]. MATSU Editor's own function; Sakura has nothing like it.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct AiPrompt17;

impl UiCommand for AiPrompt17 {
    const ID: &'static str = "ai_prompt_17";
    const SAKURA: &'static str = "F_AI_PROMPT_17";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::ai_send::run_prompt(view, 17, window, cx)
    }
}

register_ui_command!(AiPrompt17);
