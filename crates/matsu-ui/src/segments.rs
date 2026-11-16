//! Cuts the text of one row into the pieces that are painted: runs of text, tabs and full-width
//! spaces, each at its display position.
//!
//! The layout counts in `X` (a half-width character is one column, a full-width one two; see
//! `matsu_layout::X`); the window draws a column as one cell of the font's half-width. A run of
//! text is shaped as one line and placed at its position, and the run ends where the width
//! changes, so that a font whose full-width characters are not exactly two cells wide cannot push
//! the rest of the row out of its columns. With a table of widths (a proportional font) every
//! character has its own width and a run is not cut by it: the painter places each cluster of the
//! run at its position in the layout.
//! The text is also cut where the syntax coloring changes color ([`row_segments_cut`]).

use matsu_layout::{LayoutConfig, TsvMode, X};

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
    /// Where it starts.
    pub x: X,
    /// How wide it is.
    pub width: X,
    /// The text of a [`SegmentKind::Text`] segment; empty for the others.
    pub text: String,
}

/// The pieces of `text`, the text of one row that starts at `start_x`.
pub fn row_segments(text: &str, start_x: X, cfg: &LayoutConfig) -> Vec<Segment> {
    row_segments_cut(text, start_x, cfg, &[])
}

/// The pieces of `text` as [`row_segments`] gives them, and also cut at the byte offsets in `cuts`
/// (in increasing order): a run of text does not go over one, so that each piece has one color.
pub fn row_segments_cut(
    text: &str,
    start_x: X,
    cfg: &LayoutConfig,
    cuts: &[usize],
) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    let mut at = start_x;
    // How wide each character of the last text segment is, so that a long row is cut in one pass.
    let mut run_width = X::ZERO;
    // The last character was a CSV delimiter, which goes to the end of its field and so is
    // placed alone (`CFigure_Comma`).
    let mut after_delimiter = false;
    let mut next_cut = 0;
    // With a table of widths the characters of a run have widths of their own: the run is not cut
    // where the width changes, so that a mark stays with its letter and the letters of a script
    // that joins them stay together when the painter shapes the run. The painter puts each
    // cluster at its own place from the layout.
    let proportional = cfg.width.is_table();
    for (byte, c) in text.char_indices() {
        let advance = cfg.advance(c, at);
        let (x, width) = (at, advance);
        let mut cut = false;
        while next_cut < cuts.len() && cuts[next_cut] <= byte {
            cut |= cuts[next_cut] == byte;
            next_cut += 1;
        }
        let delimiter = c == ',' && cfg.tsv.mode() == TsvMode::Csv;
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
                        && !delimiter
                        && !after_delimiter
                        && last.kind == SegmentKind::Text
                        && last.x + last.width == x
                        && (proportional || run_width == width) =>
                {
                    last.text.push(c);
                    last.width += width;
                }
                _ => {
                    out.push(Segment {
                        kind: SegmentKind::Text,
                        byte,
                        x,
                        width,
                        text: c.to_string(),
                    });
                    run_width = width;
                }
            },
            Some(kind) => out.push(Segment {
                kind,
                byte,
                x,
                width,
                text: String::new(),
            }),
            // A control character has no glyph here; it keeps its place.
            None => {}
        }
        after_delimiter = delimiter;
        at += advance;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use matsu_layout::Widths;

    /// `X` in a column: the grid of today (1), the one the view uses (64) and one with character
    /// spacing (80). The segments are the same in columns whatever it is.
    const PER_COLUMN: [u32; 3] = [1, 64, 80];

    fn config(per: u32) -> LayoutConfig {
        LayoutConfig {
            width: Widths::grid_scaled(matsu_layout::WidthRule::default(), per),
            ..LayoutConfig::default()
        }
    }

    /// A segment at `col` columns, `cols` wide.
    fn seg(
        per: u32,
        kind: SegmentKind,
        byte: usize,
        (col, cols): (u32, u32),
        text: &str,
    ) -> Segment {
        Segment {
            kind,
            byte,
            x: X::from_raw(col * per),
            width: X::from_raw(cols * per),
            text: text.to_owned(),
        }
    }

    fn start(per: u32, col: u32) -> X {
        X::from_raw(col * per)
    }

    #[test]
    fn plain_text_is_one_segment_per_width() {
        for per in PER_COLUMN {
            let cfg = config(per);
            assert_eq!(
                row_segments("abc日本de", start(per, 0), &cfg),
                [
                    seg(per, SegmentKind::Text, 0, (0, 3), "abc"),
                    seg(per, SegmentKind::Text, 3, (3, 4), "日本"),
                    seg(per, SegmentKind::Text, 9, (7, 2), "de"),
                ]
            );
            assert!(row_segments("", X::ZERO, &cfg).is_empty());
        }
    }

    #[test]
    fn a_csv_comma_is_a_piece_of_its_own_at_the_end_of_its_field() {
        use matsu_layout::{TsvMode, TsvStops};
        let lines = ["ab,,cd"];
        for per in PER_COLUMN {
            let cfg = LayoutConfig {
                tsv: TsvStops::from_lines(TsvMode::Csv, &cfg_widths(per), lines),
                ..config(per)
            };
            // fields: ab (4), empty (6), cd (10); each comma goes to the end of its field
            assert_eq!(
                row_segments("ab,,cd", X::ZERO, &cfg),
                [
                    seg(per, SegmentKind::Text, 0, (0, 2), "ab"),
                    seg(per, SegmentKind::Text, 2, (2, 2), ","),
                    seg(per, SegmentKind::Text, 3, (4, 2), ","),
                    seg(per, SegmentKind::Text, 4, (6, 2), "cd"),
                ]
            );
            // without the mode a comma is an ordinary character
            assert_eq!(
                row_segments("ab,,cd", X::ZERO, &config(per)),
                [seg(per, SegmentKind::Text, 0, (0, 6), "ab,,cd")]
            );
        }
    }

    fn cfg_widths(per: u32) -> Widths {
        Widths::grid_scaled(matsu_layout::WidthRule::default(), per)
    }

    #[test]
    fn text_is_cut_where_the_color_changes() {
        for per in PER_COLUMN {
            let cfg = config(per);
            let at = |byte: usize, col: u32, cols: u32, text: &str| {
                seg(per, SegmentKind::Text, byte, (col, cols), text)
            };
            assert_eq!(
                row_segments_cut("abcdef", X::ZERO, &cfg, &[2, 4]),
                [at(0, 0, 2, "ab"), at(2, 2, 2, "cd"), at(4, 4, 2, "ef")]
            );
            // A cut at the start, or at a place that is not a character start, cuts nothing more
            // than the characters allow; wide characters are cut between characters.
            assert_eq!(
                row_segments_cut("日本語", X::ZERO, &cfg, &[0, 3]),
                [at(0, 0, 2, "日"), at(3, 2, 4, "本語")]
            );
            // Tabs are their own pieces and keep their byte.
            let segs = row_segments_cut("a\tb", X::ZERO, &cfg, &[1, 2]);
            assert_eq!(
                segs.iter().map(|s| (s.kind, s.byte)).collect::<Vec<_>>(),
                [
                    (SegmentKind::Text, 0),
                    (SegmentKind::Tab, 1),
                    (SegmentKind::Text, 2)
                ]
            );
        }
    }

    #[test]
    fn a_long_row_of_one_width_is_one_segment() {
        let text = "a".repeat(10_000);
        for per in PER_COLUMN {
            assert_eq!(
                row_segments(&text, X::ZERO, &config(per)),
                [seg(per, SegmentKind::Text, 0, (0, 10_000), &text)]
            );
        }
    }

    #[test]
    fn tabs_run_to_the_next_tab_stop() {
        for per in PER_COLUMN {
            let cfg = config(per); // tab stops every 4 columns
            assert_eq!(
                row_segments("a\tb\t\tc", X::ZERO, &cfg),
                [
                    seg(per, SegmentKind::Text, 0, (0, 1), "a"),
                    seg(per, SegmentKind::Tab, 1, (1, 3), ""),
                    seg(per, SegmentKind::Text, 2, (4, 1), "b"),
                    seg(per, SegmentKind::Tab, 3, (5, 3), ""),
                    seg(per, SegmentKind::Tab, 4, (8, 4), ""),
                    seg(per, SegmentKind::Text, 5, (12, 1), "c"),
                ]
            );
        }
    }

    #[test]
    fn a_wrapped_row_starts_at_its_indent() {
        for per in PER_COLUMN {
            let segs = row_segments("\tx", start(per, 2), &config(per));
            assert_eq!(segs[0], seg(per, SegmentKind::Tab, 0, (2, 2), ""));
            assert_eq!(segs[1], seg(per, SegmentKind::Text, 1, (4, 1), "x"));
        }
    }

    #[test]
    fn full_width_spaces_and_control_characters() {
        for per in PER_COLUMN {
            assert_eq!(
                row_segments("a\u{3000}b\u{1}c", X::ZERO, &config(per)),
                [
                    seg(per, SegmentKind::Text, 0, (0, 1), "a"),
                    seg(per, SegmentKind::FullWidthSpace, 1, (1, 2), ""),
                    seg(per, SegmentKind::Text, 4, (3, 1), "b"),
                    // the control character takes column 4 and draws nothing
                    seg(per, SegmentKind::Text, 6, (5, 1), "c"),
                ]
            );
        }
    }

    /// A proportional font: `i` narrow, `m` wide, a combining mark takes 1 X.
    fn table_config() -> LayoutConfig {
        LayoutConfig {
            width: Widths::table(64, 0, |c| match c {
                'i' => 32,
                'm' => 107,
                '\u{0301}' => 1,
                _ => 64,
            }),
            ..LayoutConfig::default()
        }
    }

    #[test]
    fn a_proportional_run_is_not_cut_where_the_width_changes() {
        let cfg = table_config();
        let text = "mime\u{301}x";
        let segs = row_segments(text, X::ZERO, &cfg);
        assert_eq!(segs.len(), 1, "{segs:?}");
        assert_eq!(segs[0].text, text);
        // m 107, i 32, m 107, e 64, the mark 1, x 64
        assert_eq!(segs[0].width.raw(), 107 + 32 + 107 + 64 + 1 + 64);
    }

    #[test]
    fn a_letter_and_its_accent_are_one_segment_with_a_different_width() {
        let cfg = table_config();
        let segs = row_segments("e\u{301}", X::from_raw(10), &cfg);
        assert_eq!(segs.len(), 1);
        assert_eq!((segs[0].x.raw(), segs[0].width.raw()), (10, 65));
        // a tab still ends the run, and a cut at a color change still does
        let tab = row_segments("ab\tcd", X::ZERO, &cfg);
        assert_eq!(tab.len(), 3);
        let cut = row_segments_cut("abcd", X::ZERO, &cfg, &[2]);
        assert_eq!(cut.len(), 2);
    }
}
