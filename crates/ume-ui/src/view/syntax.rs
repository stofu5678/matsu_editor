//! The colors of the text of a row: which color entry each piece of text is drawn in, and how the
//! selection, the search marks, the marks for tabs and spaces, and the caret line go with it.
//!
//! The logic is Sakura's `CEditView::DrawLayoutLine`, `SetCurrentColor` and
//! `CFigureSpace::DrawImp_StyleSelect` (`sakura_core/view/CEditView_Paint.cpp`,
//! `sakura_core/view/figures/CFigureStrategy.cpp`):
//!
//! * the color of a piece is the entry of the strategy that colored it; a match of the search
//!   (`CColor_Found`) takes over from it;
//! * a color whose background is the background of `TXT` has the background of the row instead
//!   (the caret line `CBK`, the odd lines `EBK`, else `TXT`);
//! * selected text is drawn in the color of `SEL` when that has two different colors, and when
//!   its text and background are the same ("mix") in colors made from the color under it
//!   ([`selection_colors_over`]);
//! * the mark of a tab or a full-width space is drawn in its own color unless that is the color
//!   of `TXT`, in which case it follows the text around it.
//!
//! This module has no window in it, so it is tested without one. `element.rs` paints with it.

use ume_coloring::{ColorKind, ColorRules, ColorRun};
use ume_config::settings::color::ColorEntry;
use ume_layout::LayoutConfig;

use crate::segments::{SegmentKind, row_segments_cut};
#[cfg(test)]
use crate::settings::view_settings::FoundColors;
use crate::settings::view_settings::{Colors, Rgb, rgb_of, selection_colors_over};

/// How a piece of text is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Style {
    pub text: Rgb,
    /// The background, or `None` for the background of the row.
    pub back: Option<Rgb>,
    pub bold: bool,
    pub underline: bool,
}

impl Style {
    /// The style of a color entry; its background is the row's when it is the one of `TXT`.
    pub(super) fn of(entry: &ColorEntry, colors: &Colors) -> Self {
        Self {
            text: rgb_of(entry.text),
            back: (entry.back != colors.text_entry.back).then(|| rgb_of(entry.back)),
            bold: entry.bold,
            underline: entry.underline,
        }
    }
}

impl Style {
    /// The color entry that draws like this style (the background of the row is the one of `TXT`).
    pub(super) fn entry(&self, colors: &Colors) -> ColorEntry {
        ColorEntry {
            disp: true,
            bold: self.bold,
            underline: self.underline,
            text: rgb_of(self.text),
            back: self.back.map_or(colors.text_entry.back, rgb_of),
        }
    }
}

/// The background of a row: the caret line `CBK`, else an odd row `EBK`, else `TXT`
/// (`cBackType` in `DrawLayoutLine`). `row` is the number of the row in the document.
pub(super) fn row_background(colors: &Colors, row: usize, caret_row: bool) -> Rgb {
    if caret_row && let Some(back) = colors.caret_line_back {
        back
    } else if row % 2 == 1
        && let Some(back) = colors.even_line_back
    {
        back
    } else {
        colors.background
    }
}

/// A match of the search on a row: from column, to column, and the number of the search color it
/// is drawn in (the first is 0; see [`crate::search::highlight::slot_of`]).
pub(super) type FoundRange = (u32, u32, u8);

/// The entry a piece of text of the kind `kind` is drawn in, or the search color `found` if it is
/// in a match of the search and a search color is shown.
pub(super) fn entry_of(
    colors: &Colors,
    rules: &ColorRules,
    kind: ColorKind,
    found: Option<u8>,
) -> ColorEntry {
    found
        .and_then(|slot| colors.found.get(slot))
        .unwrap_or_else(|| *rules.entry(kind))
}

