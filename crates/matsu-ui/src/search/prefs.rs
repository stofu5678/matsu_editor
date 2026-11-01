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
    /// `m_bGrepRealTimeView` (`bGrepRealTime`): the Grep window shows the result while the search
    /// runs. Off by default; off, the result is written when the search ends.
    pub grep_real_time: bool,
    /// `m_bGTJW_RETURN` (`bGTJW_RETURN`): Enter in a Grep result is a tag jump. On by default.
    pub grep_enter_jumps: bool,
    /// `m_bGTJW_LDBLCLK` (`bGTJW_LDBLCLK`): a double click in a Grep result is a tag jump. On by
    /// default.
    pub grep_double_click_jumps: bool,
}

impl Default for SearchPrefs {
    fn default() -> Self {
        Self {
            caret_text_for_search: true,
            grep_exit_confirm: false,
            grep_real_time: false,
            grep_enter_jumps: true,
            grep_double_click_jumps: true,
        }
    }
}

impl Global for SearchPrefs {}

/// The options now.
pub(crate) fn prefs(cx: &App) -> SearchPrefs {
    cx.try_global::<SearchPrefs>().copied().unwrap_or_default()
}
