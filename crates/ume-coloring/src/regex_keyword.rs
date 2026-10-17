// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/env/CRegexKeyword.cpp (RegexKeyCompile, RegexKeyLineStart, RegexIsKeyword,
//     RegexKeyCheckSyntax)
//   sakura_core/view/colors/CColor_RegexKeyword.cpp
//
// Original copyright notices of those files:
//   Copyright (C) 2001, MIK
//   Copyright (C) 2002, YAZAKI
//   Copyright (C) 2003, Moca
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

//! The regular expression keywords of a document type (`/pattern/k`).
//!
//! A type has up to [`MAX_REGEX_KEYWORDS`] of them, each a pattern in Perl's `m//` form and the
//! color it is drawn in. They are compiled once, when the rules of the type are made
//! (`CRegexKeyword::RegexKeyCompile`); a keyword that does not have the form, or does not compile,
//! is left out, and so is one whose color is one of `RK1` to `RKA` and is not shown.
//!
//! While a line is scanned, [`LineSearch`] answers "does a keyword begin at this position"
//! (`RegexIsKeyword`): each keyword remembers where it matches next, so a line is searched once
//! per match and not once per character. A match with no width counts as no match and switches
//! the keyword off for the rest of the line, and a keyword that must start at the head of the
//! line (`/^...`) is tried at the head only.
//!
//! The pattern is compiled as `ume-regex` does everything (bregonig's syntax); the modifier `i` of
//! the closing part (`/k` or `/ki`) ignores case, and `k`, which tells bregonig the text is
//! Japanese, means nothing here (the text is Unicode).

use std::cell::RefCell;
use std::sync::Arc;

use ume_config::settings::TypeSettings;
use ume_config::settings::type_rules::{MAX_REGEX_KEYWORDS, RegexKeyword};
use ume_regex::{Onigmo, Options, Regex, RegexEngine};

/// The opening and closing parts that a pattern may have (`RegexKeyCheckSyntax`).
const FRAMES: [(&str, &str); 6] = [
    ("/", "/k"),
    ("m/", "/k"),
    ("m#", "#k"),
    ("/", "/ki"),
    ("m/", "/ki"),
    ("m#", "#ki"),
];

/// The openings of a pattern that must start at the head of the line (`RK_HEAD_STR1` to `3`).
const HEAD_OPENINGS: [&str; 3] = ["/^", "m#^", "m/^"];

/// The names of the colors that belong to the regular expression keywords (`COLORIDX_REGEX1` to
/// `COLORIDX_REGEX10`): they are drawn only when their `disp` is on.
const REGEX_COLORS: [&str; 10] = [
    "RK1", "RK2", "RK3", "RK4", "RK5", "RK6", "RK7", "RK8", "RK9", "RKA",
];

/// One compiled keyword.
#[derive(Clone)]
struct Item {
    regex: Arc<dyn Regex>,
    /// The pattern has to start at the head of the line.
    head: bool,
    /// The index of the keyword in the type's list: the color of a run it makes.
    index: u8,
}

/// The compiled regular expression keywords of a type.
#[derive(Clone, Default)]
pub(crate) struct RegexKeywords {
    items: Vec<Item>,
    /// What they were made from (color name and pattern), to compare rules; also the keywords
    /// that were left out, so that `index` counts the list of the type.
    source: Vec<(String, String)>,
}

impl std::fmt::Debug for RegexKeywords {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegexKeywords")
            .field("source", &self.source)
            .finish()
    }
}

impl PartialEq for RegexKeywords {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

/// The inside of a pattern in the form `/.../k`, and whether it ignores case: `None` when it does
/// not have the form (`RegexKeyCheckSyntax`: it has something between the two parts).
pub(crate) fn split_pattern(pattern: &str) -> Option<(&str, bool)> {
    FRAMES.iter().find_map(|(open, close)| {
        let inside = pattern.strip_prefix(open)?.strip_suffix(close)?;
        (!inside.is_empty()).then(|| (inside, close.ends_with('i')))
    })
}

impl RegexKeywords {
    /// The keywords of `t`; none when the type does not use them.
    pub(crate) fn from_type(t: &TypeSettings) -> Self {
        let mut keywords = Self::default();
        if !t.use_regex_keyword {
            return keywords;
        }
        // The list ends at the first empty pattern (`RegexKeyCompile` counts to it).
        let list = t
            .regex_keywords
            .iter()
            .take(MAX_REGEX_KEYWORDS)
            .take_while(|k| !k.pattern.is_empty());
        for (index, keyword) in list.enumerate() {
            keywords
                .source
                .push((keyword.color.clone(), keyword.pattern.clone()));
            if let Some(item) = compile(t, index, keyword) {
                keywords.items.push(item);
            }
        }
        keywords
    }

