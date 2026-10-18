//! The ids of the controls of the type settings. A page lists its controls under these names and
//! [`values`](super::values) reads and writes the values under them, so a name is spelled once.

use ume_config::settings::ColorSet;

// ---- スクリーン ----
pub(super) const TYPE_NAME: &str = "type_name";
pub(super) const TYPE_EXTS: &str = "type_exts";
pub(super) const WRAP_METHOD: &str = "wrap_method";
pub(super) const MAX_LINE_KETAS: &str = "max_line_ketas";
pub(super) const COLUMN_SPACE: &str = "column_space";
pub(super) const LINE_SPACE: &str = "line_space";
pub(super) const TAB_SPACE: &str = "tab_space";
pub(super) const TAB_ARROW: &str = "tab_arrow";
pub(super) const TAB_VIEW_STRING: &str = "tab_view_string";
pub(super) const INS_SPACE: &str = "ins_space";
pub(super) const TSV_MODE: &str = "tsv_mode";
pub(super) const AUTO_INDENT: &str = "auto_indent";
pub(super) const AUTO_INDENT_ZENSPACE: &str = "auto_indent_zenspace";
pub(super) const SMART_INDENT: &str = "smart_indent";
pub(super) const INDENT_CHARS: &str = "indent_chars";
pub(super) const INDENT_LAYOUT: &str = "indent_layout";
pub(super) const RTRIM_PREV_LINE: &str = "rtrim_prev_line";
pub(super) const OUTLINE_MODE: &str = "outline_mode";
pub(super) const OUTLINE: &str = "outline";
pub(super) const OUTLINE_RULE_FILE: &str = "outline_rule_file";
pub(super) const OUTLINE_RULE_BROWSE: &str = "outline_rule_browse";
pub(super) const USE_TYPE_FONT: &str = "use_type_font";
pub(super) const TYPE_FONT_BUTTON: &str = "type_font_button";
pub(super) const WORD_WRAP: &str = "word_wrap";
pub(super) const KINSOKU_RET: &str = "kinsoku_ret";
pub(super) const KINSOKU_HIDE: &str = "kinsoku_hide";
pub(super) const KINSOKU_KUTO: &str = "kinsoku_kuto";
pub(super) const KINSOKU_KUTO_CHARS: &str = "kinsoku_kuto_chars";
pub(super) const KINSOKU_HEAD: &str = "kinsoku_head";
pub(super) const KINSOKU_HEAD_CHARS: &str = "kinsoku_head_chars";
pub(super) const KINSOKU_TAIL: &str = "kinsoku_tail";
pub(super) const KINSOKU_TAIL_CHARS: &str = "kinsoku_tail_chars";
/// Not a control: the number of the type, for the controls of the basic type.
pub(super) const TYPE_INDEX: &str = "type_index";

// ---- カラー ----
pub(super) const COLOR_SEL: &str = "color_sel";
pub(super) const COLOR_DISP: &str = "color_disp";
pub(super) const COLOR_BOLD: &str = "color_bold";
pub(super) const COLOR_UNDERLINE: &str = "color_underline";
pub(super) const COLOR_TEXT: &str = "color_text";
pub(super) const COLOR_BACK: &str = "color_back";
pub(super) const SAME_TEXT_COLOR: &str = "same_text_color";
pub(super) const SAME_BACK_COLOR: &str = "same_back_color";
pub(super) const COLORS_IMPORT: &str = "colors_import";
pub(super) const COLORS_EXPORT: &str = "colors_export";
pub(super) const KEYWORD1_NAME: &str = "keyword1_name";
pub(super) const KEYWORD_SELECT: &str = "keyword_select";
pub(super) const KEYWORD_EDIT: &str = "keyword_edit";
pub(super) const BLOCK1_FROM: &str = "block1_from";
pub(super) const BLOCK1_TO: &str = "block1_to";
pub(super) const BLOCK2_FROM: &str = "block2_from";
pub(super) const BLOCK2_TO: &str = "block2_to";
/// The delimiter, the check box "桁" and the column of the three line comments.
pub(super) const LINE_COMMENT: [&str; 3] = ["line_comment1", "line_comment2", "line_comment3"];
pub(super) const LINE_COMMENT_POS_ON: [&str; 3] = [
    "line_comment1_pos_on",
    "line_comment2_pos_on",
    "line_comment3_pos_on",
];
pub(super) const LINE_COMMENT_POS: [&str; 3] = [
    "line_comment1_pos",
    "line_comment2_pos",
    "line_comment3_pos",
];
pub(super) const STRING_TYPE: &str = "string_type";
pub(super) const STRING_LINE_ONLY: &str = "string_line_only";
pub(super) const STRING_END_LINE: &str = "string_end_line";
pub(super) const VERT_LINES: &str = "vert_lines";

