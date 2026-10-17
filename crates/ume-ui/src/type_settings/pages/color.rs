//! 「カラー」: `IDD_PROP_COLOR` (`sakura_core/sakura_rc.rc`), `CPropTypesColor`
//! (`sakura_core/typeprop/CPropTypesColor.cpp`).
//!
//! The list of the color items with 色分け/表示, 太字, 下線, the text color and the background
//! color of the chosen item; the comment styles, the escape of strings, and the columns of the
//! vertical lines. The controls an item does not have are greyed out
//! (`g_ColorAttributeArr`, `ColorSet::attributes`).
//!
//! The dialog keeps the five values of every item ([`item_key`](ids::item_key)); the controls
//! show those of the chosen item, and a change of a control writes it back to the item
//! ([`sync`]). OK puts the items into the type.
//!
//! Not available yet: the choice of the keyword sets (`CDlgKeywordSelect`, the sets are in the
//! common settings 強調キーワード, which UME Editor does not have yet), 文字色統一 and 背景色統一
//! (`CDlgSameColor`), and the import and export of the colors (`CImpExpManager`).

use ume_config::settings::ColorSet;

use super::super::ids::*;
use crate::common_settings::model::{Control, Item, Values};

/// `STR_STRINGESC_*`: how a quote inside a string is written (the list in `CPropTypesColor.cpp`).
pub(in crate::type_settings) const STRING_TYPES: &[(&str, i32)] = &[
    ("C/C++言語風 \\\"", 0),
    ("PL/SQL風    \"\"", 1),
    ("HTML/XML風  =\"\"", 2),
    ("C#風        @\"\"", 3),
    ("Python風    \"\"\"", 4),
];

/// `COMMENT_DELIMITER_NUM`'s column spin: 1 to 1000 (`SetUpDownRange(IDC_SPIN_LCColNum, 1, 1000)`).
/// A column typed over it is kept, as in Sakura, where the spin only limits itself.
pub(in crate::type_settings) const COMMENT_COLUMN_MAX: i32 = i32::MAX;

/// The name on the list for every color item, in the order of `ColorSet::all_names`
/// (`ColorInfo_DEFAULT`, `sakura_core/doc/CDocTypeSetting.cpp`, with the strings of
/// `sakura_rc.rc`). The number is the position in the list.
pub(in crate::type_settings) const COLOR_ITEMS: &[(&str, i32)] = &[
    ("テキスト", 0),
    ("ルーラー", 1),
    ("カーソル", 2),
    ("カーソル(IME ON)", 3),
    ("カーソル行の背景色", 4),
    ("カーソル行アンダーライン", 5),
    ("カーソル位置縦線", 6),
    ("ノート線", 7),
    ("行番号", 8),
    ("行番号(変更行)", 9),
    ("偶数行の背景色", 10),
    ("TAB記号", 11),
    ("半角空白", 12),
    ("日本語空白", 13),
    ("コントロールコード", 14),
    ("改行記号", 15),
    ("折り返し記号", 16),
    ("指定桁縦線", 17),
    ("EOF記号", 18),
    ("半角数値", 19),
    ("対括弧の強調表示", 20),
    ("選択範囲", 21),
    ("検索文字列", 22),
    ("検索文字列2", 23),
    ("検索文字列3", 24),
    ("検索文字列4", 25),
    ("検索文字列5", 26),
    ("コメント", 27),
    ("シングルクォーテーション文字列", 28),
    ("ダブルクォーテーション文字列", 29),
    ("ヒアドキュメント", 30),
    ("URL", 31),
    ("強調キーワード1", 32),
    ("強調キーワード2", 33),
    ("強調キーワード3", 34),
    ("強調キーワード4", 35),
    ("強調キーワード5", 36),
    ("強調キーワード6", 37),
    ("強調キーワード7", 38),
    ("強調キーワード8", 39),
    ("強調キーワード9", 40),
    ("強調キーワード10", 41),
    ("正規表現キーワード1", 42),
    ("正規表現キーワード2", 43),
    ("正規表現キーワード3", 44),
    ("正規表現キーワード4", 45),
    ("正規表現キーワード5", 46),
    ("正規表現キーワード6", 47),
    ("正規表現キーワード7", 48),
    ("正規表現キーワード8", 49),
    ("正規表現キーワード9", 50),
    ("正規表現キーワード10", 51),
    ("DIFF差分表示(追加)", 52),
    ("DIFF差分表示(変更)", 53),
    ("DIFF差分表示(削除)", 54),
    ("ブックマーク", 55),
    ("表示範囲(ミニマップ)", 56),
];

