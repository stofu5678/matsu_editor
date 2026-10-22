//! The settings of 共通設定 / ステータスバー (`CommonSetting_Statusbar`,
//! `sakura_core/env/CommonSetting.h`; the keys are in `[Statusbar]`,
//! `CShareData_IO::ShareData_IO_Statusbar`), and the way a window counts the selection
//! (`CEditWnd::m_nSelectCountMode`).

use gpui::{App, Global};
use matsu_core::encoding::CharCodeOptions;

/// What the page says. Sakura's defaults (`CShareData::InitShareData`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StatusbarOptions {
    /// The five check boxes of 表示文字コードの指定.
    pub char_code: CharCodeOptions,
    /// `m_bDispSelCountByByte`: the selection is counted in bytes, not in characters.
    pub select_count_by_byte: bool,
    /// `m_bDispColByChar`: the column counts characters, not the units of the ruler.
    pub column_by_char: bool,
}

impl Global for StatusbarOptions {}

/// The options now (the defaults before the settings are installed).
pub(crate) fn current(cx: &App) -> StatusbarOptions {
    cx.try_global::<StatusbarOptions>()
        .copied()
        .unwrap_or_default()
}

/// `ESelectCountMode` (`sakura_core/_main/global.h`): how the window counts the selection.
/// `Setting` (`SELECT_COUNT_TOGGLE`, the start value) follows the common setting; the function
/// `F_SELECT_COUNT_MODE` changes it for the window without writing the setting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SelectCountMode {
    #[default]
    Setting,
    /// `SELECT_COUNT_BY_CHAR`
    Chars,
    /// `SELECT_COUNT_BY_BYTE`
    Bytes,
}

impl SelectCountMode {
    /// Whether the selection is counted in bytes, given the common setting
    /// (`bCountByByte` in `CViewSelect::PrintSelectionInfoMsg`, and `IsFuncChecked`).
    pub(crate) fn by_byte(self, setting: bool) -> bool {
        match self {
            Self::Setting => setting,
            Self::Chars => false,
            Self::Bytes => true,
        }
    }

    /// `Command_SELECT_COUNT_MODE` with `SELECT_COUNT_TOGGLE`: characters become bytes and bytes
    /// become characters.
    pub(crate) fn toggled(self, setting: bool) -> Self {
        if self.by_byte(setting) {
            Self::Chars
        } else {
            Self::Bytes
        }
    }

    /// The mode a number of the function asks for: 0 toggles, 1 is characters and 2 is bytes
    /// (`Command_SELECT_COUNT_MODE`, `sakura_core/cmd/CViewCommander_Settings.cpp`). Another
    /// number changes nothing.
    pub(crate) fn after_code(self, code: i32, setting: bool) -> Self {
        match code {
            0 => self.toggled(setting),
            1 => Self::Chars,
            2 => Self::Bytes,
            _ => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mode_toggles_between_characters_and_bytes() {
        // Command_SELECT_COUNT_MODE: from the setting, the opposite of the setting.
        assert_eq!(
            SelectCountMode::Setting.toggled(false),
            SelectCountMode::Bytes
        );
        assert_eq!(
            SelectCountMode::Setting.toggled(true),
            SelectCountMode::Chars
        );
        assert_eq!(
            SelectCountMode::Bytes.toggled(false),
            SelectCountMode::Chars
        );
        assert_eq!(SelectCountMode::Chars.toggled(true), SelectCountMode::Bytes);
    }

    #[test]
    fn the_mode_follows_the_setting_until_it_is_set() {
        assert!(!SelectCountMode::Setting.by_byte(false));
        assert!(SelectCountMode::Setting.by_byte(true));
        assert!(SelectCountMode::Bytes.by_byte(false));
        assert!(!SelectCountMode::Chars.by_byte(true));
    }

    #[test]
    fn the_numbers_of_the_function() {
        let m = SelectCountMode::Setting;
        assert_eq!(m.after_code(0, false), SelectCountMode::Bytes);
        assert_eq!(m.after_code(1, true), SelectCountMode::Chars);
        assert_eq!(m.after_code(2, false), SelectCountMode::Bytes);
        // `else if( nMode == SELECT_COUNT_BY_BYTE || nMode == SELECT_COUNT_BY_CHAR )`: others do nothing.
        assert_eq!(m.after_code(7, true), SelectCountMode::Setting);
    }
}
