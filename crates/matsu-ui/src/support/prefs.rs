//! The settings of 共通設定 / 支援 (`CommonSetting_Helper`, `sakura_core/env/CommonSetting.h`; the
//! keys are in `[Common]`, `CShareData_IO::ShareData_IO_Common`) that the windows use: the keys
//! that choose a candidate of the input completion, and the external help files.
//!
//! The font of the keyword help and the paths of migemo are on the page too; the tip reads its
//! font from the file ([`crate::key_help`]) and there is no migemo.

use gpui::{App, Global};
use matsu_config::settings::FontSetting;

/// What the page says. Sakura's defaults (`CShareData::InitShareData`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HelperPrefs {
    /// `m_bHokanKey_RETURN`: Enter chooses the candidate.
    pub hokan_key_return: bool,
    /// `m_bHokanKey_TAB`: Tab chooses the candidate.
    pub hokan_key_tab: bool,
    /// `m_bHokanKey_RIGHT`: the right arrow chooses the candidate.
    pub hokan_key_right: bool,
    /// `m_szExtHelp`: the external help file (`F_EXTHELP1`).
    pub ext_help: String,
    /// `m_szExtHtmlHelp`: the external HTML help file (`F_EXTHTMLHELP`).
    pub ext_html_help: String,
    /// `m_bHtmlHelpIsSingle`: one viewer for the HTML help.
    pub html_help_is_single: bool,
    /// `m_lf`, `m_nPointSize`: the font of the keyword help tip (`None`: the font of the text).
    pub key_help_font: Option<FontSetting>,
}

impl Default for HelperPrefs {
    fn default() -> Self {
        Self {
            hokan_key_return: true,
            hokan_key_tab: false,
            hokan_key_right: true,
            ext_help: String::new(),
            ext_html_help: String::new(),
            html_help_is_single: true,
            key_help_font: None,
        }
    }
}

impl Global for HelperPrefs {}

/// The settings now (the defaults before the settings are installed).
pub(crate) fn current(cx: &App) -> HelperPrefs {
    cx.try_global::<HelperPrefs>().cloned().unwrap_or_default()
}