/// The style of selected text over text drawn in `under` on a row whose background is `row_back`
/// (`SetCurrentColor` with the selection color as the current one).
pub(super) fn selected_style(colors: &Colors, under: &ColorEntry, row_back: Rgb) -> Style {
    let sel = &colors.selection_entry;
    let txt = &colors.text_entry;
    // `info3`: the color under the selection is the row's where the text has `TXT`'s background.
    let under_back = if under.back == txt.back {
        rgb_of(row_back)
    } else {
        under.back
    };
    let (text, back) = selection_colors_over(sel, txt.back, under.text, under_back);
    // The font follows the selection color when that is a color of its own, else the text.
    let own = sel.disp && sel.text != sel.back;
    Style {
        text: rgb_of(text),
        back: Some(rgb_of(back)),
        bold: if own { sel.bold } else { under.bold },
        underline: if own { sel.underline } else { under.underline },
    }
}

/// The style of the mark of a tab or a full-width space (`DrawImp_StyleSelect`, outside the
/// selection): the mark's own colors unless they are those of `TXT`, then the colors of the text
/// around it; bold if either is.
pub(super) fn space_style(space: &ColorEntry, around: &ColorEntry, colors: &Colors) -> SpaceStyle {
    let txt = &colors.text_entry;
    let text = if space.text == txt.text {
        around.text
    } else {
        space.text
    };
    let back = if space.back == txt.back {
        around.back
    } else {
        space.back
    };
    SpaceStyle {
        style: Style {
            text: rgb_of(text),
            back: (back != txt.back).then(|| rgb_of(back)),
            bold: space.bold || around.bold,
            underline: space.underline,
        },
        // Underlined text around a mark that is not underlined: the underline is drawn in the
        // color of the text around (`DrawImp_DrawUnderline`).
        around_underline: (!space.underline && around.underline).then(|| rgb_of(around.text)),
    }
}

/// [`space_style`]'s answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SpaceStyle {
    pub style: Style,
    /// The color of an underline over the mark that comes from the text around it.
    pub around_underline: Option<Rgb>,
}

/// What a piece of a row is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PieceKind {
    Text,
    Tab,
    FullWidthSpace,
}

/// A piece of a row: text of one color, or the mark of a tab or a full-width space, at a column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Piece {
    pub kind: PieceKind,
    pub col: u32,
    pub cols: u32,
    pub text: String,
    /// The kind of run the piece is in.
    pub color: ColorKind,
    /// The piece is in a match of the search, in the search color of this number.
    pub found: Option<u8>,
}

/// The runs of a row in the bytes of the row: `runs` are the runs of the line (covering the text
/// from byte 0, in the bytes of the line), `from` is where the row starts in the line and `to`
/// where it ends.
pub(super) fn row_runs(
    runs: &[ColorRun],
    from: usize,
    to: usize,
) -> Vec<(usize, usize, ColorKind)> {
    let first = runs.partition_point(|r| r.end <= from);
    runs[first..]
        .iter()
        .take_while(|r| r.start < to)
        .map(|r| (r.start.max(from) - from, r.end.min(to) - from, r.kind))
        .filter(|(start, end, _)| start < end)
        .collect()
}

/// The kind of run at byte `at` of the row.
fn kind_at(runs: &[(usize, usize, ColorKind)], at: usize) -> ColorKind {
    let i = runs.partition_point(|(_, end, _)| *end <= at);
    runs.get(i)
        .filter(|(start, _, _)| *start <= at)
        .map_or(ColorKind::Text, |r| r.2)
}

/// Cuts the text of a row into pieces of one color. `runs` are the row's runs
/// ([`row_runs`]) and `found` the column ranges of the matches of the search on the row.
pub(super) fn pieces(
    text: &str,
    start_col: u32,
    cfg: &LayoutConfig,
    runs: &[(usize, usize, ColorKind)],
    found: &[FoundRange],
) -> Vec<Piece> {
    let cuts: Vec<usize> = runs.iter().map(|r| r.0).collect();
    // The ranges are in increasing order and do not overlap, so they are searched by bisection.
    let in_found = |col: u32| {
        let i = found.partition_point(|&(_, to, _)| to <= col);
        found
            .get(i)
            .filter(|&&(from, _, _)| from <= col)
            .map(|&(_, _, slot)| slot)
    };
    let mut out = Vec::new();
    for seg in row_segments_cut(text, start_col, cfg, &cuts) {
        let color = kind_at(runs, seg.byte);
        match seg.kind {
            SegmentKind::Text => split_text(
                &seg.text,
                seg.byte,
                seg.col,
                seg.cols,
                found,
                |byte, col, cols, text| {
                    out.push(Piece {
                        kind: PieceKind::Text,
                        col,
                        cols,
                        text: text.to_owned(),
                        color: kind_at(runs, byte),
                        found: in_found(col),
                    });
                },
            ),
            SegmentKind::Tab | SegmentKind::FullWidthSpace => out.push(Piece {
                kind: if seg.kind == SegmentKind::Tab {
                    PieceKind::Tab
                } else {
                    PieceKind::FullWidthSpace
                },
                col: seg.col,
                cols: seg.cols,
                text: String::new(),
                color,
                found: in_found(seg.col),
            }),
        }
    }
    out
}

