// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for UME Editor.
//
// Original files: sakura_core/types/CType_Cpp.cpp
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2014, the authors named in the original files (genta, zenryaku, Moca,
//     aroka, hor, YAZAKI, little YOSHI, ryoji, syat and others)
//   Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (a UTF-16 buffer
// walked with `CDocLineMgr`, the results put in `CFuncInfoArr`, a Windows dialog) was rewritten in
// Rust on the lines of `LineSource`, the positions in the layout are left to the window, and the
// tests are new.
//
// zlib License
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

//! The outline of a C or C++ program: its namespaces, classes, structures, enumerations, unions,
//! functions and the declarations of functions. The names are `Namespace::Class::member`; the
//! tree splits them at `::`.
//!
//! Sakura: `CDocOutline::MakeFuncList_C` and the preprocessor state `CCppPreprocessMng` in
//! `sakura_core/types/CType_Cpp.cpp`.
//!
//! The analyzer reads the program as a state machine, one character at a time: `mode` says
//! whether it is in a comment, a string or a word, `mode2` says what the word is part of (the
//! name of a class being read, a template, an `operator`, the name of a function...).

use crate::chars::{at, is_space};
use crate::{FuncInfoArr, LineSource, func_obj};

/// `STR_OUTLINE_CPP_NONAME`.
const NO_NAME: &str = "無名";
/// `STR_OUTLINE_CPP_DEFPOS`.
const DEFPOS: &str = "::定義位置";

/// `nMaxWordLeng`: the longest word.
const MAX_WORD_LENGTH: isize = 100;
/// `nNamespaceNestMax`.
const NAMESPACE_NEST_MAX: usize = 32;
/// `nNamespaceLenMax`.
const NAMESPACE_LEN_MAX: usize = 512;
/// `nItemNameLenMax`.
const ITEM_NAME_LEN_MAX: usize = 256;

// `MODE2`: what the words are part of.
const M2_NORMAL: u8 = 0x00;
const M2_ATTRIBUTE: u8 = 0x02;
const M2_TEMPLATE: u8 = 0x03;
const M2_NAMESPACE_SAVE: u8 = 0x11;
const M2_TEMPLATE_SAVE: u8 = 0x12;
const M2_NAMESPACE_END: u8 = 0x13;
const M2_OPERATOR_WORD: u8 = 0x14;
const M2_TEMPLATE_WORD: u8 = 0x15;
const M2_FUNC_NAME_END: u8 = 0x16;
const M2_AFTER_EQUAL: u8 = 0x05;
const M2_KR_FUNC: u8 = 0x18;
const M2_AFTER_ITEM: u8 = 0x10;

// `nMode`.
const MODE_NORMAL: u16 = 0;
const MODE_WORD: u16 = 1;
const MODE_SYMBOLS: u16 = 2;
const MODE_COMMENT: u16 = 8;
const MODE_LINE_COMMENT: u16 = 10;
const MODE_SINGLE_QUOTE: u16 = 20;
const MODE_DOUBLE_QUOTE: u16 = 21;
const MODE_TOO_LONG: u16 = 999;

/// The kinds of C-family outline (`EOutlineType` of C, C++ and C/C++).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CKind {
    /// C: the list of functions.
    C,
    /// C/C++ whose file name says neither: the tree of C++.
    CCpp,
    /// C++: the tree.
    Cpp,
}

/// `GetCLangOutlineType`: the kind that the file name decides (the extension, any case). No match
/// is [`CKind::CCpp`].
pub fn kind_of_file_name(file_name: Option<&str>) -> CKind {
    let ext = file_name
        .and_then(|p| {
            let name = p.rsplit(['/', '\\']).next().unwrap_or(p);
            name.rfind('.').map(|dot| &name[dot + 1..])
        })
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "c" => CKind::C,
        "cpp" | "c++" | "cxx" | "hpp" | "h++" | "hxx" => CKind::Cpp,
        _ => CKind::CCpp,
    }
}

/// `C_IsWordChar`: a character a function name is made of.
fn is_word_char(c: char) -> bool {
    c == '_'
        || c == ':'
        || c == '~'
        || c.is_ascii_alphanumeric()
        || ((c as u32) >= 0xa1 && !c.is_control() && !c.is_whitespace())
}

/// `C_IsOperator`: the name is `operator` itself, or ends with `:operator`.
fn is_operator(s: &[char]) -> bool {
    let n = s.len();
    if n >= 8 && s[n - 1] == 'r' {
        if n > 8 {
            return s[n - 9..] == [':', 'o', 'p', 'e', 'r', 'a', 't', 'o', 'r'];
        }
        return eqs(s, "operator");
    }
    false
}

