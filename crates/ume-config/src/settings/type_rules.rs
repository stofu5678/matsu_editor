// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/doc/CLineComment.cpp, sakura_core/doc/CLineComment.h
//   sakura_core/doc/CBlockComment.cpp, sakura_core/doc/CBlockComment.h
//   sakura_core/env/CShareData_IO.cpp (ShareData_IO_LineComments, ShareData_IO_BlockComments,
//     ShareData_IO_RegexKeyword, ShareData_IO_VertLineIdx)
//   sakura_core/types/CType.h (the enums EOutlineType, ESmartIndentType, EStringLiteralType)
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2002, YAZAKI
//   Copyright (C) 2005, D.S.Koba, Moca
//   Copyright (C) 2008, kobake
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

//! The rules of a document type that the coloring reads: line comments, block comments, the string
//! (quote) escape rule, regular expression keywords, and the enums of the outline and the smart
//! indent.
//!
//! The rules are plain data in [`TypeSettings`](super::TypeSettings); the helpers here
//! ([`LineComment::matches`], [`BlockComment::matches_start`]) are the matching rules of Sakura's
//! `CLineComment::Match` and `CBlockComment::Match_CommentFrom`/`Match_CommentTo`, so that the
//! coloring engine does not have to know them.

/// `COMMENT_DELIMITER_NUM`: how many line comment delimiters a type has.
pub const LINE_COMMENT_COUNT: usize = 3;
/// `BLOCKCOMMENT_NUM`: how many block comment pairs a type has.
pub const BLOCK_COMMENT_COUNT: usize = 2;
/// `COMMENT_DELIMITER_BUFFERSIZE`: a line comment delimiter is shorter than this, in UTF-16 units.
pub const LINE_COMMENT_BUFFER_SIZE: usize = 16;
/// `BLOCKCOMMENT_BUFFERSIZE`: a block comment delimiter is shorter than this, in UTF-16 units.
pub const BLOCK_COMMENT_BUFFER_SIZE: usize = 16;
/// `MAX_VERTLINES`: how many vertical lines at given columns a type can have.
pub const MAX_VERT_LINES: usize = 200;
/// `MAX_REGEX_KEYWORD`: how many regular expression keywords a type can have.
pub const MAX_REGEX_KEYWORDS: usize = 100;

/// `EStringLiteralType` (`sakura_core/types/CType.h`) as stored in `nStringType`: how a quote
/// inside a string is written.
pub mod string_type {
    /// C/C++ style: `\"` and `\'` (a backslash escapes).
    pub const CPP: i32 = 0;
    /// PL/SQL style: `""` and `''` (a doubled quote).
    pub const PLSQL: i32 = 1;
    /// HTML/XML style: no escape.
    pub const HTML: i32 = 2;
    /// C# style.
    pub const CSHARP: i32 = 3;
    /// Python style (triple quotes).
    pub const PYTHON: i32 = 4;
}

/// `EOutlineType` (`sakura_core/types/CType.h`) as stored in `nDefaultOutline`: the rule that
/// builds the outline of a document of the type.
pub mod outline_type {
    pub const C: i32 = 0;
    /// C/C++, chosen by the file name.
    pub const C_CPP: i32 = 1;
    pub const PLSQL: i32 = 2;
    pub const TEXT: i32 = 3;
    pub const JAVA: i32 = 4;
    pub const COBOL: i32 = 5;
    pub const ASM: i32 = 6;
    pub const PERL: i32 = 7;
    pub const VB: i32 = 8;
    /// Text with levels (WZ editor style).
    pub const WZTXT: i32 = 9;
    pub const HTML: i32 = 10;
    pub const TEX: i32 = 11;
    /// A rule file (`szOutlineRuleFilename`).
    pub const FILE: i32 = 12;
    pub const PYTHON: i32 = 13;
    pub const ERLANG: i32 = 14;
    pub const XML: i32 = 15;
    pub const CPP: i32 = 16;
}

/// `ESmartIndentType` (`sakura_core/types/CType.h`) as stored in `nSmartIndent`.
pub mod smart_indent {
    pub const NONE: i32 = 0;
    pub const CPP: i32 = 1;
}

