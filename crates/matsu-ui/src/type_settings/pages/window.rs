//! 「ウィンドウ」: `IDD_PROP_WINDOW` (`sakura_core/sakura_rc.rc`), `CPropTypesWindow`
//! (`sakura_core/typeprop/CPropTypesWindow.cpp`).
//!
//! Live: the encoding, the BOM and the line ending of a new document of the type, the code pages
//! (the check box C&P) and CESU-8 first in the detection (`crate::settings::NewDocumentDefaults`),
//! the line number unit, the least width and the separator of the line numbers
//! (`ViewSettings`, `view/element.rs`), and on Windows the IME at start (IMM32,
//! `matsu_platform::ime_mode`) and the icon of the document (`crate::doc_icon`). macOS and Linux
//! keep those two greyed out with the reason beside them (ADR-0018 type B). The background image
//! (the file, the position, the offset, the repeat, the scroll and the transparency) is drawn by
//! the view (`crate::view::back_image`).

use std::sync::OnceLock;

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
pub(in crate::type_settings) const IME_SWITCHES: &[(&str, i32)] =
    &[("そのまま", 0), ("常にON", 1), ("常にOFF", 2)];

/// `ImeStateArr`.
pub(in crate::type_settings) const IME_STATES: &[(&str, i32)] = &[
    ("標準設定", 0),
    ("全角", 1),
    ("全角ひらがな", 2),
    ("全角カタカナ", 3),
    ("無変換", 4),
];