    /// No keyword searches anything.
    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The color entry name of the keyword `index` (its position in the type's list).
    pub(crate) fn color_name(&self, index: usize) -> Option<&str> {
        self.source.get(index).map(|(color, _)| color.as_str())
    }

    /// How many keywords the type has (also those that are left out).
    pub(crate) fn len(&self) -> usize {
        self.source.len()
    }

    /// The state of the search through one line.
    pub(crate) fn line_search(&self) -> LineSearch<'_> {
        LineSearch {
            keywords: self,
            states: RefCell::new(vec![KeyState::default(); self.items.len()]),
        }
    }
}

fn compile(t: &TypeSettings, index: usize, keyword: &RegexKeyword) -> Option<Item> {
    let (inside, ignore_case) = split_pattern(&keyword.pattern)?;
    // A color of the regular expression keywords that is not shown does not search.
    if REGEX_COLORS.contains(&keyword.color.as_str())
        && !t.colors.get(&keyword.color).is_some_and(|e| e.disp)
    {
        return None;
    }
    let regex = Onigmo
        .compile(
            inside,
            Options {
                ignore_case,
                ..Options::default()
            },
        )
        .ok()?;
    Some(Item {
        regex: Arc::from(regex),
        head: HEAD_OPENINGS
            .iter()
            .any(|opening| keyword.pattern.starts_with(opening)),
        index: u8::try_from(index).ok()?,
    })
}

/// What one keyword knows of the line being scanned.
#[derive(Clone, Copy, Default)]
struct KeyState {
    /// Where the next match is, and its length, once searched.
    found: Option<(usize, usize)>,
    /// The keyword has no more matches in this line.
    done: bool,
}

/// The search through one line, from its start (`RegexKeyLineStart` makes the states).
pub(crate) struct LineSearch<'a> {
    keywords: &'a RegexKeywords,
    states: RefCell<Vec<KeyState>>,
}

