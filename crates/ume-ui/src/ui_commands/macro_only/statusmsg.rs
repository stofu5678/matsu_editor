//! `F_STATUSMSG` (the macro `StatusMsg(text, option)`): shows a message in the status bar.
//!
//! Sakura: `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`) stops the macro when the text
//! is missing; the option (default 0) is read with `option & 3`:
//!
//! * 0: `SendStatusMessage` (`CEditWnd::SendStatusMessage`): the left part of the status bar, or
//!   the menu bar when the status bar is hidden.
//! * 1: the same when the status bar is shown, else a message box (`InfoMessage`).
//! * 2: `CMainStatusBar::SendStatusMessage2`: the same part of the status bar, and nothing when
//!   the status bar is hidden.
//! * 3: nothing.
//!
//! Here the message is the notice of the status bar (what a command that cannot do its job says),
//! which the next edit or caret move clears. With the status bar hidden, option 0 has no place to
//! show it (Sakura uses the menu bar, which this program draws itself and which has no such
//! area), so only the message box of option 1 is seen.

use gpui::{Context, PromptLevel, Window};
use ume_commands::command::{Arg, Result};

use super::{int_arg, text_arg};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

/// The caption of the message box (Sakura shows its own name).
const CAPTION: &str = "UME Editor";

#[derive(Default)]
pub struct StatusMsg;

impl UiCommand for StatusMsg {
    const ID: &'static str = "statusmsg";
    const SAKURA: &'static str = "F_STATUSMSG";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let text = text_arg("StatusMsg", args, 0)?;
        let option = int_arg("StatusMsg", args, 1, 0)?;
        let status_bar = view.prefs.show_status_bar;
        match option & 0x03 {
            0 => view.notice = Some(text.to_owned()),
            1 if status_bar => view.notice = Some(text.to_owned()),
            1 => {
                let answer = window.prompt(PromptLevel::Info, CAPTION, Some(text), &["OK"], cx);
                // The box is modal for the user, not for the macro: Sakura's `InfoMessage` waits,
                // which this window does not do for a command (the macro functions that return
                // the answer, `InfoMsg`, do).
                cx.spawn_in(window, async move |_, _| {
                    let _ = answer.await;
                })
                .detach();
            }
            2 if status_bar => view.notice = Some(text.to_owned()),
            _ => {}
        }
        cx.notify();
        Ok(())
    }
}

register_ui_command!(StatusMsg);
