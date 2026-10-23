//! The options of Sakura's common settings page 検索 that the search dialogs and the Grep window
//! read (`CommonSetting_Search`, `sakura_core/env/CommonSetting.h`).

use gpui::{App, Global};

/// The options. Each field names the Sakura setting it stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SearchPrefs {
    /// `m_bCaretTextForSearch` (`bCaretTextForSearch`): the word at the caret is the first search
    /// string of the dialogs when nothing is selected. On by default.
    pub caret_text_for_search: bool,
    /// `m_bGrepExitConfirm` (`bGrepExitConfirm`): a Grep window asks about saving when it is
    /// closed. Off by default.
    pub grep_exit_confirm: bool,
}

impl Default for SearchPrefs {
    fn default() -> Self {
        Self {
            caret_text_for_search: true,
            grep_exit_confirm: false,
        }
    }
}

impl Global for SearchPrefs {}

/// The options now.
pub(crate) fn prefs(cx: &App) -> SearchPrefs {
    cx.try_global::<SearchPrefs>().copied().unwrap_or_default()
}
