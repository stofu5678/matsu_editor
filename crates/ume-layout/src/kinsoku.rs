// Derived from Sakura Editor (zlib License):
//   sakura_core/types/CType.cpp, sakura_core/types/CType_Text.cpp (the default character lists)
//   sakura_core/doc/layout/CLayoutMgr.cpp (SetLayoutInfo: how the lists are built)
// Modified: rewritten in Rust.
//
// Copyright (C) 2018-2022, Sakura Editor Organization
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

//! Kinsoku (line breaking prohibition) settings.

/// A set of characters, searched by bisection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharSet(Vec<char>);

impl CharSet {
    /// Makes a set of the characters of `chars`. Duplicates are dropped
    /// (`push_back_unique` in `CLayoutMgr::SetLayoutInfo`).
    pub fn new(chars: &str) -> Self {
        let mut v: Vec<char> = chars.chars().collect();
        v.sort_unstable();
        v.dedup();
        Self(v)
    }

    pub fn contains(&self, c: char) -> bool {
        self.0.binary_search(&c).is_ok()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<&str> for CharSet {
    fn from(chars: &str) -> Self {
        Self::new(chars)
    }
}

/// Characters that may not start a line (行頭禁則), from `m_szKinsokuHead` of the "text" type
/// (`CType_Text.cpp`, `\xa2` is U+00A2).
pub const DEFAULT_HEAD: &str =
    "!%),.:;?]}¢°’”‰′″℃、。々〉》」』】〕゛゜ゝゞ・ヽヾ！％），．：；？］｝｡｣､･ﾞﾟ￠";

/// Characters that may not end a line (行末禁則), from `m_szKinsokuTail` of the "text" type
/// (`CType_Text.cpp`, `\\` is a backslash, `\xa3` is U+00A3, `\xa5` is U+00A5).
pub const DEFAULT_TAIL: &str = "$([\\{£¥‘“〈《「『【〔＄（［｛｢￡￥";

/// Punctuation that may hang over the right edge (句読点のぶら下げ), from `m_szKinsokuKuto`
/// (`CType.cpp`, for all types).
pub const DEFAULT_KUTO: &str = "、。，．､｡,.";

/// Sakura's kinsoku settings of a document type: three switches, a "hang the line ending"
/// switch, and the three character lists.
///
/// Sakura switches all of them off by default (`CType.cpp` and `CType_Text.cpp`:
/// `m_bKinsokuHead`, `m_bKinsokuTail`, `m_bKinsokuKuto`, `m_bKinsokuRet` are `false`), and keeps
/// the lists of the "text" type ready. [`Kinsoku::default`] is the same.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kinsoku {
    /// 行頭禁則: a character of [`Kinsoku::head_chars`] does not start a line.
    pub head: bool,
    /// 行末禁則: a character of [`Kinsoku::tail_chars`] does not end a line.
    pub tail: bool,
    /// 句読点のぶら下げ: a character of [`Kinsoku::kuto_chars`] may go over the right edge.
    pub kuto: bool,
    /// 改行文字をぶら下げる: a line ending may go over the right edge.
    pub hang_eol: bool,
    pub head_chars: CharSet,
    pub tail_chars: CharSet,
    pub kuto_chars: CharSet,
}

impl Default for Kinsoku {
    fn default() -> Self {
        Self {
            head: false,
            tail: false,
            kuto: false,
            hang_eol: false,
            head_chars: DEFAULT_HEAD.into(),
            tail_chars: DEFAULT_TAIL.into(),
            kuto_chars: DEFAULT_KUTO.into(),
        }
    }
}

impl Kinsoku {
    /// All switches on, with the default lists.
    pub fn all_on() -> Self {
        Self {
            head: true,
            tail: true,
            kuto: true,
            hang_eol: true,
            ..Self::default()
        }
    }

    /// Whether any rule is on (`bKinsokuEnabled` in `_MakeOneLine`, with the word wrap).
    pub(crate) fn any(&self) -> bool {
        self.head || self.tail || self.kuto
    }

    /// `CLayoutMgr::IsKinsokuHead`. Sakura leaves the hanging punctuation out of the list when
    /// that rule is on (`SetLayoutInfo`: "except punctuation").
    pub(crate) fn is_head(&self, c: char) -> bool {
        self.head_chars.contains(c) && !self.is_kuto(c)
    }

    /// `CLayoutMgr::IsKinsokuTail`.
    pub(crate) fn is_tail(&self, c: char) -> bool {
        self.tail_chars.contains(c)
    }

    /// `CLayoutMgr::IsKinsokuKuto`: the list is empty when the rule is off (`SetLayoutInfo`).
    pub(crate) fn is_kuto(&self, c: char) -> bool {
        self.kuto && self.kuto_chars.contains(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_lists_match_sakura() {
        let k = Kinsoku::default();
        assert!(!k.any());
        for c in ['、', '。', '」'] {
            assert!(k.head_chars.contains(c), "{c}");
        }
        assert!(!k.head_chars.contains('ー'));
        assert!(k.tail_chars.contains('「') && k.tail_chars.contains('\\'));
        assert!(k.kuto_chars.contains(',') && k.kuto_chars.contains('､'));
    }

    #[test]
    fn hanging_punctuation_leaves_the_head_list() {
        // Sakura builds the head list without the hanging punctuation, only when that rule is on.
        let off = Kinsoku::default();
        assert!(off.is_head('、'));
        let on = Kinsoku { kuto: true, ..off };
        assert!(!on.is_head('、'));
        assert!(on.is_head('」'));
    }
}
