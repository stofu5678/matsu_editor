// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/env/CShareData_IO.cpp (ShareData_IO_Types and the functions it calls)
//   sakura_core/types/CType.cpp, sakura_core/types/CType_Basis.cpp (defaults)
//   sakura_core/config/app_constants.h, sakura_core/config/maxdata.h (limits)
//
// Original copyright notices of those files:
//   Copyright (C) 2003-2006, D.S.Koba
//   Copyright (C) 2004, MIK, genta
//   Copyright (C) 2006, ryoji
//   Copyright (C) 2008, kobake
//   Copyright (C) 2009, ryoji
//   Copyright (C) 2018-2026, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// zlib License
//
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! Settings of a document type: the `Types(N)` sections (type 0 is "基本", the basic type).
//!
//! Source: `sakura_core/env/CShareData_IO.cpp` (`ShareData_IO_Types`, `ShareData_IO_TypeConfig`,
//! `ShareData_IO_TypeInts`, `ShareData_IO_LineSpace`, `ShareData_IO_LineNumWidth`, `ShareData_IO_TypeId`),
//! `sakura_core/types/CType.cpp` (`_DefaultConfig`, the defaults), `sakura_core/types/CType_Basis.cpp`
//! (the basic type) and `sakura_core/config/app_constants.h` (the limits).
//!
//! Typed here: what the first editor window needs, and the rules the coloring reads (comments, strings,
//! keyword sets, numbers, regular expression keywords, the outline). Everything else in a `Types(N)` section
//! stays in the [`Profile`] as raw entries and is kept when the file is written.
//!
//! The built-in types of Sakura (基本, テキスト, C/C++, ...) are in [`super::builtin_types()`].

use super::color::{ColorSet, names};
use super::font::FontSetting;
use super::type_rules::{
    BLOCK_COMMENT_COUNT, BlockComment, DEFAULT_REGEX_COLOR, KeyHelpDict, LINE_COMMENT_COUNT,
    LineComment, MAX_KEYHELP_FILES, MAX_REGEX_KEYWORDS, MAX_VERT_LINES, RegexKeyword, outline_type,
    smart_indent, string_type,
};
use crate::profile::{Profile, ProfileError};
use crate::value::{parse_long, scan_ints};

/// `MAXLINEKETAS`: the largest wrap column.
pub const MAX_LINE_KETAS: i32 = 10240;
/// `MINLINEKETAS`: the smallest wrap column.
pub const MIN_LINE_KETAS: i32 = 10;
/// `LINENUMWIDTH_MIN`/`LINENUMWIDTH_MAX`.
pub const LINE_NUM_WIDTH_RANGE: (i32, i32) = (2, 11);
/// `LINESPACE_MAX`: the line space is from `-LINE_SPACE_MAX` to `LINE_SPACE_MAX`.
pub const LINE_SPACE_MAX: i32 = 128;
/// `MAX_TYPES` (`sakura_core/config/maxdata.h`).
pub const MAX_TYPES: i32 = 60;

/// `ECodeType` (`sakura_core/charset/charset.h`) as stored in `eDefaultCodetype`.
pub mod code_type {
    pub const SJIS: i32 = 0;
    pub const JIS: i32 = 1;
    pub const EUC: i32 = 2;
    pub const UTF16LE: i32 = 3;
    pub const UTF8: i32 = 4;
    pub const UTF7: i32 = 5;
    pub const UTF16BE: i32 = 6;
    pub const CESU8: i32 = 7;
    pub const LATIN1: i32 = 8;
    pub const UTF32LE: i32 = 12000;
    pub const UTF32BE: i32 = 12001;
}

/// `EEolType` (`sakura_core/basis/CEol.h`) as stored in `eDefaultEoltype`.
pub mod eol_type {
    /// No conversion.
    pub const NONE: i32 = 0;
    pub const CRLF: i32 = 1;
    pub const LF: i32 = 2;
    pub const CR: i32 = 3;
    pub const NEL: i32 = 4;
    pub const LS: i32 = 5;
    pub const PS: i32 = 6;
}

/// `ETabArrow` (`sakura_core/types/CType.h`) as stored in `bTabArrow`.
pub mod tab_arrow {
    /// Show `szTabViewString`.
    pub const STRING: i32 = 0;
    pub const SHORT_ARROW: i32 = 1;
    pub const LONG_ARROW: i32 = 2;
}

/// `nTextWrapMethod` (`sakura_core/_main/global.h`).
pub mod wrap_method {
    /// Do not wrap.
    pub const NONE: i32 = 0;
    /// Wrap at `nMaxLineKetas` columns.
    pub const SETTING_WIDTH: i32 = 1;
    /// Wrap at the right edge of the window.
    pub const WINDOW_WIDTH: i32 = 2;
}

/// Buffer sizes of the strings of a type (`sakura_core/config/maxdata.h`, `sakura_core/types/CType.h`), in
/// UTF-16 units including the end mark.
const MAX_TYPES_NAME: usize = 64;
const MAX_TYPES_EXTS: usize = 64;
const TAB_VIEW_STRING_SIZE: usize = 8 + 1;
const INDENT_CHARS_SIZE: usize = 64;
const KINSOKU_SIZE: usize = 200;

/// The default kinsoku characters of every type but "テキスト" are empty; `szKinsokuKuto` has a default
/// (`_DefaultConfig`, 2009.08.07 ryoji).
const DEFAULT_KINSOKU_KUTO: &str = "、。，．､｡,.";