/// The number of the color item that is chosen.
fn chosen(values: &Values) -> usize {
    usize::try_from(values.int(COLOR_SEL)).unwrap_or(0)
}

/// What the chosen item lacks (`LBN_SELCHANGE` greys out the controls it does not have).
fn attributes(values: &Values) -> ume_config::settings::ColorAttributes {
    ColorSet::all_names()
        .nth(chosen(values))
        .and_then(ColorSet::attributes)
        .unwrap_or_default()
}

fn can_switch_disp(values: &Values) -> bool {
    !attributes(values).force_disp
}

fn can_be_bold(values: &Values) -> bool {
    !attributes(values).no_bold
}

fn can_be_underlined(values: &Values) -> bool {
    !attributes(values).no_underline
}

fn has_text_color(values: &Values) -> bool {
    !attributes(values).no_text
}

fn has_back_color(values: &Values) -> bool {
    !attributes(values).no_back
}

/// The column of a line comment is for the delimiters whose box 桁 is checked
/// (`EnableTypesPropInput`).
const COLUMN_ENABLED: [fn(&Values) -> bool; 3] = [
    |v| v.bool(LINE_COMMENT_POS_ON[0]),
    |v| v.bool(LINE_COMMENT_POS_ON[1]),
    |v| v.bool(LINE_COMMENT_POS_ON[2]),
];

/// `IDC_CHECK_STRINGENDLINE` is for a string that ends at the end of the line.
fn string_ends_at_line_end(values: &Values) -> bool {
    values.bool(STRING_LINE_ONLY)
}

/// Shows the five values of the chosen item in the controls, or keeps a change of a control in
/// its item.
pub(in crate::type_settings) fn sync(values: &mut Values, changed: &'static str) {
    let item = chosen(values);
    match changed {
        COLOR_SEL => {
            for (field, id) in [
                (Field::Disp, COLOR_DISP),
                (Field::Bold, COLOR_BOLD),
                (Field::Underline, COLOR_UNDERLINE),
                (Field::Text, COLOR_TEXT),
                (Field::Back, COLOR_BACK),
            ] {
                if let Some(value) = values.get(item_key(item, field)).cloned() {
                    values.set(id, value);
                }
            }
        }
        COLOR_DISP | COLOR_BOLD | COLOR_UNDERLINE | COLOR_TEXT | COLOR_BACK => {
            let field = match changed {
                COLOR_DISP => Field::Disp,
                COLOR_BOLD => Field::Bold,
                COLOR_UNDERLINE => Field::Underline,
                COLOR_TEXT => Field::Text,
                _ => Field::Back,
            };
            if let Some(value) = values.get(changed).cloned() {
                values.set(item_key(item, field), value);
            }
        }
        _ => {}
    }
}

fn no_action(
    _: &mut crate::common_settings::CommonSettingsDialog,
    _: &mut gpui::Window,
    _: &mut gpui::Context<crate::common_settings::CommonSettingsDialog>,
) {
}