/// `C_IsLineEsc`: the line ends with a backslash before the line ending.
fn is_line_esc(s: &[char]) -> bool {
    let mut len = s.len();
    if len > 0 && crate::chars::is_line_delimiter(s[len - 1]) {
        len -= 1;
    }
    if len > 0 && s[len - 1] == '\r' {
        len -= 1;
    }
    len > 0 && s[len - 1] == '\\'
}

/// `CPP_IsFunctionAfterKeyword`.
fn is_function_after_keyword(s: &[char]) -> bool {
    [
        "const", "volatile", "throw", "default", "delete", "override", "final", "noexcept",
    ]
    .iter()
    .any(|k| eqs(s, k))
}

fn eqs(s: &[char], lit: &str) -> bool {
    s.len() == lit.chars().count() && s.iter().copied().eq(lit.chars())
}

fn to_chars(s: &str) -> Vec<char> {
    s.chars().collect()
}

/// `wcsncat(dst, src, ITEM_NAME_LEN_MAX - dst.len() - 1)`.
fn append_limited(dst: &mut Vec<char>, src: &[char]) {
    let room = ITEM_NAME_LEN_MAX.saturating_sub(dst.len() + 1);
    dst.extend(src.iter().take(room));
}

/// `final` after a name is a specifier, not part of the name (`is_final_context`).
fn is_final_context(word: &[char], item_name: &[char]) -> bool {
    eqs(word, "final") && !eqs(item_name, NO_NAME)
}

/// `CCppPreprocessMng`: which lines are skipped because of `#if 0` and the like.
///
/// The nesting is followed to 32 levels with a bit per level.
#[derive(Default)]
struct Preprocess {
    /// Inside a directive that goes over several lines.
    multi_line: bool,
    stack_ptr: i32,
    /// The bit of the level that is being read.
    bit_pattern: u32,
    /// Bits of the levels whose code is skipped.
    enable_buf: u32,
    /// Bits of the levels that already had a branch that is read.
    activated: u32,
}

const MAX_NEST_LEVEL: i32 = 32;

fn bit(level: i32) -> u32 {
    u32::try_from(level)
        .ok()
        .and_then(|l| 1u32.checked_shl(l))
        .unwrap_or(0)
}

impl Preprocess {
    /// `ScanLine`: where the C/C++ analysis of the line starts. The length of the line means the
    /// line is skipped.
    fn scan_line(&mut self, s: &[char]) -> usize {
        let length = s.len();
        let mut p = 0usize;
        // skip whitespace
        while p < length && is_space(at(s, p)) {
            p += 1;
        }
        if length <= p {
            // a blank line
            return length;
        }
        if self.multi_line {
            // the rest of a directive is ignored
            self.multi_line = is_line_esc(s);
            return length;
        }
        if s[p] != '#' {
            // not a directive: the main part reads it unless it is skipped
            if self.enable_buf != 0 {
                return length;
            }
            return p;
        }
        p += 1; // skip the "#"
        while p < length && is_space(at(s, p)) {
            p += 1;
        }

        #[derive(PartialEq)]
        enum Directive {
            None,
            If,
            Ifdef,
            Ifndef,
            Else,
            Elif,
            Endif,
        }
        let mut directive = Directive::None;
        if p + 5 < length && crate::chars::starts_at(s, p, "ifdef") {
            directive = Directive::Ifdef;
            p += 5;
        } else if p + 6 < length && crate::chars::starts_at(s, p, "ifndef") {
            directive = Directive::Ifndef;
            p += 6;
        } else if p + 2 < length && crate::chars::starts_at(s, p, "if") {
            directive = Directive::If;
            p += 2;
        } else if p + 4 < length && crate::chars::starts_at(s, p, "else") {
            directive = Directive::Else;
            p += 4;
        } else if p + 4 < length && crate::chars::starts_at(s, p, "elif") {
            directive = Directive::Elif;
            p += 4;
        } else if p + 5 < length && crate::chars::starts_at(s, p, "endif") {
            directive = Directive::Endif;
            p += 5;
        }

        match directive {
            Directive::If | Directive::Ifdef | Directive::Ifndef | Directive::Elif => {
                // "#if 0" starts with a part that is skipped, the other #if, #ifdef and #ifndef
                // start with a part that is read (2007.12.13 じゅうじ: "#if(0)" too)
                if is_space(at(s, p)) || at(s, p) == '(' {
                    while p < length && (is_space(at(s, p)) || at(s, p) == '(') {
                        p += 1;
                    }
                    let zero = at(s, p) == '0';
                    if matches!(
                        directive,
                        Directive::If | Directive::Ifdef | Directive::Ifndef
                    ) {
                        self.bit_pattern = bit(self.stack_ptr);
                        self.stack_ptr += 1;
                        if zero {
                            self.enable_buf |= self.bit_pattern;
                        } else if self.stack_ptr == 1
                            || self.activated & bit(self.stack_ptr - 2) != 0
                        {
                            self.activated |= self.bit_pattern;
                        }
                    } else if 0 < self.stack_ptr && self.stack_ptr < MAX_NEST_LEVEL {
                        if zero || self.activated & self.bit_pattern != 0 {
                            self.enable_buf |= self.bit_pattern;
                        } else {
                            self.enable_buf &= !self.bit_pattern;
                            self.activated |= self.bit_pattern;
                        }
                    }
                }
            }
            Directive::Else => {
                // 2007.12.14 genta: guard for an #else without #if
                if 0 < self.stack_ptr && self.stack_ptr < MAX_NEST_LEVEL {
                    if self.activated & self.bit_pattern != 0 {
                        self.enable_buf |= self.bit_pattern;
                    } else {
                        self.enable_buf &= !self.bit_pattern;
                        if self.stack_ptr == 1 || self.activated & bit(self.stack_ptr - 2) != 0 {
                            self.activated |= self.bit_pattern;
                        }
                    }
                }
            }
            Directive::Endif => {
                if self.stack_ptr > 0 {
                    self.stack_ptr -= 1;
                    self.enable_buf &= !self.bit_pattern;
                    self.activated &= !self.bit_pattern;
                    self.bit_pattern = bit(self.stack_ptr - 1);
                }
            }
            Directive::None => {
                // does the line end with a backslash?
                self.multi_line = is_line_esc(s);
            }
        }
        // the directives themselves are ignored
        length
    }
}