/// Typed settings of one document type.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeSettings {
    // --- `nInts`: `idx,maxLineKetas,columnSpace,tabSpace,keywordSet1,keywordSet2,stringType,lineNumIsCRLF,
    //     lineTermType,wordWrap,printSetting,tsvMode` ---
    /// Index of the type (`m_nIdx`); Sakura keeps it in the file as well.
    pub idx: i32,
    /// `nMaxLineKetas`: the wrap column. The file value is limited to 10..=10240.
    pub max_line_ketas: i32,
    /// `nColumnSpace`: space between characters.
    pub column_space: i32,
    /// `nTabSpace`: the tab width. Limited to 2..=`max_line_ketas - 2`.
    pub tab_space: i32,
    /// `nKeyWordSetIdx[0..10]` (the first two in `nInts`, the rest in `nKeywordSelect3` to `10`).
    pub keyword_set_idx: [i32; 10],
    /// `nStringType` ([`string_type`]): how a quote inside a string is written.
    pub string_type: i32,
    /// Line numbers count lines (true) or wrapped rows (false).
    pub line_num_is_crlf: bool,
    /// `nLineTermType`: 0 none, 1 vertical line, 2 `line_term_char`.
    pub line_term_type: i32,
    /// English word wrap.
    pub word_wrap: bool,
    pub current_print_setting: i32,
    pub tsv_mode: i32,

    // --- other keys ---
    /// `nLineSpace`: space between lines. Limited to -128..=128.
    pub line_space: i32,
    /// `nLineNumWidth`: the least width of the line number area. Limited to 2..=11.
    pub line_num_width: i32,
    /// `szTypeName`.
    pub type_name: String,
    /// `szTypeExts`: the file extensions, separated by `,` (or `;`).
    pub type_exts: String,
    /// `id`: Sakura keeps the absolute value.
    pub id: i32,
    /// `szTabViewString`: what a tab looks like when `tab_arrow` is [`tab_arrow::STRING`].
    pub tab_view_string: String,
    /// `bTabArrow` ([`tab_arrow`]).
    pub tab_arrow: i32,
    /// `bInsSpace`: the Tab key inserts spaces.
    pub ins_space: bool,
    /// `nTextWrapMethod` ([`wrap_method`]).
    pub text_wrap_method: i32,
    /// `szIndentChars`.
    pub indent_chars: String,
    /// `cLineTermChar`: the character between the line numbers and the text. `'\0'` is "none".
    pub line_term_char: char,
    /// `bPriorCesu8`.
    pub prior_cesu8: bool,
    /// `eDefaultCodetype` ([`code_type`]): the encoding of a new file.
    pub default_codetype: i32,
    /// `eDefaultEoltype` ([`eol_type`]): the line ending of a new file.
    pub default_eoltype: i32,
    /// `bDefaultBom`: a new file gets a BOM.
    pub default_bom: bool,
    pub auto_indent: bool,
    /// `bAutoIndent_ZENSPACE`: a full-width space counts as indent.
    pub auto_indent_zenspace: bool,
    /// `bRTrimPrevLine`: remove trailing spaces of the previous line on Enter.
    pub rtrim_prev_line: bool,
    pub indent_layout: i32,
    // Kinsoku (line breaking rules).
    /// `bKinsokuHead`: do not put `kinsoku_head` characters at the start of a line.
    pub kinsoku_head_enabled: bool,
    /// `bKinsokuTail`: do not put `kinsoku_tail` characters at the end of a line.
    pub kinsoku_tail_enabled: bool,
    /// `bKinsokuRet`: let a line end hang out.
    pub kinsoku_ret: bool,
    /// `bKinsokuKuto`: let `kinsoku_kuto` characters hang out.
    pub kinsoku_kuto_enabled: bool,
    /// `bKinsokuHide`: hide the hanging characters.
    pub kinsoku_hide: bool,
    /// `szKinsokuHead`.
    pub kinsoku_head: String,
    /// `szKinsokuTail`.
    pub kinsoku_tail: String,
    /// `szKinsokuKuto`.
    pub kinsoku_kuto: String,
    /// `bChkEnterAtEnd`: warn on saving when line endings are mixed.
    pub warn_mixed_eol: bool,
    /// `bIndentCppStringIgnore`: the C/C++ smart indent skips strings and char literals.
    pub indent_cpp_string_ignore: bool,
    /// `bIndentCppCommentIgnore`: the C/C++ smart indent skips comments.
    pub indent_cpp_comment_ignore: bool,
    /// `bIndentCppUndoSep`: the C/C++ smart indent is an undo step of its own.
    pub indent_cpp_undo_sep: bool,
    /// `bDetectIndentationStyleOnFileLoad`: detect the indentation style when a file is loaded.
    pub detect_indentation_on_load: bool,
    /// `szHokanFile`: the word file of the input completion (入力補完 単語ファイル).
    pub hokan_file: String,
    /// `nHokanType`: the plugin that adds candidates to the input completion (0: none).
    pub hokan_type: i32,
    /// `bHokanLoHiCase`: the input completion ignores the case of letters.
    pub hokan_lo_hi_case: bool,
    /// `bUseHokanByFile`: the input completion offers the words of the document.
    pub use_hokan_by_file: bool,
    /// `bUseHokanByKeyword`: the input completion offers the highlighted keywords.
    pub use_hokan_by_keyword: bool,
    /// `bUseKeyWordHelp`: the keyword help is on.
    pub use_key_word_help: bool,
    /// `bUseKeyHelpAllSearch`: the dictionaries after the one that hit are searched too.
    pub use_key_help_all_search: bool,
    /// `bUseKeyHelpKeyDisp`: the first line of the tip shows the keyword.
    pub use_key_help_key_disp: bool,
    /// `bUseKeyHelpPrefix`: a dictionary key that starts with the word counts as a hit.
    pub use_key_help_prefix: bool,
    /// `nKeyHelpRMenuShowType` (`EKeyHelpRMenuType`): 0 not in the right-click menu, 1 at the
    /// top, 2 at the bottom.
    pub key_help_rmenu_show_type: i32,
    /// `m_KeyHelpArr` (`KDct[NN]`): the keyword help dictionaries, the first has the priority.
    pub key_help: Vec<KeyHelpDict>,
    /// `szExtHelp`: the external help file of this type (empty: the one of the common settings).
    pub ext_help: String,
    /// `szExtHtmlHelp`: the external HTML help file of this type.
    pub ext_html_help: String,
    /// `bTypeHtmlHelpIsSingle`: one viewer for the HTML help of this type.
    pub html_help_is_single: bool,
    /// `bUseTypeFont`: this type has its own font.
    pub use_type_font: bool,
    /// `lf`, `lfFaceName`, `nPointSize` of this type.
    pub font: FontSetting,
    /// `C[...]`: colours and what is shown.
    pub colors: ColorSet,

    // --- the rules of the coloring ---
    /// `m_cLineComment`: up to 3 line comment delimiters (`szLineComment`, `nLineCommentColumn`).
    pub line_comments: [LineComment; LINE_COMMENT_COUNT],
    /// `m_cBlockComments`: up to 2 block comment pairs (`szBlockCommentFrom`, `szBlockCommentTo`).
    pub block_comments: [BlockComment; BLOCK_COMMENT_COUNT],
    /// `bStringLineOnly`: a string ends at the end of the line.
    pub string_line_only: bool,
    /// `bStringEndLine`: a string with no end quote is colored to the end of the line.
    pub string_end_line: bool,
    /// `m_nHeredocType` (0 PHP, 1 Ruby, 2 Perl). Sakura does not keep it in the file.
    pub heredoc_type: i32,
    /// `bUseRegexKeyword`.
    pub use_regex_keyword: bool,
    /// `m_RegexKeywordArr` and `m_RegexKeywordList` (`RxKey[NNN]`).
    pub regex_keywords: Vec<RegexKeyword>,
    /// `nVertLineIdx1` to `200`: the columns of vertical lines (1 is the first column). The list
    /// ends at the first 0; see [`TypeSettings::vert_lines`].
    pub vert_line_idx: [i32; MAX_VERT_LINES],
    /// `nNoteLineOffset`: how far the note lines (`NOT`) are moved down, in pixels.
    pub note_line_offset: i32,

    // --- the outline and the smart indent ---
    /// `nDefaultOutline` ([`outline_type`]).
    pub default_outline: i32,
    /// `szOutlineRuleFilename`: the rule file of [`outline_type::FILE`].
    pub outline_rule_filename: String,
    /// `nSmartIndent` ([`smart_indent`]).
    pub smart_indent: i32,
}

impl Default for TypeSettings {
    /// `_DefaultConfig`.
    fn default() -> Self {
        TypeSettings {
            idx: 0,
            max_line_ketas: MAX_LINE_KETAS,
            column_space: 0,
            tab_space: 4,
            keyword_set_idx: [-1; 10],
            string_type: string_type::CPP,
            line_num_is_crlf: true,
            line_term_type: 1,
            word_wrap: false,
            current_print_setting: 0,
            tsv_mode: 0,
            line_space: 1,
            line_num_width: LINE_NUM_WIDTH_RANGE.0,
            type_name: String::new(),
            type_exts: String::new(),
            id: 0,
            tab_view_string: "^       ".to_owned(),
            tab_arrow: tab_arrow::STRING,
            ins_space: false,
            text_wrap_method: wrap_method::NONE,
            indent_chars: String::new(),
            line_term_char: ':',
            prior_cesu8: false,
            default_codetype: code_type::UTF8,
            default_eoltype: eol_type::CRLF,
            default_bom: false,
            auto_indent: true,
            auto_indent_zenspace: true,
            rtrim_prev_line: false,
            indent_layout: 0,
            kinsoku_head_enabled: false,
            kinsoku_tail_enabled: false,
            kinsoku_ret: false,
            kinsoku_kuto_enabled: false,
            kinsoku_hide: false,
            kinsoku_head: String::new(),
            kinsoku_tail: String::new(),
            kinsoku_kuto: DEFAULT_KINSOKU_KUTO.to_owned(),
            warn_mixed_eol: true,
            // `_DefaultConfig` (`CType.cpp`)
            indent_cpp_string_ignore: true,
            indent_cpp_comment_ignore: true,
            indent_cpp_undo_sep: false,
            detect_indentation_on_load: true,
            // `_DefaultConfig` (`CType.cpp`)
            hokan_file: String::new(),
            hokan_type: 0,
            hokan_lo_hi_case: false,
            use_hokan_by_file: true,
            use_hokan_by_keyword: false,
            // `_DefaultConfig` (`CType.cpp`): the keyword help is off, the menu shows it at the top.
            use_key_word_help: false,
            use_key_help_all_search: false,
            use_key_help_key_disp: false,
            use_key_help_prefix: false,
            key_help_rmenu_show_type: 1,
            key_help: Vec::new(),
            ext_help: String::new(),
            ext_html_help: String::new(),
            html_help_is_single: true,
            use_type_font: false,
            // `_DefaultConfig` does not set `m_lf`; an unused, zeroed font.
            font: FontSetting {
                log_font: super::font::LogFont {
                    height: 0,
                    width: 0,
                    escapement: 0,
                    orientation: 0,
                    weight: 0,
                    italic: 0,
                    underline: 0,
                    strike_out: 0,
                    char_set: 0,
                    out_precision: 0,
                    clip_precision: 0,
                    quality: 0,
                    pitch_and_family: 0,
                    face_name: String::new(),
                },
                point_size: 0,
            },
            colors: ColorSet::default(),
            line_comments: Default::default(),
            block_comments: Default::default(),
            string_line_only: false,
            string_end_line: false,
            heredoc_type: 0,
            use_regex_keyword: false,
            regex_keywords: Vec::new(),
            vert_line_idx: [0; MAX_VERT_LINES],
            note_line_offset: 0,
            default_outline: outline_type::TEXT,
            outline_rule_filename: String::new(),
            smart_indent: smart_indent::NONE,
        }
    }
}

