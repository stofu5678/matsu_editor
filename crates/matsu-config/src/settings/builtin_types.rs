// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/types/CType.cpp (CShareData::InitTypeConfigs: the list and the order of the types)
//   sakura_core/types/CType_Basis.cpp, CType_Text.cpp, CType_Cpp.cpp, CType_Html.cpp, CType_Sql.cpp,
//   CType_Cobol.cpp, CType_Java.cpp, CType_Asm.cpp, CType_Awk.cpp, CType_Dos.cpp, CType_Pascal.cpp,
//   CType_Tex.cpp, CType_Perl.cpp, CType_Python.cpp, CType_Vb.cpp, CType_Rich.cpp, CType_Ini.cpp
//   (each type's InitTypeConfigImp)
//
// Original copyright notices of those files:
//   Copyright (C) 2007, genta
//   Copyright (C) 2008, kobake
//   Copyright (C) 2018-2022, Sakura Editor Organization
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

//! Sakura's 17 built-in document types, with the values their `InitTypeConfigImp` sets over the
//! defaults of every type ([`TypeSettings::default`], `_DefaultConfig`).
//!
//! The order is the order of `CShareData::InitTypeConfigs`, which is the index a type has in the
//! settings file (`Types(N)`):
//!
//! | N | name | extensions |
//! |---|------|------------|
//! | 0 | 基本 | (none) |
//! | 1 | テキスト | txt, log, 1st, err, ps |
//! | 2 | C/C++ | c, cpp, cxx, cc, cp, c++, h, hpp, hxx, hh, hp, h++, rc, hm |
//! | 3 | HTML | html, htm, shtml, plg |
//! | 4 | PL/SQL | plsql, pls, plb, pck, pkb, pks (Sakura: sql, plsql; `.sql` is the "SQL" type of ADR-0023) |
//! | 5 | COBOL | cbl, cpy, pco, cob |
//! | 6 | Java | java, jav |
//! | 7 | アセンブラ | asm |
//! | 8 | AWK | awk |
//! | 9 | MS-DOSバッチファイル | bat |
//! | 10 | Pascal | dpr, pas |
//! | 11 | TeX | tex, ltx, sty, bib, log, blg, aux, bbl, toc, lof, lot, idx, ind, glo |
//! | 12 | Perl | cgi, pl, pm |
//! | 13 | Python | py |
//! | 14 | Visual Basic | bas, frm, cls, ctl, pag, dob, dsr, vb |
//! | 15 | リッチテキスト | rtf |
//! | 16 | 設定ファイル | ini, inf, cnf, kwd, col |
//!
//! Sakura's `CType_Erlang.cpp` and `CType_CorbaIdl.cpp` are not in this list: Sakura does not
//! register them as types (the file `CType_Erlang.cpp` only has the outline of Erlang, and
//! `CType_CorbaIdl.cpp` only has the keyword set 5, "CORBA IDL", which no built-in type uses).
//!
//! The keyword set indexes (`nKeyWordSetIdx`) refer to [`KeywordSets::builtin`](super::KeywordSets). The only
//! difference from Sakura is the second keyword set of Python (ADR-0017). MATSU Editor's own types are in
//! [`extra_types`](super::extra_types).

use super::color::names;
use super::extra_types::ExtraSet;
use super::type_rules::{BlockComment, LineComment, outline_type, smart_indent, string_type};
use super::types::TypeSettings;

/// The kinsoku strings of "テキスト" (`CType_Text.cpp`).
const TEXT_KINSOKU_HEAD: &str =
    "!%),.:;?]}\u{a2}°’”‰′″℃、。々〉》」』】〕゛゜ゝゞ・ヽヾ！％），．：；？］｝｡｣､･ﾞﾟ￠";
const TEXT_KINSOKU_TAIL: &str = "$([\\{\u{a3}\u{a5}‘“〈《「『【〔＄（［｛｢￡￥";

/// The regular expression keywords of "テキスト": file paths become clickable (`CType_Text.cpp`).
/// The patterns with a leading `/` and the switch `k` are in Sakura's form.
const TEXT_REGEX_KEYWORDS: [&str; 2] = [
    // C:\... or \\... in double quotes
    "/(?<=\")(\\b[a-zA-Z]:|\\B\\\\\\\\)[^\"\\r\\n]*/k",
    // C:\... or \\...
    "/(\\b[a-zA-Z]:\\\\|\\B\\\\\\\\)[\\w\\-_.\\\\\\/$%~]*/k",
];

