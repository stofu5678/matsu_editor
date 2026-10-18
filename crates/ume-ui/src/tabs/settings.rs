//! The tab settings of Sakura's common settings (共通設定 / ウィンドウ / タブバー), kept in
//! `[Common]` of the settings file.
//!
//! Sakura: `CommonSetting::m_sTabBar` (`sakura_core/env/CommonSetting.h`), read and written by
//! `CShareData_IO::ShareData_IO_Common` (`sakura_core/env/CShareData_IO.cpp`), defaults in
//! `CShareData::InitShareData` (`sakura_core/env/CShareData.cpp`).

use gpui::{App, Global};
use ume_config::Profile;

/// `bDispTabWnd`: the tab bar is shown.
const KEY_SHOW: &str = "bDispTabWnd";
/// `bDispTabWndMultiWin`: "タブをまとめない". The window groups its tabs unless this is on.
const KEY_MULTI_WIN: &str = "bDispTabWndMultiWin";
/// `bNewWindow`: a file opened from outside opens in a new window.
const KEY_NEW_WINDOW: &str = "bNewWindow";

/// What the common settings say about tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TabSettings {
    /// `m_bDispTabWnd`: the tab bar is shown. Off by default, as in Sakura.
    pub show_bar: bool,
    /// `!m_bDispTabWndMultiWin`: the documents are gathered in the tabs of one window.
    pub grouped: bool,
    /// `m_bNewWindow`: a file opened from outside (the command line) gets a new window.
    pub new_window: bool,
}

impl Default for TabSettings {
    fn default() -> Self {
        Self {
            show_bar: false,
            grouped: true,
            new_window: false,
        }
    }
}

impl Global for TabSettings {}

impl TabSettings {
    pub(crate) fn from_profile(profile: &Profile) -> Self {
        let get = |key: &str, default: bool| profile.get_bool("Common", key).unwrap_or(default);
        Self {
            show_bar: get(KEY_SHOW, false),
            grouped: !get(KEY_MULTI_WIN, false),
            new_window: get(KEY_NEW_WINDOW, false),
        }
    }

    /// The tab mode of Sakura: the bar is shown and the windows are grouped
    /// (`m_bDispTabWnd && !m_bDispTabWndMultiWin`). Only then a window holds several documents.
    pub(crate) fn bound(&self) -> bool {
        self.show_bar && self.grouped
    }
}

/// `bTab_RetainEmptyWin`, `bTab_CloseOneWin`, `bSameTabWidth`, `bDispTabClose` and `bChgWndByWheel`,
/// which the common settings dialog changes (「共通設定」の「タブバー」).
const KEY_RETAIN_EMPTY: &str = "bTab_RetainEmptyWin";
const KEY_CLOSE_ONE_WIN: &str = "bTab_CloseOneWin";
const KEY_SAME_WIDTH: &str = "bSameTabWidth";
const KEY_DISP_CLOSE: &str = "bDispTabClose";
const KEY_CHANGE_BY_WHEEL: &str = "bChgWndByWheel";
/// `bSortTabList` and `bTab_ListFull`: the tab list is sorted, and shows the paths.
const KEY_SORT_LIST: &str = "bSortTabList";
const KEY_LIST_FULL: &str = "bTab_ListFull";

/// `EDispTabClose`: when a tab shows its close button (`sakura_core/env/CommonSetting.h`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum DispTabClose {
    /// `DISPTABCLOSE_NO` (0): never. Sakura's default.
    #[default]
    No,
    /// The second value (1): on every tab.
    Always,
    /// `DISPTABCLOSE_AUTO` (2): on the tab the pointer is over.
    Auto,
}

impl DispTabClose {
    pub(crate) fn from_code(code: i32) -> Self {
        match code {
            1 => Self::Always,
            2 => Self::Auto,
            _ => Self::No,
        }
    }
}

/// The tab settings besides the mode: how the bar behaves and looks. Sakura's defaults
/// (`CShareData::InitShareData`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TabOptions {
    /// `m_bTab_RetainEmptyWin`: closing the last file of the group leaves an untitled document in
    /// the window (tab mode only).
    pub retain_empty: bool,
    /// `m_bTab_CloseOneWin`: the close button of the window closes only the current file (tab mode
    /// only).
    pub close_one_win: bool,
    /// `m_bSameTabWidth`: all tabs are as wide as each other.
    pub same_width: bool,
    /// `m_bDispTabClose`: the close buttons of the tabs.
    pub disp_close: DispTabClose,
    /// `m_bChgWndByWheel`: the wheel over the bar goes to the next or previous window.
    pub change_by_wheel: bool,
    /// `m_bSortTabList`: the tab list is sorted by the text of its items.
    pub sort_list: bool,
    /// `m_bTab_ListFull`: the tab list shows the paths, not the captions. The item of the list
    /// that switches the display changes it (not the common settings).
    pub list_full: bool,
}

