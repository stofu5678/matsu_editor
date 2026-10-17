// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/view/colors/CColorStrategy.cpp (CColorStrategyPool::OnChangeSetting)
//   sakura_core/view/colors/CColor_Comment.h, sakura_core/view/colors/CColor_Quote.h,
//     sakura_core/view/colors/CColor_Heredoc.h, sakura_core/view/colors/CColor_Url.h,
//     sakura_core/view/colors/CColor_Numeric.h, sakura_core/view/colors/CColor_KeywordSet.h (Disp)
//
// Original copyright notices of those files:
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

//! The rules of one document type, as the coloring reads them.
//!
//! [`ColorRules`] is made from a type's settings and the keyword sets once, when the type is
//! applied. It says which strategies are on (`CColorStrategy::Disp` of each class: a strategy is
//! off when the `disp` flag of its color is off, or its delimiters are not set), and holds the
//! color entries the runs are drawn in.

use ume_config::settings::color::ColorEntry;
use ume_config::settings::keywords::KeywordSets;
use ume_config::settings::type_rules::{
    BLOCK_COMMENT_COUNT, BlockComment, LineComment, string_type,
};
use ume_config::settings::{ColorSet, TypeSettings};

use crate::keyword::PreparedSet;
use crate::kind::{ColorKind, KEYWORD_SLOTS};

/// `HEREDOC_PHP` (`EHereDocType`, `sakura_core/types/CType.h`): the only kind of here-document
/// that Sakura colors.
const HEREDOC_PHP: i32 = 0;

/// The kinds that have a color entry of their own, in the order of [`ColorRules::entry`].
const KINDS: [ColorKind; 7 + KEYWORD_SLOTS] = [
    ColorKind::Text,
    ColorKind::Comment,
    ColorKind::SingleQuote,
    ColorKind::DoubleQuote,
    ColorKind::Heredoc,
    ColorKind::Url,
    ColorKind::Number,
    ColorKind::Keyword(0),
    ColorKind::Keyword(1),
    ColorKind::Keyword(2),
    ColorKind::Keyword(3),
    ColorKind::Keyword(4),
    ColorKind::Keyword(5),
    ColorKind::Keyword(6),
    ColorKind::Keyword(7),
    ColorKind::Keyword(8),
    ColorKind::Keyword(9),
];

fn kind_index(kind: ColorKind) -> usize {
    match kind {
        ColorKind::Text => 0,
        ColorKind::Comment => 1,
        ColorKind::SingleQuote => 2,
        ColorKind::DoubleQuote => 3,
        ColorKind::Heredoc => 4,
        ColorKind::Url => 5,
        ColorKind::Number => 6,
        ColorKind::Keyword(slot) => 7 + usize::from(slot).min(KEYWORD_SLOTS - 1),
    }
}

/// The coloring rules of a document type.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorRules {
    /// The line comment delimiters that are set; empty when the comment color is off.
    pub(crate) line_comments: Vec<LineComment>,
    /// A line comment has to start at a given column (COBOL): the scan counts UTF-16 units then.
    pub(crate) line_comment_columns: bool,
    /// The block comments that are on.
    pub(crate) blocks: [Option<BlockComment>; BLOCK_COMMENT_COUNT],
    /// `m_ColorInfoArr[COLORIDX_SSTRING].m_bDisp`.
    pub(crate) single_quote: bool,
    /// `m_ColorInfoArr[COLORIDX_WSTRING].m_bDisp`.
    pub(crate) double_quote: bool,
    /// `m_nStringType`, one of `string_type`.
    pub(crate) string_type: i32,
    /// `m_bStringLineOnly`.
    pub(crate) string_line_only: bool,
    /// `m_bStringEndLine`.
    pub(crate) string_end_line: bool,
    /// Here-documents are on (`HDC` shown, and the type is PHP's).
    pub(crate) heredoc: bool,
    pub(crate) url: bool,
    pub(crate) number: bool,
    /// The keyword sets in use, with the slot (the color `KW1` to `KWA`) of each: a slot whose
    /// color is off or whose set is `-1` or does not exist is left out.
    pub(crate) keywords: Vec<(u8, PreparedSet)>,
    entries: [ColorEntry; KINDS.len()],
}