/// Lists the namespaces, classes and functions of a C or C++ program. `kind` is the kind of the
/// document type (`C`, `C_CPP` or `CPP`); the file name settles C/C++ (see [`kind_of_file_name`]).
/// The kind it settled on comes back with the list: C is shown as a list, the others as a tree.
/// With `visible_member_functions` (`bVisibleMemberFunc`, true in Sakura) the declarations of
/// members are listed too.
pub fn make_func_list_c(
    src: &dyn LineSource,
    kind: CKind,
    file_name: Option<&str>,
) -> (FuncInfoArr, CKind) {
    let kind = if kind == CKind::CCpp {
        kind_of_file_name(file_name)
    } else {
        kind
    };
    let mut out = FuncInfoArr::new();
    let visible_member_funcs = true;

    // the nesting
    let mut nest_global: usize = 0; // { } outside functions
    let mut nest_func: usize = 0; // { } of a function and inside it
    let mut nest_fparam: usize = 0; // ( )
    let mut nest_point_class: usize = 0; // which { belongs to a class (never set while members are visible)
    let mut in_init_list = false; // reached the initializer list (after ':')
    let mut nest_template: i32 = 0; // < > of templates

    let mut word_prev: Vec<char> = Vec::new();
    let mut word: Vec<char> = Vec::new();
    let mut word_idx: isize = 0;
    let mut mode: u16 = MODE_NORMAL;
    let mut mode2: u8 = M2_NORMAL;
    let mut mode2_old: u8 = M2_NORMAL; // before M2_TEMPLATE_WORD
    let mut mode2_att_old: u8 = M2_NORMAL;
    let mut defined_typedef = false;
    let mut no_function = true;

    // The namespace: `ns` holds the names of the levels (`szNamespace`) and `ns_len` the length
    // of the part for each level.
    let mut ns_len = [0usize; NAMESPACE_NEST_MAX + 1];
    let mut ns: Vec<char> = Vec::new();
    let mut item_name: Vec<char> = Vec::new();
    let mut template_name: Vec<char> = Vec::new();
    // a C++11 raw string
    let mut raw_tag: Vec<char> = Vec::new();
    let mut raw_tag_len = 0usize;
    let mut raw_tag_comp_len = 0usize;

    let mut item_line: isize = 0;
    let mut item_func_id: u32 = 0;

    let mut pp = Preprocess::default();

    // `szNamespace` up to `ns_len[nest_global]`, then `name`.
    fn qualified(ns: &[char], len: usize, name: &[char]) -> String {
        ns.iter().take(len).chain(name.iter()).collect::<String>()
    }

    for n in 0..src.line_count() {
        let Some(line) = src.line(n) else {
            break;
        };
        let len = line.len();
        let mut i: usize;
        // The directives of the preprocessor are read first, unless in a comment (2004.08.10 genta)
        if mode != MODE_COMMENT && mode != MODE_LINE_COMMENT {
            i = pp.scan_line(&line);
        } else {
            i = 0;
        }
        while i < len {
            let c = line[i];
            // comment
            if mode == MODE_COMMENT {
                if i + 1 < len && c == '*' && line[i + 1] == '/' {
                    i += 1;
                    mode = MODE_NORMAL;
                }
                i += 1;
                continue;
            }
            // line comment (2003/06/24 zenryaku)
            if mode == MODE_LINE_COMMENT {
                if !is_line_esc(&line) {
                    mode = MODE_NORMAL;
                }
                i = len;
                continue;
            }
            // an escape sequence is always removed (2005/12/6 じゅうじ)
            if c == '\\' && raw_tag_len == 0 {
                i += 2;
                continue;
            }
            // character constant
            if mode == MODE_SINGLE_QUOTE {
                if c == '\'' {
                    mode = MODE_NORMAL;
                }
                i += 1;
                continue;
            }
            // string
            if mode == MODE_DOUBLE_QUOTE {
                // operator "" _userliteral
                if mode2 == M2_OPERATOR_WORD && word_prev.len() + 1 < 256 {
                    word_prev.push(c);
                }
                if c == '"' {
                    if raw_tag_len != 0 {
                        // R"abc(test
                        // )abc"
                        if raw_tag_len <= i
                            && line[i - raw_tag_len..i]
                                .iter()
                                .copied()
                                .take(raw_tag_comp_len)
                                .eq(raw_tag.iter().copied().take(raw_tag_comp_len))
                        {
                            raw_tag_len = 0;
                            mode = MODE_NORMAL;
                        }
                    } else {
                        mode = MODE_NORMAL;
                    }
                }
                i += 1;
                continue;
            }
            // reading a word
            if mode == MODE_WORD {
                if is_word_char(c) {
                    word_idx += 1;
                    if word_idx >= MAX_WORD_LENGTH {
                        mode = MODE_TOO_LONG;
                        i += 1;
                        continue;
                    }
                    if c == ':' {
                        if at(&line, i + 1) == ':' || (0 < i && line[i - 1] == ':') {
                            // name ::class or class :: member
                        } else {
                            // class Klass:base: no space before the ':'
                            if mode2 == M2_NAMESPACE_SAVE {
                                // 2021.3.27: "class Klass final: public base{": the ':' does not
                                // overwrite the name
                                if !word.is_empty() && !is_final_context(&word, &item_name) {
                                    item_name = word.clone();
                                }
                                mode2 = M2_NAMESPACE_END;
                            } else if mode2 == M2_TEMPLATE_SAVE {
                                append_limited(&mut template_name, &word);
                                mode2 = M2_NAMESPACE_END;
                            }
                        }
                    }
                    word.truncate(word_idx as usize);
                    word.push(c);
                } else {
                    // the end of the word
                    if mode2 == M2_NAMESPACE_SAVE {
                        if is_final_context(&word, &item_name) {
                            // "struct name final": the final is not part of the name (but
                            // "struct final" is the name)
                        } else {
                            template_name = word.clone();
                            item_name = word.clone();
                        }
                    } else if mode2 == M2_TEMPLATE_SAVE || mode2 == M2_TEMPLATE_WORD {
                        // "struct name<X> final": the final is not part of the name
                        // ("struct name<final>": it is)
                        if !eqs(&word, "final") || nest_template != 0 {
                            let nlen = template_name.len();
                            if nlen > 0
                                && is_word_char(template_name[nlen - 1])
                                && template_name[nlen - 1] != ':'
                                && word.last() != Some(&':')
                                && nlen + 1 < ITEM_NAME_LEN_MAX
                            {
                                // "template func<const x>()": a space after "const"
                                template_name.push(' ');
                            }
                            append_limited(&mut template_name, &word);
                        }
                    } else if nest_func == 0 && (mode2 == M2_NORMAL || mode2 == M2_FUNC_NAME_END) {
                        // 2010.07.08 ryoji: a call of a macro that looks like a function is
                        // taken for a function; mode2 == M2_FUNC_NAME_END is corrected here
                        if mode2 == M2_NORMAL {
                            item_func_id = 0;
                        }
                        if eqs(&word, "class") {
                            item_func_id = func_obj::CLASS;
                        } else if eqs(&word, "struct") {
                            item_func_id = func_obj::STRUCT;
                        } else if eqs(&word, "namespace") {
                            item_func_id = func_obj::NAMESPACE;
                        } else if eqs(&word, "enum") {
                            item_func_id = func_obj::ENUM;
                        } else if eqs(&word, "union") {
                            item_func_id = func_obj::UNION;
                        } else if eqs(&word, "__interface") {
                            // 2007.05.26 genta: like a class
                            item_func_id = func_obj::INTERFACE;
                        } else if eqs(&word, "typedef") {
                            defined_typedef = true;
                        }
                        if item_func_id != 0 && item_func_id != func_obj::FUNCTION {
                            mode2 = M2_NAMESPACE_SAVE;
                            item_line = n as isize + 1;
                            item_name = to_chars(NO_NAME);
                        }
                    }
                    if mode2 == M2_FUNC_NAME_END {
                        mode2 = M2_KR_FUNC;
                    }
                    if mode2 == M2_NORMAL {
                        // the template is over
                        template_name.clear();
                    }
                    // 2004/03/12 zenryaku: PARAMS and _ are not keywords (GNU code)
                    if !(eqs(&word, "PARAMS") || eqs(&word, "_")) {
                        word_prev = word.clone();
                    }
                    word_idx = 0;
                    word.clear();
                    mode = MODE_NORMAL;
                    // read this character again
                    continue;
                }
                i += 1;
                continue;
            }
            // reading symbols
            if mode == MODE_SYMBOLS {
                if is_word_char(c)
                    || is_space(c)
                    || matches!(c, '{' | '}' | '(' | ')' | ';' | '\'' | '"')
                    || (c == '/' && (at(&line, i + 1) == '*' || at(&line, i + 1) == '/'))
                {
                    word_prev = word.clone();
                    word_idx = 0;
                    word.clear();
                    mode = MODE_NORMAL;
                    continue;
                }
                word_idx += 1;
                if word_idx >= MAX_WORD_LENGTH {
                    mode = MODE_TOO_LONG;
                    i += 1;
                    continue;
                }
                word.truncate(word_idx as usize);
                word.push(c);
                if (mode2 == M2_TEMPLATE_SAVE || mode2 == M2_TEMPLATE_WORD)
                    && template_name.len() + 1 < ITEM_NAME_LEN_MAX
                {
                    template_name.push(c);
                }
                if mode2 == M2_OPERATOR_WORD && c == '<' {
                    let w = word.len();
                    let tail = |s: &str| {
                        let s = to_chars(s);
                        w >= s.len() && word[w - s.len()..] == s[..]
                    };
                    if (w >= 9 && tail("operator<"))
                        || (w >= 10 && tail("operator<<") && 0 < i && line[i - 1] == '<')
                    {
                        // not a template: operator<<const() / operator<<()
                    } else {
                        // operator< <T>() / operator<<<T>() / operator+<T>()
                        mode2_old = mode2;
                        mode2 = M2_TEMPLATE_WORD;
                        template_name = word.iter().copied().take(ITEM_NAME_LEN_MAX - 1).collect();
                    }
                }
                if mode2 == M2_TEMPLATE || mode2 == M2_TEMPLATE_SAVE || mode2 == M2_TEMPLATE_WORD {
                    if c == '<' {
                        nest_template += 1;
                    } else if c == '>' {
                        item_name = template_name.clone();
                        nest_template -= 1;
                        if nest_template == 0 {
                            if mode2 == M2_TEMPLATE {
                                mode2 = M2_NORMAL;
                            } else if mode2 == M2_TEMPLATE_WORD {
                                mode2 = mode2_old;
                                if mode2 == M2_OPERATOR_WORD {
                                    word = template_name.clone();
                                    word_idx = word.len() as isize - 1;
                                    template_name.clear();
                                }
                            }
                        }
                    }
                }
                i += 1;
                continue;
            }
            // a word that is too long is skipped up to a blank
            if mode == MODE_TOO_LONG {
                if is_space(c) {
                    mode = MODE_NORMAL;
                }
                i += 1;
                continue;
            }

            // normal mode
            if is_space(c) {
                i += 1;
                continue;
            }
            if i + 1 < len && c == '/' && line[i + 1] == '/' {
                i += 2;
                mode = MODE_LINE_COMMENT;
                continue;
            }
            if i + 1 < len && c == '/' && line[i + 1] == '*' {
                i += 2;
                mode = MODE_COMMENT;
                continue;
            }
            if c == '\'' {
                mode = MODE_SINGLE_QUOTE;
                i += 1;
                continue;
            }
            if c == '"' {
                if mode2 == M2_NORMAL && is_operator(&word_prev) {
                    // an operator: operator ""i
                    if word_prev.len() + 1 < 256 {
                        word_prev.push(c);
                    }
                    mode2 = M2_OPERATOR_WORD;
                } else if ["R", "LR", "uR", "UR", "u8R"]
                    .iter()
                    .any(|s| eqs(&word_prev, s))
                {
                    // C++11 raw string: R"abc(string)abc"
                    for k in i + 1..len {
                        if line[k] == '(' {
                            let tag_len = (k - i - 1).min(31);
                            raw_tag_len = tag_len + 1;
                            raw_tag = std::iter::once(')')
                                .chain(line[i + 1..i + 1 + tag_len].iter().copied())
                                .collect();
                            raw_tag_comp_len = raw_tag.len();
                            break;
                        }
                    }
                }
                mode = MODE_DOUBLE_QUOTE;
                i += 1;
                continue;
            }

            // 2002/10/27 frozen
            if c == '{' {
                let add_function = mode2 == M2_FUNC_NAME_END || mode2 == M2_KR_FUNC;
                let def_pos_len = DEFPOS.chars().count();
                let item_len = item_name.len();
                if nest_func != 0 || eqs(&word_prev, "=") || mode2 == M2_AFTER_EQUAL {
                    nest_func += 1;
                } else if (mode2 & M2_AFTER_ITEM) != 0
                    && nest_global < NAMESPACE_NEST_MAX
                    && ns_len[nest_global] + item_len + def_pos_len + 1 < NAMESPACE_LEN_MAX
                    && item_line > 0
                {
                    // (when the buffer is too short nothing is added)
                    let prefix_len = ns_len[nest_global];
                    ns.truncate(prefix_len);
                    ns.extend(item_name.iter());
                    let base = qualified(&ns, prefix_len, &item_name);
                    // Jan. 30, 2005 genta: M2_KR_FUNC
                    // after a function with const, throw or an initializer comes M2_KR_FUNC,
                    // and the "{" is met in that state if there is no ";"
                    let mut name = base.clone();
                    let mut new_prefix: Option<String> = None;
                    if add_function {
                        nest_func += 1;
                    } else {
                        nest_global += 1;
                        ns_len[nest_global] = ns_len[nest_global - 1] + item_len;
                        if item_func_id == func_obj::NAMESPACE {
                            name.push_str(DEFPOS);
                        }
                        if !visible_member_funcs && nest_point_class == 0 {
                            nest_point_class = nest_global;
                        }
                        new_prefix = Some(format!("{base}::"));
                    }
                    item_name.clear();
                    out.push(item_line as usize, name, item_func_id, 0);
                    defined_typedef = false;
                    item_line = -1;
                    if let Some(prefix) = new_prefix {
                        ns = to_chars(&prefix);
                        ns_len[nest_global] += 2;
                    }
                } else if add_function {
                    nest_func += 1;
                } else {
                    nest_global += 1;
                    if nest_global <= NAMESPACE_NEST_MAX {
                        ns_len[nest_global] = ns_len[nest_global - 1];
                    }
                }
                nest_template = 0;
                mode = MODE_NORMAL;
                mode2 = M2_NORMAL;
                mode2_old = M2_NORMAL;
                i += 1;
                continue;
            }
            if c == '}' {
                if nest_func == 0 {
                    if nest_global != 0 {
                        if nest_global == nest_point_class {
                            nest_point_class = 0;
                        }
                        nest_global -= 1;
                    }
                } else {
                    nest_func -= 1;
                }
                mode = MODE_NORMAL;
                mode2 = M2_NORMAL;
                mode2_old = M2_NORMAL;
                i += 1;
                continue;
            }
            if c == '(' {
                let mut is_operator_call = false;
                if mode2 == M2_NORMAL && nest_fparam == 0 && is_operator(&word_prev) {
                    let mut k = i + 1;
                    while k < len && is_space(line[k]) {
                        k += 1;
                    }
                    if k < len && line[k] == ')' {
                        k += 1;
                        while k < len && is_space(line[k]) {
                            k += 1;
                        }
                        if k < len && (line[k] == '<' || line[k] == '(') {
                            // an operator: operator()( / operator()<
                            if word_prev.len() + 1 < 256 {
                                word_prev.push(c);
                            }
                            mode2 = M2_OPERATOR_WORD;
                            is_operator_call = true;
                        }
                        // else: operator, or a function of C called operator
                    }
                }
                // 2007.05.26 genta C++/CLI: not after the name of a class and ":" (M2_NAMESPACE_END):
                // "TEMPLATE<sizeof(int)>" must not take sizeof for a function
                if !is_operator_call
                    && nest_func == 0
                    && (mode2 == M2_NORMAL
                        || mode2 == M2_NAMESPACE_SAVE
                        || mode2 == M2_OPERATOR_WORD
                        || mode2 == M2_KR_FUNC)
                {
                    if nest_fparam == 0 {
                        let mut add = true;
                        if [
                            "__declspec",
                            "alignas",
                            "decltype",
                            "_Alignas",
                            "__attribute__",
                        ]
                        .iter()
                        .any(|k| eqs(&word_prev, k))
                        {
                            add = false;
                        } else if !(mode2 == M2_KR_FUNC && in_init_list)
                            && !(mode2 == M2_KR_FUNC && is_function_after_keyword(&word_prev))
                        {
                            // 2010.07.08 ryoji: until the initializer list the later name is the
                            // candidate for the name of the function
                            no_function = false;
                        } else {
                            add = false;
                        }
                        if add {
                            item_name = if template_name.is_empty() {
                                word_prev.clone()
                            } else {
                                template_name.clone()
                            };
                            item_line = n as isize + 1;
                        }
                    }
                    nest_fparam += 1;
                }
                if (mode2 == M2_TEMPLATE_SAVE || mode2 == M2_TEMPLATE_WORD)
                    && template_name.len() + 1 < ITEM_NAME_LEN_MAX
                {
                    template_name.push(c);
                }
                i += 1;
                continue;
            }
            if c == ')' {
                if nest_fparam > 0 {
                    nest_fparam -= 1;
                    // 2007.05.26 genta: mode2 does not change inside a C++/CLI attribute
                    if nest_fparam == 0 && mode2 != M2_ATTRIBUTE && mode2 != M2_TEMPLATE_WORD {
                        if mode2 == M2_NORMAL {
                            in_init_list = false;
                        }
                        if !no_function {
                            mode2 = M2_FUNC_NAME_END;
                            item_func_id = func_obj::FUNCTION;
                        }
                    }
                }
                if mode2 == M2_OPERATOR_WORD {
                    if word_prev.len() + 1 < 256 {
                        word_prev.push(c);
                    }
                } else if (mode2 == M2_TEMPLATE_SAVE || mode2 == M2_TEMPLATE_WORD)
                    && template_name.len() + 1 < ITEM_NAME_LEN_MAX
                {
                    template_name.push(c);
                }
                i += 1;
                continue;
            }
            // 2007.05.26 genta: C++/CLI attributes
            if c == '[' {
                if mode2 == M2_NORMAL && is_operator(&word_prev) {
                    // an operator: operator []
                    if word_prev.len() + 1 < 256 {
                        word_prev.push(c);
                    }
                    mode2 = M2_OPERATOR_WORD;
                } else if nest_func == 0
                    && nest_fparam == 0
                    && (mode2 == M2_NORMAL
                        || mode2 == M2_NAMESPACE_SAVE
                        || mode2 == M2_TEMPLATE_SAVE
                        || mode2 == M2_OPERATOR_WORD
                        || mode2 == M2_TEMPLATE_WORD)
                {
                    mode2_att_old = mode2;
                    mode2 = M2_ATTRIBUTE;
                }
                i += 1;
                continue;
            }
            if c == ']' {
                // [] may be an array inside an attribute, so the parentheses are back at their
                // level
                if nest_fparam == 0 && mode2 == M2_ATTRIBUTE {
                    mode2 = mode2_att_old;
                    i += 1;
                    continue;
                }
                if mode2 == M2_OPERATOR_WORD && word_prev.len() + 1 < 256 {
                    word_prev.push(c);
                }
                i += 1;
                continue;
            }
            if c == ';' {
                if mode2 == M2_KR_FUNC {
                    // zenryaku: after a function declaration of K&R, back to M2_FUNC_NAME_END
                    mode2 = M2_FUNC_NAME_END;
                    if kind == CKind::C {
                        // K&R definition in C
                        i += 1;
                        continue;
                    }
                    // C++: int funcname() const; / void funcname() throw(MyException); /
                    // auto funcname() -> int;
                }
                // Jan. 30, 2005 genta: a declaration is handled after K&R
                if mode2 == M2_FUNC_NAME_END
                    && nest_global < NAMESPACE_NEST_MAX
                    && ns_len[nest_global] + item_name.len() < NAMESPACE_LEN_MAX
                    && nest_point_class == 0
                    && !defined_typedef
                    && 0 < item_line
                {
                    item_func_id = func_obj::DECLARE;
                    let name = qualified(&ns, ns_len[nest_global], &item_name);
                    out.push(item_line as usize, name, item_func_id, 0);
                }
                item_line = -1;
                word_prev.clear();
                template_name.clear(); // hoge < ... ; was not a template
                nest_template = 0;
                mode2 = M2_NORMAL;
                no_function = true;
                defined_typedef = false;
                mode2_old = M2_NORMAL;
                mode = MODE_NORMAL;
                i += 1;
                continue;
            }
            if nest_fparam == 0 && mode2 != M2_ATTRIBUTE {
                // 2007.05.26 genta: nothing of the names is read inside a C++/CLI attribute
                if is_word_char(c) {
                    // Mar. 15, 2000 genta: a "::" at the end of the word before, or at the start
                    // of this word, qualifies it: the two are joined
                    let pos = word_prev.len() as isize - 2;
                    if (pos > 0
                        && word_prev[pos as usize] == ':'
                        && word_prev[pos as usize + 1] == ':')
                        || (i + 1 < len && c == ':' && line[i + 1] == ':')
                    {
                        if mode2 == M2_NORMAL || mode2 == M2_OPERATOR_WORD {
                            if !template_name.is_empty() {
                                word = std::mem::take(&mut template_name);
                            } else {
                                word = word_prev.clone();
                            }
                        }
                        word_idx = word.len() as isize;
                    } else if mode2 == M2_NORMAL && is_operator(&word_prev) {
                        // Apr. 1, 2001 genta: operator new / delete
                        if pos > -1 && is_word_char(word_prev[pos as usize + 1]) {
                            // a space, then the word follows the one before
                            word_prev.push(' ');
                        }
                        word = word_prev.clone();
                        word_idx = word.len() as isize;
                        mode2 = M2_OPERATOR_WORD;
                    } else if mode2 == M2_OPERATOR_WORD {
                        // operator goes on
                        if pos > -1 && is_word_char(word_prev[pos as usize + 1]) {
                            word_prev.push(' ');
                        }
                        word = word_prev.clone();
                        word_idx = word.len() as isize;
                    } else {
                        word_idx = 0;
                    }
                    if c == ':' && at(&line, i + 1) != ':' {
                        if mode2 == M2_NAMESPACE_SAVE {
                            if !word.is_empty() {
                                item_name = word.clone();
                            }
                            mode2 = M2_NAMESPACE_END;
                        } else if mode2 == M2_TEMPLATE_SAVE {
                            append_limited(&mut template_name, &word);
                            mode2 = M2_NAMESPACE_END;
                        } else if mode2 == M2_FUNC_NAME_END || mode2 == M2_KR_FUNC {
                            in_init_list = true;
                        }
                    }
                    // Mar. 15, 2000 genta: the length is checked
                    if word_idx < MAX_WORD_LENGTH {
                        mode = MODE_WORD;
                        word.truncate(word_idx as usize);
                        word.push(c);
                    } else {
                        mode = MODE_TOO_LONG;
                    }
                } else {
                    // 2011.12.02: templates
                    if mode2 == M2_NORMAL || mode2 == M2_OPERATOR_WORD {
                        if c == '<' {
                            if eqs(&word_prev, "template") {
                                mode2 = M2_TEMPLATE;
                                template_name.clear();
                            } else if is_operator(&word_prev) {
                                // operator<
                            } else {
                                // int func <int>();
                                mode2_old = mode2;
                                mode2 = M2_TEMPLATE_WORD;
                                template_name = word_prev
                                    .iter()
                                    .copied()
                                    .take(ITEM_NAME_LEN_MAX - 1)
                                    .collect();
                            }
                        } else {
                            template_name.clear();
                        }
                    }
                    // Aug. 13, 2004 genta: whether it is operator is judged while word_prev is
                    // alive; if it is not, "=" is an assignment: this is not a declaration
                    let nlen = word_prev.len() as isize;
                    if c == '=' && nest_func == 0 && mode2 == M2_NORMAL && !is_operator(&word_prev)
                    {
                        mode2 = M2_AFTER_EQUAL;
                    } else if mode2 == M2_NORMAL && is_operator(&word_prev) {
                        // an operator: operator +
                        word = word_prev.clone();
                        word_idx = nlen - 1;
                        mode2 = M2_OPERATOR_WORD;
                    } else if mode2 == M2_OPERATOR_WORD {
                        // operator goes on
                        word = word_prev.clone();
                        word_idx = nlen - 1;
                    } else {
                        word_prev = word.clone();
                        word_idx = -1;
                    }
                    if mode2 == M2_NAMESPACE_SAVE && c == '<' {
                        mode2 = M2_TEMPLATE_SAVE;
                    }
                    mode = MODE_SYMBOLS;
                    // read this character again
                    continue;
                }
            }
            i += 1;
        }
    }
    (out, kind)
}
