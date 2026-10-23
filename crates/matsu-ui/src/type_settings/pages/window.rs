//! 「ウィンドウ」: `IDD_PROP_WINDOW` (`sakura_core/sakura_rc.rc`), `CPropTypesWindow`
//! (`sakura_core/typeprop/CPropTypesWindow.cpp`).
//!
//! Live: the encoding, the BOM and the line ending of a new document of the type (MATSU Editor
//! uses them, `crate::settings::NewDocumentDefaults`). Not available yet: the code pages (the
//! check box CP), CESU-8 first in the detection, the IME at start, the icon of the document, the
//! line number unit, the least width and the separator of the line numbers, and the background
//! image.

use matsu_core::encoding::Encoding;

use super::super::ids::*;
use crate::common_settings::model::{Control, Item, Value, Values};

/// `aszEolStr` and `aeEolType`: the line endings of a new document.
pub(in crate::type_settings) const EOLS: &[(&str, i32)] = &[
    ("CR+LF", 1),
    ("LF (UNIX)", 2),
    ("CR (Mac)", 3),
    ("NEL", 4),
    ("LS", 5),
    ("PS", 6),
];

/// The encodings a new document may have: `CCodeTypesForCombobox` without those that are not
/// `CanDefault` (`sakura_core/charset/charset.cpp`), by their long names.
pub(in crate::type_settings) const CODES: &[(&str, i32)] = &[
    ("SJIS", 0),
    ("EUC-JP", 2),
    ("Latin1", 8),
    ("UTF-16", 3),
    ("UTF-16BE", 6),
    ("UTF-8", 4),
    ("CESU-8", 7),
];

/// `ImeSwitchArr`.
const IME_SWITCHES: &[(&str, i32)] = &[("そのまま", 0), ("常にON", 1), ("常にOFF", 2)];

/// `ImeStateArr`.
const IME_STATES: &[(&str, i32)] = &[
    ("標準設定", 0),
    ("全角", 1),
    ("全角ひらがな", 2),
    ("全角カタカナ", 3),
    ("無変換", 4),
];

/// `STR_IMAGE_POS1` to `STR_IMAGE_POS9`: where the background image is.
const IMAGE_POSITIONS: &[(&str, i32)] = &[
    ("左上", 0),
    ("右上", 1),
    ("左下", 2),
    ("右下", 3),
    ("中央", 4),
    ("中央上", 5),
    ("中央下", 6),
    ("中央左", 7),
    ("中央右", 8),
];

/// The line number is of the rows of the wrapped text (0) or of the lines (1)
/// (`bLineNumIsCRLF` is the second).
pub(in crate::type_settings) const NUMBER_ROWS: i32 = 0;
pub(in crate::type_settings) const NUMBER_LINES: i32 = 1;

/// `LINENUMWIDTH_MIN` and `LINENUMWIDTH_MAX` (`sakura_core/config/app_constants.h`).
pub(in crate::type_settings) const LINE_NUM_WIDTH_RANGE: (i32, i32) = (2, 11);

/// Whether a new document in the encoding can have a BOM (`CCodeTypeName::UseBom`).
pub(in crate::type_settings) fn code_uses_bom(code: i32) -> bool {
    Encoding::from_sakura_code(code).is_some_and(Encoding::can_have_bom)
}

/// `IDC_CHECK_DEFAULT_BOM` is enabled for an encoding with a BOM.
fn bom_is_possible(values: &Values) -> bool {
    code_uses_bom(values.int(DEFAULT_CODE))
}

/// Choosing the encoding sets the BOM to what the encoding has by default
/// (`CCodeTypeName::IsBomDefOn`, on `CBN_SELCHANGE` of `IDC_COMBO_DEFAULT_CODETYPE`).
fn code_changed(values: &mut Values, id: &'static str) {
    if id == DEFAULT_CODE {
        let code = values.int(DEFAULT_CODE);
        let on = Encoding::from_sakura_code(code)
            .is_some_and(|e| e.can_have_bom() && e.bom_default_on());
        values.set(DEFAULT_BOM, Value::Bool(on));
    }
}

/// `EnableTypesPropInput`: the separator is typed only for 任意.
fn separator_is_a_character(values: &Values) -> bool {
    values.int(LINE_TERM_TYPE) == 2
}

