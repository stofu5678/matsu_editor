//! The values of the controls of the type settings: a [`TypeSettings`] becomes the values the
//! dialog shows ([`load`]), and the values the user left become a `TypeSettings` again
//! ([`store`]).
//!
//! `store` follows `GetData` of Sakura's pages (`CPropTypesScreen::GetData`,
//! `CPropTypesColor::GetData`, `CPropTypesWindow::GetData`, `CPropTypesSupport::GetData`,
//! `CPropTypesRegex::GetData`). A control that is not available is
//! not read back: its field stays as it was. The settings of the type that no page shows stay as
//! they are too.

use matsu_config::Profile;
use matsu_config::settings::type_rules::{
    BLOCK_COMMENT_BUFFER_SIZE, BlockComment, LINE_COMMENT_BUFFER_SIZE, LineComment, MAX_VERT_LINES,
    outline_type,
};
use matsu_config::settings::types::{MAX_LINE_KETAS as WIDEST_WRAP_COLUMN, eol_type};
use matsu_config::settings::{ColorSet, DocumentTypes, RegexKeyword, TypeSettings};
use matsu_core::encoding::{Encoding, Eol};

use super::ids::*;
use super::pages::color::STRING_TYPES;
use super::pages::keyhelp::{RMENU_TYPES, dict_of, row_of};
use super::pages::regex::{color_label, color_name};
use super::pages::screen::{OUTLINE_FILE, OUTLINE_STANDARD, OUTLINES, WRAP_METHODS};
use super::pages::support::HOKAN_TYPES;
use super::pages::window::{CODES, EOLS};
use crate::common_settings::model::{Control, Kind, Value, Values};

/// The size of the buffer of the name of a type, in UTF-16 units with the end mark
/// (`MAX_TYPES_NAME`, `sakura_core/config/maxdata.h`).
const TYPE_NAME_BUFFER: usize = 64;
/// `MAX_TYPES_EXTS`.
const TYPE_EXTS_BUFFER: usize = 64;
/// `m_szKinsokuHead[200]` and the other two lists of kinsoku characters.
const KINSOKU_BUFFER: usize = 200;
/// `_MAX_PATH`: the buffer of the word file (`szHokanFile`).
const PATH_BUFFER: usize = 260;
/// `sizeof(szIndentChars)`: the buffer of the other indent chars.
const INDENT_CHARS_BUFFER: usize = 64;
/// `MAX_VERTLINES * 15`: the buffer of the text of the vertical lines.
const VERT_LINES_TEXT: usize = MAX_VERT_LINES * 15;

/// The number of a line ending in `eDefaultEoltype` (`EEolType`).
fn eol_code(eol: Eol) -> i32 {
    match eol {
        Eol::CrLf => eol_type::CRLF,
        Eol::Lf => eol_type::LF,
        Eol::Cr => eol_type::CR,
        Eol::Nel => eol_type::NEL,
        Eol::Ls => eol_type::LS,
        Eol::Ps => eol_type::PS,
    }
}

/// Type `index` as the sheet shows it: the values for a new document (the encoding, the BOM and
/// the line ending) are what a new document of the type gets (`NewDocumentDefaults`), not what a
/// [`TypeSettings`] has for a key the file does not set: MATSU Editor follows the OS for a line
/// ending the file does not say.
pub(super) fn shown_type(
    types: &DocumentTypes,
    profile: &Profile,
    index: usize,
) -> Option<TypeSettings> {
    let mut t = types.get(index)?.clone();
    let defaults = crate::settings::NewDocumentDefaults::for_type(profile, index);
    t.default_codetype = defaults.encoding.sakura_code();
    t.default_bom = defaults.bom;
    t.default_eoltype = eol_code(defaults.eol);
    Some(t)
}