impl LineSearch<'_> {
    /// `RegexIsKeyword`: the keyword that matches at `pos` of `line`, with the length of the
    /// match. The keywords are tried in the order of the list; positions are asked for in
    /// increasing order.
    pub(crate) fn match_at(&self, line: &str, pos: usize) -> Option<(u8, usize)> {
        let mut states = self.states.borrow_mut();
        for (item, state) in self.keywords.items.iter().zip(states.iter_mut()) {
            if state.done {
                continue;
            }
            if let Some((start, len)) = state.found {
                if start == pos {
                    return Some((item.index, len));
                }
                if start > pos {
                    // The match is still ahead.
                    continue;
                }
            }
            // The match found before is behind us: search again from here.
            let found = item
                .regex
                .find_at(line, pos)
                .ok()
                .flatten()
                .filter(|m| m.end > m.start);
            let Some(m) = found else {
                state.done = true;
                continue;
            };
            state.found = Some((m.start, m.end - m.start));
            if m.start == pos && (!item.head || pos == 0) {
                return Some((item.index, m.end - m.start));
            }
            if item.head {
                state.done = true;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ume_config::settings::DocumentTypes;

    fn type_with(patterns: &[(&str, &str)]) -> TypeSettings {
        let types = DocumentTypes::builtin();
        let mut t = types
            .get(types.index_of_name("基本").unwrap())
            .unwrap()
            .clone();
        t.use_regex_keyword = true;
        t.colors.get_mut("RK1").unwrap().disp = true;
        t.regex_keywords = patterns
            .iter()
            .map(|(color, pattern)| RegexKeyword {
                color: (*color).to_owned(),
                pattern: (*pattern).to_owned(),
            })
            .collect();
        t
    }

    #[test]
    fn the_pattern_needs_one_of_the_forms_and_something_inside() {
        assert_eq!(split_pattern("/ab/k"), Some(("ab", false)));
        assert_eq!(split_pattern("m/ab/k"), Some(("ab", false)));
        assert_eq!(split_pattern("m#a/b#k"), Some(("a/b", false)));
        assert_eq!(split_pattern("/ab/ki"), Some(("ab", true)));
        assert_eq!(split_pattern("m#ab#ki"), Some(("ab", true)));
        // nothing inside, no closing part, the wrong closing part, other modifiers
        assert_eq!(split_pattern("//k"), None);
        assert_eq!(split_pattern("/ab/"), None);
        assert_eq!(split_pattern("/ab#k"), None);
        assert_eq!(split_pattern("/ab/g"), None);
        assert_eq!(split_pattern("ab"), None);
        assert_eq!(split_pattern(""), None);
    }

    #[test]
    fn a_type_that_does_not_use_them_has_none() {
        let mut t = type_with(&[("RK1", "/a/k")]);
        t.use_regex_keyword = false;
        assert!(RegexKeywords::from_type(&t).is_empty());
    }

    #[test]
    fn a_bad_pattern_a_bad_form_and_a_hidden_color_are_left_out() {
        let mut t = type_with(&[
            ("RK1", "/(/k"),
            ("RK1", "abc"),
            ("RK2", "/b/k"),
            ("URL", "/c/k"),
            ("RK1", "/d/k"),
        ]);
        t.colors.get_mut("RK2").unwrap().disp = false;
        let k = RegexKeywords::from_type(&t);
        // the color URL is drawn whether it is shown or not; RK2 is not shown
        assert_eq!(k.items.iter().map(|i| i.index).collect::<Vec<_>>(), [3, 4]);
        assert_eq!(k.len(), 5);
        assert_eq!(k.color_name(3), Some("URL"));
    }

    #[test]
    fn the_list_ends_at_the_first_empty_pattern() {
        let t = type_with(&[("RK1", "/a/k"), ("RK1", ""), ("RK1", "/b/k")]);
        assert_eq!(RegexKeywords::from_type(&t).len(), 1);
    }

    #[test]
    fn a_match_is_found_where_it_begins_and_not_before() {
        let k = RegexKeywords::from_type(&type_with(&[("RK1", "/b+/k")]));
        let search = k.line_search();
        assert_eq!(search.match_at("abbc", 0), None);
        assert_eq!(search.match_at("abbc", 1), Some((0, 2)));
        assert_eq!(search.match_at("abbc", 3), None);
    }

    #[test]
    fn the_modifier_i_ignores_case() {
        let k = RegexKeywords::from_type(&type_with(&[("RK1", "/ab/ki"), ("RK2", "/cd/k")]));
        let search = k.line_search();
        assert_eq!(search.match_at("AB", 0), Some((0, 2)));
        // RK2 is not shown by default: it was left out
        assert_eq!(k.line_search().match_at("CD", 0), None);
    }

    #[test]
    fn the_first_keyword_of_the_list_wins() {
        let k = RegexKeywords::from_type(&type_with(&[("RK1", "/ab/k"), ("URL", "/abc/k")]));
        assert_eq!(k.line_search().match_at("abc", 0), Some((0, 2)));
    }

    #[test]
    fn a_pattern_for_the_head_of_the_line_is_tried_there_only() {
        let k = RegexKeywords::from_type(&type_with(&[("RK1", "/^ab/k")]));
        let search = k.line_search();
        assert_eq!(search.match_at("ab ab", 0), Some((0, 2)));
        assert_eq!(search.match_at("ab ab", 3), None);
        let k = RegexKeywords::from_type(&type_with(&[("RK1", "m#^ab#k")]));
        assert_eq!(k.line_search().match_at("x ab", 2), None);
    }

    #[test]
    fn a_match_with_no_width_switches_the_keyword_off_for_the_line() {
        let k = RegexKeywords::from_type(&type_with(&[("RK1", "/x*/k")]));
        let search = k.line_search();
        // `x*` matches nothing at 0, and the keyword is off from then on
        assert_eq!(search.match_at("axx", 0), None);
        assert_eq!(search.match_at("axx", 1), None);
        // a new line starts afresh
        assert_eq!(k.line_search().match_at("xx", 0), Some((0, 2)));
    }

    #[test]
    fn a_look_behind_sees_the_text_before_the_position() {
        let k = RegexKeywords::from_type(&type_with(&[("RK1", "/(?<=\")\\w+/k")]));
        let search = k.line_search();
        assert_eq!(search.match_at("\"ab\"", 0), None);
        assert_eq!(search.match_at("\"ab\"", 1), Some((0, 2)));
    }

    #[test]
    fn japanese_text_is_matched_by_bytes_of_utf8() {
        let k = RegexKeywords::from_type(&type_with(&[("RK1", "/ア+/k")]));
        assert_eq!(k.line_search().match_at("あアア", 3), Some((0, 6)));
    }
}
