// Derived from Sakura Editor (zlib License):
//   sakura_core/charset/charcode.cpp (WCODE::IsHankaku, CCharWidthCache::QueryPixelWidth)
//   sakura_core/mem/CNativeW.cpp (CNativeW::GetKetaOfChar, GetHabaOfChar)
// Modified: Sakura asks the font for the width of a character; this version decides it from
// Unicode data alone, so that every OS gives the same layout. It is a new rule, not a port.
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

//! The width of a character in columns, without a font.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use unicode_width::UnicodeWidthChar;

use crate::x::X;

/// The first of the stand-ins for invalid bytes (`INVALID_BYTE_BASE` in the codec contract of
/// `matsu-core`). Byte `b` decodes to `U+10FF00 + b`.
pub const INVALID_BYTE_BASE: u32 = 0x10FF00;

/// The Latin-1 characters that JIS X 0208 has and that Japanese fonts draw full-width, such as
/// MS Gothic: `§ ¨ ° ± ´ ¶ × ÷`. All are East Asian Ambiguous.
const JIS_LATIN1: [char; 8] = ['§', '¨', '°', '±', '´', '¶', '×', '÷'];

/// How wide a character is, in columns: 1 (half-width) or 2 (full-width).
///
/// Sakura measures each character with the font and calls it half-width when it is no wider
/// than the average half-width letter (`CalcHankakuByFont`, `charcode.cpp`), so the result
/// depends on the font and the OS. This rule gives what Sakura shows with its default Japanese
/// fonts (MS Gothic and the like) in most cases:
///
/// | characters | columns | Sakura |
/// |---|---|---|
/// | U+0000..=U+007E, U+007F..=U+00A0, U+FF61..=U+FF9F | 1 | the same, fixed in `IsHankaku` |
/// | U+00A1..=U+02FF | 1, except `§ ¨ ° ± ´ ¶ × ÷` which are 2 | by the font |
/// | U+E000..=U+E8FF (private use) | 2 | the same, as U+E000 (`IsHankaku`) |
/// | East Asian Wide and Fullwidth | 2 | by the font |
/// | Greek and Cyrillic letters of JIS X 0208 | 2 (or 1, as below) | by the font |
/// | East Asian Ambiguous elsewhere | 2 (or 1, see [`WidthRule::ambiguous_wide`]) | by the font |
/// | combining marks, zero-width characters | 1 | by the font |
/// | other BMP characters | 1 | by the font |
/// | outside the BMP (surrogate pairs) | 2 | the same, `GetKetaOfChar` |
/// | stand-ins for invalid bytes | [`WidthRule::invalid_byte`] (2) | 1 in `GetKetaOfChar` |
///
/// Where it differs from Sakura is listed in the crate documentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidthRule {
    /// Draws East Asian Ambiguous characters (Greek, Cyrillic, box drawing, `○ ● ■ ※ ①` and
    /// the like) in two columns, as Japanese fonts do. The default is `true`.
    pub ambiguous_wide: bool,
    /// The columns of a stand-in for an invalid byte (U+10FF00..=U+10FFFF), which the UI draws
    /// as a box with two hex digits. The default is 2.
    pub invalid_byte: u32,
}

impl Default for WidthRule {
    fn default() -> Self {
        Self {
            ambiguous_wide: true,
            invalid_byte: 2,
        }
    }
}