/// Calls `emit(byte, col, cols, text)` for the parts of a text segment that the edges of the
/// found ranges cut. All characters of a segment have the same width, so an edge is a character
/// boundary when it is a multiple of that width from the start.
fn split_text(
    text: &str,
    byte: usize,
    col: u32,
    cols: u32,
    found: &[FoundRange],
    mut emit: impl FnMut(usize, u32, u32, &str),
) {
    let chars = text.chars().count() as u32;
    if chars == 0 {
        return;
    }
    let width = (cols / chars).max(1);
    // Only the ranges that touch the segment can cut it.
    let first = found.partition_point(|&(_, to, _)| to <= col);
    let touching = found[first..]
        .iter()
        .take_while(|&&(from, _, _)| from < col + cols);
    let mut edges: Vec<u32> = touching
        .flat_map(|&(from, to, _)| [from, to])
        .filter(|&e| e > col && e < col + cols && (e - col).is_multiple_of(width))
        .map(|e| (e - col) / width)
        .collect();
    edges.sort_unstable();
    edges.dedup();
    let mut start_char = 0u32;
    let mut start_byte = 0usize;
    let mut offsets = text.char_indices().map(|(i, _)| i).chain([text.len()]);
    let mut at_char = 0u32;
    let mut at_byte = offsets.next().unwrap_or(0);
    for edge in edges.into_iter().chain([chars]) {
        while at_char < edge {
            at_char += 1;
            at_byte = offsets.next().unwrap_or(text.len());
        }
        emit(
            byte + start_byte,
            col + start_char * width,
            (edge - start_char) * width,
            &text[start_byte..at_byte],
        );
        start_char = edge;
        start_byte = at_byte;
    }
}

/// What a mark or a piece of text is called: the painter turns it into the glyphs to draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Label {
    Text(String),
    Tab,
    FullWidthSpace,
}

/// A piece to draw: at a column, with a style, and the style it has when it is selected (set only
/// for the pieces that overlap the selection).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Item {
    pub col: u32,
    pub label: Label,
    pub style: Style,
    pub selected: Option<Style>,
}

/// What `style_row` needs to know about a row.
pub(super) struct RowInput<'a> {
    /// The text of the row, without the line ending, and the column where it starts.
    pub text: &'a str,
    pub start_col: u32,
    pub cfg: &'a LayoutConfig,
    /// The runs of the row ([`row_runs`]).
    pub runs: &'a [(usize, usize, ColorKind)],
    /// The column ranges of the matches of the search on the row.
    pub found: &'a [FoundRange],
    /// The selected columns of the row.
    pub selected: Option<(u32, u32)>,
    /// The background of the row ([`row_background`]).
    pub row_back: Rgb,
}

/// How to draw a row: the pieces, and the rectangles to fill under them (from column, to column,
/// color).
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct RowStyling {
    pub items: Vec<Item>,
    /// Backgrounds of pieces that are not the background of the row.
    pub backs: Vec<(u32, u32, Rgb)>,
    /// The backgrounds of the selected parts of the pieces.
    pub selected_backs: Vec<(u32, u32, Rgb)>,
    /// Underlines under the mark of a tab or a space that come from the text around it.
    pub underlines: Vec<(u32, u32, Rgb)>,
}