/// What the reader of the file does to the TAB width (`SetValueLimit(m_nTabSpace, 2,
/// m_nMaxLineKetas - 2)` in `ShareData_IO_TypeInts`). The settings are read from the file again
/// as soon as OK has written them, so a width the reader would change is put into this range
/// before it is written, and the window shows what was chosen.
fn tab_width_in_range(tab: i32, wrap_column: i32) -> i32 {
    2.max((wrap_column - 2).min(tab))
}

/// A `COLORREF` as the dialog holds it.
pub(in crate::type_settings) fn color_value(color: u32) -> i32 {
    i32::from_ne_bytes(color.to_ne_bytes())
}

/// The `COLORREF` of a value of the dialog.
pub(in crate::type_settings) fn color_of(value: i32) -> u32 {
    u32::from_ne_bytes(value.to_ne_bytes()) & 0x00ff_ffff
}

/// Cuts `text` to what a buffer of `size` UTF-16 units (with the end mark) holds
/// (`DlgItem_GetText`), without cutting a character in two.
fn fit(text: &str, size: usize) -> String {
    let mut units = 0;
    let mut out = String::new();
    for c in text.chars() {
        units += c.len_utf16();
        if units >= size {
            break;
        }
        out.push(c);
    }
    out
}

/// The string a tab is drawn with, from the text of the box (`CPropTypesScreen::GetData`): the
/// default `"^       "`, with the first characters of the text over it, up to 8 and up to a
/// character a tab cannot show (the end of the text, `\r`, `\n` or a tab: `IsTabAvailableCode`).
pub(super) fn tab_view_string(text: &str) -> String {
    // The buffer is 8 UTF-16 units (`szTab[8+1]`); a character takes 1 or 2 of them, and one that
    // does not fit whole is left out.
    let mut units: Vec<u16> = "^       ".encode_utf16().collect();
    let mut at = 0;
    for c in text.chars() {
        if matches!(c, '\0' | '\r' | '\n' | '\t') || at + c.len_utf16() > units.len() {
            break;
        }
        c.encode_utf16(&mut units[at..]);
        at += c.len_utf16();
    }
    String::from_utf16_lossy(&units)
}

/// `value` if the list has it, else the first of the list (the selection `SetData` makes when
/// the file has a value the list does not, `nSelPos = 0`).
fn in_list(value: i32, options: &[(&str, i32)]) -> i32 {
    if options.iter().any(|(_, v)| *v == value) {
        value
    } else {
        options[0].1
    }
}

/// The `[Types(N)]` keys of what is not in a [`TypeSettings`] but is on a page (shown, not
/// written).
fn raw_i32(profile: &Profile, index: usize, key: &str, default: i32) -> i32 {
    let section = TypeSettings::section_name(i32::try_from(index).unwrap_or(0));
    profile.get_i32(&section, key).unwrap_or(default)
}

fn raw_bool(profile: &Profile, index: usize, key: &str, default: bool) -> bool {
    let section = TypeSettings::section_name(i32::try_from(index).unwrap_or(0));
    profile.get_bool(&section, key).unwrap_or(default)
}

fn raw_text(profile: &Profile, index: usize, key: &str) -> String {
    let section = TypeSettings::section_name(i32::try_from(index).unwrap_or(0));
    profile.get_string(&section, key).unwrap_or_default()
}

/// The text of the vertical lines (`CPropTypesColor::SetData`): `10,20` for single lines and
/// `5(10,50)` for a line every 5 columns from 10 to 50.
pub(super) fn format_vert_lines(columns: &[i32; MAX_VERT_LINES]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < MAX_VERT_LINES && columns[i] != 0 {
        let column = columns[i];
        if column < 0 {
            if i + 2 < MAX_VERT_LINES {
                let begin = column.saturating_neg();
                i += 1;
                let end = columns[i];
                i += 1;
                let step = columns[i];
                if end >= begin && step > 0 {
                    if !out.is_empty() {
                        out.push(',');
                    }
                    out.push_str(&format!("{step}({begin},{end})"));
                }
            }
        } else {
            if !out.is_empty() {
                out.push(',');
            }
            out.push_str(&column.to_string());
        }
        i += 1;
    }
    out
}