/// The name of an outline type, as Sakura's outline rule list shows it in English.
pub fn outline_type_name(outline: i32) -> &'static str {
    match outline {
        outline_type::C => "C",
        outline_type::C_CPP => "C/C++",
        outline_type::PLSQL => "PL/SQL",
        outline_type::TEXT => "Text",
        outline_type::JAVA => "Java",
        outline_type::COBOL => "COBOL",
        outline_type::ASM => "Assembler",
        outline_type::PERL => "Perl",
        outline_type::VB => "Visual Basic",
        outline_type::WZTXT => "Text with levels",
        outline_type::HTML => "HTML",
        outline_type::TEX => "TeX",
        outline_type::FILE => "Rule file",
        outline_type::PYTHON => "Python",
        outline_type::ERLANG => "Erlang",
        outline_type::XML => "XML",
        outline_type::CPP => "C++",
        _ => "Unknown",
    }
}

/// A line comment delimiter (`CLineComment`): the rest of the line after it is a comment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineComment {
    /// The delimiter. Empty means "not used".
    pub delimiter: String,
    /// The column the delimiter must start at (0 is the first column, counted in UTF-16 units
    /// like Sakura), or `None` when it may start anywhere.
    pub column: Option<usize>,
}

impl LineComment {
    /// `CLineComment::CopyTo`: a delimiter that is empty or is `LINE_COMMENT_BUFFER_SIZE` UTF-16
    /// units or longer is not used. A negative `column` means "anywhere".
    pub fn new(delimiter: &str, column: i32) -> Self {
        let units = delimiter.encode_utf16().count();
        if units == 0 || units >= LINE_COMMENT_BUFFER_SIZE {
            return Self::default();
        }
        Self {
            delimiter: delimiter.to_owned(),
            column: usize::try_from(column).ok(),
        }
    }

    /// The column as Sakura stores it: `-1` for "anywhere".
    pub fn column_value(&self) -> i32 {
        self.column
            .map_or(-1, |c| i32::try_from(c).unwrap_or(i32::MAX))
    }

    /// `CLineComment::Match` for one delimiter: does `line` have the delimiter at byte offset
    /// `byte_pos`, where the offset is at `column` (UTF-16 units from the start of the line)?
    /// ASCII letters are compared without case, other characters exactly.
    pub fn matches(&self, line: &str, byte_pos: usize, column: usize) -> bool {
        if self.delimiter.is_empty() || self.column.is_some_and(|c| c != column) {
            return false;
        }
        starts_with_ascii_case(line, byte_pos, &self.delimiter)
    }
}

/// A block comment pair (`CBlockComment`): from the start delimiter to the end delimiter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockComment {
    /// The start delimiter. Empty means "not used".
    pub from: String,
    /// The end delimiter. Empty means "not used".
    pub to: String,
}

impl BlockComment {
    /// `CBlockComment::SetBlockCommentRule`: each delimiter that is empty or is
    /// `BLOCK_COMMENT_BUFFER_SIZE` UTF-16 units or longer is dropped, on its own.
    pub fn new(from: &str, to: &str) -> Self {
        let keep = |s: &str| {
            let units = s.encode_utf16().count();
            if units > 0 && units < BLOCK_COMMENT_BUFFER_SIZE {
                s.to_owned()
            } else {
                String::new()
            }
        };
        Self {
            from: keep(from),
            to: keep(to),
        }
    }

    /// Both delimiters are set (only then the pair works).
    pub fn is_active(&self) -> bool {
        !self.from.is_empty() && !self.to.is_empty()
    }

    /// `CBlockComment::Match_CommentFrom`: does `line` have the start delimiter at `byte_pos`?
    pub fn matches_start(&self, line: &str, byte_pos: usize) -> bool {
        self.is_active() && starts_with_ascii_case(line, byte_pos, &self.from)
    }

    /// `CBlockComment::Match_CommentTo`: does `line` have the end delimiter at `byte_pos`?
    pub fn matches_end(&self, line: &str, byte_pos: usize) -> bool {
        !self.to.is_empty() && starts_with_ascii_case(line, byte_pos, &self.to)
    }
}

/// `wmemicmp_ascii`: ASCII letters without case, everything else exactly.
fn starts_with_ascii_case(text: &str, byte_pos: usize, prefix: &str) -> bool {
    text.as_bytes()
        .get(byte_pos..)
        .and_then(|rest| rest.get(..prefix.len()))
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix.as_bytes()))
}

