//! The toolbar and the function key bar (Sakura's ツールバー and ファンクションキー).
//!
//! * [`BarSettings`]: whether each bar is shown, and the place and the grouping of the function
//!   keys (`bDispTOOLBAR`, `bDispFUNCKEYWND`, `nFUNCKEYWND_Place`, `nFUNCKEYWND_GroupNum` of
//!   `[Common]`). `F_SHOWTOOLBAR` and `F_SHOWFUNCKEY` change them and write the file; the
//!   common settings dialog changes them with OK, for every window at once.
//! * [`toolbar`]: the buttons of `[Toolbar]` (Sakura's default list without settings), each with
//!   its icon, tooltip and the function it runs.
//! * [`funckeys`]: the 12 buttons F1-F12 labelled with the functions the key map gives them for
//!   the modifiers held now.
//! * [`icons`]: the pictures, cut out of Sakura's `mytool.bmp` (see its `NOTICE`).

pub(crate) mod funckeys;
pub(crate) mod icons;
mod open_menu;
pub(crate) mod search_box;
mod semantics;
mod tool_buttons;
pub(crate) mod toolbar;

#[cfg(test)]
mod tests;

use gpui::{App, Global};
use matsu_config::Profile;

/// `bDispTOOLBAR`.
const KEY_TOOLBAR: &str = "bDispTOOLBAR";
/// `bDispFUNCKEYWND`.
const KEY_FUNCKEYS: &str = "bDispFUNCKEYWND";
/// `nFUNCKEYWND_Place`.
const KEY_PLACE: &str = "nFUNCKEYWND_Place";
/// `nFUNCKEYWND_GroupNum`.
const KEY_GROUP_NUM: &str = "nFUNCKEYWND_GroupNum";

/// Where the function key bar is (`m_nFUNCKEYWND_Place`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum FuncKeyPlace {
    /// 0: under the tool bar, above the document.
    #[default]
    Top,
    /// 1: under the document.
    Bottom,
}

/// What the common settings say about the two bars. Sakura's defaults
/// (`CShareData::InitShareData`): the tool bar is shown, the function keys are not, at the top, in
/// groups of 4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BarSettings {
    /// `m_bDispTOOLBAR`.
    pub toolbar: bool,
    /// `m_bDispFUNCKEYWND`.
    pub funckeys: bool,
    /// `m_nFUNCKEYWND_Place`.
    pub funckey_place: FuncKeyPlace,
    /// `m_nFUNCKEYWND_GroupNum`, 1 to 12.
    pub funckey_group_num: usize,
}

impl Default for BarSettings {
    fn default() -> Self {
        Self {
            toolbar: true,
            funckeys: false,
            funckey_place: FuncKeyPlace::Top,
            funckey_group_num: 4,
        }
    }
}

impl Global for BarSettings {}

impl BarSettings {
    pub(crate) fn from_profile(profile: &Profile) -> Self {
        let default = Self::default();
        let get = |key: &str, default: bool| profile.get_bool("Common", key).unwrap_or(default);
        Self {
            toolbar: get(KEY_TOOLBAR, default.toolbar),
            funckeys: get(KEY_FUNCKEYS, default.funckeys),
            funckey_place: place_of(profile.get_i32("Common", KEY_PLACE).unwrap_or(0)),
            funckey_group_num: group_num_of(profile.get_i32("Common", KEY_GROUP_NUM).unwrap_or(4)),
        }
    }
}

/// `CFuncKeyWnd::Open`: only 1 is the bottom, whatever else is read is the top.
fn place_of(value: i32) -> FuncKeyPlace {
    if value == 1 {
        FuncKeyPlace::Bottom
    } else {
        FuncKeyPlace::Top
    }
}

/// `CFuncKeyWnd::Open`: a group size outside 1 to 12 is 4.
fn group_num_of(value: i32) -> usize {
    if (1..=12).contains(&value) {
        value as usize
    } else {
        4
    }
}

