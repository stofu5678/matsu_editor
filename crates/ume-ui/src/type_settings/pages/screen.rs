//! 「スクリーン」: `IDD_PROP_SCREEN` (`sakura_core/sakura_rc.rc`), `CPropTypesScreen`
//! (`sakura_core/typeprop/CPropTypesScreen.cpp`).
//!
//! Live: the name and the extensions, the wrap method and column, the TAB width, the English word
//! wrap, the hanging of line ends and punctuation, and the standard rule of the outline. Not
//! available yet: the space between characters and lines, how a TAB is drawn, TSV mode, the
//! indent (auto indent, smart indent, the wrapped line indent, the trailing spaces), the rule
//! file of the outline, the font of the type, the line breaking rules for the head and the tail
//! of a line and the characters that hang.

use ume_config::settings::type_rules::outline_type;

use super::super::ids::*;
use crate::common_settings::CommonSettingsDialog;
use crate::common_settings::model::{Control, Item, Value, Values};

/// `COLUMNSPACE_MAX` (`sakura_core/config/app_constants.h`).
pub(in crate::type_settings) const COLUMN_SPACE_MAX: i32 = 64;
/// `LINESPACE_MAX`.
pub(in crate::type_settings) const LINE_SPACE_MAX: i32 = 128;
/// `TABSPACE_MAX`, and the least TAB width of `GetData` (`t_max(1, t_min(64, ...))`).
pub(in crate::type_settings) const TAB_SPACE_RANGE: (i32, i32) = (1, 64);
/// `MINLINEKETAS` and `MAXLINEKETAS`.
pub(in crate::type_settings) const WRAP_COLUMN_RANGE: (i32, i32) = (10, 10240);

/// `WrapMethodArr`: `WRAP_NO_TEXT_WRAP`, `WRAP_SETTING_WIDTH`, `WRAP_WINDOW_WIDTH`.
pub(in crate::type_settings) const WRAP_METHODS: &[(&str, i32)] = &[
    ("折り返さない", 0),
    ("指定桁で折り返す", 1),
    ("右端で折り返す", 2),
];

/// `TabArrowArr`.
pub(in crate::type_settings) const TAB_ARROWS: &[(&str, i32)] =
    &[("文字指定", 0), ("短い矢印", 1), ("長い矢印", 2)];

/// `TsvModeArr`.
pub(in crate::type_settings) const TSV_MODES: &[(&str, i32)] =
    &[("通常", 0), ("TSV", 1), ("CSV", 2)];

/// `SmartIndentArr`.
pub(in crate::type_settings) const SMART_INDENTS: &[(&str, i32)] = &[("なし", 0), ("C/C++", 1)];

/// `IndentTypeArr`.
pub(in crate::type_settings) const INDENT_LAYOUTS: &[(&str, i32)] =
    &[("なし", 0), ("tx2x", 1), ("論理行先頭", 2)];

/// `OlmArr`: the standard rules of the outline, in Sakura's order.
pub(in crate::type_settings) const OUTLINES: &[(&str, i32)] = &[
    ("C/C++", outline_type::C_CPP),
    ("C", outline_type::C),
    ("C++", outline_type::CPP),
    ("PL/SQL", outline_type::PLSQL),
    ("Java", outline_type::JAVA),
    ("COBOL", outline_type::COBOL),
    ("Perl", outline_type::PERL),
    ("アセンブラ", outline_type::ASM),
    ("Visual Basic", outline_type::VB),
    ("Python", outline_type::PYTHON),
    ("Erlang", outline_type::ERLANG),
    ("WZ階層付テキスト", outline_type::WZTXT),
    ("HTML", outline_type::HTML),
    ("XML", outline_type::XML),
    ("TeX", outline_type::TEX),
    ("テキスト", outline_type::TEXT),
];

/// The outline is the rule of the list (0) or a rule file (1).
pub(in crate::type_settings) const OUTLINE_STANDARD: i32 = 0;
pub(in crate::type_settings) const OUTLINE_FILE: i32 = 1;

/// The name and the extensions of the basic type cannot be changed (`m_nIdx == 0`).
fn not_basic_type(values: &Values) -> bool {
    values.int(TYPE_INDEX) != 0
}

/// `EnableWindow(IDC_EDIT_TABVIEWSTRING, m_bTabArrow == TABARROW_STRING)`.
fn tab_is_a_string(values: &Values) -> bool {
    values.int(TAB_ARROW) == 0
}

/// `IDC_COMBO_OUTLINES` is for the standard rule.
fn outline_is_standard(values: &Values) -> bool {
    values.int(OUTLINE_MODE) == OUTLINE_STANDARD
}