/// Reads the digits at `at` of `chars` as a number (0 when there are none).
fn digits(chars: &[char], at: &mut usize) -> i32 {
    let mut value = 0_i32;
    while let Some(d) = chars.get(*at).and_then(|c| c.to_digit(10)) {
        value = value.saturating_mul(10).saturating_add(d as i32);
        *at += 1;
    }
    value
}

/// The columns of the vertical lines from the text (`CPropTypesColor::GetData`): `10,20` and
/// `5(10,50)`; the text is read up to the first thing it cannot read.
pub(super) fn parse_vert_lines(text: &str) -> [i32; MAX_VERT_LINES] {
    let chars: Vec<char> = fit(text, VERT_LINES_TEXT + 1).chars().collect();
    let mut out = [0; MAX_VERT_LINES];
    let mut at = 0;
    let mut i = 0;
    while i < MAX_VERT_LINES {
        let value = digits(&chars, &mut at);
        if value <= 0 {
            break;
        }
        if chars.get(at) == Some(&'(') {
            at += 1;
            let begin = digits(&chars, &mut at);
            if begin <= 0 {
                break;
            }
            match chars.get(at) {
                Some(',') => at += 1,
                Some(')') => {}
                _ => break,
            }
            let mut end = digits(&chars, &mut at);
            if end <= 0 {
                end = WIDEST_WRAP_COLUMN;
            }
            if chars.get(at) != Some(&')') {
                break;
            }
            at += 1;
            if i + 2 < MAX_VERT_LINES {
                out[i] = -begin;
                out[i + 1] = end;
                out[i + 2] = value;
                i += 3;
            } else {
                break;
            }
        } else {
            out[i] = value;
            i += 1;
        }
        if chars.get(at) != Some(&',') {
            break;
        }
        at += 1;
    }
    out
}

