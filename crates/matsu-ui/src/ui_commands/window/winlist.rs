//! `F_WINLIST`: shows the list of the editor windows as a popup (ウィンドウ一覧...).
//!
//! Sakura: `Command_WINLIST` (`sakura_core/cmd/CViewCommander_Window.cpp`) calls
//! `CEditWnd::PopupWinList`, a popup menu of the windows (`WinListMenu`) at the mouse, or at the
//! top right of the view when it came from the keyboard. Choosing an item brings that window to
//! the front. The list is `crate::window_ops::list`. While the tab bar is shown, the popup is the
//! menu of the bar's ▼ button (`CTabWnd::TabListMenu`, `crate::tabs::list`), as in Sakura.

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::window_ops::list;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct Winlist;

impl UiCommand for Winlist {
    const ID: &'static str = "winlist";
    const SAKURA: &'static str = "F_WINLIST";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        // `CEditWnd::PopupWinList`: with the tab bar the popup is the tab list of the bar.
        if crate::tabs::settings::current(cx).show_bar {
            crate::tabs::list::open(view, crate::tabs::list::Start::FUNCTION, window, cx);
        } else {
            list::open(view, false, window, cx);
        }
        Ok(())
    }
}

register_ui_command!(Winlist);