/// `IDC_EDIT_OUTLINERULEFILE` is for the rule file.
fn outline_is_file(values: &Values) -> bool {
    values.int(OUTLINE_MODE) == OUTLINE_FILE
}

/// `IDC_CHECK_KINSOKUHIDE` is enabled while a line end or punctuation hangs.
fn something_hangs(values: &Values) -> bool {
    values.bool(KINSOKU_RET) || values.bool(KINSOKU_KUTO)
}

/// `EnableWindow(IDC_CHECK_USETYPEFONT, m_bUseTypeFont)`: the check box can only be cleared; the
/// button of the font checks it again.
fn type_font_is_used(values: &Values) -> bool {
    values.bool(USE_TYPE_FONT)
}

/// `IDC_BUTTON_TYPEFONT` (`CPropTypesScreen::DispatchEvent`): the font dialog starts with the font of
/// the type if it is used, else the common font. The font chosen is the font of the type.
fn choose_type_font(
    dialog: &mut CommonSettingsDialog,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) {
    let values = dialog.values();
    let initial = if values.bool(USE_TYPE_FONT) {
        values.font(TYPE_FONT)
    } else {
        crate::font_dialog::apply::common_font(cx).font
    };
    dialog.choose_font(TYPE_FONT, initial, window, cx);
}

/// A font chosen is used (`m_bUseTypeFont = true`).
fn type_font_chosen(values: &mut Values, _: &'static str) {
    values.set(USE_TYPE_FONT, Value::Bool(true));
}

/// `IDC_STATIC_TYPEFONT`: the face and the size of the font of the type, empty while it is not used
/// (`SetFontLabel`).
fn type_font_label(
    dialog: &CommonSettingsDialog,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) -> gpui::AnyElement {
    let values = dialog.values();
    let font = values.bool(USE_TYPE_FONT).then(|| values.font(TYPE_FONT));
    crate::common_settings::model::font_label(font.as_ref(), cx)
}

/// Choosing the standard rule selects the first of the list (`IDC_RADIO_OUTLINEDEFAULT`).
fn outline_mode_changed(values: &mut Values, id: &'static str) {
    if id == OUTLINE_MODE && values.int(OUTLINE_MODE) == OUTLINE_STANDARD {
        values.set(
            OUTLINE,
            crate::common_settings::model::Value::Int(OUTLINES[0].1),
        );
    }
}