/// How many built-in types there are (the size of the table in `InitTypeConfigs`).
pub const BUILTIN_TYPE_COUNT: usize = 17;

/// `STR_TRAY_TYPE_NAME` ("設定%d"): the name of a type that is added to the 17 built-in ones.
pub fn added_type_name(index: i32) -> String {
    format!("設定{index}")
}

pub(super) fn set_disp(t: &mut TypeSettings, name: &str, disp: bool) {
    if let Some(c) = t.colors.get_mut(name) {
        c.disp = disp;
    }
}

pub(super) fn named(index: usize, name: &str, exts: &str) -> TypeSettings {
    // `CType::InitTypeConfig`: the defaults, the index and the id, then the type's own values.
    let index = i32::try_from(index).unwrap_or(0);
    TypeSettings {
        idx: index,
        id: index,
        type_name: name.to_owned(),
        type_exts: exts.to_owned(),
        ..TypeSettings::default()
    }
}

/// `CShareData::InitTypeConfigs`: the built-in types, in the order of their index.
pub fn builtin_types() -> Vec<TypeSettings> {
    let mut types = vec![
        basic(),
        text(),
        cpp(),
        html(),
        sql(),
        cobol(),
        java(),
        asm(),
        awk(),
        dos(),
        pascal(),
        tex(),
        perl(),
        python(),
        vb(),
        rich(),
        ini(),
    ];
    for (i, t) in types.iter_mut().enumerate() {
        let index = i32::try_from(i).unwrap_or(0);
        t.idx = index;
        t.id = index;
    }
    types
}

/// `CType_Basis`.
pub(super) fn basic() -> TypeSettings {
    let mut t = named(0, "基本", "");
    t.default_outline = outline_type::TEXT;
    set_disp(&mut t, names::SINGLE_QUOTE, false);
    set_disp(&mut t, names::DOUBLE_QUOTE, false);
    t
}

/// `CType_Text`.
fn text() -> TypeSettings {
    let mut t = named(1, "テキスト", "txt,log,1st,err,ps");
    t.max_line_ketas = 120;
    t.default_outline = outline_type::TEXT;
    set_disp(&mut t, names::SINGLE_QUOTE, false);
    set_disp(&mut t, names::DOUBLE_QUOTE, false);
    t.kinsoku_head_enabled = false;
    t.kinsoku_tail_enabled = false;
    t.kinsoku_ret = false;
    t.kinsoku_kuto_enabled = false;
    t.kinsoku_head = TEXT_KINSOKU_HEAD.to_owned();
    t.kinsoku_tail = TEXT_KINSOKU_TAIL.to_owned();
    t.use_regex_keyword = true;
    t.regex_keywords = TEXT_REGEX_KEYWORDS
        .iter()
        .map(|p| super::type_rules::RegexKeyword {
            color: names::URL.to_owned(),
            pattern: (*p).to_owned(),
        })
        .collect();
    t
}

/// `CType_Cpp`.
fn cpp() -> TypeSettings {
    let mut t = named(2, "C/C++", "c,cpp,cxx,cc,cp,c++,h,hpp,hxx,hh,hp,h++,rc,hm");
    t.line_comments[0] = LineComment::new("//", -1);
    t.block_comments[0] = BlockComment::new("/*", "*/");
    t.block_comments[1] = BlockComment::new("#if 0", "#endif");
    t.keyword_set_idx[0] = 0;
    t.default_outline = outline_type::C_CPP;
    t.smart_indent = smart_indent::CPP;
    set_disp(&mut t, names::NUMBER, true);
    set_disp(&mut t, names::BRACKET_PAIR, true);
    t.string_line_only = true;
    t
}

/// `CType_Html`.
fn html() -> TypeSettings {
    let mut t = named(3, "HTML", "html,htm,shtml,plg");
    t.block_comments[0] = BlockComment::new("<!--", "-->");
    t.string_type = string_type::HTML;
    t.string_line_only = true;
    t.keyword_set_idx[0] = 1;
    t.default_outline = outline_type::HTML;
    set_disp(&mut t, names::SINGLE_QUOTE, true);
    t
}

