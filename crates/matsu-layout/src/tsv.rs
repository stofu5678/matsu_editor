// Derived from Sakura Editor (zlib License):
//   sakura_core/doc/layout/CTsvModeInfo.h, CTsvModeInfo.cpp (CalcTabLength, GetActualTabLength),
//   sakura_core/doc/layout/CLayoutMgr.h (GetActualTsvSpace),
//   sakura_core/mem/CMemoryIterator.h (scanNext)
// Modified: rewritten in Rust, measured in columns instead of pixels, reading a document one line
// at a time through `matsu_core`.
//
// Copyright (C) 2015, syat
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

//! The TSV and CSV modes (`TSV_MODE_*`, `STypeConfig::m_nTsvMode`): the fields of a table line up
//! in columns.
//!
//! In these modes the delimiter (a tab in TSV, a comma in CSV) does not go to a tab stop of
//! the tab width; it goes to the end of the widest field of that column in the whole document,
//! plus two columns. [`TsvStops::scan`] finds those ends the way `CTsvModeInfo::CalcTabLength`
//! does, and [`LayoutConfig::columns_of`](crate::LayoutConfig::columns_of) uses them.
//!
//! As in Sakura:
//!
//! - The stops are worked out once, when the mode is set or a file is loaded, and not after each
//!   edit (`CalcTabLength` is called from `SetLayoutInfo` when the mode changes,
//!   `ChangeLayoutParam` and `CLoadAgent::OnLoad`). A line edited later is laid out with the
//!   stops of the text as it was; loading the file again, or changing the mode, brings them up
//!   to date.
//! - There is no limit on the lines that are read or on the fields: the whole document is
//!   scanned. A comma between quotation marks is a delimiter too (`CalcTabLength` and
//!   `CFigure_Comma` know nothing of quotes).
//! - A field is as wide as the columns of its characters (a tab inside a CSV field is 1), and the
//!   last field of a line counts as well.
//!
//! The document is read one line at a time, never as one string. A very long line (see
//! [`Document::is_long_line`]) is skipped: the layout does not decode such a line either.

use std::sync::Arc;

use matsu_core::buffer::{Document, Result};

use crate::width::WidthRule;

/// `TSV_MODE_NONE`, `TSV_MODE_TSV` and `TSV_MODE_CSV` (`CTsvModeInfo.h`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TsvMode {
    /// 通常: tabs go to the tab stops of the tab width.
    #[default]
    None,
    /// TSV: tabs separate the fields.
    Tsv,
    /// CSV: commas separate the fields. A tab takes 1 column.
    Csv,
}

impl TsvMode {
    /// The mode of the setting `m_nTsvMode`: 1 is TSV and 2 is CSV; everything else is 通常.
    pub fn from_setting(value: i32) -> Self {
        match value {
            1 => Self::Tsv,
            2 => Self::Csv,
            _ => Self::None,
        }
    }

    /// The character that ends a field, if the mode has one.
    pub fn delimiter(self) -> Option<char> {
        match self {
            Self::None => None,
            Self::Tsv => Some('\t'),
            Self::Csv => Some(','),
        }
    }
}

/// Where the fields end: the mode, and for each field of a line the column of its stop
/// (`CTsvModeInfo::m_tabLength`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TsvStops {
    mode: TsvMode,
    /// For field `i`, the column where the delimiter after it ends: the widest field `0..=i`
    /// of any line, with two columns after each. Increasing.
    ends: Arc<[u32]>,
}

/// The characters that end the line in the loop of `CalcTabLength` (`WCODE::IsLineDelimiter(ch,
/// true)`: with the extended line endings, whatever the setting says).
fn is_line_delimiter(c: char) -> bool {
    matches!(c, '\r' | '\n' | '\u{85}' | '\u{2028}' | '\u{2029}')
}

impl TsvStops {
    /// No stops: [`TsvMode::None`].
    pub fn none() -> Self {
        Self::default()
    }

    /// The stops of the lines `lines` (their text without the line ending) in `mode`
    /// (`CTsvModeInfo::CalcTabLength`). `width` gives the columns of a character
    /// (`CNativeW::GetKetaOfChar`).
    pub fn from_lines<I, S>(mode: TsvMode, width: &WidthRule, lines: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let Some(delimiter) = mode.delimiter() else {
            return Self::none();
        };
        // The widest field of each field number.
        let mut widest: Vec<u32> = Vec::new();
        for line in lines {
            let mut field = 0;
            let mut field_width = 0u32;
            for c in line.as_ref().chars() {
                if is_line_delimiter(c) {
                    break;
                }
                if c == delimiter {
                    note(&mut widest, field, field_width);
                    field += 1;
                    field_width = 0;
                    continue;
                }
                // A tab that is not the delimiter (CSV) counts 1.
                field_width =
                    field_width.saturating_add(if c == '\t' { 1 } else { width.columns(c) });
            }
            note(&mut widest, field, field_width);
        }
        // "m_tabLength[ui] += m_tabLength[ui-1] + 2"
        let mut end = 0u32;
        let ends: Vec<u32> = widest
            .into_iter()
            .map(|w| {
                end = end.saturating_add(w).saturating_add(2);
                end
            })
            .collect();
        Self {
            mode,
            ends: ends.into(),
        }
    }

