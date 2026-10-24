//! The options of the find and replace dialogs that the next dialog starts with.
//!
//! Sakura: `CommonSetting::m_sSearch` (`m_sSearchOption`, `m_bConsecutiveAll`, `m_bNOTIFYNOTFOUND`,
//! `m_bSearchAll`, `m_bSelectedArea`; `sakura_core/env/CommonSetting.h`), read by
//! `CDlgFind::SetData` / `CDlgReplace::SetData` when the dialog opens and written by their
//! `GetData` when a search is done with a pattern that is valid (`sakura_core/dlg/CDlgFind.cpp`
//! 240, `CDlgReplace.cpp` 275). The keys are in `[Common]`: `bSearchRegularExp`, `bSearchLoHiCase`,
//! `bSearchWordOnly`, `bSearchConsecutiveAll`, `bSearchNOTIFYNOTFOUND`, `bSearchAll` and
//! `bSearchSelectedArea` (`ShareData_IO_Common`).

use gpui::{App, Global};
use matsu_config::Profile;

use crate::search::state::{ReplaceSettings, SearchOptions};

const KEY_REGEX: &str = "bSearchRegularExp";
const KEY_CASE: &str = "bSearchLoHiCase";
const KEY_WORD: &str = "bSearchWordOnly";
const KEY_CONSECUTIVE: &str = "bSearchConsecutiveAll";
const KEY_NOTIFY: &str = "bSearchNOTIFYNOTFOUND";
const KEY_ALL: &str = "bSearchAll";
const KEY_SELECTED_AREA: &str = "bSearchSelectedArea";

/// What the dialogs start with. Sakura's defaults (`CShareData::InitShareData`): the options are
/// off, and a search that finds nothing says so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SavedSearch {
    pub regex: bool,
    pub match_case: bool,
    pub whole_word: bool,
    pub consecutive_all: bool,
    pub notify_not_found: bool,
    pub wrap: bool,
    pub selected_area: bool,
}

impl Default for SavedSearch {
    fn default() -> Self {
        Self {
            regex: false,
            match_case: false,
            whole_word: false,
            consecutive_all: false,
            notify_not_found: true,
            wrap: false,
            selected_area: false,
        }
    }
}

impl Global for SavedSearch {}

impl SavedSearch {
    pub(crate) fn from_profile(profile: &Profile) -> Self {
        let default = Self::default();
        let get = |key: &str, default: bool| profile.get_bool("Common", key).unwrap_or(default);
        Self {
            regex: get(KEY_REGEX, default.regex),
            match_case: get(KEY_CASE, default.match_case),
            whole_word: get(KEY_WORD, default.whole_word),
            consecutive_all: get(KEY_CONSECUTIVE, default.consecutive_all),
            notify_not_found: get(KEY_NOTIFY, default.notify_not_found),
            wrap: get(KEY_ALL, default.wrap),
            selected_area: get(KEY_SELECTED_AREA, default.selected_area),
        }
    }

    /// The options of a dialog that opens now (`SetData`): these, and the dialog's own
    /// `auto_close`, which Sakura does not keep in the file.
    pub(crate) fn options(self, current: SearchOptions) -> SearchOptions {
        SearchOptions {
            match_case: self.match_case,
            whole_word: self.whole_word,
            notify_not_found: self.notify_not_found,
            wrap: self.wrap,
            regex: self.regex,
            ..current
        }
    }

    fn pairs(self) -> [(&'static str, bool); 7] {
        [
            (KEY_REGEX, self.regex),
            (KEY_CASE, self.match_case),
            (KEY_WORD, self.whole_word),
            (KEY_CONSECUTIVE, self.consecutive_all),
            (KEY_NOTIFY, self.notify_not_found),
            (KEY_ALL, self.wrap),
            (KEY_SELECTED_AREA, self.selected_area),
        ]
    }
}

/// The options now.
pub(crate) fn saved(cx: &App) -> SavedSearch {
    cx.try_global::<SavedSearch>().copied().unwrap_or_default()
}

/// A search was done from a dialog (`GetData`): the options of the dialog are the ones the next
/// dialog starts with, in every window, and the file keeps them. The find dialog has no
/// 置換の繰返し and no 選択範囲内, so it leaves those as they were.
pub(crate) fn remember(cx: &mut App, options: SearchOptions, replace: Option<&ReplaceSettings>) {
    let old = saved(cx);
    let now = SavedSearch {
        regex: options.regex,
        match_case: options.match_case,
        whole_word: options.whole_word,
        notify_not_found: options.notify_not_found,
        wrap: options.wrap,
        consecutive_all: replace.map_or(old.consecutive_all, |r| r.consecutive_all),
        selected_area: replace.map_or(old.selected_area, |r| r.in_selection),
    };
    if now == old && cx.has_global::<SavedSearch>() {
        return;
    }
    cx.set_global(now);
    crate::history::write_settings(cx, |profile| {
        let mut changed = false;
        for (key, on) in now.pairs() {
            let value = if on { "1" } else { "0" };
            if profile.get("Common", key) != Some(value) {
                profile.set("Common", key, value)?;
                changed = true;
            }
        }
        Ok(changed)
    });
}
