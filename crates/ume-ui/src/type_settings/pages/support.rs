//! 「支援」: `IDD_PROP_SUPPORT` (`sakura_core/sakura_rc.rc`), `CPropTypesSupport`
//! (`sakura_core/typeprop/CPropTypesSupport.cpp`).
//!
//! Live: the input completion (入力補完): the word file, 英大文字小文字を同一視, and the sources
//! of the candidates, the words of the document and the highlighted keywords
//! (`crate::support::completion`).
//!
//! Also live: the external help files of the type (`F_EXTHELP1`, `F_EXTHTMLHELP` take them before
//! the ones of the common settings, `crate::support::help`).
//!
//! Not available yet: the plugin that adds candidates (the list has only なし), the warning about
//! mixed line endings on saving (`CCodeChecker::OnCheckSave`), the C/C++ smart indent options
//! (UME Editor has no smart indent) and the detection of the indentation style on loading
//! (`CDocEditor::OnAfterLoad`).

use super::super::ids::*;
use crate::common_settings::CommonSettingsDialog;
use crate::common_settings::model::{Control, Item};

/// `GetHokanMethodList`: the plugins that complete are added to the list after なし
/// (`STR_SMART_INDENT_NONE`); UME Editor has no plugins that complete.
pub(in crate::type_settings) const HOKAN_TYPES: &[(&str, i32)] = &[("なし", 0)];

#[cfg(test)]
pub(in crate::type_settings) use crate::support::browse::path_for_box;

/// `IDC_BUTTON_HOKANFILE_REF`: chooses the word file.
fn browse_word_file(
    dialog: &mut CommonSettingsDialog,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) {
    crate::support::browse::browse_into(dialog, window, cx, HOKAN_FILE, "単語ファイルの選択");
}

/// `IDC_BUTTON_TYPEOPENHELP1`: chooses the external help file (`*.hlp;*.chm;*.col`).
fn browse_ext_help(
    dialog: &mut CommonSettingsDialog,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) {
    crate::support::browse::browse_into(dialog, window, cx, EXT_HELP, "外部ヘルプファイルの選択");
}

/// `IDC_BUTTON_TYPEOPENEXTHTMLHELP`: chooses the external HTML help file (`*.chm;*.col`).
fn browse_ext_html_help(
    dialog: &mut CommonSettingsDialog,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) {
    crate::support::browse::browse_into(
        dialog,
        window,
        cx,
        EXT_HTML_HELP,
        "外部HTMLヘルプファイルの選択",
    );
}

pub(in crate::type_settings) fn items() -> Vec<Item> {
    vec![
        Item::Group(
            "入力補完機能",
            vec![
                Item::Row(vec![
                    Control::text(HOKAN_FILE, "単語ファイル(&W)", "", "").into(),
                    Control::button(HOKAN_FILE_BROWSE, "(&1)...", browse_word_file)
                        .named("単語ファイルを参照")
                        .into(),
                    Control::choice(HOKAN_TYPE, "", "", 0, HOKAN_TYPES)
                        .named("入力補完機能の種類")
                        .unavailable()
                        .into(),
                ]),
                Item::Row(vec![
                    Control::check(HOKAN_LO_HI_CASE, "英大文字小文字を同一視(&I)", "", false)
                        .into(),
                    Item::Label("候補："),
                    Control::check(HOKAN_BY_FILE, "編集中のファイル(&F)", "", true).into(),
                    Control::check(HOKAN_BY_KEYWORD, "強調キーワード(&K)", "", false).into(),
                ]),
            ],
        ),
        Item::Group(
            "外部ヘルプの設定(&L)",
            vec![Item::Row(vec![
                Control::text(EXT_HELP, "", "", "")
                    .named("外部ヘルプの設定")
                    .into(),
                Control::button(EXT_HELP_BROWSE, "(&2)...", browse_ext_help)
                    .named("外部ヘルプの設定を参照")
                    .into(),
            ])],
        ),
        Item::Group(
            "外部HTMLヘルプの設定(&P)",
            vec![
                Item::Row(vec![
                    Control::text(EXT_HTML_HELP, "", "", "")
                        .named("外部HTMLヘルプの設定")
                        .into(),
                    Control::button(EXT_HTML_HELP_BROWSE, "(&3)...", browse_ext_html_help)
                        .named("外部HTMLヘルプの設定を参照")
                        .into(),
                ]),
                Control::check(HTML_HELP_SINGLE, "ビューアを複数起動しない(&N)", "", true).into(),
            ],
        ),
        Control::check(
            CHK_ENTER_AT_END,
            "保存時に改行コードの混在を警告する(&E)",
            "",
            true,
        )
        .unavailable()
        .into(),
        Item::Group(
            "C/C++インデント詳細設定",
            vec![
                Item::Row(vec![
                    Control::check(INDENT_CPP_STRING, "文字列を無視する(&S)", "", true)
                        .unavailable()
                        .into(),
                    Control::check(INDENT_CPP_COMMENT, "コメントを無視する(&C)", "", true)
                        .unavailable()
                        .into(),
                ]),
                Control::check(INDENT_CPP_UNDO, "Undoバッファを分ける(&U)", "", false)
                    .unavailable()
                    .into(),
            ],
        ),
        Control::check(
            DETECT_INDENT,
            "ファイル読み込み時にインデントスタイルを検出する(&T)",
            "",
            true,
        )
        .unavailable()
        .into(),
    ]
}