/// Puts the values of `t` (and of the keys of the file that the pages show but the type does not
/// keep) into the values of the dialog, for the controls of the sheet.
pub(super) fn load(
    controls: &[Control],
    t: &TypeSettings,
    profile: &Profile,
    index: usize,
    keyword_names: &[String],
) -> Values {
    let mut v = Values::default();
    let b = |v: &mut Values, id: &'static str, on: bool| v.set(id, Value::Bool(on));
    let n = |v: &mut Values, id: &'static str, i: i32| v.set(id, Value::Int(i));
    let s = |v: &mut Values, id: &'static str, text: &str| v.set(id, Value::Text(text.to_owned()));

    n(&mut v, TYPE_INDEX, i32::try_from(index).unwrap_or(0));

    // スクリーン
    s(&mut v, TYPE_NAME, &t.type_name);
    s(&mut v, TYPE_EXTS, &t.type_exts);
    n(
        &mut v,
        WRAP_METHOD,
        in_list(t.text_wrap_method, WRAP_METHODS),
    );
    n(&mut v, MAX_LINE_KETAS, t.max_line_ketas);
    n(&mut v, COLUMN_SPACE, t.column_space);
    n(&mut v, LINE_SPACE, t.line_space);
    n(&mut v, TAB_SPACE, t.tab_space);
    n(&mut v, TAB_ARROW, t.tab_arrow);
    s(&mut v, TAB_VIEW_STRING, &t.tab_view_string);
    b(&mut v, INS_SPACE, t.ins_space);
    n(&mut v, TSV_MODE, t.tsv_mode);
    b(&mut v, AUTO_INDENT, t.auto_indent);
    b(&mut v, AUTO_INDENT_ZENSPACE, t.auto_indent_zenspace);
    n(&mut v, SMART_INDENT, t.smart_indent);
    s(&mut v, INDENT_CHARS, &t.indent_chars);
    n(&mut v, INDENT_LAYOUT, t.indent_layout);
    b(&mut v, RTRIM_PREV_LINE, t.rtrim_prev_line);
    let file_rule = t.default_outline == outline_type::FILE;
    n(
        &mut v,
        OUTLINE_MODE,
        if file_rule {
            OUTLINE_FILE
        } else {
            OUTLINE_STANDARD
        },
    );
    n(&mut v, OUTLINE, in_list(t.default_outline, OUTLINES));
    s(&mut v, OUTLINE_RULE_FILE, &t.outline_rule_filename);
    b(&mut v, USE_TYPE_FONT, t.use_type_font);
    v.set(TYPE_FONT, Value::Font(t.font.clone()));
    b(&mut v, WORD_WRAP, t.word_wrap);
    b(&mut v, KINSOKU_RET, t.kinsoku_ret);
    b(&mut v, KINSOKU_HIDE, t.kinsoku_hide);
    b(&mut v, KINSOKU_KUTO, t.kinsoku_kuto_enabled);
    s(&mut v, KINSOKU_KUTO_CHARS, &t.kinsoku_kuto);
    b(&mut v, KINSOKU_HEAD, t.kinsoku_head_enabled);
    s(&mut v, KINSOKU_HEAD_CHARS, &t.kinsoku_head);
    b(&mut v, KINSOKU_TAIL, t.kinsoku_tail_enabled);
    s(&mut v, KINSOKU_TAIL_CHARS, &t.kinsoku_tail);

    // カラー
    for (item, name) in ColorSet::all_names().enumerate() {
        let entry = t.colors.get(name).copied().unwrap_or_else(|| {
            *ColorSet::default()
                .get(name)
                .expect("a color of Sakura's table")
        });
        b(&mut v, item_key(item, Field::Disp), entry.disp);
        b(&mut v, item_key(item, Field::Bold), entry.bold);
        b(&mut v, item_key(item, Field::Underline), entry.underline);
        n(&mut v, item_key(item, Field::Text), color_value(entry.text));
        n(&mut v, item_key(item, Field::Back), color_value(entry.back));
    }
    v.set(KEYWORD_SETS, Value::Numbers(t.keyword_set_idx.to_vec()));
    v.set(KEYWORD_NAMES, names_value(keyword_names));
    n(&mut v, COLOR_SEL, 0);
    super::pages::color::sync(&mut v, COLOR_SEL);
    s(&mut v, BLOCK1_FROM, &t.block_comments[0].from);
    s(&mut v, BLOCK1_TO, &t.block_comments[0].to);
    s(&mut v, BLOCK2_FROM, &t.block_comments[1].from);
    s(&mut v, BLOCK2_TO, &t.block_comments[1].to);
    for (i, line) in t.line_comments.iter().enumerate() {
        s(&mut v, LINE_COMMENT[i], &line.delimiter);
        // The column is counted from 1 on the page; "anywhere" shows as 1 with the box off.
        b(&mut v, LINE_COMMENT_POS_ON[i], line.column.is_some());
        let column = line.column.map_or(1, |c| {
            i32::try_from(c).map_or(i32::MAX, |c| c.saturating_add(1))
        });
        n(&mut v, LINE_COMMENT_POS[i], column);
    }
    n(&mut v, STRING_TYPE, in_list(t.string_type, STRING_TYPES));
    b(&mut v, STRING_LINE_ONLY, t.string_line_only);
    b(&mut v, STRING_END_LINE, t.string_end_line);
    s(&mut v, VERT_LINES, &format_vert_lines(&t.vert_line_idx));

    // ウィンドウ
    let eol = if EOLS.iter().any(|(_, e)| *e == t.default_eoltype) {
        t.default_eoltype
    } else {
        // `SetData`: a value that is no line ending selects the first, CR+LF.
        eol_type::CRLF
    };
    n(&mut v, DEFAULT_EOL, eol);
    // A code the list does not have (a code page) stays as the file has it.
    n(&mut v, DEFAULT_CODE, t.default_codetype);
    let bom = t.default_bom
        && Encoding::from_sakura_code(t.default_codetype).is_some_and(Encoding::can_have_bom);
    b(&mut v, DEFAULT_BOM, bom);
    b(
        &mut v,
        CODE_PAGE,
        !CODES.iter().any(|(_, c)| *c == t.default_codetype),
    );
    b(&mut v, PRIOR_CESU8, t.prior_cesu8);
    // `m_nImeState`: the on/off state in the low two bits, the input mode above them.
    let ime = raw_i32(profile, index, "nImeState", 0x10);
    n(&mut v, IME_SWITCH, ime & 3);
    n(&mut v, IME_STATE, ime >> 2);
    b(
        &mut v,
        USE_DOC_ICON,
        raw_bool(profile, index, "bUseDocumentIcon", false),
    );
    n(
        &mut v,
        LINE_NUM_MODE,
        if t.line_num_is_crlf {
            super::pages::window::NUMBER_LINES
        } else {
            super::pages::window::NUMBER_ROWS
        },
    );
    n(&mut v, LINE_NUM_WIDTH, t.line_num_width);
    n(&mut v, LINE_TERM_TYPE, t.line_term_type);
    s(&mut v, LINE_TERM_CHAR, &t.line_term_char.to_string());
    s(&mut v, BG_PATH, &raw_text(profile, index, "bgImgPath"));
    n(&mut v, BG_POS, raw_i32(profile, index, "bgImgPos", 0));
    n(
        &mut v,
        BG_OFFSET_X,
        raw_i32(profile, index, "bgImgPosOffsetX", 0),
    );
    n(
        &mut v,
        BG_OFFSET_Y,
        raw_i32(profile, index, "bgImgPosOffsetY", 0),
    );
    b(
        &mut v,
        BG_REPEAT_X,
        raw_bool(profile, index, "bgImgRepeartX", true),
    );
    b(
        &mut v,
        BG_REPEAT_Y,
        raw_bool(profile, index, "bgImgRepeartY", true),
    );
    b(
        &mut v,
        BG_SCROLL_X,
        raw_bool(profile, index, "bgImgScrollX", true),
    );
    b(
        &mut v,
        BG_SCROLL_Y,
        raw_bool(profile, index, "bgImgScrollY", true),
    );
    n(
        &mut v,
        BG_TRANSPARENCY,
        255 - raw_i32(profile, index, "bgImgOpacity", 255).clamp(0, 255),
    );

    // 支援
    s(&mut v, HOKAN_FILE, &t.hokan_file);
    // A plugin the list does not have shows as なし, as `SetData` does (nothing is written back).
    n(&mut v, HOKAN_TYPE, in_list(t.hokan_type, HOKAN_TYPES));
    b(&mut v, HOKAN_LO_HI_CASE, t.hokan_lo_hi_case);
    b(&mut v, HOKAN_BY_FILE, t.use_hokan_by_file);
    b(&mut v, HOKAN_BY_KEYWORD, t.use_hokan_by_keyword);
    s(&mut v, EXT_HELP, &t.ext_help);
    s(&mut v, EXT_HTML_HELP, &t.ext_html_help);
    b(&mut v, HTML_HELP_SINGLE, t.html_help_is_single);
    b(&mut v, CHK_ENTER_AT_END, t.warn_mixed_eol);
    b(
        &mut v,
        INDENT_CPP_STRING,
        raw_bool(profile, index, "bIndentCppStringIgnore", true),
    );
    b(
        &mut v,
        INDENT_CPP_COMMENT,
        raw_bool(profile, index, "bIndentCppCommentIgnore", true),
    );
    b(
        &mut v,
        INDENT_CPP_UNDO,
        raw_bool(profile, index, "bIndentCppUndoSep", false),
    );
    b(
        &mut v,
        DETECT_INDENT,
        raw_bool(profile, index, "bDetectIndentationStyleOnFileLoad", true),
    );

    // 正規表現キーワード
    b(&mut v, USE_REGEX, t.use_regex_keyword);
    v.set(
        REGEX_LIST,
        Value::Rows(
            t.regex_keywords
                .iter()
                .map(|k| vec![k.pattern.clone(), color_label(&k.color).to_owned()])
                .collect(),
        ),
    );

    // キーワードヘルプ
    b(&mut v, USE_KEYHELP, t.use_key_word_help);
    v.set(
        KEYHELP_LIST,
        Value::Rows(t.key_help.iter().map(row_of).collect()),
    );
    b(&mut v, KEYHELP_ALL_SEARCH, t.use_key_help_all_search);
    b(&mut v, KEYHELP_KEY_DISP, t.use_key_help_key_disp);
    b(&mut v, KEYHELP_PREFIX, t.use_key_help_prefix);
    n(
        &mut v,
        KEYHELP_RMENU,
        in_list(t.key_help_rmenu_show_type, RMENU_TYPES),
    );

    // Every control must have a value; a page that lists a control `load` forgot would show 0.
    debug_assert!(
        controls
            .iter()
            .all(|c| matches!(c.kind, Kind::Button { .. }) || v.get(c.id).is_some()),
        "a control without a value"
    );
    v
}