pub(in crate::type_settings) fn items() -> Vec<Item> {
    let (width_min, width_max) = LINE_NUM_WIDTH_RANGE;
    vec![
        Item::Columns(vec![
            vec![Item::Group(
                "デフォルトの文字コード",
                vec![
                    Item::Row(vec![
                        Control::choice(DEFAULT_EOL, "改行コード(&E)", "", 1, EOLS).into(),
                        Control::check(DEFAULT_BOM, "&BOM", "", false)
                            .when(bom_is_possible)
                            .into(),
                    ]),
                    Item::Row(vec![
                        Control::choice(DEFAULT_CODE, "文字コード(&C)", "", 4, CODES)
                            .on_change(code_changed)
                            .into(),
                        Control::check(CODE_PAGE, "C&P", "", false)
                            .unavailable()
                            .into(),
                    ]),
                    Control::check(PRIOR_CESU8, "自動判別時にCESU-8 を優先する(&U)", "", false)
                        .unavailable()
                        .into(),
                ],
            )],
            vec![Item::Group(
                "起動時のIME (日本語入力変換)",
                vec![
                    Control::choice(IME_SWITCH, "ON/OFF状態(&M)", "", 0, IME_SWITCHES)
                        .unavailable()
                        .into(),
                    Control::choice(IME_STATE, "入力モード(&D)", "", 4, IME_STATES)
                        .unavailable()
                        .into(),
                ],
            )],
        ]),
        Item::Group(
            "ウィンドウ",
            vec![
                Control::check(USE_DOC_ICON, "文書アイコンを使う(&O)", "", false)
                    .unavailable()
                    .into(),
            ],
        ),
        Item::Columns(vec![
            vec![
                Item::Group(
                    "行番号の表示",
                    vec![
                        Control::radio(
                            LINE_NUM_MODE,
                            "折り返し単位(&R)",
                            "",
                            NUMBER_ROWS,
                            NUMBER_LINES,
                        )
                        .unavailable()
                        .into(),
                        Control::radio(
                            LINE_NUM_MODE,
                            "改行単位(&W)",
                            "",
                            NUMBER_LINES,
                            NUMBER_LINES,
                        )
                        .unavailable()
                        .into(),
                        Control::number(
                            LINE_NUM_WIDTH,
                            "桁数",
                            "",
                            width_min,
                            width_min,
                            width_max,
                        )
                        .unavailable()
                        .into(),
                    ],
                ),
                Item::Group(
                    "行番号区切り",
                    vec![
                        Control::radio(LINE_TERM_TYPE, "なし(&N)", "", 0, 1)
                            .unavailable()
                            .into(),
                        Control::radio(LINE_TERM_TYPE, "縦線(&V)", "", 1, 1)
                            .unavailable()
                            .into(),
                        Control::radio(LINE_TERM_TYPE, "任意(&Y)", "", 2, 1)
                            .unavailable()
                            .into(),
                        Control::text(LINE_TERM_CHAR, "半角(&S)", "", ":")
                            .unavailable()
                            .when(separator_is_a_character)
                            .into(),
                    ],
                ),
            ],
            vec![Item::Group(
                "背景画像",
                vec![
                    Item::Row(vec![
                        Control::text(BG_PATH, "", "", "")
                            .named("背景画像のファイル")
                            .unavailable()
                            .into(),
                        Control::button(BG_BROWSE, "...", |_, _, _| {})
                            .named("背景画像のファイルを参照")
                            .unavailable()
                            .into(),
                    ]),
                    Item::Row(vec![
                        Control::choice(BG_POS, "表示位置", "", 0, IMAGE_POSITIONS)
                            .unavailable()
                            .into(),
                    ]),
                    Item::Row(vec![
                        Control::number(BG_OFFSET_X, "Offset X", "", 0, -99999, 99999)
                            .unavailable()
                            .into(),
                        Control::number(BG_OFFSET_Y, "Offset Y", "", 0, -99999, 99999)
                            .unavailable()
                            .into(),
                    ]),
                    Item::Row(vec![
                        Control::check(BG_REPEAT_X, "Repeat X", "", true)
                            .unavailable()
                            .into(),
                        Control::check(BG_REPEAT_Y, "Repeat Y", "", true)
                            .unavailable()
                            .into(),
                    ]),
                    Item::Row(vec![
                        Control::check(BG_SCROLL_X, "Scroll X", "", true)
                            .unavailable()
                            .into(),
                        Control::check(BG_SCROLL_Y, "Scroll Y", "", true)
                            .unavailable()
                            .into(),
                    ]),
                    Control::number(BG_TRANSPARENCY, "透明度", "", 0, 0, 255)
                        .unavailable()
                        .into(),
                ],
            )],
        ]),
    ]
}
