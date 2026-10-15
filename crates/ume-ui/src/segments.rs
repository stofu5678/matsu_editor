//! Cuts the text of one row into the pieces that are painted: runs of text, tabs and full-width
//! spaces, each at its display column.
//!
//! The layout counts columns (a half-width character is 1, a full-width one 2); the window draws
//! a column as one cell of the font's half-width. A run of text is shaped as one line and placed
//! at its column, and the run ends where the width changes, so that a font whose full-width
//! characters are not exactly two cells wide cannot push the rest of the row out of its columns.

use ume_layout::LayoutConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentKind {
    /// Characters that are drawn as they are.
    Text,
    /// A tab. Sakura marks it with an arrow (`sakura_core/view/figures/CFigure_Tab.cpp`).
    Tab,
    /// An ideographic space, U+3000. Sakura marks it with a box (`sakura_core/view/figures/CFigure_ZenSpace.cpp`).
    FullWidthSpace,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub kind: SegmentKind,
    /// The display column where it starts.
    pub col: u32,
    /// How many columns it covers.
    pub cols: u32,
    /// The text of a [`SegmentKind::Text`] segment; empty for the others.
    pub text: String,
}

/// The pieces of `text`, the text of one row that starts at column `start_col`.
pub fn row_segments(text: &str, start_col: u32, cfg: &LayoutConfig) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    let mut col = start_col;
    for c in text.chars() {
        let cols = cfg.columns_of(c, col);
        let kind = match c {
            '\t' => Some(SegmentKind::Tab),
            '\u{3000}' => Some(SegmentKind::FullWidthSpace),
            c if c.is_control() => None,
            _ => Some(SegmentKind::Text),
        };
        match kind {
            Some(SegmentKind::Text) => match out.last_mut() {
                Some(last)
                    if last.kind == SegmentKind::Text
                        && last.col + last.cols == col
                        && last.cols / last.text.chars().count() as u32 == cols =>
                {
                    last.text.push(c);
                    last.cols += cols;
                }
                _ => out.push(Segment {
                    kind: SegmentKind::Text,
                    col,
                    cols,
                    text: c.to_string(),
                }),
            },
            Some(kind) => out.push(Segment {
                kind,
                col,
                cols,
                text: String::new(),
            }),
            // A control character has no glyph here; it keeps its column.
            None => {}
        }
        col += cols;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(kind: SegmentKind, col: u32, cols: u32, text: &str) -> Segment {
        Segment {
            kind,
            col,
            cols,
            text: text.to_owned(),
        }
    }

    #[test]
    fn plain_text_is_one_segment_per_width() {
        let cfg = LayoutConfig::default();
        assert_eq!(
            row_segments("abc日本de", 0, &cfg),
            [
                seg(SegmentKind::Text, 0, 3, "abc"),
                seg(SegmentKind::Text, 3, 4, "日本"),
                seg(SegmentKind::Text, 7, 2, "de"),
            ]
        );
        assert!(row_segments("", 0, &cfg).is_empty());
    }

    #[test]
    fn tabs_run_to_the_next_tab_stop() {
        let cfg = LayoutConfig::default(); // tab stops every 4 columns
        assert_eq!(
            row_segments("a\tb\t\tc", 0, &cfg),
            [
                seg(SegmentKind::Text, 0, 1, "a"),
                seg(SegmentKind::Tab, 1, 3, ""),
                seg(SegmentKind::Text, 4, 1, "b"),
                seg(SegmentKind::Tab, 5, 3, ""),
                seg(SegmentKind::Tab, 8, 4, ""),
                seg(SegmentKind::Text, 12, 1, "c"),
            ]
        );
    }

    #[test]
    fn a_wrapped_row_starts_at_its_indent() {
        let cfg = LayoutConfig::default();
        let segs = row_segments("\tx", 2, &cfg);
        assert_eq!(segs[0], seg(SegmentKind::Tab, 2, 2, ""));
        assert_eq!(segs[1], seg(SegmentKind::Text, 4, 1, "x"));
    }

    #[test]
    fn full_width_spaces_and_control_characters() {
        let cfg = LayoutConfig::default();
        assert_eq!(
            row_segments("a\u{3000}b\u{1}c", 0, &cfg),
            [
                seg(SegmentKind::Text, 0, 1, "a"),
                seg(SegmentKind::FullWidthSpace, 1, 2, ""),
                seg(SegmentKind::Text, 3, 1, "b"),
                // the control character takes column 4 and draws nothing
                seg(SegmentKind::Text, 5, 1, "c"),
            ]
        );
    }
}