fn clamp_to(value: i32, min: i32, max: i32) -> i32 {
    // `SetValueLimit`: `max(min, min(max, value))`; the minimum wins when the range is empty.
    min.max(max.min(value))
}

/// The new-file values that one `Types(N)` section sets ([`TypeSettings::new_file_keys`]); `None`
/// where the section has no (usable) value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NewFileKeys {
    /// `eDefaultCodetype` ([`code_type`]).
    pub codetype: Option<i32>,
    /// `eDefaultEoltype` ([`eol_type`]).
    pub eoltype: Option<i32>,
    /// `bDefaultBom`.
    pub bom: Option<bool>,
}

impl TypeSettings {
    /// The section name for a type index: `Types(0)`.
    pub fn section_name(index: i32) -> String {
        format!("Types({index})")
    }

    /// The basic type "基本" with its defaults (`CType_Basis::InitTypeConfigImp`): single and double quoted
    /// strings are not coloured.
    pub fn basic() -> Self {
        super::builtin_types::basic()
    }

    /// Reads the settings of type `index` from its `Types(N)` section over `self`. Keys that are missing or
    /// broken keep the value `self` has, as in Sakura.
    ///
    /// Sakura starts from a built-in setting for each of its 17 default types and from a copy of the basic
    /// type for the others. This crate only has the basic type's defaults ([`TypeSettings::basic`]).
    pub fn read_over(&mut self, profile: &Profile, index: i32) {
        let s = Self::section_name(index);
        let s = s.as_str();

        // ShareData_IO_TypeInts: all 12 numbers are needed.
        if let Some(n) = profile.get(s, "nInts").and_then(scan_ints::<12>) {
            self.idx = n[0];
            self.max_line_ketas = clamp_to(n[1], MIN_LINE_KETAS, MAX_LINE_KETAS);
            self.column_space = n[2];
            self.tab_space = clamp_to(n[3], 2, self.max_line_ketas - 2);
            self.keyword_set_idx[0] = n[4];
            self.keyword_set_idx[1] = n[5];
            self.string_type = n[6];
            self.line_num_is_crlf = n[7] != 0;
            self.line_term_type = n[8];
            self.word_wrap = n[9] != 0;
            self.current_print_setting = n[10];
            self.tsv_mode = n[11];
        }
        for i in 2..10 {
            if let Some(v) = profile.get_i32(s, &format!("nKeywordSelect{}", i + 1)) {
                self.keyword_set_idx[i] = v;
            }
        }
        if let Some(v) = profile.get_i32(s, "nLineSpace") {
            self.line_space = clamp_to(v, -LINE_SPACE_MAX, LINE_SPACE_MAX);
        }
        if let Some(v) = profile.get_i32(s, "nLineNumWidth") {
            self.line_num_width = clamp_to(v, LINE_NUM_WIDTH_RANGE.0, LINE_NUM_WIDTH_RANGE.1);
        }
        read_string(
            profile,
            s,
            "szTypeName",
            &mut self.type_name,
            MAX_TYPES_NAME,
        );
        read_string(
            profile,
            s,
            "szTypeExts",
            &mut self.type_exts,
            MAX_TYPES_EXTS,
        );
        if let Some(v) = profile.get_i32(s, "id") {
            // ShareData_IO_TypeId
            self.id = v.wrapping_abs();
        }
        read_string(
            profile,
            s,
            "szTabViewString",
            &mut self.tab_view_string,
            TAB_VIEW_STRING_SIZE,
        );
        read_i32(profile, s, "bTabArrow", &mut self.tab_arrow);
        read_bool(profile, s, "bInsSpace", &mut self.ins_space);
        read_i32(profile, s, "nTextWrapMethod", &mut self.text_wrap_method);
        read_string(
            profile,
            s,
            "szIndentChars",
            &mut self.indent_chars,
            INDENT_CHARS_SIZE,
        );
        if let Some(c) = profile.get_wchar(s, "cLineTermChar") {
            self.line_term_char = c;
        }
        read_bool(profile, s, "bPriorCesu8", &mut self.prior_cesu8);
        read_i32(profile, s, "eDefaultCodetype", &mut self.default_codetype);
        read_i32(profile, s, "eDefaultEoltype", &mut self.default_eoltype);
        read_bool(profile, s, "bDefaultBom", &mut self.default_bom);
        read_bool(profile, s, "bAutoIndent", &mut self.auto_indent);
        read_bool(
            profile,
            s,
            "bAutoIndent_ZENSPACE",
            &mut self.auto_indent_zenspace,
        );
        read_bool(profile, s, "bRTrimPrevLine", &mut self.rtrim_prev_line);
        read_i32(profile, s, "nIndentLayout", &mut self.indent_layout);
        read_bool(profile, s, "bKinsokuHead", &mut self.kinsoku_head_enabled);
        read_bool(profile, s, "bKinsokuTail", &mut self.kinsoku_tail_enabled);
        read_bool(profile, s, "bKinsokuRet", &mut self.kinsoku_ret);
        read_bool(profile, s, "bKinsokuKuto", &mut self.kinsoku_kuto_enabled);
        read_bool(profile, s, "bKinsokuHide", &mut self.kinsoku_hide);
        read_string(
            profile,
            s,
            "szKinsokuHead",
            &mut self.kinsoku_head,
            KINSOKU_SIZE,
        );
        read_string(
            profile,
            s,
            "szKinsokuTail",
            &mut self.kinsoku_tail,
            KINSOKU_SIZE,
        );
        read_string(
            profile,
            s,
            "szKinsokuKuto",
            &mut self.kinsoku_kuto,
            KINSOKU_SIZE,
        );
        read_bool(profile, s, "bChkEnterAtEnd", &mut self.warn_mixed_eol);
        read_bool(
            profile,
            s,
            "bIndentCppStringIgnore",
            &mut self.indent_cpp_string_ignore,
        );
        read_bool(
            profile,
            s,
            "bIndentCppCommentIgnore",
            &mut self.indent_cpp_comment_ignore,
        );
        read_bool(
            profile,
            s,
            "bIndentCppUndoSep",
            &mut self.indent_cpp_undo_sep,
        );
        read_bool(
            profile,
            s,
            "bDetectIndentationStyleOnFileLoad",
            &mut self.detect_indentation_on_load,
        );
        read_bool(profile, s, "bUseKeyWordHelp", &mut self.use_key_word_help);
        read_bool(
            profile,
            s,
            "bUseKeyHelpAllSearch",
            &mut self.use_key_help_all_search,
        );
        read_bool(
            profile,
            s,
            "bUseKeyHelpKeyDisp",
            &mut self.use_key_help_key_disp,
        );
        read_bool(
            profile,
            s,
            "bUseKeyHelpPrefix",
            &mut self.use_key_help_prefix,
        );
        read_i32(
            profile,
            s,
            "nKeyHelpRMenuShowType",
            &mut self.key_help_rmenu_show_type,
        );
        self.read_key_help(profile, s);
        read_string(profile, s, "szExtHelp", &mut self.ext_help, MAX_PATH);
        read_string(
            profile,
            s,
            "szExtHtmlHelp",
            &mut self.ext_html_help,
            MAX_PATH,
        );
        read_bool(
            profile,
            s,
            "bTypeHtmlHelpIsSingle",
            &mut self.html_help_is_single,
        );
        read_string(profile, s, "szHokanFile", &mut self.hokan_file, MAX_PATH);
        read_i32(profile, s, "nHokanType", &mut self.hokan_type);
        read_bool(profile, s, "bHokanLoHiCase", &mut self.hokan_lo_hi_case);
        read_bool(profile, s, "bUseHokanByFile", &mut self.use_hokan_by_file);
        read_bool(
            profile,
            s,
            "bUseHokanByKeyword",
            &mut self.use_hokan_by_keyword,
        );
        read_bool(profile, s, "bUseTypeFont", &mut self.use_type_font);
        self.read_rules(profile, s);
        self.font.read(profile, s, "lf", None, None);
        self.colors.read(profile, s);
    }

