//! Cuts the text of one row into the pieces that are painted: runs of text, tabs and full-width
//! spaces, each at its display column.
//!
//! The layout counts columns (a half-width character is 1, a full-width one 2); the window draws
//! a column as one cell of the font's half-width. A run of text is shaped as one line and placed
//! at its column, and the run ends where the width changes, so that a font whose full-width
//! characters are not exactly two cells wide cannot push the rest of the row out of its columns.
//! The text is also cut where the syntax coloring changes color ([`row_segments_cut`]).

use matsu_layout::LayoutConfig;

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
    /// The byte offset in the text of the row where the segment starts.
    pub byte: usize,
    /// The display column where it starts.
    pub col: u32,
    /// How many columns it covers.
    pub cols: u32,
    /// The text of a [`SegmentKind::Text`] segment; empty for the others.
    pub text: String,
}

/// The pieces of `text`, the text of one row that starts at column `start_col`.
pub fn row_segments(text: &str, start_col: u32, cfg: &LayoutConfig) -> Vec<Segment> {
    row_segments_cut(text, start_col, cfg, &[])
}

/// The pieces of `text` as [`row_segments`] gives them, and also cut at the byte offsets in `cuts`
/// (in increasing order): a run of text does not go over one, so that each piece has one color.
pub fn row_segments_cut(
    text: &str,
    start_col: u32,
    cfg: &LayoutConfig,
    cuts: &[usize],
) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    let mut col = start_col;
    // How wide each character of the last text segment is, so that a long row is cut in one pass.
    let mut run_width = 0;
    let mut next_cut = 0;
    for (byte, c) in text.char_indices() {
        let cols = cfg.columns_of(c, col);
        let mut cut = false;
        while next_cut < cuts.len() && cuts[next_cut] <= byte {
            cut |= cuts[next_cut] == byte;
            next_cut += 1;
        }
        let kind = match c {
            '\t' => Some(SegmentKind::Tab),
            '\u{3000}' => Some(SegmentKind::FullWidthSpace),
            c if c.is_control() => None,
            _ => Some(SegmentKind::Text),
        };
        match kind {
            Some(SegmentKind::Text) => match out.last_mut() {
                Some(last)
                    if !cut
                        && last.kind == SegmentKind::Text
                        && last.col + last.cols == col
                        && run_width == cols =>
                {
                    last.text.push(c);
                    last.cols += cols;
                }
                _ => {
                    out.push(Segment {
                        kind: SegmentKind::Text,
                        byte,
                        col,
                        cols,
                        text: c.to_string(),
                    });
                    run_width = cols;
                }
            },
            Some(kind) => out.push(Segment {
                kind,
                byte,
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

    fn seg(kind: SegmentKind, byte: usize, col: u32, cols: u32, text: &str) -> Segment {
        Segment {
            kind,
            byte,
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
                seg(SegmentKind::Text, 0, 0, 3, "abc"),
                seg(SegmentKind::Text, 3, 3, 4, "日本"),
                seg(SegmentKind::Text, 9, 7, 2, "de"),
            ]
        );
        assert!(row_segments("", 0, &cfg).is_empty());
    }

    #[test]
    fn text_is_cut_where_the_color_changes() {
        let cfg = LayoutConfig::default();
        let at = |byte: usize, col: u32, cols: u32, text: &str| {
            seg(SegmentKind::Text, byte, col, cols, text)
        };
        assert_eq!(
            row_segments_cut("abcdef", 0, &cfg, &[2, 4]),
            [at(0, 0, 2, "ab"), at(2, 2, 2, "cd"), at(4, 4, 2, "ef")]
        );
        // A cut at the start, or at a place that is not a character start, cuts nothing more
        // than the characters allow; wide characters are cut between characters.
        assert_eq!(
            row_segments_cut("日本語", 0, &cfg, &[0, 3]),
            [at(0, 0, 2, "日"), at(3, 2, 4, "本語")]
        );
        // Tabs are their own pieces and keep their byte.
        let segs = row_segments_cut("a\tb", 0, &cfg, &[1, 2]);
        assert_eq!(
            segs.iter().map(|s| (s.kind, s.byte)).collect::<Vec<_>>(),
            [
                (SegmentKind::Text, 0),
                (SegmentKind::Tab, 1),
                (SegmentKind::Text, 2)
            ]
        );
    }

    #[test]
    fn a_long_row_of_one_width_is_one_segment() {
        let cfg = LayoutConfig::default();
        let text = "a".repeat(10_000);
        assert_eq!(
            row_segments(&text, 0, &cfg),
            [seg(SegmentKind::Text, 0, 0, 10_000, &text)]
        );
    }

    #[test]
    fn tabs_run_to_the_next_tab_stop() {
        let cfg = LayoutConfig::default(); // tab stops every 4 columns
        assert_eq!(
            row_segments("a\tb\t\tc", 0, &cfg),
            [
                seg(SegmentKind::Text, 0, 0, 1, "a"),
                seg(SegmentKind::Tab, 1, 1, 3, ""),
                seg(SegmentKind::Text, 2, 4, 1, "b"),
                seg(SegmentKind::Tab, 3, 5, 3, ""),
                seg(SegmentKind::Tab, 4, 8, 4, ""),
                seg(SegmentKind::Text, 5, 12, 1, "c"),
            ]
        );
    }

    #[test]
    fn a_wrapped_row_starts_at_its_indent() {
        let cfg = LayoutConfig::default();
        let segs = row_segments("\tx", 2, &cfg);
        assert_eq!(segs[0], seg(SegmentKind::Tab, 0, 2, 2, ""));
        assert_eq!(segs[1], seg(SegmentKind::Text, 1, 4, 1, "x"));
    }

    #[test]
    fn full_width_spaces_and_control_characters() {
        let cfg = LayoutConfig::default();
        assert_eq!(
            row_segments("a\u{3000}b\u{1}c", 0, &cfg),
            [
                seg(SegmentKind::Text, 0, 0, 1, "a"),
                seg(SegmentKind::FullWidthSpace, 1, 1, 2, ""),
                seg(SegmentKind::Text, 4, 3, 1, "b"),
                // the control character takes column 4 and draws nothing
                seg(SegmentKind::Text, 6, 5, 1, "c"),
            ]
        );
    }
}