/// `CType_Sql`.
fn sql() -> TypeSettings {
    let mut t = named(4, "PL/SQL", "plsql,pls,plb,pck,pkb,pks");
    t.line_comments[0] = LineComment::new("--", -1);
    t.block_comments[0] = BlockComment::new("/*", "*/");
    t.string_type = string_type::PLSQL;
    t.indent_chars = "|★".to_owned();
    t.keyword_set_idx[0] = 2;
    t.default_outline = outline_type::PLSQL;
    t
}

/// `CType_Cobol`.
fn cobol() -> TypeSettings {
    let mut t = named(5, "COBOL", "cbl,cpy,pco,cob");
    t.line_comments[0] = LineComment::new("*", 6);
    t.line_comments[1] = LineComment::new("D", 6);
    t.string_type = string_type::PLSQL;
    t.indent_chars = "*".to_owned();
    t.keyword_set_idx[0] = 3;
    t.default_outline = outline_type::COBOL;
    set_disp(&mut t, names::VERT_LINE, true);
    t.vert_line_idx[..4].copy_from_slice(&[7, 8, 12, 73]);
    t
}

/// `CType_Java`.
fn java() -> TypeSettings {
    let mut t = named(6, "Java", "java,jav");
    t.line_comments[0] = LineComment::new("//", -1);
    t.block_comments[0] = BlockComment::new("/*", "*/");
    t.keyword_set_idx[0] = 4;
    t.default_outline = outline_type::JAVA;
    t.smart_indent = smart_indent::CPP;
    set_disp(&mut t, names::NUMBER, true);
    set_disp(&mut t, names::BRACKET_PAIR, true);
    t.string_line_only = true;
    t
}

/// `CType_Asm`.
fn asm() -> TypeSettings {
    let mut t = named(7, "アセンブラ", "asm");
    t.line_comments[0] = LineComment::new(";", -1);
    t.default_outline = outline_type::ASM;
    set_disp(&mut t, names::NUMBER, true);
    t
}

/// `CType_Awk`.
fn awk() -> TypeSettings {
    let mut t = named(8, "AWK", "awk");
    t.line_comments[0] = LineComment::new("#", -1);
    t.default_outline = outline_type::TEXT;
    t.keyword_set_idx[0] = 6;
    t
}

/// `CType_Dos`.
fn dos() -> TypeSettings {
    let mut t = named(9, "MS-DOSバッチファイル", "bat");
    t.line_comments[0] = LineComment::new("REM ", -1);
    t.default_outline = outline_type::TEXT;
    t.keyword_set_idx[0] = 7;
    t
}

/// `CType_Pascal`.
fn pascal() -> TypeSettings {
    let mut t = named(10, "Pascal", "dpr,pas");
    t.line_comments[0] = LineComment::new("//", -1);
    t.block_comments[0] = BlockComment::new("{", "}");
    t.block_comments[1] = BlockComment::new("(*", "*)");
    t.string_type = string_type::PLSQL;
    t.keyword_set_idx[0] = 8;
    set_disp(&mut t, names::NUMBER, true);
    t.string_line_only = true;
    t
}

/// `CType_Tex`.
fn tex() -> TypeSettings {
    let mut t = named(
        11,
        "TeX",
        "tex,ltx,sty,bib,log,blg,aux,bbl,toc,lof,lot,idx,ind,glo",
    );
    t.line_comments[0] = LineComment::new("%", -1);
    t.default_outline = outline_type::TEX;
    t.keyword_set_idx[0] = 9;
    t.keyword_set_idx[1] = 10;
    set_disp(&mut t, names::SINGLE_QUOTE, false);
    set_disp(&mut t, names::DOUBLE_QUOTE, false);
    t
}

/// `CType_Perl`.
fn perl() -> TypeSettings {
    let mut t = named(12, "Perl", "cgi,pl,pm");
    t.line_comments[0] = LineComment::new("#", -1);
    t.default_outline = outline_type::PERL;
    t.keyword_set_idx[0] = 11;
    t.keyword_set_idx[1] = 12;
    set_disp(&mut t, names::NUMBER, true);
    set_disp(&mut t, names::BRACKET_PAIR, true);
    t.string_line_only = true;
    t
}