    /// Reads `doc` a line at a time and works out the stops. A very long line is left out.
    pub fn scan(doc: &Document, mode: TsvMode, width: &WidthRule) -> Result<Self> {
        if mode == TsvMode::None {
            return Ok(Self::none());
        }
        let mut failure = None;
        let lines = (0..doc.line_count()).filter_map(|i| {
            if failure.is_some() || doc.is_long_line(i) {
                return None;
            }
            match doc.line(i) {
                Ok(line) => line.map(|l| l.text),
                Err(e) => {
                    failure = Some(e);
                    None
                }
            }
        });
        let stops = Self::from_lines(mode, width, lines);
        match failure {
            Some(e) => Err(e),
            None => Ok(stops),
        }
    }

    pub fn mode(&self) -> TsvMode {
        self.mode
    }

    /// The column where each field's delimiter ends, for the first field, the second, and so on.
    pub fn ends(&self) -> &[u32] {
        &self.ends
    }

    /// The columns a delimiter takes when it starts at column `x`
    /// (`CTsvModeInfo::GetActualTabLength`): up to the first stop after `x`, or 2 when `x` is
    /// past all of them.
    pub fn delimiter_columns(&self, x: u32) -> u32 {
        let next = self.ends.partition_point(|&end| end <= x);
        match self.ends.get(next) {
            Some(end) => end - x,
            None => 2,
        }
    }
}

/// Records that field number `field` of a line is `width` columns wide.
fn note(widest: &mut Vec<u32>, field: usize, width: u32) {
    if let Some(known) = widest.get_mut(field) {
        *known = (*known).max(width);
    } else {
        // Fields come one after the other, so this is the next one (`nField == m_tabLength.size()`).
        widest.push(width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stops(mode: TsvMode, lines: &[&str]) -> Vec<u32> {
        TsvStops::from_lines(mode, &WidthRule::default(), lines.iter().copied())
            .ends()
            .to_vec()
    }

    #[test]
    fn the_mode_none_has_no_stops() {
        assert!(stops(TsvMode::None, &["a\tb,c"]).is_empty());
    }

    #[test]
    fn a_stop_is_the_widest_field_plus_two_after_the_one_before() {
        // field 0: max(1, 3) = 3 -> 5; field 1: max(2, 1) = 2 -> 5 + 2 + 2 = 9; field 2: 4 -> 15
        assert_eq!(
            stops(TsvMode::Tsv, &["a\tbb\tcccc", "ddd\te"]),
            vec![5, 9, 15]
        );
    }

    #[test]
    fn the_csv_delimiter_is_the_comma_and_a_tab_in_a_field_counts_1() {
        assert_eq!(stops(TsvMode::Csv, &["a,b\tc"]), vec![3, 8]);
        // the tab is the delimiter of TSV and not of CSV
        assert_eq!(stops(TsvMode::Tsv, &["a,b\tc"]), vec![5, 8]);
    }

    #[test]
    fn full_width_characters_are_two_columns_and_empty_fields_are_empty() {
        assert_eq!(stops(TsvMode::Csv, &["日本,,x"]), vec![6, 8, 11]);
    }

    #[test]
    fn a_comma_between_quotes_is_a_delimiter_as_in_sakura() {
        // "a | b" | c, each with its quote
        assert_eq!(stops(TsvMode::Csv, &["\"a,b\",c"]), vec![4, 8, 11]);
    }

    #[test]
    fn a_line_ending_character_ends_the_scan_of_a_line() {
        assert_eq!(stops(TsvMode::Csv, &["ab\u{2028},cdef"]), vec![4]);
    }

    #[test]
    fn a_long_field_makes_a_long_stop() {
        let long = "x".repeat(300);
        assert_eq!(stops(TsvMode::Tsv, &[&long, "a\tb"]), vec![302, 305]);
    }

    #[test]
    fn a_delimiter_goes_to_the_next_stop_or_takes_two_columns_past_the_last() {
        let s = TsvStops::from_lines(TsvMode::Tsv, &WidthRule::default(), ["abc\td"]);
        assert_eq!(s.ends(), &[5, 8]);
        assert_eq!(s.delimiter_columns(0), 5);
        assert_eq!(s.delimiter_columns(3), 2);
        assert_eq!(s.delimiter_columns(4), 1);
        // a stop is the first column of the next field: the delimiter that starts on it is
        // the next one (`pos < tabLength`)
        assert_eq!(s.delimiter_columns(5), 3);
        assert_eq!(s.delimiter_columns(8), 2);
        assert_eq!(s.delimiter_columns(100), 2);
    }

    #[test]
    fn scan_reads_the_document_by_lines() {
        let doc = Document::from_utf8(b"a\tbb\r\nccc\td\r\n".to_vec()).unwrap();
        let s = TsvStops::scan(&doc, TsvMode::Tsv, &WidthRule::default()).unwrap();
        assert_eq!(s.ends(), &[5, 9]);
        assert_eq!(s.mode(), TsvMode::Tsv);
        let none = TsvStops::scan(&doc, TsvMode::None, &WidthRule::default()).unwrap();
        assert_eq!(none, TsvStops::none());
    }

    #[test]
    fn the_mode_of_a_setting() {
        assert_eq!(TsvMode::from_setting(0), TsvMode::None);
        assert_eq!(TsvMode::from_setting(1), TsvMode::Tsv);
        assert_eq!(TsvMode::from_setting(2), TsvMode::Csv);
        assert_eq!(TsvMode::from_setting(7), TsvMode::None);
        assert_eq!(TsvMode::from_setting(-1), TsvMode::None);
    }
}
