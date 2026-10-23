//! 「正規表現キーワード」: `IDD_PROP_REGEX` (`sakura_core/sakura_rc.rc`), `CPropTypesRegex`
//! (`sakura_core/typeprop/CPropTypesRegex.cpp`).
//!
//! Live: 正規表現キーワードを使用する (`bUseRegexKeyword`) and the list of the keywords
//! (`RxKey[NNN]`) with the edit boxes and the buttons 先頭, 上へ, 下へ, 最終, 挿入, 追加, 更新, 削除
//! (the editable list of the settings, [`crate::common_settings::table`]). The coloring reads both
//! (`matsu_coloring::regex_keyword`).
//!
//! The rules of the buttons are those of the list (`CPropFileName` and `CPropTypesRegex` are the
//! same); a keyword is checked before it is put in (`CheckKeywordList`): it must be framed as
//! `/.../k` (`RegexKeyCheckSyntax`) and must not be there already. Sakura also compiles the
//! pattern and asks whether to keep a keyword that does not compile; MATSU Editor keeps it, and the
//! coloring leaves out what does not compile.
//!
//! The color is typed in an edit box here (Sakura has a drop-down list): a name that is not in
//! the list of colors a keyword may have becomes 正規表現キーワード1.
//!
//! インポート and エクスポート read and write `.rkw` files ([`super::regex_file`]).

use std::path::Path;
use std::sync::OnceLock;

use matsu_config::Profile;
use matsu_config::settings::ColorSet;
use matsu_config::settings::type_rules::MAX_REGEX_KEYWORDS;

use super::super::ids::*;
use super::color::COLOR_ITEMS;
use crate::common_settings::model::{Control, Item};
use crate::common_settings::table::{Column, Op, Row, Storage, TableSpec};

/// `MAX_REGEX_KEYWORDLEN - 1`: the longest keyword.
const KEYWORD_CHARS: usize = 999;

/// The list. The type sheet keeps the rows in the type (`values.rs`), not by `storage`.
pub(in crate::type_settings) static LIST: TableSpec = TableSpec {
    columns: &[
        Column {
            title: "キーワード",
            edit_label: "正規表現(&N)",
            key: "",
            width: 54,
            max_chars: KEYWORD_CHARS,
        },
        Column {
            title: "色指定",
            edit_label: "色指定(&C)",
            key: "",
            width: 38,
            max_chars: 255,
        },
    ],
    max_rows: MAX_REGEX_KEYWORDS,
    ops: &Op::ALL,
    defaults: &[],
    storage: Storage::Custom {
        load: |_: &Profile| None,
        store: |_, _| Ok(false),
    },
    // `STR_PROPTYPEREGEX_NOREG`
    full_message: "これ以上登録できません。",
    prepare: Some(prepare),
};

/// `STR_PROPTYPEREGEX_KAKOMI`
const NOT_FRAMED: &str = "正規表現キーワードを / と /k で囲ってください。\nキーワードに / がある場合は m# と #k で囲ってください。";
/// `STR_PROPTYPEREGEX_ALREADY`
const ALREADY: &str = "同じキーワードで登録済みです。";

/// `CPropTypesRegex::CheckKeywordList`: the keyword is framed, and no other row has it.
fn prepare(
    row: &mut Row,
    rows: &[Row],
    updating: Option<usize>,
    _base: Option<&Path>,
) -> Result<(), String> {
    if !matsu_coloring::has_keyword_form(&row[0]) {
        return Err(NOT_FRAMED.to_owned());
    }
    let repeated = rows
        .iter()
        .enumerate()
        .any(|(i, other)| Some(i) != updating && other[0] == row[0]);
    if repeated {
        return Err(ALREADY.to_owned());
    }
    // The drop-down list of Sakura has only these colors.
    if !color_options().iter().any(|(label, _)| *label == row[1]) {
        row[1] = color_label("RK1").to_owned();
    }
    Ok(())
}

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

/// The label of the color entry `name` (`TXT`, `RK1` ...) in the list of colors; 正規表現キーワード1
/// for a name that is none.
pub(in crate::type_settings) fn color_label(name: &str) -> &'static str {
    let find = |wanted: &str| {
        ColorSet::all_names()
            .zip(COLOR_ITEMS)
            .find(|(n, _)| *n == wanted)
            .map(|(_, item)| item.0)
    };
    find(name).or_else(|| find("RK1")).unwrap_or_default()
}

/// The name of the color entry that has `label` in the list; `RK1` for a label that is none
/// (`GetData`: a color that is not found is `COLORIDX_REGEX1`).
pub(in crate::type_settings) fn color_name(label: &str) -> &'static str {
    ColorSet::all_names()
        .zip(COLOR_ITEMS)
        .find(|(_, item)| item.0 == label)
        .map_or("RK1", |(name, _)| name)
}

pub(in crate::type_settings) fn items() -> Vec<Item> {
    vec![
        Item::Row(vec![
            Control::check(USE_REGEX, "正規表現キーワードを使用する(&R)", "", false).into(),
            Item::Label("正規表現エンジン: Onigmo"),
        ]),
        Control::table(REGEX_LIST, "正規表現キーワード(&K)", &LIST).into(),
        Item::Row(vec![
            Item::Label("[色指定]で「URL」を選択するとマッチ文字列がクリッカブルになります"),
            Control::button(REGEX_IMPORT, "インポート(&I)...", |d, window, cx| {
                d.regex_import(window, cx)
            })
            .into(),
            Control::button(
                REGEX_EXPORT,
                "エクスポート(&X)...",
                |d, window, cx| d.regex_export(window, cx),
            )
            .into(),
        ]),
    ]
}