    /// `ShareData_IO_KeyHelpArr`, read side: the old key `szKeyWordHelpFile` is one dictionary (not
    /// used) and ends the reading; else `KDct[00]`, `KDct[01]` ... up to the first that is missing,
    /// does not have the form, or has no path.
    fn read_key_help(&mut self, profile: &Profile, s: &str) {
        if let Some(path) = profile.get_string(s, "szKeyWordHelpFile") {
            self.key_help = vec![KeyHelpDict {
                used: false,
                about: String::new(),
                path,
            }];
            return;
        }
        self.key_help = (0..MAX_KEYHELP_FILES)
            .map_while(|i| {
                profile
                    .get(s, &format!("KDct[{i:02}]"))
                    .and_then(KeyHelpDict::parse)
                    .filter(|dict| !dict.path.is_empty())
            })
            .collect();
    }

    /// `ShareData_IO_KeyHelpArr`, write side: the dictionaries, and no key of a dictionary that
    /// is gone (Sakura writes the file anew).
    fn write_key_help(&self, profile: &mut Profile, s: &str) -> Result<(), ProfileError> {
        profile.remove(s, "szKeyWordHelpFile");
        for i in 0..MAX_KEYHELP_FILES {
            let key = format!("KDct[{i:02}]");
            match self.key_help.get(i) {
                Some(dict) => profile.set(s, &key, &dict.format())?,
                None => {
                    profile.remove(s, &key);
                }
            }
        }
        Ok(())
    }

    /// The coloring rules, the outline and the smart indent of a `Types(N)` section.
    fn read_rules(&mut self, profile: &Profile, s: &str) {
        read_bool(profile, s, "bStringLineOnly", &mut self.string_line_only);
        read_bool(profile, s, "bStringEndLine", &mut self.string_end_line);
        // ShareData_IO_BlockComments: the start and the end are read together; if one is missing the pair stays.
        for (i, block) in self.block_comments.iter_mut().enumerate() {
            let suffix = if i > 0 {
                (i + 1).to_string()
            } else {
                String::new()
            };
            let from = profile.get(s, &format!("szBlockCommentFrom{suffix}"));
            let to = profile.get(s, &format!("szBlockCommentTo{suffix}"));
            if let (Some(from), Some(to)) = (from, to) {
                *block = BlockComment::new(from, to);
            }
        }
        // ShareData_IO_LineComments: the delimiter and the column are read together.
        for (i, line) in self.line_comments.iter_mut().enumerate() {
            let suffix = if i > 0 {
                (i + 1).to_string()
            } else {
                String::new()
            };
            let text = profile.get(s, &format!("szLineComment{suffix}"));
            let column = profile.get_i32(s, &format!("nLineCommentColumn{suffix}"));
            if let (Some(text), Some(column)) = (text, column) {
                *line = LineComment::new(text, column);
            }
        }
        read_string(
            profile,
            s,
            "szOutlineRuleFilename",
            &mut self.outline_rule_filename,
            MAX_PATH,
        );
        read_i32(profile, s, "nDefaultOutline", &mut self.default_outline);
        read_i32(profile, s, "nSmartIndent", &mut self.smart_indent);
        // ShareData_IO_VertLineIdx: a column that is 0 ends the list.
        for (i, slot) in self.vert_line_idx.iter_mut().enumerate() {
            if let Some(v) = profile.get_i32(s, &format!("nVertLineIdx{}", i + 1)) {
                *slot = v;
                if v == 0 {
                    break;
                }
            }
        }
        read_i32(profile, s, "nNoteLineOffset", &mut self.note_line_offset);
        // ShareData_IO_RegexKeyword: `RxKey[NNN]=color,pattern`; the first missing number ends it.
        read_bool(profile, s, "bUseRegexKeyword", &mut self.use_regex_keyword);
        for i in 0..MAX_REGEX_KEYWORDS {
            let Some(value) = profile.get(s, &format!("RxKey[{i:03}]")) else {
                break;
            };
            let keyword = parse_regex_keyword(value);
            match self.regex_keywords.get_mut(i) {
                Some(slot) => *slot = keyword,
                None => self.regex_keywords.push(keyword),
            }
        }
        // The list ends at the first empty pattern (`RegexKeyCompile` counts to it), so a blank
        // `RxKey[NNN]=` after the last keyword also ends the defaults of a built-in type.
        if let Some(end) = self
            .regex_keywords
            .iter()
            .position(|k| k.pattern.is_empty())
        {
            self.regex_keywords.truncate(end);
        }
    }

    /// Reads type `index` over the defaults of the basic type.
    pub fn read(profile: &Profile, index: i32) -> Self {
        let mut t = TypeSettings::basic();
        t.read_over(profile, index);
        t
    }

    /// Which of the new-file values (`eDefaultCodetype`, `eDefaultEoltype`, `bDefaultBom`) the
    /// `Types(N)` section of type `index` sets itself. [`read`](Self::read) cannot tell a value
    /// the file sets from the built-in default; MATSU Editor needs to, because its default for the
    /// line ending is not Sakura's (it follows the OS when the file does not say).
    pub fn new_file_keys(profile: &Profile, index: i32) -> NewFileKeys {
        let s = Self::section_name(index);
        NewFileKeys {
            codetype: profile.get_i32(&s, "eDefaultCodetype"),
            eoltype: profile.get_i32(&s, "eDefaultEoltype"),
            bom: profile.get_bool(&s, "bDefaultBom"),
        }
    }