impl ColorRules {
    /// The rules of `t`, which uses `keywords` for its keyword sets.
    pub fn from_type(t: &TypeSettings, keywords: &KeywordSets) -> Self {
        let colors = &t.colors;
        let disp = |name: &str| colors.get(name).is_some_and(|e| e.disp);
        let comment = disp("CMT");
        let line_comments: Vec<LineComment> = if comment {
            t.line_comments
                .iter()
                .filter(|c| !c.delimiter.is_empty())
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        let line_comment_columns = line_comments.iter().any(|c| c.column.is_some());
        let blocks = std::array::from_fn(|i| {
            let block = &t.block_comments[i];
            (comment && block.is_active()).then(|| block.clone())
        });
        let keywords = (0..KEYWORD_SLOTS)
            .filter_map(|slot| {
                let name = ColorKind::Keyword(slot as u8).entry_name();
                if !disp(name) {
                    return None;
                }
                let set = keywords.get(t.keyword_set_idx[slot])?;
                Some((slot as u8, PreparedSet::new(set)))
            })
            .collect();
        let entries = std::array::from_fn(|i| entry_of(colors, KINDS[i]));
        Self {
            line_comments,
            line_comment_columns,
            blocks,
            single_quote: disp("SQT"),
            double_quote: disp("WQT"),
            string_type: if (0..=string_type::PYTHON).contains(&t.string_type) {
                t.string_type
            } else {
                string_type::CPP
            },
            string_line_only: t.string_line_only,
            string_end_line: t.string_end_line,
            heredoc: disp("HDC") && t.heredoc_type == HEREDOC_PHP,
            url: disp("URL"),
            number: disp("NUM"),
            keywords,
            entries,
        }
    }

    /// Rules that color nothing: every strategy is off.
    pub fn none() -> Self {
        let colors = ColorSet::default();
        Self {
            line_comments: Vec::new(),
            line_comment_columns: false,
            blocks: [None, None],
            single_quote: false,
            double_quote: false,
            string_type: string_type::CPP,
            string_line_only: false,
            string_end_line: false,
            heredoc: false,
            url: false,
            number: false,
            keywords: Vec::new(),
            entries: std::array::from_fn(|i| entry_of(&colors, KINDS[i])),
        }
    }

    /// The color entry of a kind of run.
    pub fn entry(&self, kind: ColorKind) -> &ColorEntry {
        &self.entries[kind_index(kind)]
    }

    /// Some strategy can carry a state over the end of a line (comments, strings, here-documents;
    /// `HasRangeBasedColorStrategies`). Without one, every line starts in the normal state.
    pub fn has_range_based(&self) -> bool {
        !self.line_comments.is_empty()
            || self.blocks.iter().any(Option::is_some)
            || self.single_quote
            || self.double_quote
            || self.heredoc
    }

    /// No strategy colors anything (`IsSkipBeforeLayout`, for the rules alone): the text is drawn
    /// in `TXT` throughout.
    pub fn colors_nothing(&self) -> bool {
        !self.has_range_based() && !self.url && !self.number && self.keywords.is_empty()
    }
}

fn entry_of(colors: &ColorSet, kind: ColorKind) -> ColorEntry {
    *colors
        .get(kind.entry_name())
        .expect("a color entry of Sakura's table")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ume_config::settings::DocumentTypes;

    fn rules_of(name: &str) -> ColorRules {
        let types = DocumentTypes::builtin();
        let index = types.index_of_name(name).expect("a built-in type");
        ColorRules::from_type(types.get(index).unwrap(), types.keywords())
    }

    #[test]
    fn the_basic_type_colors_only_urls() {
        // No comment delimiters, no string colors, no numbers and no keyword set; URLs are on.
        let basic = rules_of("基本");
        assert!(!basic.has_range_based());
        assert!(basic.keywords.is_empty());
        assert!(!basic.number);
        assert!(basic.url);
        assert!(!basic.colors_nothing());
    }

    #[test]
    fn the_c_type_has_comments_quotes_numbers_and_keywords() {
        let cpp = rules_of("C/C++");
        assert_eq!(cpp.line_comments.len(), 1);
        assert_eq!(cpp.line_comments[0].delimiter, "//");
        assert!(cpp.blocks[0].is_some());
        assert!(cpp.single_quote && cpp.double_quote);
        assert!(cpp.url);
        assert!(!cpp.keywords.is_empty());
        assert!(cpp.has_range_based());
        assert!(!cpp.colors_nothing());
    }

    #[test]
    fn a_strategy_is_off_when_its_color_is_off() {
        let types = DocumentTypes::builtin();
        let index = types.index_of_name("C/C++").unwrap();
        let mut t = types.get(index).unwrap().clone();
        t.colors.get_mut("CMT").unwrap().disp = false;
        t.colors.get_mut("KW1").unwrap().disp = false;
        let rules = ColorRules::from_type(&t, types.keywords());
        assert!(rules.line_comments.is_empty());
        assert!(rules.blocks.iter().all(Option::is_none));
        assert!(rules.keywords.iter().all(|(slot, _)| *slot != 0));
        // Strings stay on.
        assert!(rules.double_quote);
    }

    #[test]
    fn no_rules_color_nothing() {
        assert!(ColorRules::none().colors_nothing());
        assert!(!ColorRules::none().has_range_based());
    }
}
