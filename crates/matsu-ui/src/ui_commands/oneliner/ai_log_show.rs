//! `F_AI_LOG_SHOW`: 「AI ログを表示」 (ADR-0030): goes to the log of what was sent to a generative AI, opening the tab
//! if the window has none. MATSU Editor's own function; Sakura has nothing like it.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct AiLogShow;

impl UiCommand for AiLogShow {
    const ID: &'static str = "ai_log_show";
    const SAKURA: &'static str = "F_AI_LOG_SHOW";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.with_workspace(window, cx, |workspace, window, cx| {
            crate::ai_log::open(workspace, window, cx);
        })
    }
}

register_ui_command!(AiLogShow);