    /// Writes the settings to the `Types(N)` section of type `index`. Entries that already hold the same
    /// value are left as the user wrote them; other entries of the section are not touched.
    pub fn write(&self, profile: &mut Profile, index: i32) -> Result<(), ProfileError> {
        let s = Self::section_name(index);
        let s = s.as_str();

        // ShareData_IO_TypeInts. The numbers are compared one by one so that spacing is kept when nothing
        // changed. Reading limits two of them, so a value the file holds out of range is also kept.
        let ints = [
            self.idx,
            self.max_line_ketas,
            self.column_space,
            self.tab_space,
            self.keyword_set_idx[0],
            self.keyword_set_idx[1],
            self.string_type,
            i32::from(self.line_num_is_crlf),
            self.line_term_type,
            i32::from(self.word_wrap),
            self.current_print_setting,
            self.tsv_mode,
        ];
        let current = profile.get(s, "nInts").and_then(scan_ints::<12>);
        let same = current.is_some_and(|c| {
            let mut c = c;
            c[1] = clamp_to(c[1], MIN_LINE_KETAS, MAX_LINE_KETAS);
            c[3] = clamp_to(c[3], 2, c[1] - 2);
            c[7] = i32::from(c[7] != 0);
            c[9] = i32::from(c[9] != 0);
            c == ints
        });
        if !same {
            let value = ints.map(|n| n.to_string()).join(",");
            profile.set(s, "nInts", &value)?;
        }
        for i in 2..10 {
            profile.set_i32(
                s,
                &format!("nKeywordSelect{}", i + 1),
                self.keyword_set_idx[i],
            )?;
        }
        write_clamped(
            profile,
            s,
            "nLineSpace",
            self.line_space,
            -LINE_SPACE_MAX,
            LINE_SPACE_MAX,
        )?;
        write_clamped(
            profile,
            s,
            "nLineNumWidth",
            self.line_num_width,
            LINE_NUM_WIDTH_RANGE.0,
            LINE_NUM_WIDTH_RANGE.1,
        )?;
        profile.set_string(s, "szTypeName", &self.type_name)?;
        profile.set_string(s, "szTypeExts", &self.type_exts)?;
        // The file may hold a negative id that reads as its absolute value.
        if profile.get_i32(s, "id").map(i32::wrapping_abs) != Some(self.id) {
            profile.set_i32(s, "id", self.id)?;
        }
        profile.set_string(s, "szTabViewString", &self.tab_view_string)?;
        profile.set_i32(s, "bTabArrow", self.tab_arrow)?;
        profile.set_bool(s, "bInsSpace", self.ins_space)?;
        profile.set_i32(s, "nTextWrapMethod", self.text_wrap_method)?;
        profile.set_string(s, "szIndentChars", &self.indent_chars)?;
        profile.set_wchar(s, "cLineTermChar", self.line_term_char)?;
        profile.set_bool(s, "bPriorCesu8", self.prior_cesu8)?;
        profile.set_i32(s, "eDefaultCodetype", self.default_codetype)?;
        profile.set_i32(s, "eDefaultEoltype", self.default_eoltype)?;
        profile.set_bool(s, "bDefaultBom", self.default_bom)?;
        profile.set_bool(s, "bAutoIndent", self.auto_indent)?;
        profile.set_bool(s, "bAutoIndent_ZENSPACE", self.auto_indent_zenspace)?;
        profile.set_bool(s, "bRTrimPrevLine", self.rtrim_prev_line)?;
        profile.set_i32(s, "nIndentLayout", self.indent_layout)?;
        profile.set_bool(s, "bKinsokuHead", self.kinsoku_head_enabled)?;
        profile.set_bool(s, "bKinsokuTail", self.kinsoku_tail_enabled)?;
        profile.set_bool(s, "bKinsokuRet", self.kinsoku_ret)?;
        profile.set_bool(s, "bKinsokuKuto", self.kinsoku_kuto_enabled)?;
        profile.set_bool(s, "bKinsokuHide", self.kinsoku_hide)?;
        profile.set_string(s, "szKinsokuHead", &self.kinsoku_head)?;
        profile.set_string(s, "szKinsokuTail", &self.kinsoku_tail)?;
        profile.set_string(s, "szKinsokuKuto", &self.kinsoku_kuto)?;
        profile.set_bool(s, "bChkEnterAtEnd", self.warn_mixed_eol)?;
        profile.set_bool(s, "bIndentCppStringIgnore", self.indent_cpp_string_ignore)?;
        profile.set_bool(s, "bIndentCppCommentIgnore", self.indent_cpp_comment_ignore)?;
        profile.set_bool(s, "bIndentCppUndoSep", self.indent_cpp_undo_sep)?;
        profile.set_bool(
            s,
            "bDetectIndentationStyleOnFileLoad",
            self.detect_indentation_on_load,
        )?;
        profile.set_string(s, "szHokanFile", &self.hokan_file)?;
        profile.set_i32(s, "nHokanType", self.hokan_type)?;
        profile.set_bool(s, "bHokanLoHiCase", self.hokan_lo_hi_case)?;
        profile.set_bool(s, "bUseHokanByFile", self.use_hokan_by_file)?;
        profile.set_bool(s, "bUseHokanByKeyword", self.use_hokan_by_keyword)?;
        profile.set_bool(s, "bUseKeyWordHelp", self.use_key_word_help)?;
        profile.set_bool(s, "bUseKeyHelpAllSearch", self.use_key_help_all_search)?;
        profile.set_bool(s, "bUseKeyHelpKeyDisp", self.use_key_help_key_disp)?;
        profile.set_bool(s, "bUseKeyHelpPrefix", self.use_key_help_prefix)?;
        profile.set_i32(s, "nKeyHelpRMenuShowType", self.key_help_rmenu_show_type)?;
        self.write_key_help(profile, s)?;
        profile.set_string(s, "szExtHelp", &self.ext_help)?;
        profile.set_string(s, "szExtHtmlHelp", &self.ext_html_help)?;
        profile.set_bool(s, "bTypeHtmlHelpIsSingle", self.html_help_is_single)?;
        profile.set_bool(s, "bUseTypeFont", self.use_type_font)?;
        self.write_rules(profile, s)?;
        // Sakura always writes the font of the type (an all-zero one when the type has none). A type that does
        // not use its own font does not need it, so it is not added to a file that has none.
        if self.use_type_font || profile.get(s, "lf").is_some() {
            self.font.write(profile, s, "lf", None, None)?;
        }
        self.colors.write(profile, s)
    }

    fn write_rules(&self, profile: &mut Profile, s: &str) -> Result<(), ProfileError> {
        profile.set_bool(s, "bStringLineOnly", self.string_line_only)?;
        profile.set_bool(s, "bStringEndLine", self.string_end_line)?;
        for (i, block) in self.block_comments.iter().enumerate() {
            let suffix = if i > 0 {
                (i + 1).to_string()
            } else {
                String::new()
            };
            profile.set_string(s, &format!("szBlockCommentFrom{suffix}"), &block.from)?;
            profile.set_string(s, &format!("szBlockCommentTo{suffix}"), &block.to)?;
        }
        for (i, line) in self.line_comments.iter().enumerate() {
            let suffix = if i > 0 {
                (i + 1).to_string()
            } else {
                String::new()
            };
            profile.set_string(s, &format!("szLineComment{suffix}"), &line.delimiter)?;
            profile.set_i32(
                s,
                &format!("nLineCommentColumn{suffix}"),
                line.column_value(),
            )?;
        }
        profile.set_string(s, "szOutlineRuleFilename", &self.outline_rule_filename)?;
        profile.set_i32(s, "nDefaultOutline", self.default_outline)?;
        profile.set_i32(s, "nSmartIndent", self.smart_indent)?;
        // Like Sakura, up to and including the first 0.
        for (i, &v) in self.vert_line_idx.iter().enumerate() {
            profile.set_i32(s, &format!("nVertLineIdx{}", i + 1), v)?;
            if v == 0 {
                break;
            }
        }
        profile.set_i32(s, "nNoteLineOffset", self.note_line_offset)?;
        profile.set_bool(s, "bUseRegexKeyword", self.use_regex_keyword)?;
        // Contiguous numbers: reading stops at the first missing one. An undefined keyword (no
        // pattern) is skipped, as Sakura does.
        let defined = self.regex_keywords.iter().filter(|k| !k.pattern.is_empty());
        let mut written = 0;
        for (n, k) in defined.take(MAX_REGEX_KEYWORDS).enumerate() {
            profile.set_string(
                s,
                &format!("RxKey[{n:03}]"),
                &format!("{},{}", k.color, k.pattern),
            )?;
            written = n + 1;
        }
        // Sakura writes the file anew: the keys of keywords that are gone must not stay. The first
        // of them becomes a blank `RxKey[NNN]=`, which ends the list when it is read: without it
        // the keywords a built-in type starts with would come back.
        for n in written..MAX_REGEX_KEYWORDS {
            let key = format!("RxKey[{n:03}]");
            if n == written && profile.get(s, &key).is_some() {
                profile.set_string(s, &key, "")?;
            } else {
                profile.remove(s, &key);
            }
        }
        Ok(())
    }