impl WidthRule {
    /// The columns of `c`, 1 or 2 (a tab is not handled here, see `LayoutConfig::advance`).
    pub fn columns(&self, c: char) -> u32 {
        let cp = c as u32;
        // charcode.cpp, IsHankaku: "wc<=0x007E", and "0x7F ~ 0xA0 are half-width too".
        if cp <= 0xA0 {
            return 1;
        }
        // charcode.cpp, IsHankaku: half-width katakana, FF61..=FF9F.
        if (0xFF61..=0xFF9F).contains(&cp) {
            return 1;
        }
        if cp > 0xFFFF {
            // CNativeW.cpp, GetKetaOfChar: "outside the BMP is full-width" (2008/7/5 Uchi).
            return if cp >= INVALID_BYTE_BASE {
                self.invalid_byte
            } else {
                2
            };
        }
        // charcode.cpp, IsHankaku: private use, E000..=E8FF, is measured as U+E000.
        if (0xE000..=0xE8FF).contains(&cp) {
            return 2;
        }
        if cp <= 0x02FF {
            return if self.ambiguous_wide && JIS_LATIN1.contains(&c) {
                2
            } else {
                1
            };
        }
        // unicode-width leaves Greek and Cyrillic letters narrow even in its CJK mode, but the
        // JIS X 0208 ones (rows 6 and 7) are full-width in Japanese fonts. They are East Asian
        // Ambiguous in UAX #11.
        if self.ambiguous_wide
            && matches!(cp,
                0x0391..=0x03A1 | 0x03A3..=0x03A9 | 0x03B1..=0x03C1 | 0x03C3..=0x03C9
                | 0x0401 | 0x0410..=0x044F | 0x0451)
        {
            return 2;
        }
        let width = if self.ambiguous_wide {
            c.width_cjk()
        } else {
            c.width()
        };
        // Zero-width marks and controls get a cell of their own: Sakura draws each UTF-16 unit
        // at its own position and never joins them.
        match width {
            Some(2) => 2,
            _ => 1,
        }
    }
}

/// How wide a character is in [`X`], and the one place where columns and `X` are converted
/// (ADR-0034, "Widths").
///
/// There are two forms:
///
/// - [`Widths::Grid`]: the [`WidthRule`] gives 1 or 2 columns and a column is
///   [`per_column`](Self::per_column) `X`. It does not look at the font. This is the form for
///   monospaced fonts (ADR-0034, decided 3), and it gives what the layout gave before `X`.
/// - [`Widths::Table`]: the width of each character comes from a [`WidthTable`] that the UI fills
///   by measuring the font (stage 4). There is no limit known for the widest character, so
///   [`Widths::widest_char`] is `None` for it and a caller of that must not take a shortcut.
///
/// `per_column` includes the character spacing (ADR-0034, case C: 64 plus the spacing).
#[derive(Clone, Debug)]
pub enum Widths {
    /// Every character is 1 or 2 columns by Unicode ([`WidthRule`]); a column is `per_column` `X`.
    Grid {
        rule: WidthRule,
        /// `X` in one column. 0 counts as 1.
        per_column: u32,
    },
    /// Every character has the width the font gives it.
    Table(WidthTable),
}