/// `STR_IMAGE_POS1` to `STR_IMAGE_POS9`: where the background image is.
pub(in crate::type_settings) const IMAGE_POSITIONS: &[(&str, i32)] = &[
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

/// The code pages the combo box gets after C&P is checked (`CCodePage::AddComboCodePages`):
/// those MATSU Editor can read and write, by their number, with the name and the description.
/// Sakura lists the code pages installed in Windows; the ones MATSU Editor has no table for
/// (437, 850, 20932, ISO-2022-KR and the like) are not in the list, so they cannot be chosen (see
/// [`CODE_PAGE_NOTE`]).
pub(in crate::type_settings) fn code_page_options() -> &'static [(&'static str, i32)] {
    static OPTIONS: OnceLock<Vec<(&'static str, i32)>> = OnceLock::new();
    OPTIONS.get_or_init(|| {
        Encoding::choices()
            .into_iter()
            .filter(|e| {
                matches!(
                    e,
                    Encoding::CodePage(_) | Encoding::Utf32Le | Encoding::Utf32Be
                ) && e.can_be_default()
            })
            .map(|e| (e.list_name(), e.sakura_code()))
            .collect()
    })
}

/// Why some code pages are not in the list.
const CODE_PAGE_NOTE: &str = "CP で選べるのは、MATSU Editor が変換表を持つコードページだけです。\
    437・850 などの OEM のコードページと、ISO-2022-KR など変換表のないものは、選べません。\
    Windows の変換機能を使わず、どの OS でも同じ結果にするためです。";

/// The reason the IME at start is greyed out where the OS does not allow it.
const IME_NOTE: &str = "この OS では、アプリから IME のオン・オフと入力モードを決められないため、使えません。\
    (Windows の IMM32 だけが、アプリから IME を操作できます。)";

/// The reason the document icon is greyed out where the OS does not allow it.
const DOC_ICON_NOTE: &str = "この OS では、ウィンドウのアイコンを文書ごとに変えられないため、使えません。\
    (タブのアイコンは、「タブバー」の「アイコン表示」で決まります。)";

/// `IDC_BUTTON_BACKIMG_PATH_SEL`: chooses the image (`SelectFile` with `*.bmp;*.jpg;*.jpeg;*.png`).
/// The title of the dialog is MATSU Editor's own wording (ADR-0018 type F: Sakura has none).
fn browse_background_image(
    dialog: &mut crate::common_settings::CommonSettingsDialog,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<crate::common_settings::CommonSettingsDialog>,
) {
    crate::support::browse::browse_into_filtered(
        dialog,
        window,
        cx,
        BG_PATH,
        "背景画像ファイルの選択",
        Some(BACKGROUND_IMAGE_WILDCARD),
    );
}

/// The files the browse button lists (`CPropTypesWindow.cpp`, `IDC_BUTTON_BACKIMG_PATH_SEL`).
const BACKGROUND_IMAGE_WILDCARD: &str = "*.bmp;*.jpg;*.jpeg;*.png";

/// The control, or the control greyed out where the OS does not have what it needs.
fn windows_only(control: Control) -> Control {
    if cfg!(windows) {
        control
    } else {
        control.unavailable()
    }
}

/// C&P cannot be unchecked once it is checked (`IDC_CHECK_CP` is disabled by the click).
fn code_page_not_chosen_yet(values: &Values) -> bool {
    !values.bool(CODE_PAGE)
}

/// The list of code pages is for the one who checked C&P.
fn code_page_chosen(values: &Values) -> bool {
    values.bool(CODE_PAGE)
}

/// `IDC_CHECK_DEFAULT_BOM` is enabled for an encoding with a BOM.
fn bom_is_possible(values: &Values) -> bool {
    code_uses_bom(values.int(DEFAULT_CODE))
}

/// Choosing the encoding sets the BOM to what the encoding has by default
/// (`CCodeTypeName::IsBomDefOn`, on `CBN_SELCHANGE` of `IDC_COMBO_DEFAULT_CODETYPE`).
fn code_changed(values: &mut Values, id: &'static str) {
    if id == CODE_PAGE_NUMBER {
        // One code the page holds: the code page list chooses it as the combo box does.
        let code = values.int(CODE_PAGE_NUMBER);
        values.set(DEFAULT_CODE, Value::Int(code));
    }
    if id == DEFAULT_CODE {
        let code = values.int(DEFAULT_CODE);
        values.set(CODE_PAGE_NUMBER, Value::Int(code));
    }
    if id == DEFAULT_CODE || id == CODE_PAGE_NUMBER {
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
                            .when(code_page_not_chosen_yet)
                            .into(),
                    ]),
                    Control::choice(CODE_PAGE_NUMBER, "コードページ", "", 4, code_page_options())
                        .on_change(code_changed)
                        .when(code_page_chosen)
                        .into(),
                    Item::Label(CODE_PAGE_NOTE),
                    Control::check(PRIOR_CESU8, "自動判別時にCESU-8 を優先する(&U)", "", false)
                        .into(),
                ],
            )],
            vec![Item::Group(
                "起動時のIME (日本語入力変換)",
                vec![
                    windows_only(Control::choice(
                        IME_SWITCH,
                        "ON/OFF状態(&M)",
                        "",
                        0,
                        IME_SWITCHES,
                    ))
                    .into(),
                    windows_only(Control::choice(
                        IME_STATE,
                        "入力モード(&D)",
                        "",
                        0,
                        IME_STATES,
                    ))
                    .into(),
                ]
                .into_iter()
                .chain((!cfg!(windows)).then_some(Item::Label(IME_NOTE)))
                .collect(),
            )],
        ]),
        Item::Group(
            "ウィンドウ",
            vec![Item::from(windows_only(Control::check(
                USE_DOC_ICON,
                "文書アイコンを使う(&O)",
                "",
                false,
            )))]
            .into_iter()
            .chain((!cfg!(windows)).then_some(Item::Label(DOC_ICON_NOTE)))
            .collect(),
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
                        .into(),
                        Control::radio(
                            LINE_NUM_MODE,
                            "改行単位(&W)",
                            "",
                            NUMBER_LINES,
                            NUMBER_LINES,
                        )
                        .into(),
                        Control::number(
                            LINE_NUM_WIDTH,
                            "桁数",
                            "",
                            width_min,
                            width_min,
                            width_max,
                        )
                        .into(),
                    ],
                ),
                Item::Group(
                    "行番号区切り",
                    vec![
                        Control::radio(LINE_TERM_TYPE, "なし(&N)", "", 0, 1).into(),
                        Control::radio(LINE_TERM_TYPE, "縦線(&V)", "", 1, 1).into(),
                        Control::radio(LINE_TERM_TYPE, "任意(&Y)", "", 2, 1).into(),
                        Control::text(LINE_TERM_CHAR, "半角(&S)", "", ":")
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
                            .into(),
                        Control::button(BG_BROWSE, "...", browse_background_image)
                            .named("背景画像のファイルを参照")
                            .into(),
                    ]),
                    Item::Row(vec![
                        Control::choice(BG_POS, "表示位置", "", 0, IMAGE_POSITIONS).into(),
                    ]),
                    Item::Row(vec![
                        Control::number(BG_OFFSET_X, "Offset X", "", 0, -99999, 99999).into(),
                        Control::number(BG_OFFSET_Y, "Offset Y", "", 0, -99999, 99999).into(),
                    ]),
                    Item::Row(vec![
                        Control::check(BG_REPEAT_X, "Repeat X", "", true).into(),
                        Control::check(BG_REPEAT_Y, "Repeat Y", "", true).into(),
                    ]),
                    Item::Row(vec![
                        Control::check(BG_SCROLL_X, "Scroll X", "", true).into(),
                        Control::check(BG_SCROLL_Y, "Scroll Y", "", true).into(),
                    ]),
                    Control::number(BG_TRANSPARENCY, "透明度", "", 0, 0, 255).into(),
                ],
            )],
        ]),
    ]
}