    /// The columns of the vertical lines (`nVertLineIdx`): the entries before the first 0.
    pub fn vert_lines(&self) -> impl Iterator<Item = u32> + '_ {
        self.vert_line_idx
            .iter()
            .take_while(|&&c| c != 0)
            .filter_map(|&c| u32::try_from(c).ok())
    }

    /// Numbers are colored (`C[NUM]` is shown).
    pub fn colors_numbers(&self) -> bool {
        self.colors.get(names::NUMBER).is_some_and(|c| c.disp)
    }
}

/// How many types Sakura has by default (`nTypesCount` of a file that it writes).
pub const SAKURA_TYPES_COUNT: i32 = 30;

/// How many types MATSU Editor has by default: Sakura's 17, MATSU Editor's own (ADR-0017, ADR-0023: 16 of
/// them), and 3 free numbers, as Sakura's 30 left 3 after its 17 and the 10 first types of MATSU Editor.
pub const DEFAULT_TYPES_COUNT: i32 = 36;

/// `nTypesCount` in `[Other]`: how many `Types(N)` sections there are; the value is limited to
/// 1..=`MAX_TYPES` (`ShareData_IO_Types`). A file without it (an old version) has [`DEFAULT_TYPES_COUNT`],
/// and so has a file that says 30, the number Sakura writes (a file that Sakura or an older MATSU Editor wrote
/// was not meant to leave out the types that came later): the numbers after 30 have no sections and are free.
/// Any other number is what the file chose.
pub fn types_count(profile: &Profile) -> i32 {
    let n = profile
        .get("Other", "nTypesCount")
        .and_then(parse_long)
        .unwrap_or(DEFAULT_TYPES_COUNT);
    let n = if n == SAKURA_TYPES_COUNT {
        DEFAULT_TYPES_COUNT
    } else {
        n
    };
    clamp_to(n, 1, MAX_TYPES)
}

/// `MAX_PATH`: the size of `szOutlineRuleFilename`, in UTF-16 units.
const MAX_PATH: usize = 260;

/// One `RxKey[NNN]` value, `color,pattern` (`swscanf` with `%[^,],%[^\n]`). The color is a name or a number
/// (the index in Sakura's table); anything else is the first regular expression color. A value that does not
/// have both parts is an empty keyword of that color.
fn parse_regex_keyword(value: &str) -> RegexKeyword {
    let Some((color, pattern)) = value.split_once(',') else {
        return RegexKeyword {
            color: DEFAULT_REGEX_COLOR.to_owned(),
            pattern: String::new(),
        };
    };
    let pattern = pattern.split('\n').next().unwrap_or_default();
    let names: Vec<_> = ColorSet::all_names().collect();
    let color = if names.contains(&color) {
        color.to_owned()
    } else {
        parse_long(color)
            .and_then(|i| usize::try_from(i).ok())
            .and_then(|i| names.get(i))
            .map_or_else(|| DEFAULT_REGEX_COLOR.to_owned(), |n| (*n).to_owned())
    };
    RegexKeyword {
        color,
        pattern: pattern.to_owned(),
    }
}

/// Reads a string into a buffer of `capacity` UTF-16 units including the end mark. Sakura refuses a value that
/// does not fit (`IOProfileData` for a `WritableBuffer`), and the setting keeps its value then.
fn read_string(profile: &Profile, section: &str, key: &str, target: &mut String, capacity: usize) {
    if let Some(v) = profile.get(section, key)
        && v.encode_utf16().count() < capacity
    {
        v.clone_into(target);
    }
}

fn read_i32(profile: &Profile, section: &str, key: &str, target: &mut i32) {
    if let Some(v) = profile.get_i32(section, key) {
        *target = v;
    }
}

fn read_bool(profile: &Profile, section: &str, key: &str, target: &mut bool) {
    if let Some(v) = profile.get_bool(section, key) {
        *target = v;
    }
}