/// The names of the keyword sets as the dialog holds them.
pub(super) fn names_value(names: &[String]) -> Value {
    Value::Rows(names.iter().map(|name| vec![name.clone()]).collect())
}

/// The type the values make over `original` (`GetData` of the pages). The numbers have been
/// clamped into their ranges by the dialog (`Values::clamp_numbers`).
pub(super) fn store(v: &Values, original: &TypeSettings) -> TypeSettings {
    let mut t = original.clone();

    // スクリーン
    t.type_name = fit(v.text(TYPE_NAME), TYPE_NAME_BUFFER);
    t.type_exts = fit(v.text(TYPE_EXTS), TYPE_EXTS_BUFFER);
    t.text_wrap_method = v.int(WRAP_METHOD);
    t.max_line_ketas = v.int(MAX_LINE_KETAS);
    t.tab_space = tab_width_in_range(v.int(TAB_SPACE), t.max_line_ketas);
    t.use_type_font = v.bool(USE_TYPE_FONT);
    t.font = v.font(TYPE_FONT);
    t.word_wrap = v.bool(WORD_WRAP);
    t.kinsoku_ret = v.bool(KINSOKU_RET);
    t.kinsoku_hide = v.bool(KINSOKU_HIDE);
    t.kinsoku_kuto_enabled = v.bool(KINSOKU_KUTO);
    t.kinsoku_head_enabled = v.bool(KINSOKU_HEAD);
    t.kinsoku_tail_enabled = v.bool(KINSOKU_TAIL);
    t.kinsoku_head = fit(v.text(KINSOKU_HEAD_CHARS), KINSOKU_BUFFER);
    t.kinsoku_tail = fit(v.text(KINSOKU_TAIL_CHARS), KINSOKU_BUFFER);
    t.kinsoku_kuto = fit(v.text(KINSOKU_KUTO_CHARS), KINSOKU_BUFFER);
    t.tab_arrow = v.int(TAB_ARROW);
    // A string the file has, and the user did not change, stays as it is.
    if v.text(TAB_VIEW_STRING) != original.tab_view_string {
        t.tab_view_string = tab_view_string(v.text(TAB_VIEW_STRING));
    }
    t.indent_layout = v.int(INDENT_LAYOUT);
    t.default_outline = if v.int(OUTLINE_MODE) == OUTLINE_FILE {
        outline_type::FILE
    } else {
        v.int(OUTLINE)
    };
    // the name is kept whichever rule is chosen (`GetData`: 「ルールを使っていなくてもファイル名を保持」)
    t.outline_rule_filename = fit(v.text(OUTLINE_RULE_FILE), PATH_BUFFER);
    // the indent of what is typed (`CPropTypesScreen::GetData`)
    t.ins_space = v.bool(INS_SPACE);
    t.auto_indent = v.bool(AUTO_INDENT);
    t.auto_indent_zenspace = v.bool(AUTO_INDENT_ZENSPACE);
    t.smart_indent = v.int(SMART_INDENT);
    t.indent_chars = fit(v.text(INDENT_CHARS), INDENT_CHARS_BUFFER);
    t.rtrim_prev_line = v.bool(RTRIM_PREV_LINE);

    // カラー
    for (item, name) in ColorSet::all_names().enumerate() {
        if let Some(entry) = t.colors.get_mut(name) {
            entry.disp = v.bool(item_key(item, Field::Disp));
            entry.bold = v.bool(item_key(item, Field::Bold));
            entry.underline = v.bool(item_key(item, Field::Underline));
            entry.text = color_of(v.int(item_key(item, Field::Text)));
            entry.back = color_of(v.int(item_key(item, Field::Back)));
        }
    }
    t.block_comments[0] = BlockComment::new(
        &fit(v.text(BLOCK1_FROM), BLOCK_COMMENT_BUFFER_SIZE),
        &fit(v.text(BLOCK1_TO), BLOCK_COMMENT_BUFFER_SIZE),
    );
    t.block_comments[1] = BlockComment::new(
        &fit(v.text(BLOCK2_FROM), BLOCK_COMMENT_BUFFER_SIZE),
        &fit(v.text(BLOCK2_TO), BLOCK_COMMENT_BUFFER_SIZE),
    );
    for i in 0..t.line_comments.len() {
        // `GetData`: a column of 0 is no column; "on" with a column counts from 1.
        let column = v.int(LINE_COMMENT_POS[i]);
        let on = v.bool(LINE_COMMENT_POS_ON[i]) && column > 0;
        t.line_comments[i] = LineComment::new(
            &fit(v.text(LINE_COMMENT[i]), LINE_COMMENT_BUFFER_SIZE),
            if on { column - 1 } else { -1 },
        );
    }
    t.string_type = in_list(v.int(STRING_TYPE), STRING_TYPES);
    t.string_line_only = v.bool(STRING_LINE_ONLY);
    t.string_end_line = v.bool(STRING_END_LINE);
    t.vert_line_idx = parse_vert_lines(v.text(VERT_LINES));
    // `GetData`: `m_nKeyWordSetIdx` is `m_nSet`, the first from its combo box and the rest from
    // `CDlgKeywordSelect`.
    for (slot, set) in v.numbers(KEYWORD_SETS).iter().enumerate() {
        if let Some(target) = t.keyword_set_idx.get_mut(slot) {
            *target = *set;
        }
    }

    // ウィンドウ
    t.default_eoltype = v.int(DEFAULT_EOL);
    t.default_codetype = v.int(DEFAULT_CODE);
    // `SetData`/`CBN_SELCHANGE`: an encoding without a BOM has none.
    t.default_bom = v.bool(DEFAULT_BOM)
        && Encoding::from_sakura_code(t.default_codetype).is_some_and(Encoding::can_have_bom);

    // 支援 (`CPropTypesSupport::GetData`)
    t.hokan_file = fit(v.text(HOKAN_FILE), PATH_BUFFER);
    t.hokan_lo_hi_case = v.bool(HOKAN_LO_HI_CASE);
    t.use_hokan_by_file = v.bool(HOKAN_BY_FILE);
    t.use_hokan_by_keyword = v.bool(HOKAN_BY_KEYWORD);
    t.ext_help = fit(v.text(EXT_HELP), PATH_BUFFER);
    t.ext_html_help = fit(v.text(EXT_HTML_HELP), PATH_BUFFER);
    t.html_help_is_single = v.bool(HTML_HELP_SINGLE);

    // 正規表現キーワード (`CPropTypesRegex::GetData`)
    t.use_regex_keyword = v.bool(USE_REGEX);
    t.regex_keywords = v
        .rows(REGEX_LIST)
        .iter()
        .filter(|row| !row[0].is_empty())
        .map(|row| RegexKeyword {
            color: color_name(&row[1]).to_owned(),
            pattern: row[0].clone(),
        })
        .collect();

    // キーワードヘルプ (`CPropTypesKeyHelp::GetData`)
    t.use_key_word_help = v.bool(USE_KEYHELP);
    t.use_key_help_all_search = v.bool(KEYHELP_ALL_SEARCH);
    t.use_key_help_key_disp = v.bool(KEYHELP_KEY_DISP);
    t.use_key_help_prefix = v.bool(KEYHELP_PREFIX);
    t.key_help_rmenu_show_type = in_list(v.int(KEYHELP_RMENU), RMENU_TYPES);
    t.key_help = v
        .rows(KEYHELP_LIST)
        .iter()
        .filter(|row| !row[0].is_empty())
        .map(dict_of)
        .collect();
    t
}