/// `CType_Python`.
fn python() -> TypeSettings {
    let mut t = named(13, "Python", "py");
    t.line_comments[0] = LineComment::new("#", -1);
    t.default_outline = outline_type::PYTHON;
    t.keyword_set_idx[0] = 16;
    // ADR-0017: the built-in functions are MATSU Editor's addition. The index is the one the set has when the
    // file has Sakura's 17 sets only; `DocumentTypes::from_profile` moves it when the file has more.
    t.keyword_set_idx[1] = ExtraSet::PythonBuiltins.default_index();
    set_disp(&mut t, names::NUMBER, true);
    set_disp(&mut t, names::BRACKET_PAIR, true);
    t.string_type = string_type::PYTHON;
    t.string_line_only = true;
    t
}

/// `CType_Vb`.
fn vb() -> TypeSettings {
    let mut t = named(14, "Visual Basic", "bas,frm,cls,ctl,pag,dob,dsr,vb");
    t.line_comments[0] = LineComment::new("'", -1);
    t.default_outline = outline_type::VB;
    t.keyword_set_idx[0] = 13;
    t.keyword_set_idx[1] = 14;
    set_disp(&mut t, names::NUMBER, true);
    t.string_type = string_type::PLSQL;
    set_disp(&mut t, names::SINGLE_QUOTE, false);
    t.string_line_only = true;
    t
}

/// `CType_Rich`.
fn rich() -> TypeSettings {
    let mut t = named(15, "リッチテキスト", "rtf");
    t.default_outline = outline_type::TEXT;
    t.keyword_set_idx[0] = 15;
    set_disp(&mut t, names::NUMBER, true);
    set_disp(&mut t, names::SINGLE_QUOTE, false);
    set_disp(&mut t, names::DOUBLE_QUOTE, false);
    set_disp(&mut t, names::URL, false);
    t
}