/// Writes a value that reading limits to `min..=max`. A file value outside the range reads as the limit, so
/// it is left alone when the setting equals that limit.
fn write_clamped(
    profile: &mut Profile,
    section: &str,
    key: &str,
    value: i32,
    min: i32,
    max: i32,
) -> Result<(), ProfileError> {
    if profile.get_i32(section, key).map(|v| clamp_to(v, min, max)) == Some(value) {
        return Ok(());
    }
    profile.set_i32(section, key, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UTF-8 with a BOM (without one, the bytes are read as Shift_JIS).
    fn utf8(s: &str) -> Vec<u8> {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(s.as_bytes());
        bytes
    }

    #[test]
    fn defaults_follow_default_config() {
        let t = TypeSettings::default();
        assert_eq!(t.max_line_ketas, 10240);
        assert_eq!(t.tab_space, 4);
        assert_eq!(t.line_space, 1);
        assert_eq!(t.text_wrap_method, wrap_method::NONE);
        assert_eq!(t.default_codetype, code_type::UTF8);
        assert_eq!(t.default_eoltype, eol_type::CRLF);
        assert!(!t.default_bom);
        assert_eq!(t.line_term_char, ':');
        assert_eq!(t.kinsoku_kuto, "、。，．､｡,.");
        assert!(!t.kinsoku_head_enabled);
        let b = TypeSettings::basic();
        assert_eq!(b.type_name, "基本");
        assert!(!b.colors.get("SQT").unwrap().disp);
        assert!(!b.colors.get("WQT").unwrap().disp);
        assert!(b.colors.get("CMT").unwrap().disp);
    }

    #[test]
    fn missing_section_gives_the_defaults() {
        let p = Profile::new();
        assert_eq!(TypeSettings::read(&p, 0), TypeSettings::basic());
        assert_eq!(types_count(&p), 36);
    }

    #[test]
    fn reads_ints_and_limits_them() {
        // nInts: idx,maxLineKetas,columnSpace,tabSpace,kw1,kw2,stringType,lineNumIsCRLF,lineTermType,wordWrap,print,tsv
        let p = Profile::from_bytes(
            b"[Types(0)]\r\nnInts=0,120,2,8,-1,3,0,1,2,1,0,0\r\nnLineSpace=500\r\nnLineNumWidth=1\r\nnTextWrapMethod=1\r\n",
        );
        let t = TypeSettings::read(&p, 0);
        assert_eq!(t.max_line_ketas, 120);
        assert_eq!(t.column_space, 2);
        assert_eq!(t.tab_space, 8);
        assert_eq!(t.keyword_set_idx[1], 3);
        assert_eq!(t.line_term_type, 2);
        assert!(t.word_wrap);
        assert_eq!(t.line_space, 128);
        assert_eq!(t.line_num_width, 2);
        assert_eq!(t.text_wrap_method, wrap_method::SETTING_WIDTH);

        // The wrap column is limited to 10..=10240 and the tab width to 2..=wrap-2.
        let p = Profile::from_bytes(b"[Types(1)]\r\nnInts=1,3,0,99,-1,-1,0,1,1,0,0,0\r\n");
        let t = TypeSettings::read(&p, 1);
        assert_eq!(t.max_line_ketas, 10);
        assert_eq!(t.tab_space, 8);
    }

    #[test]
    fn strings_that_do_not_fit_sakuras_buffers_are_refused() {
        // `szTabViewString` holds 8 units, `szTypeName` 63.
        let long_name = "あ".repeat(64);
        let p = Profile::from_bytes(&utf8(&format!(
            "[Types(0)]\r\nszTabViewString=12345678\r\nszTypeName={long_name}\r\nszTypeExts={}\r\n",
            "x".repeat(63)
        )));
        let t = TypeSettings::read(&p, 0);
        assert_eq!(t.tab_view_string, "12345678");
        assert_eq!(t.type_name, "基本");
        assert_eq!(t.type_exts, "x".repeat(63));
        let p = Profile::from_bytes(&utf8("[Types(0)]\r\nszTabViewString=123456789\r\n"));
        assert_eq!(TypeSettings::read(&p, 0).tab_view_string, "^       ");
    }

    #[test]
    fn tells_which_new_file_values_the_section_sets() {
        let p = Profile::from_bytes(&utf8(
            "[Types(1)]\r\neDefaultEoltype=2\r\nbDefaultBom=0\r\n[Types(2)]\r\neDefaultCodetype=0\r\n",
        ));
        assert_eq!(
            TypeSettings::new_file_keys(&p, 1),
            NewFileKeys {
                codetype: None,
                eoltype: Some(eol_type::LF),
                bom: Some(false),
            }
        );
        assert_eq!(
            TypeSettings::new_file_keys(&p, 2),
            NewFileKeys {
                codetype: Some(code_type::SJIS),
                ..NewFileKeys::default()
            }
        );
        assert_eq!(TypeSettings::new_file_keys(&p, 3), NewFileKeys::default());
    }

    #[test]
    fn broken_ints_are_ignored() {
        // 11 numbers: refused as a whole (`12 != swscanf_s`).
        let p = Profile::from_bytes(b"[Types(0)]\r\nnInts=0,120,2,8,-1,3,0,1,2,1,0\r\n");
        assert_eq!(TypeSettings::read(&p, 0), TypeSettings::basic());
    }

    #[test]
    fn reads_encoding_and_kinsoku() {
        let p = Profile::from_bytes(&utf8(
            "[Types(2)]\r\neDefaultCodetype=0\r\neDefaultEoltype=2\r\nbDefaultBom=1\r\nbKinsokuHead=1\r\nszKinsokuHead=、。\r\nszKinsokuKuto=\r\nid=-7\r\ncLineTermChar=|\r\nbUseTypeFont=1\r\nlf=-16,0,0,0,400,0,0,0,128,3,2,1,49\r\nlfFaceName=BIZ UDゴシック\r\n",
        ));
        let t = TypeSettings::read(&p, 2);
        assert_eq!(t.default_codetype, code_type::SJIS);
        assert_eq!(t.default_eoltype, eol_type::LF);
        assert!(t.default_bom);
        assert!(t.kinsoku_head_enabled);
        assert_eq!(t.kinsoku_head, "、。");
        // An empty value is a value: the default punctuation list is replaced by nothing.
        assert_eq!(t.kinsoku_kuto, "");
        assert_eq!(t.id, 7);
        assert_eq!(t.line_term_char, '|');
        assert!(t.use_type_font);
        assert_eq!(t.font.log_font.face_name, "BIZ UDゴシック");
        assert_eq!(t.font.log_font.height, -16);
    }

    #[test]
    fn write_then_read_gives_the_same_settings() {
        let mut t = TypeSettings::basic();
        t.max_line_ketas = 80;
        t.tab_space = 2;
        t.text_wrap_method = wrap_method::SETTING_WIDTH;
        t.kinsoku_head_enabled = true;
        t.kinsoku_head = "、。」".to_owned();
        t.default_codetype = code_type::SJIS;
        t.default_bom = true;
        t.line_term_char = '\0';
        t.colors.get_mut(names::TAB).unwrap().disp = false;
        t.use_type_font = true;
        t.font.point_size = 120;
        let mut p = Profile::new();
        t.write(&mut p, 0).unwrap();
        assert_eq!(TypeSettings::read(&p, 0), t);
        assert_eq!(p.get("Types(0)", "cLineTermChar"), Some(""));
        assert_eq!(
            p.get("Types(0)", "nInts"),
            Some("0,80,0,2,-1,-1,0,1,1,0,0,0")
        );
    }

    #[test]
    fn writing_unchanged_settings_keeps_the_file_and_other_entries() {
        let src = "; user comment\r\n[Types(0)]\r\nnInts=0, 120,0,4,-1,-1,0,1,1,0,0,0\r\nbAutoIndent=1\r\nszUnknownKey=keep me\r\nnLineSpace=1000\r\n\r\n[Other]\r\nnTypesCount=2\r\n";
        let mut p = Profile::from_bytes(&utf8(src));
        let t = TypeSettings::read(&p, 0);
        t.write(&mut p, 0).unwrap();
        let out = String::from_utf8(p.to_bytes()).unwrap();
        let out = out.trim_start_matches('\u{feff}');
        // Everything that was in the file is still there, in place.
        assert!(out.starts_with(
            "; user comment\r\n[Types(0)]\r\nnInts=0, 120,0,4,-1,-1,0,1,1,0,0,0\r\nbAutoIndent=1\r\nszUnknownKey=keep me\r\nnLineSpace=1000\r\n"
        ));
        assert!(out.ends_with("\r\n\r\n[Other]\r\nnTypesCount=2\r\n"));
        assert_eq!(types_count(&p), 2);
    }

    #[test]
    fn types_count_is_limited() {
        let p = Profile::from_bytes(b"[Other]\r\nnTypesCount=100\r\n");
        assert_eq!(types_count(&p), 60);
        let p = Profile::from_bytes(b"[Other]\r\nnTypesCount=0\r\n");
        assert_eq!(types_count(&p), 1);
        // Sakura's 30 is read as the default (the types that came later are not left out); others are kept
        let p = Profile::from_bytes(b"[Other]\r\nnTypesCount=30\r\n");
        assert_eq!(types_count(&p), 36);
        let p = Profile::from_bytes(b"[Other]\r\nnTypesCount=31\r\n");
        assert_eq!(types_count(&p), 31);
    }

    #[test]
    fn the_coloring_rules_are_read_from_the_section() {
        let p = Profile::from_bytes(&utf8(
            "[Types(3)]\r\nszLineComment=%\r\nnLineCommentColumn=2\r\nszLineComment2=REM \r\n\
             szBlockCommentFrom=<!--\r\nszBlockCommentTo=-->\r\nszBlockCommentFrom2=only from\r\n\
             bStringLineOnly=1\r\nbStringEndLine=1\r\nnDefaultOutline=10\r\nnSmartIndent=1\r\n\
             szOutlineRuleFilename=rules.txt\r\nnVertLineIdx1=5\r\nnVertLineIdx2=0\r\nnVertLineIdx3=9\r\n\
             bUseRegexKeyword=1\r\nRxKey[000]=URL,/a+/k\r\nRxKey[001]=7,/b/\r\nRxKey[003]=RK2,/skipped/\r\n",
        ));
        let t = TypeSettings::read(&p, 3);
        // The column is read together with the delimiter: the second one has none, so it is not used.
        assert_eq!(t.line_comments[0], LineComment::new("%", 2));
        assert_eq!(t.line_comments[1], LineComment::default());
        // A block comment needs both parts.
        assert_eq!(t.block_comments[0], BlockComment::new("<!--", "-->"));
        assert_eq!(t.block_comments[1], BlockComment::default());
        assert!(t.string_line_only && t.string_end_line);
        assert_eq!(t.default_outline, outline_type::HTML);
        assert_eq!(t.smart_indent, smart_indent::CPP);
        assert_eq!(t.outline_rule_filename, "rules.txt");
        // The list ends at the 0, and the entries after it are not read.
        assert_eq!(t.vert_line_idx[..3], [5, 0, 0]);
        assert_eq!(t.vert_lines().collect::<Vec<_>>(), [5]);
        assert!(t.use_regex_keyword);
        // A number is the index in Sakura's color table; the first missing number ends the list.
        assert_eq!(t.regex_keywords.len(), 2);
        assert_eq!(t.regex_keywords[0].color, "URL");
        assert_eq!(t.regex_keywords[0].pattern, "/a+/k");
        assert_eq!(t.regex_keywords[1].color, "NOT");
    }

    #[test]
    fn a_regex_keyword_that_is_gone_is_removed_from_the_file() {
        let mut p = Profile::from_bytes(&utf8(
            "[Types(1)]\r\nRxKey[000]=URL,/a/k\r\nRxKey[001]=RK2,/b/k\r\nRxKey[002]=RK3,/c/k\r\n",
        ));
        let mut t = TypeSettings::read(&p, 1);
        assert_eq!(t.regex_keywords.len(), 3);
        t.regex_keywords.remove(1);
        t.write(&mut p, 1).unwrap();
        assert_eq!(p.get("Types(1)", "RxKey[001]"), Some("RK3,/c/k"));
        // The first key that is gone is blank: it ends the list, and nothing is after it.
        assert_eq!(p.get("Types(1)", "RxKey[002]"), Some(""));
        assert_eq!(p.get("Types(1)", "RxKey[003]"), None);
        assert_eq!(TypeSettings::read(&p, 1).regex_keywords, t.regex_keywords);
        t.regex_keywords.clear();
        t.write(&mut p, 1).unwrap();
        assert_eq!(p.get("Types(1)", "RxKey[000]"), Some(""));
        assert_eq!(p.get("Types(1)", "RxKey[001]"), None);
        assert!(TypeSettings::read(&p, 1).regex_keywords.is_empty());
        // A list that was never in the file does not get a blank key.
        let mut fresh = Profile::new();
        TypeSettings::basic().write(&mut fresh, 1).unwrap();
        assert_eq!(fresh.get("Types(1)", "RxKey[000]"), None);
    }

    #[test]
    fn the_input_completion_settings_are_read_written_and_default_like_sakura() {
        // `_DefaultConfig`: the words of the document, no word file, no keywords, case matters.
        let t = TypeSettings::basic();
        assert_eq!(t.hokan_file, "");
        assert_eq!(t.hokan_type, 0);
        assert!(!t.hokan_lo_hi_case && t.use_hokan_by_file && !t.use_hokan_by_keyword);
        let p = Profile::from_bytes(&utf8(
            "[Types(2)]\r\nszHokanFile=words.kwd\r\nnHokanType=3\r\nbHokanLoHiCase=1\r\n\
             bUseHokanByFile=0\r\nbUseHokanByKeyword=1\r\n",
        ));
        let t = TypeSettings::read(&p, 2);
        assert_eq!(t.hokan_file, "words.kwd");
        assert_eq!(t.hokan_type, 3);
        assert!(t.hokan_lo_hi_case && !t.use_hokan_by_file && t.use_hokan_by_keyword);
        let mut out = Profile::new();
        t.write(&mut out, 2).unwrap();
        assert_eq!(TypeSettings::read(&out, 2), t);
        assert_eq!(out.get("Types(2)", "szHokanFile"), Some("words.kwd"));
        assert_eq!(out.get("Types(2)", "bUseHokanByFile"), Some("0"));
    }

    #[test]
    fn the_keyword_help_options_and_the_help_files_are_read_written_and_default_like_sakura() {
        // `_DefaultConfig`: the keyword help is off and the menu shows it at the top; no external
        // help files, and one viewer for the HTML help.
        let t = TypeSettings::basic();
        assert!(!t.use_key_word_help && !t.use_key_help_all_search);
        assert!(!t.use_key_help_key_disp && !t.use_key_help_prefix);
        assert_eq!(t.key_help_rmenu_show_type, 1);
        assert_eq!((t.ext_help.as_str(), t.ext_html_help.as_str()), ("", ""));
        assert!(t.html_help_is_single);
        let p = Profile::from_bytes(&utf8(
            "[Types(2)]\r\nbUseKeyWordHelp=1\r\nbUseKeyHelpAllSearch=1\r\nbUseKeyHelpKeyDisp=1\r\n\
             bUseKeyHelpPrefix=1\r\nnKeyHelpRMenuShowType=2\r\nszExtHelp=a.hlp\r\n\
             szExtHtmlHelp=b.chm\r\nbTypeHtmlHelpIsSingle=0\r\n",
        ));
        let t = TypeSettings::read(&p, 2);
        assert!(t.use_key_word_help && t.use_key_help_all_search);
        assert!(t.use_key_help_key_disp && t.use_key_help_prefix);
        assert_eq!(t.key_help_rmenu_show_type, 2);
        assert_eq!(
            (t.ext_help.as_str(), t.ext_html_help.as_str()),
            ("a.hlp", "b.chm")
        );
        assert!(!t.html_help_is_single);
        let mut out = Profile::new();
        t.write(&mut out, 2).unwrap();
        assert_eq!(TypeSettings::read(&out, 2), t);
        assert_eq!(out.get("Types(2)", "bUseKeyHelpPrefix"), Some("1"));
        assert_eq!(out.get("Types(2)", "nKeyHelpRMenuShowType"), Some("2"));
        assert_eq!(out.get("Types(2)", "bTypeHtmlHelpIsSingle"), Some("0"));
        assert_eq!(out.get("Types(2)", "szExtHtmlHelp"), Some("b.chm"));
    }

    #[test]
    fn the_keyword_help_dictionaries_are_read_and_written_like_sakura() {
        // `KDct[NN]=use,about,path`; an empty description is allowed, a path may have commas;
        // the first number that is missing ends the list.
        let p = Profile::from_bytes(&utf8(
            "[Types(2)]\r\nKDct[00]=1,Memo,dict one.txt\r\nKDct[01]=0,,a,b.txt\r\n\
             KDct[03]=1,Lost,c.txt\r\n",
        ));
        let t = TypeSettings::read(&p, 2);
        assert_eq!(
            t.key_help,
            [
                KeyHelpDict {
                    used: true,
                    about: "Memo".to_owned(),
                    path: "dict one.txt".to_owned()
                },
                KeyHelpDict {
                    used: false,
                    about: String::new(),
                    path: "a,b.txt".to_owned()
                },
            ]
        );
        // A line that does not have the form, or has no path, ends the list too.
        let p = Profile::from_bytes(&utf8(
            "[Types(2)]\r\nKDct[00]=1,x,a.txt\r\nKDct[01]=bad\r\nKDct[02]=1,x,b.txt\r\n",
        ));
        assert_eq!(TypeSettings::read(&p, 2).key_help.len(), 1);
        // The old key is one dictionary that is not used, and wins over KDct.
        let p = Profile::from_bytes(&utf8(
            "[Types(2)]\r\nszKeyWordHelpFile=old.txt\r\nKDct[00]=1,x,new.txt\r\n",
        ));
        let old = TypeSettings::read(&p, 2);
        assert_eq!(old.key_help.len(), 1);
        assert_eq!(old.key_help[0].path, "old.txt");
        assert!(!old.key_help[0].used);
        // Writing keeps the list, drops the keys of dictionaries that are gone and the old key.
        let mut out = p;
        let mut t = old;
        t.key_help.push(KeyHelpDict {
            used: true,
            about: "B".to_owned(),
            path: "b.txt".to_owned(),
        });
        t.write(&mut out, 2).unwrap();
        assert_eq!(out.get("Types(2)", "szKeyWordHelpFile"), None);
        assert_eq!(out.get("Types(2)", "KDct[01]"), Some("1,B,b.txt"));
        assert_eq!(TypeSettings::read(&out, 2).key_help, t.key_help);
        t.key_help.truncate(1);
        t.write(&mut out, 2).unwrap();
        assert_eq!(out.get("Types(2)", "KDct[01]"), None);
    }

    #[test]
    fn the_coloring_rules_survive_a_write_and_a_read() {
        let mut t = TypeSettings::basic();
        t.line_comments[0] = LineComment::new("//", -1);
        t.line_comments[2] = LineComment::new("*", 6);
        t.block_comments[1] = BlockComment::new("#if 0", "#endif");
        t.string_type = string_type::PYTHON;
        t.string_line_only = true;
        t.default_outline = outline_type::PYTHON;
        t.vert_line_idx[..3].copy_from_slice(&[7, 8, 0]);
        t.note_line_offset = -3;
        t.use_regex_keyword = true;
        t.regex_keywords = vec![RegexKeyword {
            color: "RK3".to_owned(),
            pattern: "/x,y/k".to_owned(),
        }];
        let mut p = Profile::new();
        t.write(&mut p, 5).unwrap();
        let back = TypeSettings::read(&p, 5);
        assert_eq!(back.line_comments, t.line_comments);
        assert_eq!(back.block_comments, t.block_comments);
        assert_eq!(back.string_type, string_type::PYTHON);
        assert!(back.string_line_only);
        assert_eq!(back.default_outline, outline_type::PYTHON);
        assert_eq!(back.note_line_offset, -3);
        assert_eq!(back.vert_line_idx[..3], [7, 8, 0]);
        // The pattern may have a comma: only the first one splits the color from the pattern.
        assert_eq!(back.regex_keywords, t.regex_keywords);
    }
}