/// The changes the user made (`old` to `new`) put over `base`, the type as the file has it now.
/// Only what the pages can change is taken, and only what the user changed: if another process
/// changed the type while the sheet was open, its other changes stay.
pub(super) fn merge(base: &TypeSettings, old: &TypeSettings, new: &TypeSettings) -> TypeSettings {
    let mut out = base.clone();
    macro_rules! take_field {
        ($target:ident, $before:ident, $after:ident, $($field:ident),*) => {
            $(if $after.$field != $before.$field {
                $target.$field = $after.$field;
            })*
        };
    }
    macro_rules! take {
        ($($field:ident),* $(,)?) => {
            $(if new.$field != old.$field {
                out.$field = new.$field.clone();
            })*
        };
    }
    take!(
        type_name,
        type_exts,
        text_wrap_method,
        max_line_ketas,
        tab_space,
        word_wrap,
        kinsoku_ret,
        kinsoku_hide,
        kinsoku_kuto_enabled,
        kinsoku_head_enabled,
        kinsoku_tail_enabled,
        kinsoku_head,
        kinsoku_tail,
        kinsoku_kuto,
        tab_arrow,
        tab_view_string,
        indent_layout,
        default_outline,
        outline_rule_filename,
        ins_space,
        auto_indent,
        auto_indent_zenspace,
        smart_indent,
        indent_chars,
        rtrim_prev_line,
        use_type_font,
        font,
        string_type,
        string_line_only,
        string_end_line,
        vert_line_idx,
        default_eoltype,
        default_codetype,
        default_bom,
        hokan_file,
        ext_help,
        ext_html_help,
        html_help_is_single,
        use_key_word_help,
        use_key_help_all_search,
        use_key_help_key_disp,
        use_key_help_prefix,
        key_help_rmenu_show_type,
        hokan_lo_hi_case,
        use_hokan_by_file,
        use_hokan_by_keyword,
        use_regex_keyword,
        regex_keywords,
        key_help,
    );
    // The keyword sets: each of the ten slots on its own, so that another process's change to one slot
    // stays when the user changed another.
    for i in 0..new.keyword_set_idx.len() {
        if new.keyword_set_idx[i] != old.keyword_set_idx[i] {
            out.keyword_set_idx[i] = new.keyword_set_idx[i];
        }
    }
    for i in 0..new.block_comments.len() {
        if new.block_comments[i] != old.block_comments[i] {
            out.block_comments[i] = new.block_comments[i].clone();
        }
    }
    for i in 0..new.line_comments.len() {
        if new.line_comments[i] != old.line_comments[i] {
            out.line_comments[i] = new.line_comments[i].clone();
        }
    }
    // A color item: each of its five values on its own.
    for name in ColorSet::all_names() {
        if let (Some(before), Some(after), Some(target)) = (
            old.colors.get(name),
            new.colors.get(name),
            out.colors.get_mut(name),
        ) {
            take_field!(target, before, after, disp, bold, underline, text, back);
        }
    }
    out
}