/// `CType_Ini`.
fn ini() -> TypeSettings {
    let mut t = named(16, "設定ファイル", "ini,inf,cnf,kwd,col");
    t.line_comments[0] = LineComment::new("//", -1);
    t.line_comments[1] = LineComment::new(";", -1);
    t.default_outline = outline_type::TEXT;
    set_disp(&mut t, names::SINGLE_QUOTE, false);
    set_disp(&mut t, names::DOUBLE_QUOTE, false);
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn by_name<'a>(types: &'a [TypeSettings], name: &str) -> &'a TypeSettings {
        types.iter().find(|t| t.type_name == name).expect(name)
    }

    #[test]
    fn there_are_17_types_in_sakuras_order() {
        let types = builtin_types();
        assert_eq!(types.len(), BUILTIN_TYPE_COUNT);
        let names: Vec<&str> = types.iter().map(|t| t.type_name.as_str()).collect();
        assert_eq!(
            names,
            [
                "基本",
                "テキスト",
                "C/C++",
                "HTML",
                "PL/SQL",
                "COBOL",
                "Java",
                "アセンブラ",
                "AWK",
                "MS-DOSバッチファイル",
                "Pascal",
                "TeX",
                "Perl",
                "Python",
                "Visual Basic",
                "リッチテキスト",
                "設定ファイル",
            ]
        );
        for (i, t) in types.iter().enumerate() {
            assert_eq!(t.idx as usize, i);
            assert_eq!(t.id as usize, i);
        }
    }

    #[test]
    fn the_basic_type_has_no_extensions_and_no_string_colors() {
        let types = builtin_types();
        let basic = &types[0];
        assert_eq!(basic.type_exts, "");
        assert_eq!(basic.max_line_ketas, 10240);
        assert_eq!(basic.tab_space, 4);
        assert!(!basic.colors.get("SQT").unwrap().disp);
        assert!(!basic.colors.get("WQT").unwrap().disp);
        assert_eq!(basic, &TypeSettings::basic());
    }

    #[test]
    fn text_wraps_at_120_and_has_the_path_keywords() {
        // CType_Text.cpp
        let types = builtin_types();
        let text = by_name(&types, "テキスト");
        assert_eq!(text.type_exts, "txt,log,1st,err,ps");
        assert_eq!(text.max_line_ketas, 120);
        assert!(text.kinsoku_head.starts_with("!%),.:;?]}\u{a2}°"));
        assert!(text.kinsoku_tail.starts_with("$([\\{\u{a3}\u{a5}"));
        assert!(!text.kinsoku_head_enabled);
        assert!(text.use_regex_keyword);
        assert_eq!(text.regex_keywords.len(), 2);
        assert!(text.regex_keywords.iter().all(|k| k.color == "URL"));
        assert!(text.regex_keywords[1].pattern.ends_with("/k"));
    }

    #[test]
    fn cpp_has_the_c_comments_and_the_if_0_block() {
        // CType_Cpp.cpp
        let types = builtin_types();
        let cpp = by_name(&types, "C/C++");
        assert_eq!(cpp.line_comments[0], LineComment::new("//", -1));
        assert_eq!(cpp.line_comments[1], LineComment::default());
        assert_eq!(cpp.block_comments[0], BlockComment::new("/*", "*/"));
        assert_eq!(cpp.block_comments[1], BlockComment::new("#if 0", "#endif"));
        assert_eq!(cpp.keyword_set_idx[0], 0);
        assert_eq!(cpp.keyword_set_idx[1], -1);
        assert_eq!(cpp.default_outline, outline_type::C_CPP);
        assert_eq!(cpp.smart_indent, smart_indent::CPP);
        assert!(cpp.colors_numbers());
        assert!(cpp.colors.get("BRC").unwrap().disp);
        assert!(cpp.string_line_only);
        assert_eq!(cpp.string_type, string_type::CPP);
        // Double and single quoted strings are colored, unlike the basic type.
        assert!(cpp.colors.get("WQT").unwrap().disp);
    }

    #[test]
    fn cobol_comments_depend_on_the_column() {
        // CType_Cobol.cpp
        let types = builtin_types();
        let cobol = by_name(&types, "COBOL");
        assert_eq!(cobol.line_comments[0], LineComment::new("*", 6));
        assert_eq!(cobol.line_comments[1], LineComment::new("D", 6));
        assert_eq!(cobol.string_type, string_type::PLSQL);
        assert_eq!(cobol.indent_chars, "*");
        assert!(cobol.colors.get("VER").unwrap().disp);
        assert_eq!(cobol.vert_lines().collect::<Vec<_>>(), [7, 8, 12, 73]);
    }

    #[test]
    fn the_other_types_follow_their_sources() {
        let types = builtin_types();
        let t = |n: &str| by_name(&types, n);
        // CType_Html.cpp: HTML strings, single quotes colored.
        assert_eq!(t("HTML").string_type, string_type::HTML);
        assert_eq!(
            t("HTML").block_comments[0],
            BlockComment::new("<!--", "-->")
        );
        assert!(t("HTML").colors.get("SQT").unwrap().disp);
        // CType_Sql.cpp
        assert_eq!(t("PL/SQL").line_comments[0].delimiter, "--");
        assert_eq!(t("PL/SQL").indent_chars, "|★");
        // CType_Dos.cpp: the delimiter has a space and ignores case.
        assert_eq!(t("MS-DOSバッチファイル").line_comments[0].delimiter, "REM ");
        // CType_Pascal.cpp
        assert_eq!(t("Pascal").block_comments[0], BlockComment::new("{", "}"));
        assert_eq!(t("Pascal").block_comments[1], BlockComment::new("(*", "*)"));
        // CType_Tex.cpp: two keyword sets.
        assert_eq!(t("TeX").keyword_set_idx[..3], [9, 10, -1]);
        // CType_Python.cpp
        assert_eq!(t("Python").string_type, string_type::PYTHON);
        // CType_Vb.cpp: ' is the comment, so single quote strings are off.
        assert_eq!(t("Visual Basic").line_comments[0].delimiter, "'");
        assert!(!t("Visual Basic").colors.get("SQT").unwrap().disp);
        // CType_Rich.cpp: URLs are not underlined.
        assert!(!t("リッチテキスト").colors.get("URL").unwrap().disp);
        // CType_Ini.cpp: two comment styles.
        assert_eq!(t("設定ファイル").line_comments[1].delimiter, ";");
        // CType_Asm.cpp, CType_Awk.cpp, CType_Perl.cpp, CType_Java.cpp
        assert_eq!(t("アセンブラ").line_comments[0].delimiter, ";");
        assert_eq!(t("AWK").keyword_set_idx[0], 6);
        assert_eq!(t("Perl").keyword_set_idx[..2], [11, 12]);
        assert_eq!(t("Java").default_outline, outline_type::JAVA);
    }

    #[test]
    fn every_keyword_index_points_at_a_builtin_set() {
        let sets = super::super::KeywordSets::builtin();
        for t in builtin_types() {
            for &i in &t.keyword_set_idx {
                assert!(i == -1 || sets.get(i).is_some(), "{} uses {i}", t.type_name);
            }
        }
    }

    #[test]
    fn only_the_type_that_uses_a_comment_has_it() {
        let types = builtin_types();
        assert!(
            by_name(&types, "基本")
                .line_comments
                .iter()
                .all(|c| c.delimiter.is_empty())
        );
        assert_eq!(added_type_name(17), "設定17");
    }
}

