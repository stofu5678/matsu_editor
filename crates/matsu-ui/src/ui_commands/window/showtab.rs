//! `F_SHOWTAB`: shows or hides the tab bar (タブの表示/非表示).
//!
//! Sakura: `CViewCommander::Command_SHOWTAB` (`sakura_core/cmd/CViewCommander_Settings.cpp`) flips
//! `m_bDispTabWnd` of the common settings, lays the bar out, and ends with `ResetGroupId`: in tab
//! mode all the windows join one group, else each stands alone again. Here that is
//! `tabs::flows::regroup`: the documents of all windows gather in the tabs of this window, or
//! every tab gets a window. The setting is written to `[Common]` of the settings file.

use crate::tabs::flows;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct Showtab;

impl UiCommand for Showtab {
    const ID: &'static str = "showtab";
    const SAKURA: &'static str = "F_SHOWTAB";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.defer_in_window(window, cx, |window, app| {
            flows::change_settings(app, window, |s| s.show_bar = !s.show_bar);
        })
    }
}

register_ui_command!(Showtab);
