//! 「正規表現キーワード」: `IDD_PROP_REGEX` (`sakura_core/sakura_rc.rc`), `CPropTypesRegex`
//! (`sakura_core/typeprop/CPropTypesRegex.cpp`).
//!
//! Live: 正規表現キーワードを使用する (`bUseRegexKeyword`; the coloring reads it,
//! `ume_coloring::regex_keyword`).
//!
//! Not available yet: the list of the keywords and the controls that edit it (the pattern, the
//! color, 挿入, 追加, 更新, 削除, 先頭, 上へ, 下へ, 最終, インポート, エクスポート). They need a list control
//! that the settings dialog does not have yet. The keywords the file has (`RxKey[NNN]`) are kept
//! as they are.

use std::sync::OnceLock;

use ume_config::settings::ColorSet;

use super::super::ids::*;
use super::color::COLOR_ITEMS;
use crate::common_settings::CommonSettingsDialog;
use crate::common_settings::model::{Control, Item};

/// The pattern the box starts with (`SetData`: `//k`).
pub(in crate::type_settings) const NEW_PATTERN: &str = "//k";

/// The colors a keyword may have: the items that have both a text color and a background
/// (`SetData`: not `COLOR_ATTRIB_NO_TEXT` and not `COLOR_ATTRIB_NO_BACK`), as the label and the
/// number in `COLOR_ITEMS`.
pub(in crate::type_settings) fn color_options() -> &'static [(&'static str, i32)] {
    static OPTIONS: OnceLock<Vec<(&'static str, i32)>> = OnceLock::new();
    OPTIONS.get_or_init(|| {
        ColorSet::all_names()
            .zip(COLOR_ITEMS)
            .filter(|(name, _)| {
                ColorSet::attributes(name).is_some_and(|a| !a.no_text && !a.no_back)
            })
            .map(|(_, item)| *item)
            .collect()
    })
}

/// The number of 正規表現キーワード1 in `COLOR_ITEMS` (`COLORIDX_REGEX1`), which the color box
/// starts with.
pub(in crate::type_settings) fn first_regex_color() -> i32 {
    ColorSet::all_names()
        .position(|name| name == "RK1")
        .and_then(|position| i32::try_from(position).ok())
        .unwrap_or(0)
}

fn no_action(
    _: &mut CommonSettingsDialog,
    _: &mut gpui::Window,
    _: &mut gpui::Context<CommonSettingsDialog>,
) {
}

fn button(id: &'static str, label: &'static str) -> Item {
    Control::button(id, label, no_action).unavailable().into()
}

pub(in crate::type_settings) fn items() -> Vec<Item> {
    vec![
        Item::Row(vec![
            Control::check(USE_REGEX, "正規表現キーワードを使用する(&R)", "", false).into(),
            Item::Label("正規表現エンジン: Onigmo"),
        ]),
        Item::Group(
            "正規表現キーワード(&K)",
            vec![
                Item::Label("キーワードの一覧は未対応です (ファイルのキーワードは変わりません)"),
                Item::Row(vec![
                    button(REGEX_TOP, "先頭(&T)"),
                    button(REGEX_UP, "上へ(&U)"),
                    button(REGEX_DOWN, "下へ(&G)"),
                    button(REGEX_LAST, "最終(&B)"),
                ]),
                Item::Row(vec![
                    Control::text(REGEX_PATTERN, "正規表現(&N)", "", NEW_PATTERN)
                        .unavailable()
                        .into(),
                ]),
                Item::Row(vec![
                    Control::choice(REGEX_COLOR, "色指定(&C)", "", 0, color_options())
                        .unavailable()
                        .into(),
                ]),
                Item::Row(vec![
                    button(REGEX_INS, "挿入(&S)"),
                    button(REGEX_ADD, "追加(&A)"),
                    button(REGEX_UPD, "更新(&E)"),
                    button(REGEX_DEL, "削除(&D)"),
                ]),
            ],
        ),
        Item::Row(vec![
            Item::Label("[色指定]で「URL」を選択するとマッチ文字列がクリッカブルになります"),
            button(REGEX_IMPORT, "インポート(&I)..."),
            button(REGEX_EXPORT, "エクスポート(&X)..."),
        ]),
    ]
}
