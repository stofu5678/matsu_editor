//! 「キーワードヘルプ」: `IDD_PROP_KEYHELP` (`sakura_core/sakura_rc.rc`), `CPropTypesKeyHelp`
//! (`sakura_core/typeprop/CPropTypesKeyHelp.cpp`).
//!
//! Not available yet: all of it. UME Editor has no keyword help (the tip with the meaning of the
//! word under the mouse or the caret, `CEditView::KeySearchCore`), so the controls show what the
//! file has (`bUseKeyWordHelp`, `bUseKeyHelpAllSearch`, `bUseKeyHelpKeyDisp`, `bUseKeyHelpPrefix`,
//! `nKeyHelpRMenuShowType`) and nothing is written. The list of the dictionaries (`KDct[NN]`) needs
//! a list control that the settings dialog does not have yet; the dictionaries the file has are
//! kept as they are.

use super::super::ids::*;
use crate::common_settings::CommonSettingsDialog;
use crate::common_settings::model::{Control, Item, Values};

/// `STR_KEYHELP_RMENU_NONE`, `_TOP` and `_BOTTOM` (`EKeyHelpRMenuType`): whether the right-click
/// menu shows the meaning of the word, and where.
pub(in crate::type_settings) const RMENU_TYPES: &[(&str, i32)] =
    &[("表示しない", 0), ("先頭に表示", 1), ("末尾に表示", 2)];

/// `KEYHELP_RMENU_TOP`: the default (`CType.cpp`).
pub(in crate::type_settings) const RMENU_DEFAULT: i32 = 1;

/// `IDC_CHECK_KEYHELP` enables the other controls (`BN_CLICKED` of it).
fn keyhelp_is_used(values: &Values) -> bool {
    values.bool(USE_KEYHELP)
}

fn no_action(
    _: &mut CommonSettingsDialog,
    _: &mut gpui::Window,
    _: &mut gpui::Context<CommonSettingsDialog>,
) {
}

fn button(id: &'static str, label: &'static str) -> Item {
    Control::button(id, label, no_action)
        .unavailable()
        .when(keyhelp_is_used)
        .into()
}

fn check(id: &'static str, label: &'static str) -> Item {
    Control::check(id, label, "", false)
        .unavailable()
        .when(keyhelp_is_used)
        .into()
}

pub(in crate::type_settings) fn items() -> Vec<Item> {
    vec![
        Control::check(USE_KEYHELP, "キーワードヘルプ機能を使う(&K)", "", false)
            .unavailable()
            .into(),
        Item::Group(
            "辞書ファイル一覧(&L)",
            vec![
                Item::Label("辞書の一覧は未対応です (ファイルの辞書は変わりません)  ↑優先度(高)"),
                Item::Row(vec![
                    button(KEYHELP_TOP, "先頭(&T)"),
                    button(KEYHELP_UP, "上へ(&U)"),
                    button(KEYHELP_DOWN, "下へ(&G)"),
                    button(KEYHELP_LAST, "最終(&B)"),
                ]),
                Item::Row(vec![
                    Control::text(KEYHELP_PATH, "辞書ファイル", "", "")
                        .unavailable()
                        .when(keyhelp_is_used)
                        .into(),
                    button(KEYHELP_REF, "..."),
                ]),
                Item::Row(vec![
                    button(KEYHELP_INS, "挿入(&S)"),
                    button(KEYHELP_UPD, "更新(&E)"),
                    button(KEYHELP_DEL, "削除(&D)"),
                ]),
            ],
        ),
        Item::Row(vec![
            check(KEYHELP_ALL_SEARCH, "ヒットした次の辞書も検索(&A)"),
            button(KEYHELP_IMPORT, "インポート(&I)..."),
            button(KEYHELP_EXPORT, "エクスポート(&X)..."),
        ]),
        check(KEYHELP_KEY_DISP, "キーワードも表示する(&W)"),
        Item::Row(vec![
            check(KEYHELP_PREFIX, "選択範囲で前方一致検索(&P)"),
            Control::choice(
                KEYHELP_RMENU,
                "右クリックメニュー(&R)",
                "",
                RMENU_DEFAULT,
                RMENU_TYPES,
            )
            .unavailable()
            .when(keyhelp_is_used)
            .into(),
        ]),
    ]
}
