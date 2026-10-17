//! Sakura's custom menus (カスタムメニュー) and the right-click menu: a popup of the functions a menu
//! holds (`[CustMenu]` in the settings file, `ume_config::settings::custom_menu`).
//!
//! * [`model`] builds the rows of a menu (separators, submenus, items with their shortcut and whether they
//!   can run), without GPUI.
//! * [`popup`] shows them in a GPUI popup at a point of the window. An item dispatches
//!   [`RunFunction`](crate::view::RunFunction) to the editor, the same action the main menu's items use, so
//!   the chosen function runs through `EditorView::run_function` and a key macro records it.
//! * [`mouse`] is the right click in the text (`CEditView::OnRBUTTONDOWN` / `OnRBUTTONUP`).
//!
//! `F_MENU_RBUTTON` and `F_CUSTMENU_1` to `F_CUSTMENU_24` are UI commands (`ui_commands/custom_menu`).

pub(crate) mod model;
mod mouse;
mod popup;

pub(crate) use popup::CustomMenuState;

use gpui::{Context, Window};
use ume_commands::command::Result;

use crate::menu::{Item, Lists};
use crate::view::EditorView;

/// What the UI commands of the group do: show custom menu `index`.
pub(crate) fn run(
    index: usize,
    view: &mut EditorView,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) -> Result {
    view.show_custom_menu(index, window, cx);
    Ok(())
}

/// Two sets of run-time lists as one: the first that has items for a code answers.
pub(crate) struct ChainedLists<'a>(pub &'a dyn Lists, pub &'a dyn Lists);

impl Lists for ChainedLists<'_> {
    fn items(&self, code: u16) -> Vec<Item> {
        let first = self.0.items(code);
        if first.is_empty() {
            self.1.items(code)
        } else {
            first
        }
    }
}

#[cfg(test)]
mod tests;
