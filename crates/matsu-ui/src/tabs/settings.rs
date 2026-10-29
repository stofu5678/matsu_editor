//! The tab settings of Sakura's common settings (共通設定 / ウィンドウ / タブバー), kept in
//! `[Common]` of the settings file.
//!
//! Sakura: `CommonSetting::m_sTabBar` (`sakura_core/env/CommonSetting.h`), read and written by
//! `CShareData_IO::ShareData_IO_Common` (`sakura_core/env/CShareData_IO.cpp`), defaults in
//! `CShareData::InitShareData` (`sakura_core/env/CShareData.cpp`).

use gpui::{App, Global};
use matsu_config::Profile;

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
/// `bDispTabIcon`: the tabs show an icon.
const KEY_DISP_ICON: &str = "bDispTabIcon";
/// `bTabMultiLine`: the tabs wrap into several rows.
const KEY_MULTI_LINE: &str = "bTabMultiLine";
/// `eTabPosition`: the bar is at the top or the bottom.
const KEY_POSITION: &str = "eTabPosition";
/// `nTabMaxWidth`, `nTabMinWidth` and `nTabMinWidthOnMulti`: the widths of a tab, in pixels.
/// Sakura's dialog has no control for them; they are set in the file.
const KEY_MAX_WIDTH: &str = "nTabMaxWidth";
const KEY_MIN_WIDTH: &str = "nTabMinWidth";
const KEY_MIN_WIDTH_MULTI: &str = "nTabMinWidthOnMulti";

/// `ETabPosition`: where the tab bar is (`sakura_core/env/CommonSetting.h`).
///
/// Sakura has `Left` (2) and `Right` (3) too, but the dialog has them under `#if 0`
/// (`CPropComTab.cpp`) and `CEditWnd::OnSize` places the bar only for `Top` and `Bottom`, so any
/// other value in the file is read as the top.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TabPosition {
    /// `TabPosition_Top` (0): under the toolbar. Sakura's default.
    #[default]
    Top,
    /// `TabPosition_Bottom` (1): above the function keys and the status bar.
    Bottom,
}

impl TabPosition {
    pub(crate) fn from_code(code: i32) -> Self {
        if code == 1 { Self::Bottom } else { Self::Top }
    }
}

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
    /// `m_bDispTabIcon`: each tab shows an icon.
    pub disp_icon: bool,
    /// `m_bTabMultiLine`: the tabs that do not fit in a row go on to the next row.
    pub multi_line: bool,
    /// `m_eTabPosition`: the bar is at the top or at the bottom.
    pub position: TabPosition,
    /// `m_nTabMaxWidth`: the widest a tab gets (pixels).
    pub max_width: i32,
    /// `m_nTabMinWidth`: the narrowest a tab gets in a single row (pixels).
    pub min_width: i32,
    /// `m_nTabMinWidthOnMulti`: the narrowest a tab gets in several rows (pixels).
    pub min_width_multi: i32,
}

impl TabOptions {
    /// The width limits of a tab as the layout uses them: `(min, max)` in pixels. A value that is
    /// not positive is not a width, so it is taken as 1; a minimum above the maximum is cut to it
    /// (`CTabWnd::Refresh` tests `MAX_TABITEM_WIDTH < cx` first, so the maximum wins there).
    pub(crate) fn width_limits(&self) -> (i32, i32) {
        let max = self.max_width.max(1);
        let min = if self.multi_line {
            self.min_width_multi
        } else {
            self.min_width
        };
        (min.clamp(1, max), max)
    }
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
            disp_icon: false,
            multi_line: false,
            position: TabPosition::Top,
            max_width: 200,
            min_width: 60,
            min_width_multi: 100,
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
            disp_icon: get(KEY_DISP_ICON, default.disp_icon),
            multi_line: get(KEY_MULTI_LINE, default.multi_line),
            position: profile
                .get_i32("Common", KEY_POSITION)
                .map_or(default.position, TabPosition::from_code),
            max_width: profile
                .get_i32("Common", KEY_MAX_WIDTH)
                .unwrap_or(default.max_width),
            min_width: profile
                .get_i32("Common", KEY_MIN_WIDTH)
                .unwrap_or(default.min_width),
            min_width_multi: profile
                .get_i32("Common", KEY_MIN_WIDTH_MULTI)
                .unwrap_or(default.min_width_multi),
        }
    }
}

