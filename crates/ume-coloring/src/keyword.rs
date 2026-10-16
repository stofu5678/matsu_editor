// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/env/CKeyWordSetMgr.cpp (SearchKeyWord2)
//   sakura_core/view/colors/CColor_KeywordSet.cpp
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2001, MIK
//   Copyright (C) 2002, genta, Moca
//   Copyright (C) 2004, Moca
//   Copyright (C) 2005, Moca, genta
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

//! Keyword sets as the coloring searches them.
//!
//! Sakura asks `CKeyWordSetMgr::SearchKeyWord2` whether a candidate is a keyword, a start of a
//! longer keyword, both, or neither, and the coloring then extends the candidate word by word to
//! find the longest keyword ([`crate::scan`]). This is the same question over a sorted list.

use ume_config::settings::keywords::KeywordSet;

/// The answer of `SearchKeyWord2`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lookup {
    /// Not a keyword and not the start of one (`-1`).
    No,
    /// Not a keyword, but the start of a longer one (`-2`): try a longer candidate.
    Prefix,
    /// A keyword, and no longer keyword starts with it (`>= 0`): this is the longest.
    Exact,
    /// A keyword, and the start of a longer one (`INT_MAX`): try a longer candidate, and keep this
    /// one if there is no better.
    ExactAndLonger,
}

/// A keyword set ready for searching.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedSet {
    case_sensitive: bool,
    /// The words, folded to lower case unless the set is case sensitive, sorted, without
    /// duplicates.
    words: Vec<String>,
    /// The longest word in bytes after folding; a longer candidate is not a keyword.
    max_bytes: usize,
}

impl PreparedSet {
    pub fn new(set: &KeywordSet) -> Self {
        let case_sensitive = set.case_sensitive;
        let mut words: Vec<String> = set
            .words()
            .iter()
            .map(|w| fold(w, case_sensitive))
            .collect();
        words.sort_unstable();
        words.dedup();
        let max_bytes = words.iter().map(String::len).max().unwrap_or(0);
        Self {
            case_sensitive,
            words,
            max_bytes,
        }
    }

    /// `SearchKeyWord2` for the candidate `word`.
    pub fn lookup(&self, word: &str) -> Lookup {
        // A candidate longer than every keyword is rejected before it is folded (`KeywordMaxLen`);
        // folding can shorten a text by a few bytes only, so this cut leaves room for that.
        if word.len() > self.max_bytes.saturating_mul(3) {
            return Lookup::No;
        }
        let folded = fold(word, self.case_sensitive);
        if folded.len() > self.max_bytes {
            return Lookup::No;
        }
        let at = self.words.partition_point(|w| w.as_str() < folded.as_str());
        let exact = self.words.get(at).is_some_and(|w| *w == folded);
        let next = if exact { at + 1 } else { at };
        let longer = self
            .words
            .get(next)
            .is_some_and(|w| w.len() > folded.len() && w.starts_with(&folded));
        match (exact, longer) {
            (true, true) => Lookup::ExactAndLonger,
            (true, false) => Lookup::Exact,
            (false, true) => Lookup::Prefix,
            (false, false) => Lookup::No,
        }
    }
}

/// Lower case for the sets that ignore case (`_wcsnicmp`).
fn fold(s: &str, case_sensitive: bool) -> String {
    if case_sensitive {
        s.to_owned()
    } else {
        s.chars().flat_map(char::to_lowercase).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(case: bool, words: &[&str]) -> PreparedSet {
        PreparedSet::new(&KeywordSet::new(
            "t",
            case,
            words.iter().map(|w| (*w).to_owned()),
        ))
    }

    #[test]
    fn exact_prefix_and_both() {
        let s = set(true, &["if", "int", "interface", "while"]);
        assert_eq!(s.lookup("if"), Lookup::Exact);
        assert_eq!(s.lookup("i"), Lookup::Prefix);
        assert_eq!(s.lookup("in"), Lookup::Prefix);
        assert_eq!(s.lookup("int"), Lookup::ExactAndLonger);
        assert_eq!(s.lookup("inte"), Lookup::Prefix);
        assert_eq!(s.lookup("interface"), Lookup::Exact);
        assert_eq!(s.lookup("interfaces"), Lookup::No);
        assert_eq!(s.lookup("for"), Lookup::No);
        assert_eq!(s.lookup(""), Lookup::Prefix);
    }

    #[test]
    fn case_rules_of_the_set() {
        let sensitive = set(true, &["Begin"]);
        assert_eq!(sensitive.lookup("Begin"), Lookup::Exact);
        assert_eq!(sensitive.lookup("begin"), Lookup::No);
        let insensitive = set(false, &["Begin", "END"]);
        assert_eq!(insensitive.lookup("begin"), Lookup::Exact);
        assert_eq!(insensitive.lookup("BEGIN"), Lookup::Exact);
        assert_eq!(insensitive.lookup("end"), Lookup::Exact);
        assert_eq!(insensitive.lookup("En"), Lookup::Prefix);
    }

    #[test]
    fn keywords_with_symbols_and_japanese() {
        let s = set(true, &["#include", "#if", "ＩＦ", "if"]);
        assert_eq!(s.lookup("#include"), Lookup::Exact);
        assert_eq!(s.lookup("#i"), Lookup::Prefix);
        assert_eq!(s.lookup("ＩＦ"), Lookup::Exact);
    }
}
