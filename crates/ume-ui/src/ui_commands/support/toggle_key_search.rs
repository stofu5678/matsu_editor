//! `F_TOGGLE_KEY_SEARCH`: turns the lookup of the word at the caret in the keyword help on or off.
//!
//! Sakura: `CViewCommander::Command_ToggleKeySearch` (`sakura_core/cmd/CViewCommander_Support.cpp`).
//! The number is 0 (toggle; a missing number is 0), 1 (on) or 2 (off); another number changes
//! nothing. It changes `m_bUseCaretKeyWord` for every window, and the file does not have it
//! ([`crate::key_help`]). The menu check is `IsFuncChecked` (`sakura_core/func/Funccode.cpp`), here
//! [`crate::menu_state`].

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::key_help::{caret_key_word, set_caret_key_word};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::ui_commands::macro_only::int_arg;
use crate::view::EditorView;

#[derive(Default)]
pub struct ToggleKeySearch;

/// The setting after `option`, from `current`, or `None` when the number changes nothing.
fn after(option: i64, current: bool) -> Option<bool> {
    match option {
        0 => Some(!current),
        1 => Some(true),
        2 => Some(false),
        _ => None,
    }
}

impl UiCommand for ToggleKeySearch {
    const ID: &'static str = "toggle_key_search";
    const SAKURA: &'static str = "F_TOGGLE_KEY_SEARCH";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let option = int_arg("ToggleKeyHelpSearch", args, 0, 0)?;
        if let Some(on) = after(option, caret_key_word(cx)) {
            set_caret_key_word(cx, on);
            // The tip of this window follows the setting.
            view.hide_key_help(cx);
            cx.notify();
        }
        Ok(())
    }
}

register_ui_command!(ToggleKeySearch);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_numbers_of_the_function() {
        // `Command_ToggleKeySearch`
        assert_eq!(after(0, false), Some(true));
        assert_eq!(after(0, true), Some(false));
        assert_eq!(after(1, false), Some(true));
        assert_eq!(after(1, true), Some(true));
        assert_eq!(after(2, true), Some(false));
        assert_eq!(after(2, false), Some(false));
        assert_eq!(after(3, true), None);
        assert_eq!(after(-1, false), None);
    }
}