pub(in crate::type_settings) fn items() -> Vec<Item> {
    vec![Item::Columns(vec![
        vec![Item::Group(
            "色指定(&L)",
            vec![
                Control::list(COLOR_SEL, "", "", 0, COLOR_ITEMS)
                    .on_change(sync)
                    .into(),
                Item::Row(vec![
                    Control::check(COLOR_DISP, "色分け/表示(&D)", "", true)
                        .when(can_switch_disp)
                        .on_change(sync)
                        .into(),
                    Control::check(COLOR_BOLD, "太字(&B)", "", false)
                        .when(can_be_bold)
                        .on_change(sync)
                        .into(),
                    Control::check(COLOR_UNDERLINE, "下線(&U)", "", false)
                        .when(can_be_underlined)
                        .on_change(sync)
                        .into(),
                ]),
                Item::Row(vec![
                    Control::color(COLOR_TEXT, "文字色(C)", "", 0)
                        .when(has_text_color)
                        .on_change(sync)
                        .into(),
                    Control::button(SAME_TEXT_COLOR, "文字色統一(&<)...", no_action)
                        .unavailable()
                        .when(has_text_color)
                        .into(),
                ]),
                Item::Row(vec![
                    Control::color(COLOR_BACK, "背景色(K)", "", 0)
                        .when(has_back_color)
                        .on_change(sync)
                        .into(),
                    Control::button(SAME_BACK_COLOR, "背景色統一(&>)...", no_action)
                        .unavailable()
                        .when(has_back_color)
                        .into(),
                ]),
                Item::Row(vec![
                    Control::button(COLORS_IMPORT, "インポート(&I)...", no_action)
                        .unavailable()
                        .into(),
                    Control::button(COLORS_EXPORT, "エクスポート(&X)...", no_action)
                        .unavailable()
                        .into(),
                ]),
            ],
        )],
        vec![
            Item::Group(
                "強調キーワード",
                vec![
                    Control::text(KEYWORD1_NAME, "強調キーワード&1", "", "")
                        .unavailable()
                        .into(),
                    Item::Row(vec![
                        Control::button(KEYWORD_SELECT, "&2～10...", no_action)
                            .unavailable()
                            .into(),
                        Control::button(KEYWORD_EDIT, "共通設定(&C)...", no_action)
                            .unavailable()
                            .into(),
                    ]),
                ],
            ),
            Item::Group(
                "コメントスタイル",
                vec![
                    Item::Row(vec![
                        Control::text(BLOCK1_FROM, "ブロック型(&F)", "", "").into(),
                        Control::text(BLOCK1_TO, "～(&T)", "", "").into(),
                    ]),
                    Item::Row(vec![
                        Control::text(BLOCK2_FROM, "ブロック型(&A)", "", "").into(),
                        Control::text(BLOCK2_TO, "～(&Z)", "", "").into(),
                    ]),
                    line_comment_row(0, "行型(&M)", "桁(&P)"),
                    line_comment_row(1, "行型(&E)", "桁(&O)"),
                    line_comment_row(2, "行型(&G)", "桁(&J)"),
                ],
            ),
            Item::Group(
                "文字列エスケープ(&Q)",
                vec![
                    Control::choice(STRING_TYPE, "", "", 0, STRING_TYPES).into(),
                    Control::check(STRING_LINE_ONLY, "行内のみ(&\\)", "", false).into(),
                    Control::check(
                        STRING_END_LINE,
                        "終了文字がない場合行末まで色分け(&N)",
                        "",
                        false,
                    )
                    .when(string_ends_at_line_end)
                    .into(),
                ],
            ),
            Control::text(VERT_LINES, "縦線桁指定(&3)", "", "").into(),
            Item::Label("*桁またはStep(Begin, End)でコンマ区切り"),
        ],
    ])]
}

/// A line comment: the delimiter, the box 桁 and the column.
fn line_comment_row(n: usize, label: &'static str, column_label: &'static str) -> Item {
    Item::Row(vec![
        Control::text(LINE_COMMENT[n], label, "", "").into(),
        Control::check(LINE_COMMENT_POS_ON[n], column_label, "", false).into(),
        Control::number(LINE_COMMENT_POS[n], "", "", 1, 0, COMMENT_COLUMN_MAX)
            .when(COLUMN_ENABLED[n])
            .into(),
    ])
}