/// Expectations copied from Sakura's `InitTypeConfigImp` of each type, written with the raw numbers
/// of Sakura's enums (not this crate's constants), so that a slip in the constants shows up too.
#[cfg(test)]
mod sakura_table {
    use super::*;

    /// One row: what `CType_*.cpp` sets over `_DefaultConfig`. A field left at `NONE`'s value means
    /// "not set by the type".
    struct Row {
        /// `m_szTypeName`.
        name: &'static str,
        /// `m_szTypeExts`.
        exts: &'static str,
        /// `m_cLineComment.CopyTo(i, delimiter, column)`.
        line: &'static [(&'static str, i32)],
        /// `m_cBlockComments[i].SetBlockCommentRule(from, to)`.
        block: &'static [(&'static str, &'static str)],
        /// `m_nStringType`: 0 C++, 1 PL/SQL, 2 HTML, 3 C#, 4 Python.
        string_type: i32,
        /// `m_bStringLineOnly`.
        line_only: bool,
        /// `m_szIndentChars`.
        indent_chars: &'static str,
        /// `m_nKeyWordSetIdx[0]` and `[1]` (-1 when not set).
        keywords: [i32; 2],
        /// `m_eDefaultOutline`: 1 C_CPP, 2 PLSQL, 3 TEXT, 4 JAVA, 5 COBOL, 6 ASM, 7 PERL, 8 VB, 10 HTML,
        /// 11 TEX, 13 PYTHON.
        outline: i32,
        /// `m_eSmartIndent`: 0 none, 1 C/C++.
        smart_indent: i32,
        /// Colors whose `m_bDisp` the type sets to true.
        disp_on: &'static [&'static str],
        /// Colors whose `m_bDisp` the type sets to false.
        disp_off: &'static [&'static str],
    }

    const NONE: Row = Row {
        name: "",
        exts: "",
        line: &[],
        block: &[],
        string_type: 0,
        line_only: false,
        indent_chars: "",
        keywords: [-1, -1],
        outline: 3,
        smart_indent: 0,
        disp_on: &[],
        disp_off: &[],
    };

    const QUOTES_OFF: &[&str] = &["SQT", "WQT"];

    /// The rows, in the order of `CShareData::InitTypeConfigs`.
    fn table() -> Vec<Row> {
        vec![
            // CType_Basis.cpp
            Row {
                name: "基本",
                disp_off: QUOTES_OFF,
                ..NONE
            },
            // CType_Text.cpp (the width, the kinsoku and the regular expressions are checked apart)
            Row {
                name: "テキスト",
                exts: "txt,log,1st,err,ps",
                disp_off: QUOTES_OFF,
                ..NONE
            },
            // CType_Cpp.cpp
            Row {
                name: "C/C++",
                exts: "c,cpp,cxx,cc,cp,c++,h,hpp,hxx,hh,hp,h++,rc,hm",
                line: &[("//", -1)],
                block: &[("/*", "*/"), ("#if 0", "#endif")],
                line_only: true,
                keywords: [0, -1],
                outline: 1,
                smart_indent: 1,
                disp_on: &["NUM", "BRC"],
                ..NONE
            },
            // CType_Html.cpp
            Row {
                name: "HTML",
                exts: "html,htm,shtml,plg",
                block: &[("<!--", "-->")],
                string_type: 2,
                line_only: true,
                keywords: [1, -1],
                outline: 10,
                disp_on: &["SQT"],
                ..NONE
            },
            // CType_Sql.cpp (Sakura's extensions are "sql,plsql"; see ADR-0023)
            Row {
                name: "PL/SQL",
                exts: "plsql,pls,plb,pck,pkb,pks",
                line: &[("--", -1)],
                block: &[("/*", "*/")],
                string_type: 1,
                indent_chars: "|★",
                keywords: [2, -1],
                outline: 2,
                ..NONE
            },
            // CType_Cobol.cpp
            Row {
                name: "COBOL",
                exts: "cbl,cpy,pco,cob",
                line: &[("*", 6), ("D", 6)],
                string_type: 1,
                indent_chars: "*",
                keywords: [3, -1],
                outline: 5,
                disp_on: &["VER"],
                ..NONE
            },
            // CType_Java.cpp
            Row {
                name: "Java",
                exts: "java,jav",
                line: &[("//", -1)],
                block: &[("/*", "*/")],
                line_only: true,
                keywords: [4, -1],
                outline: 4,
                smart_indent: 1,
                disp_on: &["NUM", "BRC"],
                ..NONE
            },
            // CType_Asm.cpp
            Row {
                name: "アセンブラ",
                exts: "asm",
                line: &[(";", -1)],
                outline: 6,
                disp_on: &["NUM"],
                ..NONE
            },
            // CType_Awk.cpp
            Row {
                name: "AWK",
                exts: "awk",
                line: &[("#", -1)],
                keywords: [6, -1],
                ..NONE
            },
            // CType_Dos.cpp
            Row {
                name: "MS-DOSバッチファイル",
                exts: "bat",
                line: &[("REM ", -1)],
                keywords: [7, -1],
                ..NONE
            },
            // CType_Pascal.cpp (it does not set the outline: the default, text)
            Row {
                name: "Pascal",
                exts: "dpr,pas",
                line: &[("//", -1)],
                block: &[("{", "}"), ("(*", "*)")],
                string_type: 1,
                line_only: true,
                keywords: [8, -1],
                disp_on: &["NUM"],
                ..NONE
            },
            // CType_Tex.cpp
            Row {
                name: "TeX",
                exts: "tex,ltx,sty,bib,log,blg,aux,bbl,toc,lof,lot,idx,ind,glo",
                line: &[("%", -1)],
                keywords: [9, 10],
                outline: 11,
                disp_off: QUOTES_OFF,
                ..NONE
            },
            // CType_Perl.cpp
            Row {
                name: "Perl",
                exts: "cgi,pl,pm",
                line: &[("#", -1)],
                line_only: true,
                keywords: [11, 12],
                outline: 7,
                disp_on: &["NUM", "BRC"],
                ..NONE
            },
            // CType_Python.cpp (the second keyword set is ADR-0017's addition, checked apart)
            Row {
                name: "Python",
                exts: "py",
                line: &[("#", -1)],
                string_type: 4,
                line_only: true,
                keywords: [16, -1],
                outline: 13,
                disp_on: &["NUM", "BRC"],
                ..NONE
            },
            // CType_Vb.cpp
            Row {
                name: "Visual Basic",
                exts: "bas,frm,cls,ctl,pag,dob,dsr,vb",
                line: &[("'", -1)],
                string_type: 1,
                line_only: true,
                keywords: [13, 14],
                outline: 8,
                disp_on: &["NUM"],
                disp_off: &["SQT"],
                ..NONE
            },
            // CType_Rich.cpp
            Row {
                name: "リッチテキスト",
                exts: "rtf",
                keywords: [15, -1],
                disp_on: &["NUM"],
                disp_off: &["SQT", "WQT", "URL"],
                ..NONE
            },
            // CType_Ini.cpp
            Row {
                name: "設定ファイル",
                exts: "ini,inf,cnf,kwd,col",
                line: &[("//", -1), (";", -1)],
                disp_off: QUOTES_OFF,
                ..NONE
            },
        ]
    }

    /// Builds the type Sakura would have: `_DefaultConfig` plus the row.
    fn expected(index: usize, row: &Row) -> TypeSettings {
        let mut t = TypeSettings::default();
        let index = i32::try_from(index).unwrap();
        t.idx = index;
        t.id = index;
        t.type_name = row.name.to_owned();
        t.type_exts = row.exts.to_owned();
        for (i, &(delimiter, column)) in row.line.iter().enumerate() {
            t.line_comments[i] = LineComment::new(delimiter, column);
        }
        for (i, &(from, to)) in row.block.iter().enumerate() {
            t.block_comments[i] = BlockComment::new(from, to);
        }
        t.string_type = row.string_type;
        t.string_line_only = row.line_only;
        t.indent_chars = row.indent_chars.to_owned();
        t.keyword_set_idx[0] = row.keywords[0];
        t.keyword_set_idx[1] = row.keywords[1];
        t.default_outline = row.outline;
        t.smart_indent = row.smart_indent;
        for name in row.disp_on {
            t.colors.get_mut(name).unwrap().disp = true;
        }
        for name in row.disp_off {
            t.colors.get_mut(name).unwrap().disp = false;
        }
        t
    }

    #[test]
    fn every_type_is_the_default_plus_what_its_sakura_file_sets() {
        let actual = builtin_types();
        let table = table();
        assert_eq!(actual.len(), table.len());
        for (i, (got, row)) in actual.iter().zip(&table).enumerate() {
            let mut want = expected(i, row);
            match row.name {
                "テキスト" => {
                    want.max_line_ketas = 120;
                    want.kinsoku_head = TEXT_KINSOKU_HEAD.to_owned();
                    want.kinsoku_tail = TEXT_KINSOKU_TAIL.to_owned();
                    want.use_regex_keyword = true;
                    want.regex_keywords = got.regex_keywords.clone();
                }
                "COBOL" => want.vert_line_idx[..4].copy_from_slice(&[7, 8, 12, 73]),
                "Python" => want.keyword_set_idx[1] = got.keyword_set_idx[1],
                _ => {}
            }
            assert_eq!(got, &want, "{}", row.name);
        }
    }

    #[test]
    fn text_has_sakuras_kinsoku_and_path_patterns() {
        // CType_Text.cpp. `\xa2`, `\xa3` and `\xa5` are written as in the source.
        let types = builtin_types();
        let t = &types[1];
        assert_eq!(t.max_line_ketas, 120);
        assert_eq!(
            t.kinsoku_head,
            "!%),.:;?]}\u{a2}°’”‰′″℃、。々〉》」』】〕゛゜ゝゞ・ヽヾ！％），．：；？］｝｡｣､･ﾞﾟ￠"
        );
        assert_eq!(
            t.kinsoku_tail,
            "$([\\{\u{a3}\u{a5}‘“〈《「『【〔＄（［｛｢￡￥"
        );
        // `_DefaultConfig`: the hanging characters are set for every type.
        assert_eq!(t.kinsoku_kuto, "、。，．､｡,.");
        assert!(!t.kinsoku_head_enabled && !t.kinsoku_tail_enabled);
        assert!(!t.kinsoku_ret && !t.kinsoku_kuto_enabled);
        assert!(t.use_regex_keyword);
        let patterns: Vec<&str> = t
            .regex_keywords
            .iter()
            .map(|k| k.pattern.as_str())
            .collect();
        assert_eq!(
            patterns,
            [
                r#"/(?<=")(\b[a-zA-Z]:|\B\\\\)[^"\r\n]*/k"#,
                r#"/(\b[a-zA-Z]:\\|\B\\\\)[\w\-_.\\\/$%~]*/k"#,
            ]
        );
        assert!(t.regex_keywords.iter().all(|k| k.color == "URL"));
    }

    #[test]
    fn python_has_the_builtins_as_its_second_set() {
        // ADR-0017: the only keyword difference from Sakura.
        let types = builtin_types();
        assert_eq!(
            types[13].keyword_set_idx[1],
            ExtraSet::PythonBuiltins.default_index()
        );
    }
}