/// `m_sTabBar.m_lf` and `m_nPointSize` (`lfTabFont`, `lfTabFontPs`, `lfTabFaceName` of `[Common]`): the
/// font of the text of the tabs. `None` is the font of the program's windows; Sakura's default is
/// the font of icon titles of Windows (`lfIconTitle`), which other systems do not have.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TabFont(pub Option<matsu_config::settings::FontSetting>);

impl Global for TabFont {}

impl TabFont {
    /// The font of the file, if it has one with a face name.
    pub(crate) fn from_profile(profile: &Profile) -> Self {
        let mut font = crate::common_settings::model::not_set();
        font.read(
            profile,
            "Common",
            "lfTabFont",
            Some("lfTabFontPs"),
            Some("lfTabFaceName"),
        );
        Self((!font.log_font.face_name.is_empty()).then_some(font))
    }

    /// The family to ask GPUI for; `None` keeps the font of the window.
    pub(crate) fn family(&self) -> Option<String> {
        let font = self.0.as_ref()?;
        crate::settings::view_settings::font_family_for(
            &font.log_font.face_name,
            matsu_config::location::Os::current(),
        )
    }

    /// The size of the text in pixels: the point size at 96 dpi (GPUI's pixels are logical, and the
    /// height of a `LOGFONT` is in device pixels, so it would be too large on a screen of high
    /// resolution); the height is used for a font with no point size.
    pub(crate) fn size_px(&self) -> Option<f32> {
        let font = self.0.as_ref()?;
        if font.point_size <= 0 && font.log_font.height != 0 {
            return Some(font.log_font.height.unsigned_abs() as f32);
        }
        Some(crate::settings::view_settings::points_to_px(
            font.point_size_for_dpi(crate::settings::view_settings::DPI),
        ))
    }

    /// The weight of the text (`lfWeight`); 0 is normal.
    pub(crate) fn weight(&self) -> Option<f32> {
        let weight = self.0.as_ref()?.log_font.weight;
        Some(if weight > 0 { weight as f32 } else { 400.0 })
    }

    pub(crate) fn italic(&self) -> bool {
        self.0.as_ref().is_some_and(|f| f.log_font.italic != 0)
    }
}

/// The font of the tabs now.
pub(crate) fn tab_font(cx: &App) -> TabFont {
    cx.try_global::<TabFont>().cloned().unwrap_or_default()
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
            .map_or_else(TabFont::default, TabFont::from_profile),
    );
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

    #[test]
    fn the_defaults_of_the_bar_are_sakuras() {
        // CShareData::InitShareData: bDispTabIcon FALSE, bTabMultiLine false, TabPosition_Top,
        // nTabMaxWidth 200, nTabMinWidth 60, nTabMinWidthOnMulti 100.
        let options = TabOptions::from_profile(&Profile::new());
        assert_eq!(options, TabOptions::default());
        assert!(!options.disp_icon && !options.multi_line);
        assert_eq!(options.position, TabPosition::Top);
        assert_eq!(options.width_limits(), (60, 200));
        let multi = TabOptions {
            multi_line: true,
            ..options
        };
        assert_eq!(multi.width_limits(), (100, 200));
    }

    #[test]
    fn the_keys_of_the_bar_are_read() {
        let p = profile(
            "[Common]\r\nbDispTabIcon=1\r\nbTabMultiLine=1\r\neTabPosition=1\r\n\
             nTabMaxWidth=300\r\nnTabMinWidth=80\r\nnTabMinWidthOnMulti=120\r\n",
        );
        let o = TabOptions::from_profile(&p);
        assert!(o.disp_icon && o.multi_line);
        assert_eq!(o.position, TabPosition::Bottom);
        assert_eq!(
            (o.max_width, o.min_width, o.min_width_multi),
            (300, 80, 120)
        );
        assert_eq!(o.width_limits(), (120, 300), "rows have their own minimum");
    }

    #[test]
    fn a_position_sakura_does_not_place_is_the_top() {
        // Left (2) and Right (3) exist in the enum, but only Top and Bottom are placed.
        for code in [0, 2, 3, -1, 99] {
            assert_eq!(TabPosition::from_code(code), TabPosition::Top, "{code}");
        }
        assert_eq!(TabPosition::from_code(1), TabPosition::Bottom);
    }

    #[test]
    fn the_width_limits_stay_usable() {
        let wide = TabOptions {
            min_width: 500,
            max_width: 200,
            ..Default::default()
        };
        assert_eq!(wide.width_limits(), (200, 200), "the maximum wins");
        let zero = TabOptions {
            min_width: 0,
            max_width: -5,
            ..Default::default()
        };
        assert_eq!(zero.width_limits(), (1, 1));
    }
}