pub(in crate::type_settings) fn items() -> Vec<Item> {
    let (wrap_min, wrap_max) = WRAP_COLUMN_RANGE;
    let (tab_min, tab_max) = TAB_SPACE_RANGE;
    vec![
        Item::Row(vec![
            Control::text(TYPE_NAME, "設定の名前(&N)", "", "")
                .when(not_basic_type)
                .into(),
            Control::text(TYPE_EXTS, "ファイル拡張子(&X)", "", "")
                .when(not_basic_type)
                .into(),
        ]),
        Item::Columns(vec![
            vec![
                Item::Group(
                    "レイアウト",
                    vec![
                        Control::choice(WRAP_METHOD, "折り返し方法(&<)", "", 0, WRAP_METHODS)
                            .into(),
                        Control::number(
                            MAX_LINE_KETAS,
                            "折り返し桁数(&R)",
                            "",
                            wrap_max,
                            wrap_min,
                            wrap_max,
                        )
                        .into(),
                        Item::Row(vec![
                            Control::number(
                                COLUMN_SPACE,
                                "文字の間隔(&C)",
                                "",
                                0,
                                0,
                                COLUMN_SPACE_MAX,
                            )
                            .unavailable()
                            .into(),
                            Item::Label("ドット"),
                        ]),
                        Item::Row(vec![
                            Control::number(
                                LINE_SPACE,
                                "行の間隔(&L)",
                                "",
                                1,
                                -LINE_SPACE_MAX,
                                LINE_SPACE_MAX,
                            )
                            .unavailable()
                            .into(),
                            Item::Label("ドット"),
                        ]),
                        Control::number(TAB_SPACE, "TAB幅(&T)", "", 4, tab_min, tab_max).into(),
                        Item::Row(vec![
                            Control::choice(TAB_ARROW, "T&AB表示", "", 0, TAB_ARROWS)
                                .unavailable()
                                .into(),
                            Control::text(TAB_VIEW_STRING, "", "", "^       ")
                                .named("TAB表示の文字列")
                                .unavailable()
                                .when(tab_is_a_string)
                                .into(),
                        ]),
                        Control::check(INS_SPACE, "S&PACEの挿入", "", false)
                            .unavailable()
                            .into(),
                        Control::choice(TSV_MODE, "", "", 0, TSV_MODES)
                            .named("TSVモード")
                            .unavailable()
                            .into(),
                    ],
                ),
                Item::Group(
                    "インデント",
                    vec![
                        Item::Row(vec![
                            Control::check(AUTO_INDENT, "自動インデント(&U)", "", true)
                                .unavailable()
                                .into(),
                            Control::check(AUTO_INDENT_ZENSPACE, "全角空白も(&Z)", "", true)
                                .unavailable()
                                .into(),
                        ]),
                        Control::choice(
                            SMART_INDENT,
                            "スマートインデント(&S)",
                            "",
                            0,
                            SMART_INDENTS,
                        )
                        .unavailable()
                        .into(),
                        Control::text(INDENT_CHARS, "その他のインデント対象文字(&I)", "", "")
                            .unavailable()
                            .into(),
                        Control::choice(
                            INDENT_LAYOUT,
                            "折り返し行インデント(&2)",
                            "",
                            0,
                            INDENT_LAYOUTS,
                        )
                        .unavailable()
                        .into(),
                        Control::check(RTRIM_PREV_LINE, "改行時に末尾の空白を削除(&E)", "", false)
                            .unavailable()
                            .into(),
                    ],
                ),
            ],
            vec![
                Item::Group(
                    "アウトライン解析方法",
                    vec![
                        Control::radio(
                            OUTLINE_MODE,
                            "標準ルール(&B)",
                            "",
                            OUTLINE_STANDARD,
                            OUTLINE_STANDARD,
                        )
                        .on_change(outline_mode_changed)
                        .into(),
                        Control::choice(OUTLINE, "", "", OUTLINES[0].1, OUTLINES)
                            .named("標準ルールの種類")
                            .when(outline_is_standard)
                            .into(),
                        Control::radio(
                            OUTLINE_MODE,
                            "ルールファイル(&D)",
                            "",
                            OUTLINE_FILE,
                            OUTLINE_STANDARD,
                        )
                        .unavailable()
                        .on_change(outline_mode_changed)
                        .into(),
                        Item::Row(vec![
                            Control::text(OUTLINE_RULE_FILE, "", "", "")
                                .named("ルールファイル")
                                .unavailable()
                                .when(outline_is_file)
                                .into(),
                            Control::button(OUTLINE_RULE_BROWSE, "(&1)...", |_, _, _| {})
                                .named("ルールファイルを参照")
                                .unavailable()
                                .when(outline_is_file)
                                .into(),
                        ]),
                    ],
                ),
                Item::Group(
                    "タイプ別フォント",
                    vec![
                        Item::Row(vec![
                            Control::check(USE_TYPE_FONT, "使用する(&G)", "", false)
                                .when(type_font_is_used)
                                .into(),
                            Control::button(TYPE_FONT_BUTTON, "フォント(&F)...", choose_type_font)
                                .into(),
                            Item::Custom(type_font_label),
                        ]),
                        Item::Hidden(vec![
                            Control::font(TYPE_FONT, "", None, None).on_change(type_font_chosen),
                        ]),
                    ],
                ),
                Item::Group(
                    "禁則処理",
                    vec![
                        Control::check(WORD_WRAP, "英文ワードラップ(&W)", "", false).into(),
                        Control::check(KINSOKU_RET, "改行ぶら下げ(&^)", "", false).into(),
                        Control::check(KINSOKU_HIDE, "ぶら下げを隠す(&-)", "", false)
                            .when(something_hangs)
                            .into(),
                        Item::Row(vec![
                            Control::check(KINSOKU_KUTO, "句読点ぶら下げ(&\\)", "", false).into(),
                            Control::text(KINSOKU_KUTO_CHARS, "", "", "、。，．､｡,.")
                                .named("句読点ぶら下げの文字")
                                .unavailable()
                                .into(),
                        ]),
                        Item::Row(vec![
                            Control::check(KINSOKU_HEAD, "行頭禁則(&[)", "", false)
                                .unavailable()
                                .into(),
                            Control::text(KINSOKU_HEAD_CHARS, "", "", "")
                                .named("行頭禁則の文字")
                                .unavailable()
                                .into(),
                        ]),
                        Item::Row(vec![
                            Control::check(KINSOKU_TAIL, "行末禁則(&])", "", false)
                                .unavailable()
                                .into(),
                            Control::text(KINSOKU_TAIL_CHARS, "", "", "")
                                .named("行末禁則の文字")
                                .unavailable()
                                .into(),
                        ]),
                    ],
                ),
            ],
        ]),
    ]
}
