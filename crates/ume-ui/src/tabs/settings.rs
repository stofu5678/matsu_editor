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