/// A regular expression keyword (`RegexKeywordInfo`): text that matches `pattern` is drawn in the
/// color `color`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexKeyword {
    /// The name of the color entry (`RK1` to `RKA`, or any other, like `URL`).
    pub color: String,
    /// The pattern in Sakura's `/pattern/switches` form (see the help "正規表現キーワード").
    pub pattern: String,
}

/// `MAX_KEYHELP_FILE`: how many keyword help dictionaries a type can have.
pub const MAX_KEYHELP_FILES: usize = 20;
/// `DICT_ABOUT_LEN`: the buffer of the description of a dictionary, with the end mark.
pub const DICT_ABOUT_LEN: usize = 50;

/// A keyword help dictionary (`KeyHelpInfo`): `KDct[NN]=use,about,path`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyHelpDict {
    /// The dictionary is searched (`m_bUse`).
    pub used: bool,
    /// The description, made from the first line of the file (`m_szAbout`).
    pub about: String,
    /// The path of the dictionary file (`m_szPath`); never empty in a list.
    pub path: String,
}

impl KeyHelpDict {
    /// Reads `use,about,path` (`CDataProfile::IOProfileData` for `KeyHelpInfo`: `%d,%[^,],%[^\n]`, or
    /// `%d,,%[^\n]` for an empty description). `None` for a value that does not have the form.
    pub fn parse(value: &str) -> Option<Self> {
        let (used, rest) = value.split_once(',')?;
        let used = used.trim().parse::<i32>().ok()? != 0;
        let (about, path) = rest.split_once(',')?;
        let path = path.split('\n').next().unwrap_or_default();
        Some(Self {
            used,
            about: about.to_owned(),
            path: path.to_owned(),
        })
    }

    /// The value of `KDct[NN]`.
    pub fn format(&self) -> String {
        format!("{},{},{}", i32::from(self.used), self.about, self.path)
    }
}

/// The name of the color a regular expression keyword gets when the file does not say
/// (`COLORIDX_REGEX1`).
pub const DEFAULT_REGEX_COLOR: &str = "RK1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_comment_that_is_empty_or_too_long_is_not_used() {
        assert_eq!(LineComment::new("", 3), LineComment::default());
        assert_eq!(
            LineComment::new(&"a".repeat(16), -1),
            LineComment::default()
        );
        let c = LineComment::new(&"a".repeat(15), -1);
        assert_eq!(c.delimiter.len(), 15);
        assert_eq!(c.column, None);
        assert_eq!(c.column_value(), -1);
    }

    #[test]
    fn a_line_comment_with_a_column_matches_only_there() {
        // COBOL: `*` in column 7 (index 6).
        let c = LineComment::new("*", 6);
        assert_eq!(c.column, Some(6));
        assert!(c.matches("      * comment", 6, 6));
        assert!(!c.matches("      * comment", 6, 5));
        assert!(!c.matches("*", 0, 0));
    }

    #[test]
    fn line_comments_compare_ascii_letters_without_case() {
        let rem = LineComment::new("REM ", -1);
        assert!(rem.matches("rem hello", 0, 0));
        assert!(rem.matches("Rem hello", 0, 0));
        assert!(!rem.matches("remark", 0, 0));
        // Non-ASCII is exact.
        let c = LineComment::new("※", -1);
        assert!(c.matches("※メモ", 0, 0));
        assert!(!c.matches("メモ", 0, 0));
        // Past the end of the line.
        assert!(!rem.matches("re", 0, 0));
        assert!(!rem.matches("x", 5, 5));
    }

    #[test]
    fn a_block_comment_needs_both_delimiters() {
        let c = BlockComment::new("/*", "*/");
        assert!(c.is_active());
        assert!(c.matches_start("a /* b", 2));
        assert!(c.matches_end("a */", 2));
        assert!(!c.matches_start("a /* b", 3));
        let half = BlockComment::new("/*", "");
        assert!(!half.is_active());
        assert!(!half.matches_start("/*", 0));
        // Each delimiter is dropped on its own when it is too long.
        let long = BlockComment::new("<!--", &"x".repeat(16));
        assert_eq!(long.from, "<!--");
        assert_eq!(long.to, "");
    }

    #[test]
    fn the_names_of_outline_types() {
        assert_eq!(outline_type_name(outline_type::C_CPP), "C/C++");
        assert_eq!(outline_type_name(999), "Unknown");
    }
}