impl Default for TabOptions {
    fn default() -> Self {
        Self {
            retain_empty: true,
            close_one_win: false,
            same_width: false,
            disp_close: DispTabClose::No,
            change_by_wheel: false,
            sort_list: true,
            list_full: false,
        }
    }
}

impl Global for TabOptions {}

impl TabOptions {
    pub(crate) fn from_profile(profile: &Profile) -> Self {
        let default = Self::default();
        let get = |key: &str, default: bool| profile.get_bool("Common", key).unwrap_or(default);
        Self {
            retain_empty: get(KEY_RETAIN_EMPTY, default.retain_empty),
            close_one_win: get(KEY_CLOSE_ONE_WIN, default.close_one_win),
            same_width: get(KEY_SAME_WIDTH, default.same_width),
            disp_close: profile
                .get_i32("Common", KEY_DISP_CLOSE)
                .map_or(default.disp_close, DispTabClose::from_code),
            change_by_wheel: get(KEY_CHANGE_BY_WHEEL, default.change_by_wheel),
            sort_list: get(KEY_SORT_LIST, default.sort_list),
            list_full: get(KEY_LIST_FULL, default.list_full),
        }
    }
}

/// The options now. Sakura's defaults before the application installs them.
pub(crate) fn options(cx: &App) -> TabOptions {
    cx.try_global::<TabOptions>().copied().unwrap_or_default()
}

/// The settings now. Sakura's defaults before the application installs them.
pub(crate) fn current(cx: &App) -> TabSettings {
    cx.try_global::<TabSettings>().copied().unwrap_or_default()
}

/// Starts the settings from the file that was read.
pub(crate) fn install(cx: &mut App, user: &crate::settings::UserSettings) {
    let settings = user
        .profile()
        .map_or_else(TabSettings::default, TabSettings::from_profile);
    cx.set_global(settings);
    cx.set_global(
        user.profile()
            .map_or_else(TabOptions::default, TabOptions::from_profile),
    );
}

/// The item of the tab list that switches between the captions and the paths keeps the choice
/// (`m_bTab_ListFull`, written with the other settings).
pub(crate) fn set_list_full(cx: &mut App, full: bool) {
    let mut options = options(cx);
    options.list_full = full;
    cx.set_global(options);
    crate::history::write_settings(cx, |profile| {
        let value = if full { "1" } else { "0" };
        if profile.get("Common", KEY_LIST_FULL) == Some(value) {
            return Ok(false);
        }
        profile.set("Common", KEY_LIST_FULL, value)?;
        Ok(true)
    });
}

/// Changes the settings: windows notice (`observe_global`), and the file keeps them.
pub(crate) fn update(cx: &mut App, change: impl FnOnce(&mut TabSettings)) {
    let mut settings = current(cx);
    change(&mut settings);
    cx.set_global(settings);
    crate::history::write_settings(cx, |profile| {
        let mut changed = false;
        for (key, on) in [
            (KEY_SHOW, settings.show_bar),
            (KEY_MULTI_WIN, !settings.grouped),
        ] {
            let value = if on { "1" } else { "0" };
            if profile.get("Common", key) != Some(value) {
                profile.set("Common", key, value)?;
                changed = true;
            }
        }
        Ok(changed)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(text: &str) -> Profile {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        Profile::from_bytes(&bytes)
    }

    #[test]
    fn the_defaults_are_sakuras() {
        let none = TabSettings::from_profile(&Profile::new());
        assert_eq!(none, TabSettings::default());
        assert!(!none.show_bar && none.grouped && !none.bound());
    }

    #[test]
    fn the_keys_are_read() {
        let p = profile("[Common]\r\nbDispTabWnd=1\r\nbDispTabWndMultiWin=0\r\nbNewWindow=1\r\n");
        let s = TabSettings::from_profile(&p);
        assert!(s.show_bar && s.grouped && s.bound() && s.new_window);
        let p = profile("[Common]\r\nbDispTabWnd=1\r\nbDispTabWndMultiWin=1\r\n");
        let s = TabSettings::from_profile(&p);
        assert!(s.show_bar && !s.grouped && !s.bound());
    }
}
