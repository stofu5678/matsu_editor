// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/view/colors/EColorIndexType.h
//   sakura_core/view/colors/CColorStrategy.h (ToColorInfoArrIndex)
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

//! The color entries that the engine puts on text.
//!
//! A run of text is drawn in the entry of the strategy that colored it. The names are the keys of
//! the `C[XXX]` entries of the type (`ume_config::settings::color`); the block comments share the
//! entry of the line comments (`ToColorInfoArrIndex` in `CColorStrategy.h`).

/// How many keyword colors there are (`MAX_KEYWORDSET_PER_TYPE`): `KW1` to `KWA`.
pub const KEYWORD_SLOTS: usize = 10;

/// The color entry of a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorKind {
    /// No strategy applies: the color `TXT`.
    Text,
    /// Line comments and block comments: `CMT` (`COLORIDX_COMMENT`, `COLORIDX_BLOCK1/2`).
    Comment,
    /// A string in single quotes: `SQT`.
    SingleQuote,
    /// A string in double quotes: `WQT`.
    DoubleQuote,
    /// A here-document: `HDC`.
    Heredoc,
    /// A URL or a mail address: `URL`.
    Url,
    /// A half-width number: `NUM`.
    Number,
    /// A keyword of the set in slot 0 to 9: `KW1` to `KWA`.
    Keyword(u8),
    /// A regular expression keyword: the position in the type's list. Its color is the entry the
    /// keyword names (any entry, not only `RK1` to `RKA`), see [`ColorRules::entry`].
    ///
    /// [`ColorRules::entry`]: crate::ColorRules::entry
    Regex(u8),
}

impl ColorKind {
    /// The name of the color entry, as in the settings file (`C[CMT]`). For a regular expression
    /// keyword this is the default color name; its real entry is [`ColorRules::entry`].
    ///
    /// [`ColorRules::entry`]: crate::ColorRules::entry
    pub fn entry_name(self) -> &'static str {
        const KEYWORDS: [&str; KEYWORD_SLOTS] = [
            "KW1", "KW2", "KW3", "KW4", "KW5", "KW6", "KW7", "KW8", "KW9", "KWA",
        ];
        match self {
            ColorKind::Text => "TXT",
            ColorKind::Comment => "CMT",
            ColorKind::SingleQuote => "SQT",
            ColorKind::DoubleQuote => "WQT",
            ColorKind::Heredoc => "HDC",
            ColorKind::Url => "URL",
            ColorKind::Number => "NUM",
            ColorKind::Keyword(slot) => KEYWORDS[usize::from(slot).min(KEYWORD_SLOTS - 1)],
            // The entry is the keyword's own; `RK1` is what a keyword without a color gets.
            ColorKind::Regex(_) => "RK1",
        }
    }
}

/// A piece of a line in one color. The offsets are bytes of the line text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorRun {
    pub start: usize,
    pub end: usize,
    pub kind: ColorKind,
}