/// Two `Widths` are equal when the layout they give is the same: grids when their rule and
/// column are, tables when they are the same generation ([`WidthTable::generation`]). A table
/// is never compared by what it has measured so far, which changes as characters are used.
impl PartialEq for Widths {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Grid { rule, per_column },
                Self::Grid {
                    rule: other_rule,
                    per_column: other_per,
                },
            ) => rule == other_rule && (*per_column).max(1) == (*other_per).max(1),
            (Self::Table(a), Self::Table(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Widths {}

impl Default for Widths {
    fn default() -> Self {
        Self::grid(WidthRule::default())
    }
}

impl Widths {
    /// The grid with one `X` for a column, which is what the layout counted before `X`.
    pub fn grid(rule: WidthRule) -> Self {
        Self::grid_scaled(rule, 1)
    }

    /// The grid with `per_column` `X` for a column.
    pub fn grid_scaled(rule: WidthRule, per_column: u32) -> Self {
        Self::Grid {
            rule,
            per_column: per_column.max(1),
        }
    }

    /// The table form: `half` `X` for the width of a half-width letter without spacing (64 in
    /// the UI), `space_units` `X` of character spacing for each column of a character, and
    /// `measure` giving the advance of a character in `X` (without spacing). See [`WidthTable`].
    pub fn table(
        half: u32,
        space_units: u32,
        measure: impl Fn(char) -> u32 + Send + Sync + 'static,
    ) -> Self {
        Self::Table(WidthTable::new(half, space_units, measure))
    }

    /// The same form with `per_column` `X` for a column. A table has its column from its own
    /// half-width and spacing, so it is returned as it is.
    pub fn with_per_column(self, per_column: u32) -> Self {
        match self {
            Self::Grid { rule, .. } => Self::grid_scaled(rule, per_column),
            table @ Self::Table(_) => table,
        }
    }

    /// The rule of a grid; the default one for a table, which does not use a rule.
    pub fn rule(&self) -> WidthRule {
        match self {
            Self::Grid { rule, .. } => *rule,
            Self::Table(_) => WidthRule::default(),
        }
    }

    /// Whether the widths come from a table (a font that is not monospaced).
    pub fn is_table(&self) -> bool {
        matches!(self, Self::Table(_))
    }

    /// `X` in one column.
    pub fn per_column(&self) -> u32 {
        match self {
            Self::Grid { per_column, .. } => (*per_column).max(1),
            Self::Table(table) => table.per_column(),
        }
    }

    /// `columns` columns as `X`.
    pub fn to_x(&self, columns: u32) -> X {
        X::from_raw(columns.saturating_mul(self.per_column()))
    }

    /// The whole columns in `x`, the rest dropped (the status bar's column is the caret's `X`
    /// divided by the width of a column and rounded down: `CCaret.cpp`, ADR-0034).
    pub fn to_columns(&self, x: X) -> u32 {
        x.raw() / self.per_column()
    }

    /// The columns that hold `x`, a part of a column counted as a whole one: what a scroll range
    /// needs to reach the end of the text.
    pub fn to_columns_ceil(&self, x: X) -> u32 {
        x.raw().div_ceil(self.per_column())
    }

    /// The width of `c` in `X`, a tab not included (see `LayoutConfig::advance`).
    pub fn width_of(&self, c: char) -> X {
        match self {
            Self::Grid { rule, .. } => self.to_x(rule.columns(c)),
            Self::Table(table) => table.width_of(c),
        }
    }

    /// The columns of `c` that the character spacing is counted for: 1 or 2
    /// (`CNativeW::GetKetaOfChar`). On a grid that is the width in whole columns; in a table it
    /// is 1 for a character no wider than a half-width letter and 2 for a wider one.
    pub fn columns(&self, c: char) -> u32 {
        match self {
            Self::Grid { .. } => self.to_columns(self.width_of(c)),
            Self::Table(table) => table.columns_of(c),
        }
    }

    /// The most `X` one character other than a tab can take, when there is a limit that holds
    /// for every character of every text; `None` when it cannot be known without measuring
    /// (the table form). The layout of a document takes a shortcut only with `Some`.
    pub fn widest_char(&self) -> Option<X> {
        match self {
            Self::Grid { rule, .. } => Some(self.to_x(rule.invalid_byte.max(2))),
            Self::Table(_) => None,
        }
    }

    /// The width of the ASCII `bytes`, one after the other: a word of the word wrap
    /// (`_GetKeywordLength` adds up the layout widths of its characters).
    pub fn ascii_width(&self, bytes: &[u8]) -> X {
        match self {
            Self::Grid { .. } => self.to_x(u32::try_from(bytes.len()).unwrap_or(u32::MAX)),
            Self::Table(table) => bytes.iter().map(|&b| table.width_of(char::from(b))).sum(),
        }
    }
}

/// The widths of a font's characters in `X`, filled in as characters are used (ADR-0034,
/// "Widths", the table form). Like Sakura's cache of pixel widths (`m_nCharPxWidthCache` of
/// `CCharWidthCache`, `sakura_core/charset/charcode.cpp`, 0 = not measured yet), a character is
/// measured the first time the layout asks for it.
///
/// The measuring function is the UI's: it gives the advance of one character in `X` without the
/// spacing. The table adds what Sakura adds per character
/// (`CLayoutMgr::GetLayoutXOfChar`, `sakura_core/doc/layout/CLayoutMgr.h`): the spacing times the
/// columns of the character, 1 when it is no wider than a half-width letter and 2 when it is
/// wider (`CNativeW::GetKetaOfChar`; outside the BMP always 2). A width is at least 1
/// (`CCharWidthCache::QueryPixelWidth`: `std::max(1, size.cx)`).
///
/// The BMP is held in pages of 256 characters that exist once a character of them is used (a
/// page is 512 bytes, all of them 128 KiB at most); the characters outside it are in a small
/// map. The table is shared by `Arc`: a clone is the same table, and two tables are equal when
/// they are the same generation. A new table (a new font, size or spacing) is a new generation.
#[derive(Clone)]
pub struct WidthTable(Arc<TableData>);

type Page = [AtomicU16; PAGE];

struct TableData {
    generation: u64,
    /// The `X` of a half-width letter without spacing.
    half: u32,
    space_units: u32,
    measure: Box<dyn Fn(char) -> u32 + Send + Sync>,
    /// The width with the spacing of each BMP character, 0 when it is not measured yet.
    pages: Box<[OnceLock<Box<Page>>]>,
    astral: Mutex<HashMap<char, u32>>,
}

/// Characters in a page of the table.
const PAGE: usize = 256;
/// The pages of the BMP.
const PAGES: usize = 0x10000 / PAGE;

/// The numbers of the generations: each new table takes the next one.
static GENERATIONS: AtomicU64 = AtomicU64::new(1);

impl WidthTable {
    /// An empty table. `half` is the `X` of a half-width letter without spacing (at least 1),
    /// `space_units` the spacing of a column, and `measure` the advance of a character in `X`
    /// without spacing (called once for each character used, from any thread).
    pub fn new(
        half: u32,
        space_units: u32,
        measure: impl Fn(char) -> u32 + Send + Sync + 'static,
    ) -> Self {
        Self(Arc::new(TableData {
            generation: GENERATIONS.fetch_add(1, Ordering::Relaxed),
            half: half.max(1),
            space_units,
            measure: Box::new(measure),
            pages: (0..PAGES).map(|_| OnceLock::new()).collect(),
            astral: Mutex::new(HashMap::new()),
        }))
    }

    /// The number that tells this table from another: the tables of two fonts (or of one font at
    /// two sizes, or with two spacings) have different generations.
    pub fn generation(&self) -> u64 {
        self.0.generation
    }

    /// `X` in one column: a half-width letter and the spacing.
    pub fn per_column(&self) -> u32 {
        self.0.half.saturating_add(self.0.space_units)
    }

    /// The width of `c` with its spacing, measuring it if that was not done yet.
    pub fn width_of(&self, c: char) -> X {
        let data = &*self.0;
        let code = c as usize;
        if code < PAGES * PAGE {
            let page = data.pages[code / PAGE]
                .get_or_init(|| Box::new(std::array::from_fn(|_| AtomicU16::new(0))));
            let slot = &page[code % PAGE];
            let known = slot.load(Ordering::Relaxed);
            if known != 0 {
                return X::from_raw(u32::from(known));
            }
            // 65535 `X` is a thousand columns: a character wider than that is cut to it.
            let width = data.with_spacing(c).min(u32::from(u16::MAX));
            slot.store(width as u16, Ordering::Relaxed);
            return X::from_raw(width);
        }
        let mut astral = data.astral.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(&known) = astral.get(&c) {
            return X::from_raw(known);
        }
        let width = data.with_spacing(c);
        astral.insert(c, width);
        X::from_raw(width)
    }

    /// The columns the spacing is counted for: 1 or 2.
    pub fn columns_of(&self, c: char) -> u32 {
        if (c as u32) > 0xFFFF || self.width_of(c).raw() > self.per_column() {
            2
        } else {
            1
        }
    }

    /// How many characters of the BMP have been measured.
    pub fn measured_in_bmp(&self) -> usize {
        self.0
            .pages
            .iter()
            .filter_map(OnceLock::get)
            .map(|page| {
                page.iter()
                    .filter(|w| w.load(Ordering::Relaxed) != 0)
                    .count()
            })
            .sum()
    }

    /// How many pages of the BMP exist.
    pub fn pages(&self) -> usize {
        self.0.pages.iter().filter(|p| p.get().is_some()).count()
    }
}

impl TableData {
    /// The width of `c` with the spacing of its columns.
    fn with_spacing(&self, c: char) -> u32 {
        let width = (self.measure)(c).max(1);
        let columns = if (c as u32) > 0xFFFF || width > self.half {
            2
        } else {
            1
        };
        width.saturating_add(self.space_units.saturating_mul(columns))
    }
}

impl PartialEq for WidthTable {
    fn eq(&self, other: &Self) -> bool {
        self.0.generation == other.0.generation
    }
}

impl Eq for WidthTable {}

impl std::fmt::Debug for WidthTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidthTable")
            .field("generation", &self.0.generation)
            .field("half", &self.0.half)
            .field("space_units", &self.0.space_units)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_and_latin1_controls_are_half_width() {
        let rule = WidthRule::default();
        // sakura_core/charset/charcode.cpp, IsHankaku: wc<=0x007E and 0x7F..=0xA0.
        for c in ['a', ' ', '~', '\t', '\0', '\u{7F}', '\u{80}', '\u{A0}'] {
            assert_eq!(rule.columns(c), 1, "{c:?}");
        }
        // IsHankaku: half-width katakana.
        assert_eq!(rule.columns('ｱ'), 1);
        assert_eq!(rule.columns('ﾟ'), 1);
    }

    #[test]
    fn japanese_is_full_width() {
        let rule = WidthRule::default();
        for c in [
            'あ', 'ア', '漢', '，', '　', 'Ａ', '한', '\u{E000}', '\u{E8FF}',
        ] {
            assert_eq!(rule.columns(c), 2, "{c:?}");
        }
    }

    #[test]
    fn non_bmp_is_full_width() {
        let rule = WidthRule::default();
        // CNativeW.cpp, GetKetaOfChar: surrogate pairs are 2.
        for c in ['😀', '𠮷', '\u{10000}', '\u{1F1EF}'] {
            assert_eq!(rule.columns(c), 2, "{c:?}");
        }
    }

    #[test]
    fn invalid_byte_stand_ins_are_configurable() {
        let rule = WidthRule::default();
        assert_eq!(rule.columns('\u{10FF00}'), 2);
        assert_eq!(rule.columns('\u{10FFFF}'), 2);
        let one = WidthRule {
            invalid_byte: 1,
            ..rule
        };
        assert_eq!(one.columns('\u{10FF80}'), 1);
        // The char just below the range is an ordinary astral char.
        assert_eq!(one.columns('\u{10FEFF}'), 2);
    }

    #[test]
    fn ambiguous_follows_the_setting() {
        let wide = WidthRule::default();
        let narrow = WidthRule {
            ambiguous_wide: false,
            ..wide
        };
        // Greek, Cyrillic, box drawing, geometric shapes, curly quotes, and the JIS symbols of
        // Latin-1 are drawn full-width by Japanese fonts.
        for c in ['α', 'Я', '─', '○', '※', '“', '°', '×', '①'] {
            assert_eq!(wide.columns(c), 2, "{c:?}");
            assert_eq!(narrow.columns(c), 1, "{c:?}");
        }
        // Latin letters with accents are half-width in Japanese fonts too, ambiguous or not.
        for c in ['é', 'ü', 'ñ', '¡', '¿', '½', '·'] {
            assert_eq!(wide.columns(c), 1, "{c:?}");
        }
    }

    #[test]
    fn marks_and_zero_width_take_a_cell() {
        let rule = WidthRule::default();
        for c in ['\u{0301}', '\u{200B}', '\u{200D}', '\u{FE0F}', '\u{3099}'] {
            assert_eq!(rule.columns(c), 1, "{c:?}");
        }
    }
}
