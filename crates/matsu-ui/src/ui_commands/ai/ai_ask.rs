//! `F_AI_ASK` (「指示をつけて送る...」): the modal of mode b (ADR-0035, 4), where the person types the instruction.
//! MATSU Editor's own function; Sakura has nothing like it.
//!
//! The modal is a later step of ADR-0035. Until then the item is grayed out in the menus (`crate::menu_state`) and
//! a key or a macro that calls the function gets [`NOT_READY`] on the status bar and nothing is sent.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

/// What the status bar says while mode b has no modal.
pub(crate) const NOT_READY: &str = "指示をつけて送る（モード b）は、まだ使えません。";

#[derive(Default)]
pub struct AiAsk;

impl UiCommand for AiAsk {
    const ID: &'static str = "ai_ask";
    const SAKURA: &'static str = "F_AI_ASK";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.notice = Some(NOT_READY.to_owned());
        cx.notify();
        Ok(())
    }
}

register_ui_command!(AiAsk);