/// Starts the settings and the toolbar's buttons from the file that was read.
pub(crate) fn install(cx: &mut App, user: &crate::settings::UserSettings) {
    icons::install(user);
    toolbar::install(cx, user);
    cx.set_global(
        user.profile()
            .map_or_else(BarSettings::default, BarSettings::from_profile),
    );
}

/// The settings now. Sakura's defaults before the application installs them.
pub(crate) fn current(cx: &App) -> BarSettings {
    cx.try_global::<BarSettings>().copied().unwrap_or_default()
}

/// Takes `settings` (the windows notice with `observe_global`), without writing the file: the
/// common settings dialog has written it already.
pub(crate) fn set(cx: &mut App, settings: BarSettings) {
    cx.set_global(settings);
}

/// Changes the settings for every window and writes the keys that changed to `[Common]` of the
/// settings file.
pub(crate) fn update(cx: &mut App, change: impl FnOnce(&mut BarSettings)) {
    let before = current(cx);
    let mut settings = before;
    change(&mut settings);
    cx.set_global(settings);
    crate::history::write_settings(cx, |profile| {
        let place = |s: BarSettings| match s.funckey_place {
            FuncKeyPlace::Top => "0",
            FuncKeyPlace::Bottom => "1",
        };
        let flag = |on: bool| if on { "1" } else { "0" };
        let group_num = settings.funckey_group_num.to_string();
        let mut changed = false;
        for (key, value, is_new) in [
            (
                KEY_TOOLBAR,
                flag(settings.toolbar),
                settings.toolbar != before.toolbar,
            ),
            (
                KEY_FUNCKEYS,
                flag(settings.funckeys),
                settings.funckeys != before.funckeys,
            ),
            (
                KEY_PLACE,
                place(settings),
                settings.funckey_place != before.funckey_place,
            ),
            (
                KEY_GROUP_NUM,
                group_num.as_str(),
                settings.funckey_group_num != before.funckey_group_num,
            ),
        ] {
            if is_new && profile.get("Common", key) != Some(value) {
                profile.set("Common", key, value)?;
                changed = true;
            }
        }
        Ok(changed)
    });
}

/// The keys of the settings now, as the common settings dialog starts from them.
pub(crate) fn put_into(profile: &mut Profile, settings: BarSettings) {
    let mut put = |key: &str, value: &str| {
        // A key that cannot be written to the copy is left as the file has it.
        let _ = profile.set("Common", key, value);
    };
    put(KEY_TOOLBAR, if settings.toolbar { "1" } else { "0" });
    put(KEY_FUNCKEYS, if settings.funckeys { "1" } else { "0" });
    put(
        KEY_PLACE,
        match settings.funckey_place {
            FuncKeyPlace::Top => "0",
            FuncKeyPlace::Bottom => "1",
        },
    );
    put(KEY_GROUP_NUM, &settings.funckey_group_num.to_string());
}

/// Whether the program has a command for Sakura's function `code` (a button for any other is
/// grayed out).
pub(crate) fn is_available(code: u16) -> bool {
    use crate::menu::Available as _;
    code != 0 && crate::menu::Registered.is_available(code)
}

/// How the shortcuts are written: as the menus do.
pub(crate) fn display_style() -> matsu_commands::keymap::DisplayStyle {
    if cfg!(target_os = "macos") {
        matsu_commands::keymap::DisplayStyle::Mac
    } else {
        matsu_commands::keymap::DisplayStyle::Sakura
    }
}

/// `CMenuDrawer::GetIconIdByFuncId`: the icon of function `code`, the picture of the first
/// toolbar button of the table that stands for it. Functions with no button, and the
/// placeholders up to `F_DUMMY_MAX_CODE`, have none.
pub(crate) fn icon_of_function(code: u16) -> Option<usize> {
    if code <= 10 {
        return None;
    }
    tool_buttons::FUNCTIONS_OF_BUTTONS
        .iter()
        .position(|&c| c == code)
}

/// The function of the toolbar button with the table index `index` (button number minus one), or
/// `None` past the end of the table. 0 is a free place with no function.
pub(crate) fn button_function(index: usize) -> Option<u16> {
    tool_buttons::FUNCTIONS_OF_BUTTONS.get(index).copied()
}
