//! 「支援」: `IDD_PROP_HELPER` (`sakura_core/sakura_rc.rc`), `CPropHelper`
//! (`sakura_core/prop/CPropComHelper.cpp`). The keys are in `[Common]`
//! (`CShareData_IO::ShareData_IO_Common`).
//!
//! Live: the keys that choose a candidate of the input completion (Enter, Tab, →,
//! [`crate::support::completion`]), and the external help file and the external HTML help file with
//! the choice of one viewer (`F_EXTHELP1`, `F_EXTHTMLHELP`, [`crate::support::help`]). A type can
//! have its own help files (the type settings, 支援).
//!
//! The font of the keyword help (`khlf`, `khps`) is chosen with the font dialog, shown next to
//! the button as `SetFontLabel` does, and used by the tip from OK on ([`crate::key_help`]). Not
//! available: the DLL and the dictionary of migemo (UME Editor has no migemo).

use super::super::CommonSettingsDialog;
use ume_config::settings::FontSetting;

use super::super::Env;
use super::super::model::{Control, Item, Values};
use crate::support::browse::browse_into;
use crate::support::prefs::HelperPrefs;

const EXT_HELP: &str = "helper_ext_help";
const EXT_HTML_HELP: &str = "helper_ext_html_help";

/// `IDC_BUTTON_OPENHELP1`: chooses the external help file (`*.hlp;*.chm;*.col`).
fn browse_ext_help(
    dialog: &mut CommonSettingsDialog,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) {
    browse_into(dialog, window, cx, EXT_HELP, "外部ヘルプファイルの選択");
}

/// `IDC_BUTTON_OPENEXTHTMLHELP`: chooses the external HTML help file (`*.chm;*.col`).
fn browse_ext_html_help(
    dialog: &mut CommonSettingsDialog,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) {
    browse_into(
        dialog,
        window,
        cx,
        EXT_HTML_HELP,
        "外部HTMLヘルプファイルの選択",
    );
}

/// The font of the keyword help (`m_sHelper.m_lf`, `m_nPointSize`): not drawn, the button and the
/// label use it.
const FONT: &str = "helper_font_value";

/// `IDC_BUTTON_KEYWORDHELPFONT` (`CPropHelper::DispatchEvent`): the font dialog starts with the
/// font of the file; when it has none, with the default font.
fn choose_font(
    dialog: &mut CommonSettingsDialog,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) {
    let font = dialog.values().font(FONT);
    let initial = if font.log_font.face_name.is_empty() {
        FontSetting::default()
    } else {
        font
    };
    dialog.choose_font(FONT, initial, window, cx);
}

/// `IDC_STATIC_KEYWORDHELPFONT`: the face and the size of the font.
fn font_label(
    dialog: &CommonSettingsDialog,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) -> gpui::AnyElement {
    let font = dialog.values().font(FONT);
    super::super::model::font_label(Some(&font), cx)
}

fn no_action(
    _: &mut CommonSettingsDialog,
    _: &mut gpui::Window,
    _: &mut gpui::Context<CommonSettingsDialog>,
) {
}

pub(in crate::common_settings) fn items() -> Vec<Item> {
    vec![
        Item::Group(
            "入力補完機能",
            vec![Item::Row(vec![
                Item::Label("候補決定キー"),
                Control::check("helper_hokan_return", "&Enter", "bHokanKey_RETURN", true).into(),
                Control::check("helper_hokan_tab", "&Tab", "bHokanKey_TAB", false).into(),
                Control::check("helper_hokan_right", "→(&R)", "bHokanKey_RIGHT", true).into(),
            ])],
        ),
        Item::Group(
            "外部ヘルプの設定(&L)",
            vec![Item::Row(vec![
                Control::text(EXT_HELP, "", "szExtHelp", "").into(),
                Control::button("helper_ext_help_ref", "(&1)...", browse_ext_help).into(),
            ])],
        ),
        Item::Group(
            "外部HTMLヘルプの設定(&P)",
            vec![
                Item::Row(vec![
                    Control::text(EXT_HTML_HELP, "", "szExtHtmlHelp", "").into(),
                    Control::button("helper_ext_html_help_ref", "(&2)...", browse_ext_html_help)
                        .into(),
                ]),
                Control::check(
                    "helper_html_help_single",
                    "ビューアを複数起動しない(&N)",
                    "bHtmlHelpIsSingle",
                    true,
                )
                .into(),
            ],
        ),
        Item::Group(
            "キーワードヘルプ",
            vec![Item::Row(vec![
                Item::Custom(font_label),
                Control::button("helper_font_button", "フォント(&F)...", choose_font).into(),
                Item::Hidden(vec![Control::font(FONT, "khlf", Some("khps"), None)]),
            ])],
        ),
        Item::Group(
            "migemo設定",
            vec![
                Item::Row(vec![
                    Control::text("helper_migemo_dll", "DLL(&M)", "szMigemoDll", "")
                        .unavailable()
                        .into(),
                    Control::button("helper_migemo_dll_ref", "(&3)...", no_action)
                        .unavailable()
                        .into(),
                ]),
                Item::Row(vec![
                    Control::text("helper_migemo_dict", "辞書(&I)", "szMigemoDict", "")
                        .unavailable()
                        .into(),
                    Control::button("helper_migemo_dict_ref", "(&4)...", no_action)
                        .unavailable()
                        .into(),
                ]),
            ],
        ),
    ]
}

/// The settings of the page as the windows use them.
fn prefs(values: &Values) -> HelperPrefs {
    HelperPrefs {
        hokan_key_return: values.bool("helper_hokan_return"),
        hokan_key_tab: values.bool("helper_hokan_tab"),
        hokan_key_right: values.bool("helper_hokan_right"),
        ext_help: values.text(EXT_HELP).to_owned(),
        ext_html_help: values.text(EXT_HTML_HELP).to_owned(),
        html_help_is_single: values.bool("helper_html_help_single"),
        key_help_font: Some(values.font(FONT)).filter(|f| !f.log_font.face_name.is_empty()),
    }
}

/// What OK does to the windows: the keys of the completion list and the help files are the new
/// ones from the next key and the next call.
pub(in crate::common_settings) fn apply(env: &mut Env, after: &Values, _before: Option<&Values>) {
    env.cx.set_global(prefs(after));
}

#[cfg(test)]
mod tests;