/// Decides the colors of everything on a row (`DrawLayoutLine` with `SetCurrentColor` and the
/// figures): the text in the color of its run or of the match of the search, the marks of tabs
/// and full-width spaces by the rule of `CFigureSpace`, and under the selection the colors of the
/// selection mixed with the ones under it.
pub(super) fn style_row(colors: &Colors, rules: &ColorRules, input: &RowInput<'_>) -> RowStyling {
    let mut out = RowStyling::default();
    let overlap = |from: u32, to: u32| {
        input
            .selected
            .filter(|&(a, b)| a < to && from < b)
            .map(|(a, b)| (a.max(from), b.min(to)))
    };
    for piece in pieces(
        input.text,
        input.start_col,
        input.cfg,
        input.runs,
        input.found,
    ) {
        let around = entry_of(colors, rules, piece.color, piece.found);
        let (from, to) = (piece.col, piece.col + piece.cols);
        let (label, style) = match piece.kind {
            PieceKind::Text => (Label::Text(piece.text), Style::of(&around, colors)),
            PieceKind::Tab | PieceKind::FullWidthSpace => {
                let (entry, label) = if piece.kind == PieceKind::Tab {
                    (colors.tab_entry, Label::Tab)
                } else {
                    (colors.full_width_space_entry, Label::FullWidthSpace)
                };
                // A mark that is not shown has the colors of the text around it.
                let shown = entry.disp;
                let space = if shown { entry } else { colors.text_entry };
                let space = space_style(&space, &around, colors);
                if let Some(color) = space.around_underline {
                    out.underlines.push((from, to, color));
                }
                if !shown {
                    // Nothing to draw: the cell only has the colors around it.
                    if let Some(back) = space.style.back {
                        out.backs.push((from, to, back));
                    }
                    if let Some((a, b)) = overlap(from, to) {
                        let sel =
                            selected_style(colors, &space.style.entry(colors), input.row_back);
                        out.selected_backs
                            .push((a, b, sel.back.unwrap_or(input.row_back)));
                    }
                    continue;
                }
                (label, space.style)
            }
        };
        if let Some(back) = style.back {
            out.backs.push((from, to, back));
        }
        let mut selected = None;
        if let Some((a, b)) = overlap(from, to) {
            let sel = selected_style(colors, &style.entry(colors), input.row_back);
            out.selected_backs
                .push((a, b, sel.back.unwrap_or(input.row_back)));
            selected = Some(sel);
        }
        out.items.push(Item {
            col: from,
            label,
            style,
            selected,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ViewSettings;

    fn cfg() -> LayoutConfig {
        LayoutConfig::default()
    }

    fn defaults() -> (Colors, std::sync::Arc<ColorRules>) {
        let settings = ViewSettings::default();
        (settings.colors, settings.coloring)
    }

    fn run(start: usize, end: usize, kind: ColorKind) -> ColorRun {
        ColorRun { start, end, kind }
    }

    #[test]
    fn the_runs_of_a_row_are_cut_out_of_the_runs_of_the_line() {
        let runs = [
            run(0, 3, ColorKind::Keyword(0)),
            run(3, 10, ColorKind::Text),
            run(10, 20, ColorKind::Comment),
        ];
        assert_eq!(
            row_runs(&runs, 0, 20),
            [
                (0, 3, ColorKind::Keyword(0)),
                (3, 10, ColorKind::Text),
                (10, 20, ColorKind::Comment)
            ]
        );
        // The second row of a wrapped line starts inside the text and ends inside the comment.
        assert_eq!(
            row_runs(&runs, 5, 12),
            [(0, 5, ColorKind::Text), (5, 7, ColorKind::Comment)]
        );
        assert_eq!(row_runs(&runs, 20, 25), []);
        assert_eq!(row_runs(&[], 0, 5), []);
    }

    #[test]
    fn a_row_is_cut_where_the_color_changes() {
        let runs = [
            (0, 3, ColorKind::Keyword(0)),
            (3, 4, ColorKind::Text),
            (4, 5, ColorKind::Number),
        ];
        let pieces = pieces("int1", 0, &cfg(), &runs, &[]);
        let got: Vec<(&str, ColorKind)> =
            pieces.iter().map(|p| (p.text.as_str(), p.color)).collect();
        assert_eq!(
            got,
            [("int", ColorKind::Keyword(0)), ("1", ColorKind::Text)]
        );
        assert_eq!((pieces[1].col, pieces[1].cols), (3, 1));
    }

    #[test]
    fn tabs_and_spaces_take_the_color_of_their_run() {
        let runs = [(0, 1, ColorKind::Text), (1, 3, ColorKind::Comment)];
        let pieces = pieces("a\t\u{3000}", 0, &cfg(), &runs, &[]);
        assert_eq!(pieces[1].kind, PieceKind::Tab);
        assert_eq!(pieces[1].color, ColorKind::Comment);
        assert_eq!(pieces[2].kind, PieceKind::FullWidthSpace);
        assert_eq!(pieces[2].color, ColorKind::Comment);
    }

    #[test]
    fn matches_of_the_search_cut_the_text_and_override_the_color() {
        let runs = [(0, 8, ColorKind::Comment)];
        let pieces = pieces("abcdefgh", 0, &cfg(), &runs, &[(2, 5, 0)]);
        let got: Vec<(&str, u32, Option<u8>)> = pieces
            .iter()
            .map(|p| (p.text.as_str(), p.col, p.found))
            .collect();
        assert_eq!(
            got,
            [("ab", 0, None), ("cde", 2, Some(0)), ("fgh", 5, None)]
        );
        assert!(pieces.iter().all(|p| p.color == ColorKind::Comment));
    }

    #[test]
    fn matches_cut_wide_characters_at_their_edges() {
        // Two columns for each character: a match from column 2 to 6 is the 2nd and 3rd.
        let pieces = pieces("日本語です", 0, &cfg(), &[], &[(2, 6, 0)]);
        let got: Vec<(&str, u32, u32, Option<u8>)> = pieces
            .iter()
            .map(|p| (p.text.as_str(), p.col, p.cols, p.found))
            .collect();
        assert_eq!(
            got,
            [
                ("日", 0, 2, None),
                ("本語", 2, 4, Some(0)),
                ("です", 6, 4, None)
            ]
        );
    }

    #[test]
    fn thousands_of_matches_on_a_row_are_cut_in_a_short_time() {
        // Every other character is a match: 10 000 matches on a row of 20 000 columns.
        let text = "ab".repeat(10_000);
        let found: Vec<FoundRange> = (0..10_000).map(|i| (2 * i, 2 * i + 1, 0)).collect();
        let started = std::time::Instant::now();
        let pieces = pieces(&text, 0, &cfg(), &[], &found);
        assert_eq!(pieces.len(), 20_000);
        assert!(pieces[0].found.is_some() && pieces[1].found.is_none());
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
    }

    #[test]
    fn a_match_covering_a_tab_marks_the_tab() {
        let pieces = pieces("a\tb", 0, &cfg(), &[], &[(1, 4, 0)]);
        assert!(pieces[0].found.is_none());
        assert!(pieces[1].found.is_some());
        assert!(pieces[2].found.is_none());
    }

    #[test]
    fn a_wrapped_row_uses_the_columns_it_starts_at() {
        let pieces = pieces("ab", 4, &cfg(), &[], &[(5, 6, 0)]);
        let got: Vec<(&str, u32, Option<u8>)> = pieces
            .iter()
            .map(|p| (p.text.as_str(), p.col, p.found))
            .collect();
        assert_eq!(got, [("a", 4, None), ("b", 5, Some(0))]);
    }

    #[test]
    fn a_color_with_the_background_of_text_has_the_background_of_the_row() {
        let (colors, rules) = defaults();
        // Sakura's default comment color is green on the background of the text.
        let style = Style::of(rules.entry(ColorKind::Comment), &colors);
        assert_eq!(style.back, None);
        assert_eq!(style.text, 0x008000);
        // The number color too; the match of the search has its own background.
        let found = entry_of(&colors, &rules, ColorKind::Comment, Some(0));
        let style = Style::of(&found, &colors);
        assert_eq!(style.back, Some(0xffff00));
        assert_eq!(style.text, 0x000000);
    }

    #[test]
    fn keywords_and_urls_have_their_entries() {
        let (colors, rules) = defaults();
        let kw = Style::of(
            &entry_of(&colors, &rules, ColorKind::Keyword(0), None),
            &colors,
        );
        assert_eq!(kw.text, 0x0000ff);
        let url = Style::of(&entry_of(&colors, &rules, ColorKind::Url, None), &colors);
        assert_eq!(url.text, 0x0000ff);
        assert!(url.underline);
        // KW2 is orange.
        let kw2 = Style::of(
            &entry_of(&colors, &rules, ColorKind::Keyword(1), None),
            &colors,
        );
        assert_eq!(kw2.text, 0xff8000);
    }

    #[test]
    fn matches_take_the_search_colors_by_their_number() {
        let (mut colors, rules) = defaults();
        let entry = |text| ColorEntry {
            disp: true,
            bold: false,
            underline: false,
            text,
            back: 0x00ff_ffff,
        };
        colors.found = FoundColors::new([entry(0x01), entry(0x02)]);
        let text_of = |slot| entry_of(&colors, &rules, ColorKind::Text, Some(slot)).text;
        assert_eq!((text_of(0), text_of(1), text_of(2)), (0x01, 0x02, 0x01));
        // in a row: the second match has the second color
        let styling = style(&colors, &rules, "ab cd", &[], &[(0, 2, 0), (3, 5, 1)], None);
        let colors_of: Vec<Rgb> = styling.items.iter().map(|i| i.style.text).collect();
        assert_eq!(colors_of, [rgb_of(0x01), colors.text, rgb_of(0x02)]);
    }

    #[test]
    fn a_found_color_that_is_off_leaves_the_syntax_color() {
        let (mut colors, rules) = defaults();
        colors.found = FoundColors::new([]);
        let got = entry_of(&colors, &rules, ColorKind::Comment, Some(0));
        assert_eq!(got, *rules.entry(ColorKind::Comment));
    }

    #[test]
    fn rows_have_the_background_of_the_text_unless_the_caret_line_or_odd_lines_are_on() {
        let (colors, _) = defaults();
        assert_eq!(row_background(&colors, 3, true), colors.background);
        let on = Colors {
            caret_line_back: Some(0xffff80),
            even_line_back: Some(0xf3f3f3),
            ..colors
        };
        assert_eq!(row_background(&on, 3, true), 0xffff80);
        assert_eq!(row_background(&on, 3, false), 0xf3f3f3);
        assert_eq!(row_background(&on, 4, false), on.background);
        assert_eq!(row_background(&on, 4, true), 0xffff80);
    }

    #[test]
    fn selected_text_mixes_its_color_with_the_color_under_it() {
        let (colors, rules) = defaults();
        // The default selection has the same text and background: a mix with what is under.
        assert_eq!(
            colors.selection_entry.text, colors.selection_entry.back,
            "Sakura's default"
        );
        let plain = selected_style(&colors, rules.entry(ColorKind::Text), colors.background);
        let comment = selected_style(&colors, rules.entry(ColorKind::Comment), colors.background);
        // The same as the selection of text without coloring (the old behaviour of the window).
        assert_eq!(plain.back, Some(colors.selection));
        assert_eq!(plain.text, colors.selection_text);
        // Over green text the text of the selection is another color.
        assert_ne!(comment.text, plain.text);
        assert_eq!(comment.back, plain.back);
    }

    #[test]
    fn a_selection_with_two_colors_is_used_as_it_is_with_its_own_font() {
        let (mut colors, rules) = defaults();
        colors.selection_entry = ColorEntry {
            disp: true,
            bold: true,
            underline: false,
            text: 0x00ffffff,
            back: 0x00800000,
        };
        let style = selected_style(
            &colors,
            rules.entry(ColorKind::Keyword(0)),
            colors.background,
        );
        assert_eq!(style.text, 0xffffff);
        assert_eq!(style.back, Some(0x000080));
        assert!(style.bold);
        assert!(!style.underline);
    }

    #[test]
    fn a_selection_that_is_not_shown_inverts_the_colors() {
        let (mut colors, rules) = defaults();
        colors.selection_entry.disp = false;
        let style = selected_style(&colors, rules.entry(ColorKind::Text), colors.background);
        assert_eq!(style.text, 0xffffff);
        assert_eq!(style.back, Some(colors.background ^ 0xffffff));
    }

    #[test]
    fn selected_text_over_the_caret_line_mixes_with_the_row() {
        let (colors, rules) = defaults();
        let on = Colors {
            caret_line_back: Some(0xffff80),
            ..colors
        };
        let plain_row = selected_style(&on, rules.entry(ColorKind::Text), on.background);
        let caret_row = selected_style(&on, rules.entry(ColorKind::Text), 0xffff80);
        assert_ne!(plain_row.back, caret_row.back);
    }

    #[test]
    fn the_mark_of_a_tab_keeps_its_color_and_follows_the_text_for_the_rest() {
        let (colors, rules) = defaults();
        let tab = *ume_config::settings::ColorSet::default()
            .get("TAB")
            .unwrap();
        // Gray mark on the text's background: gray, over the background of the text around it.
        let in_text = space_style(&tab, rules.entry(ColorKind::Text), &colors);
        assert_eq!(in_text.style.text, 0x808080);
        assert_eq!(in_text.style.back, None);
        assert_eq!(in_text.around_underline, None);
        // In a comment of another background, the mark is on that background.
        let mut comment = *rules.entry(ColorKind::Comment);
        comment.back = 0x00ffe0e0;
        let in_comment = space_style(&tab, &comment, &colors);
        assert_eq!(in_comment.style.text, 0x808080);
        assert_eq!(in_comment.style.back, Some(rgb_of(0x00ffe0e0)));
        // A mark in the color of the text takes the color around it.
        let mut same = tab;
        same.text = colors.text_entry.text;
        let same_style = space_style(&same, &comment, &colors);
        assert_eq!(same_style.style.text, rgb_of(comment.text));
        // Bold if the text around is bold; an underline around is drawn in the text's color.
        let mut url = *rules.entry(ColorKind::Url);
        url.bold = true;
        let in_url = space_style(&tab, &url, &colors);
        assert!(in_url.style.bold);
        assert!(!in_url.style.underline);
        assert_eq!(in_url.around_underline, Some(rgb_of(url.text)));
    }

    fn style(
        colors: &Colors,
        rules: &ColorRules,
        text: &str,
        runs: &[(usize, usize, ColorKind)],
        found: &[FoundRange],
        selected: Option<(u32, u32)>,
    ) -> RowStyling {
        style_row(
            colors,
            rules,
            &RowInput {
                text,
                start_col: 0,
                cfg: &cfg(),
                runs,
                found,
                selected,
                row_back: colors.background,
            },
        )
    }

    fn texts(styling: &RowStyling) -> Vec<(String, Rgb)> {
        styling
            .items
            .iter()
            .map(|item| {
                let label = match &item.label {
                    Label::Text(t) => t.clone(),
                    Label::Tab => "<tab>".to_owned(),
                    Label::FullWidthSpace => "<zen>".to_owned(),
                };
                (label, item.style.text)
            })
            .collect()
    }

    #[test]
    fn each_piece_of_a_row_has_the_color_of_its_run() {
        let (colors, rules) = defaults();
        let runs = [
            (0, 3, ColorKind::Keyword(0)),
            (3, 4, ColorKind::Text),
            (4, 8, ColorKind::Comment),
        ];
        let styling = style(&colors, &rules, "int //ab", &runs, &[], None);
        assert_eq!(
            texts(&styling),
            [
                ("int".to_owned(), 0x0000ff),
                (" ".to_owned(), 0x000000),
                ("//ab".to_owned(), 0x008000),
            ]
        );
        // No piece has a background of its own and nothing is selected.
        assert!(styling.backs.is_empty());
        assert!(styling.selected_backs.is_empty());
        assert!(styling.items.iter().all(|i| i.selected.is_none()));
    }

    #[test]
    fn a_match_of_the_search_has_its_own_colors_over_the_syntax_colors() {
        let (colors, rules) = defaults();
        let runs = [(0, 8, ColorKind::Comment)];
        let styling = style(&colors, &rules, "abcdefgh", &runs, &[(2, 5, 0)], None);
        assert_eq!(
            texts(&styling),
            [
                ("ab".to_owned(), 0x008000),
                ("cde".to_owned(), 0x000000),
                ("fgh".to_owned(), 0x008000),
            ]
        );
        assert_eq!(styling.backs, [(2, 5, 0xffff00)]);
    }

    #[test]
    fn selected_pieces_get_the_selection_mixed_with_their_own_colors() {
        let (colors, rules) = defaults();
        let runs = [(0, 4, ColorKind::Text), (4, 8, ColorKind::Comment)];
        let styling = style(&colors, &rules, "abcdefgh", &runs, &[], Some((2, 6)));
        // Pieces are cut by color, not by the selection: the first (`abcd`) is selected from
        // column 2, the second (`efgh`) up to column 6; the painter clips them.
        let selected: Vec<bool> = styling.items.iter().map(|i| i.selected.is_some()).collect();
        assert_eq!(selected, [true, true]);
        assert_eq!(styling.selected_backs.len(), 2);
        assert_eq!(
            (styling.selected_backs[0].0, styling.selected_backs[0].1),
            (2, 4)
        );
        assert_eq!(
            (styling.selected_backs[1].0, styling.selected_backs[1].1),
            (4, 6)
        );
        // Text and comment under the same selection color mix to different text colors.
        let [text, comment] = &styling.items[..] else {
            panic!("two pieces");
        };
        assert_ne!(text.selected.unwrap().text, comment.selected.unwrap().text);
        // The selected background is the selection's mix with the background under it: the same
        // here for both, because both have the background of the text.
        assert_eq!(styling.selected_backs[0].2, styling.selected_backs[1].2);
        assert_eq!(styling.selected_backs[0].2, colors.selection);
    }

    #[test]
    fn a_tab_that_is_not_shown_draws_nothing_but_keeps_the_background() {
        let (mut colors, rules) = defaults();
        colors.tab_entry.disp = false;
        let runs = [(0, 2, ColorKind::Comment)];
        let styling = style(&colors, &rules, "a\t", &runs, &[], None);
        assert_eq!(
            texts(&styling),
            [("a".to_owned(), 0x008000)],
            "the tab is not an item"
        );
        // With the default comment background (the text's) there is nothing to fill.
        assert!(styling.backs.is_empty());
        // A tab that is shown is an item with the color of its own.
        let (colors, rules) = defaults();
        let styling = style(&colors, &rules, "a\t", &runs, &[], None);
        assert_eq!(
            texts(&styling),
            [("a".to_owned(), 0x008000), ("<tab>".to_owned(), 0x808080)]
        );
        assert_eq!(styling.items[1].col, 1);
    }

    #[test]
    fn a_selected_tab_that_is_not_shown_still_shows_the_selection() {
        let (mut colors, rules) = defaults();
        colors.tab_entry.disp = false;
        let styling = style(&colors, &rules, "a\tb", &[], &[], Some((0, 4)));
        assert_eq!(styling.items.len(), 2);
        // The selection background under the tab cell: columns 1 to 4.
        assert!(styling.selected_backs.contains(&(1, 4, colors.selection)));
    }

    #[test]
    fn full_width_spaces_follow_the_same_rules() {
        let (colors, rules) = defaults();
        let runs = [(0, 3, ColorKind::Comment)];
        let styling = style(&colors, &rules, "\u{3000}", &runs, &[], None);
        assert_eq!(texts(&styling), [("<zen>".to_owned(), 0xc0c0c0)]);
        assert_eq!(styling.items[0].label, Label::FullWidthSpace);
        assert_eq!(styling.items[0].col, 0);
    }

    #[test]
    fn an_underlined_run_underlines_the_marks_in_it() {
        let (colors, rules) = defaults();
        // URLs are underlined in Sakura's defaults.
        let runs = [(0, 3, ColorKind::Url)];
        let styling = style(&colors, &rules, "a\tb", &runs, &[], None);
        assert_eq!(styling.underlines, [(1, 4, 0x0000ff)]);
        assert!(styling.items[0].style.underline);
        // The mark itself is not underlined; the line under it comes from the run.
        assert!(!styling.items[1].style.underline);
    }
}