// ---- ウィンドウ ----
pub(super) const DEFAULT_EOL: &str = "default_eol";
pub(super) const DEFAULT_BOM: &str = "default_bom";
pub(super) const DEFAULT_CODE: &str = "default_code";
pub(super) const CODE_PAGE: &str = "code_page";
pub(super) const PRIOR_CESU8: &str = "prior_cesu8";
pub(super) const IME_SWITCH: &str = "ime_switch";
pub(super) const IME_STATE: &str = "ime_state";
pub(super) const USE_DOC_ICON: &str = "use_doc_icon";
pub(super) const LINE_NUM_MODE: &str = "line_num_mode";
pub(super) const LINE_NUM_WIDTH: &str = "line_num_width";
pub(super) const LINE_TERM_TYPE: &str = "line_term_type";
pub(super) const LINE_TERM_CHAR: &str = "line_term_char";
pub(super) const BG_PATH: &str = "bg_path";
pub(super) const BG_BROWSE: &str = "bg_browse";
pub(super) const BG_POS: &str = "bg_pos";
pub(super) const BG_OFFSET_X: &str = "bg_offset_x";
pub(super) const BG_OFFSET_Y: &str = "bg_offset_y";
pub(super) const BG_REPEAT_X: &str = "bg_repeat_x";
pub(super) const BG_REPEAT_Y: &str = "bg_repeat_y";
pub(super) const BG_SCROLL_X: &str = "bg_scroll_x";
pub(super) const BG_SCROLL_Y: &str = "bg_scroll_y";
pub(super) const BG_TRANSPARENCY: &str = "bg_transparency";

/// What of a color item the dialog keeps for every item, whichever is chosen.
#[derive(Clone, Copy)]
pub(super) enum Field {
    Disp,
    Bold,
    Underline,
    Text,
    Back,
}

impl Field {
    pub(super) const ALL: [Field; 5] = [
        Field::Disp,
        Field::Bold,
        Field::Underline,
        Field::Text,
        Field::Back,
    ];

    fn suffix(self) -> &'static str {
        match self {
            Field::Disp => "disp",
            Field::Bold => "bold",
            Field::Underline => "underline",
            Field::Text => "text",
            Field::Back => "back",
        }
    }
}

/// The id under which the dialog keeps `field` of the color item number `item` (the order of
/// [`ColorSet::all_names`]). The names are made once and live as long as the program: a value is
/// found by a `&'static str`.
pub(super) fn item_key(item: usize, field: Field) -> &'static str {
    use std::sync::OnceLock;
    static KEYS: OnceLock<Vec<[&'static str; 5]>> = OnceLock::new();
    let keys = KEYS.get_or_init(|| {
        (0..ColorSet::all_names().count())
            .map(|item| {
                Field::ALL.map(|field| {
                    let name: &'static str =
                        Box::leak(format!("color_item_{item}_{}", field.suffix()).into_boxed_str());
                    name
                })
            })
            .collect()
    });
    keys[item.min(keys.len() - 1)][field as usize]
}

// ---- 支援 ----
pub(super) const HOKAN_FILE: &str = "hokan_file";
pub(super) const HOKAN_FILE_BROWSE: &str = "hokan_file_browse";
pub(super) const HOKAN_TYPE: &str = "hokan_type";
pub(super) const HOKAN_LO_HI_CASE: &str = "hokan_lo_hi_case";
pub(super) const HOKAN_BY_FILE: &str = "hokan_by_file";
pub(super) const HOKAN_BY_KEYWORD: &str = "hokan_by_keyword";
pub(super) const EXT_HELP: &str = "ext_help";
pub(super) const EXT_HELP_BROWSE: &str = "ext_help_browse";
pub(super) const EXT_HTML_HELP: &str = "ext_html_help";
pub(super) const EXT_HTML_HELP_BROWSE: &str = "ext_html_help_browse";
pub(super) const HTML_HELP_SINGLE: &str = "html_help_single";
pub(super) const CHK_ENTER_AT_END: &str = "chk_enter_at_end";
pub(super) const INDENT_CPP_STRING: &str = "indent_cpp_string";
pub(super) const INDENT_CPP_COMMENT: &str = "indent_cpp_comment";
pub(super) const INDENT_CPP_UNDO: &str = "indent_cpp_undo";
pub(super) const DETECT_INDENT: &str = "detect_indent";

// ---- 正規表現キーワード ----
pub(super) const USE_REGEX: &str = "use_regex";
pub(super) const REGEX_LIST: &str = "regex_list";
pub(super) const REGEX_IMPORT: &str = "regex_import";
pub(super) const REGEX_EXPORT: &str = "regex_export";

// ---- キーワードヘルプ ----
pub(super) const USE_KEYHELP: &str = "use_keyhelp";
pub(super) const KEYHELP_LIST: &str = "keyhelp_list";
pub(super) const KEYHELP_IMPORT: &str = "keyhelp_import";
pub(super) const KEYHELP_EXPORT: &str = "keyhelp_export";
pub(super) const KEYHELP_ALL_SEARCH: &str = "keyhelp_all_search";
pub(super) const KEYHELP_KEY_DISP: &str = "keyhelp_key_disp";
pub(super) const KEYHELP_PREFIX: &str = "keyhelp_prefix";
pub(super) const KEYHELP_RMENU: &str = "keyhelp_rmenu";
