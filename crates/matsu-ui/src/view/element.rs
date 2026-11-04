//! The element that paints the rows in view: line numbers, selection, text, Sakura's marks, the
//! caret, the IME underline and the scroll bar. It shapes only the rows on the screen.
//!
//! Every position comes from the layout in columns and is turned into pixels with the width of a
//! half-width character, so the caret and the selection stay on the grid whatever the font does.
//! Sakura draws the marks that this paints in `sakura_core/view/figures/` (`CFigure_Tab.cpp`,
//! `CFigure_ZenSpace.cpp`, `CFigure_Eol.cpp`).
//!
//! The text is cut into pieces of one color (comments, strings, numbers, keywords, the matches of
//! the search; see `syntax.rs` for the rules, `matsu-coloring` for the strategies), and each piece is
//! drawn with its own colors, bold and underline. Sakura's `CEditView::DrawLayoutLine` does this.
//!
//! Over and around the text it draws the ruler (`ruler.rs`), the vertical lines, the note lines,
//! the underline of the caret line and the line of the caret column (`lines.rs`), and the
//! highlight of the bracket pair (`bracket_marks.rs`).

use gpui::{
    App, Bounds, ContentMask, Element, ElementId, ElementInputHandler, Entity, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, ShapedLine, SharedString, Style, TextAlign,
    TextRun, UnderlineStyle, Window, fill, point, px, relative, rgb, size,
};
use matsu_coloring::ColorState;
use matsu_core::buffer::Eol;
use matsu_layout::{LayoutConfig, RowInfo};
use matsu_print::LineTerm;

use super::caret_shape::{CaretShape, caret_shape};
use super::colors::LineColorRequest;
use super::syntax::{self, Label, Style as TextStyle};
use super::{EditorView, PaintInfo, a11y_text, bracket_marks, diff_mark, lines, ruler};
use crate::geometry::{
    HSCROLL_ARROW, HSCROLLBAR_HEIGHT, HScrollbar, Metrics, SCROLLBAR_WIDTH, Scrollbar,
    column_width, row_height,
};
use crate::hit::selected_columns;
use crate::search::highlight::{highlight_columns, slot_of};
use crate::settings::view_settings::{TabMark, rgb_of};
use crate::settings::{Colors, DiffColor, Marks};
use matsu_config::settings::color::ColorEntry;
use matsu_core::diff::DiffMark;

/// The caret of overwrite mode: the caret color with some transparency.
const CARET_BLOCK_ALPHA: u32 = 0x70;

/// The mark for an ideographic space: a box (`CFigure_ZenSpace.cpp`).
const FULL_WIDTH_SPACE_MARK: &str = "□";
/// The mark for a line break, by kind. Sakura draws an arrow bent to the lower left for CRLF, an
/// arrow to the left for CR, an arrow down for LF, and a straight arrow to the lower left for NEL,
/// LS and PS (`_DrawEOL` in `sakura_core/view/figures/CFigure_Eol.cpp`).
fn eol_mark(eol: Eol) -> &'static str {
    match eol {
        Eol::CrLf => "↵",
        Eol::Cr => "←",
        Eol::Lf => "↓",
        Eol::Nel | Eol::Ls | Eol::Ps => "↙",
    }
}

const EOF_MARK: &str = "[EOF]";

pub(super) struct EditorElement {
    editor: Entity<EditorView>,
}

impl EditorElement {
    pub(super) fn new(editor: Entity<EditorView>) -> Self {
        Self { editor }
    }
}

/// The most columns of a tab that its mark is made for. A tab in a wide field of the TSV or CSV
/// mode can be as wide as the field.
const MAX_TAB_MARK_COLUMNS: u32 = 1024;

/// A long arrow over a tab wider than [`MAX_TAB_MARK_COLUMNS`]: its head is at the end.
fn wide_long_arrow(mark: &TabMark, cols: u32) -> bool {
    cols > MAX_TAB_MARK_COLUMNS && matches!(mark, TabMark::LongArrow)
}

/// A shaped piece and the column where it goes. A character of spaced text has its underline
/// apart from the glyph, over all its columns, so that the underline has no gaps; a space has
/// no glyph.
struct Placed {
    col: u32,
    line: Option<ShapedLine>,
    /// The columns the underline covers and its color.
    underline: Option<(u32, u32)>,
}

/// Paints a piece of text: its glyphs at its column, then its underline.
fn paint_placed(
    window: &mut Window,
    cx: &mut App,
    placed: &Placed,
    x_of: &dyn Fn(u32) -> Pixels,
    y: Pixels,
    lh: Pixels,
) {
    if let Some(line) = &placed.line {
        line.paint(
            point(x_of(placed.col), y),
            lh,
            TextAlign::Left,
            None,
            window,
            cx,
        )
        .ok();
    }
    if let Some((cols, color)) = placed.underline {
        window.paint_quad(fill(
            Bounds::from_corners(
                point(x_of(placed.col), y + lh - px(3.)),
                point(x_of(placed.col + cols), y + lh - px(2.)),
            ),
            rgb(color),
        ));
    }
}

/// How the line number area of a row shows a bookmark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GutterMark {
    None,
    /// The area has the background of the bookmark color (and the number its text color).
    Background,
    /// A vertical line two pixels wide at the left edge, in the color of the line number.
    Line,
}

/// Sakura: `CTextDrawer::DispLineNumber` (`sakura_core/view/CTextDrawer.cpp`). When the bookmark
/// color is shown, the line number is drawn in it, which needs the line numbers to be shown
/// (the area is filled by the text drawing); when it is not shown, a bookmark is the vertical
/// line, whether the line numbers are shown or not.
fn gutter_mark(bookmarked: bool, colored: bool, line_numbers: bool) -> GutterMark {
    match (bookmarked, colored, line_numbers) {
        (false, ..) | (true, true, false) => GutterMark::None,
        (true, true, true) => GutterMark::Background,
        (true, false, _) => GutterMark::Line,
    }
}

/// How the line number of a row is drawn.
///
/// Sakura: `CTextDrawer::DispLineNumber` (`sakura_core/view/CTextDrawer.cpp`). The color of a
/// bookmark (when it is shown) wins over the plain line number color. A modified line (with the
/// color 変更行 shown, and the document modified) takes the color of 変更行; when a bookmark's
/// color is used, the 変更行 text color replaces it only if the bookmark's text color is the line
/// number's own, and the same for the background and for bold. `background` is the fill of the
/// line number area of the row, if it is not the area's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GutterStyle {
    text: u32,
    background: Option<u32>,
    bold: bool,
}

/// The colors of the DIFF mark of a line, if they are shown (`CDiffLineGetter::GetDiffColor`).
fn diff_color(colors: &Colors, mark: DiffMark) -> Option<DiffColor> {
    let color = match mark {
        DiffMark::Append => colors.diff_append,
        DiffMark::Change => colors.diff_change,
        DiffMark::Delete | DiffMark::DeleteEx => colors.diff_delete,
    };
    color.shown.then_some(color)
}

/// Which entry decided the colors of a line number.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ColorSource {
    Plain,
    Modified,
    Other,
}

/// The colors are chosen in this order, the last that applies wins: the line number, a modified
/// line, a DIFF mark, a bookmark (`CTextDrawer::DispLineNumber`). A modified line passes its
/// colors on to a line that has another color of its own, where that color is the line number's
/// own (the DIFF and bookmark entries of the default colors do not differ in bold, for example).
fn gutter_style(
    colors: &Colors,
    marks: &Marks,
    bookmarked: bool,
    modified: bool,
    diff: Option<DiffMark>,
    line_numbers: bool,
) -> GutterStyle {
    let modified = modified && marks.modified;
    let bookmark_colored = bookmarked && marks.bookmark;
    let mut source = ColorSource::Plain;
    let (mut text, mut back, mut bold) = (
        colors.line_number,
        colors.line_number_background,
        colors.line_number_bold,
    );
    if modified {
        (text, back, bold) = (
            colors.modified,
            colors.modified_background,
            colors.modified_bold,
        );
        source = ColorSource::Modified;
    }
    if let Some(color) = diff.and_then(|mark| diff_color(colors, mark)) {
        (text, back, bold) = (color.text, color.background, color.bold);
        source = ColorSource::Other;
    }
    if bookmark_colored {
        (text, back, bold) = (
            colors.bookmark,
            colors.bookmark_background,
            colors.bookmark_bold,
        );
        source = ColorSource::Other;
    }
    if modified && source == ColorSource::Other {
        if colors.line_number == text {
            text = colors.modified;
        }
        if colors.line_number_background == back {
            back = colors.modified_background;
        }
        if colors.line_number_bold == bold {
            bold = colors.modified_bold;
        }
    }
    GutterStyle {
        text,
        // the color is drawn with the line number, which is not there without line numbers
        background: (line_numbers && source != ColorSource::Plain).then_some(back),
        bold,
    }
}

/// What follows the line number of a row (`CTextDrawer::DispLineNumber`, 行番号区切り).
enum Term {
    /// A vertical line at the right edge of the line number area, in the color of the number.
    Line,
    /// A character after the number.
    Char(Box<ShapedLine>),
}

struct RowPaint {
    selection: Option<(u32, u32)>,
    /// The line number pieces and how far right of the left edge of the numbers each goes.
    numbers: Vec<(f32, ShapedLine)>,
    /// The line number separator of the row, if the type has one and the row has a layout.
    term: Option<Term>,
    /// The color of the line number (what the separator line is drawn in).
    number_color: u32,
    /// Where the note line starts in the line number area: the left edge when the area has the
    /// background of the text, else the right edge of the numbers (`DispNoteLine` from
    /// `DispLineNumber`).
    note_from: f32,
    /// The line of this row has a bookmark (every row of a wrapped line shows it, as in Sakura:
    /// `CTextDrawer::DispLineNumber` looks at the line of each layout row).
    bookmarked: bool,
    /// The fill of the line number area of this row (see [`GutterStyle`]).
    gutter_background: Option<u32>,
    /// The DIFF mark of the line and the color it is drawn in (the text color of the line number).
    diff: Option<(DiffMark, u32)>,
    /// The background of the whole row, when it is not the one of the text: the caret line and
    /// the odd rows (`CBK`, `EBK`).
    row_background: Option<u32>,
    /// The backgrounds of pieces of text that are not the text's: from and to column, and color.
    backs: Vec<(u32, u32, u32)>,
    /// The backgrounds of the selected parts of the pieces.
    selected_backs: Vec<(u32, u32, u32)>,
    /// Underlines under the mark of a tab or a space, from the text around it.
    underlines: Vec<(u32, u32, u32)>,
    text: Vec<Placed>,
    /// The same pieces in the colors of selected text, when the row has a selection.
    selected_text: Vec<Placed>,
    marks: Vec<Placed>,
    /// The columns of the IME composition on this row.
    marked: Option<(u32, u32)>,
}

/// A bracket of the pair drawn in the color `BRC`.
struct BracketPaint {
    /// The index in `rows`.
    row: usize,
    col: u32,
    cols: u32,
    /// The fill under the bracket.
    back: u32,
    text: ShapedLine,
}

/// The ruler: its scale, and the numbers of the tens.
struct RulerPaint {
    height: f32,
    background: u32,
    color: u32,
    scale: ruler::Scale,
    numbers: Vec<(f32, ShapedLine)>,
    /// The mark of the caret: from x, to x, from y, to y; and its color.
    caret: Option<((f32, f32, f32, f32), u32)>,
}

/// The lines over the text, in pixels from the left of the text area.
#[derive(Default)]
struct LinesPaint {
    /// The x of the vertical lines at given columns, their color, and whether they are dotted.
    vertical: Vec<f32>,
    vertical_color: u32,
    vertical_dotted: bool,
    /// The y of a note line inside a row, and its color.
    note: Option<(f32, u32)>,
    /// The x of the line of the caret column (and the second one of a bold line), and its color.
    caret_column: Vec<f32>,
    caret_column_color: u32,
    /// The index in `rows` of the caret row, and the color of its underline.
    underline: Option<(usize, u32)>,
    /// The x of the vertical line at the wrap column (`RAP`), and its color.
    wrap: Option<(f32, u32)>,
}

pub(super) struct Prepaint {
    rows: Vec<RowPaint>,
    /// The space above the first row: the ruler and the space under it.
    top_margin: f32,
    /// The height of the ruler (0 when it is not shown), the top part of the margin.
    ruler_px: f32,
    ruler: Option<RulerPaint>,
    lines: LinesPaint,
    brackets: Vec<BracketPaint>,
    /// The index in `rows`, the column and the shape of the caret (`caret_shape`).
    caret: Option<(usize, u32, CaretShape)>,
    colors: Colors,
    show_line_numbers: bool,
    /// The width of the line numbers and what is right of them up to the text: the line number
    /// area without the space before the text (`CTextArea::GetLineNumberWidth`).
    number_area_width: f32,
    /// The digits of the line numbers.
    digits: usize,
    /// The bookmark color is shown (see `Marks::bookmark`).
    bookmark_colored: bool,
    cell: f32,
    line_height: f32,
    gutter_width: f32,
    y_offset: f32,
    widest: u32,
    total_rows: usize,
    /// The height of the horizontal scroll bar at the bottom: 0 when the view has none.
    hbar_height: f32,
    /// The right end of the selection in columns, for the range of the horizontal scroll bar.
    selection_edge: u32,
    /// The background image of the type, if it has one that is read and not fully transparent.
    back_image: Option<super::back_paint::BackPaint>,
}

impl IntoElement for EditorElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

/// Puts the shaped `label` (at `col`, in `style`) in `out`: as one piece, or, when `width_of` is
/// given (the columns have spacing), as a piece for each character, `width_of` columns apart. A
/// font cannot space its glyphs, and Sakura draws each character of a row at its own place
/// (`CTextDrawer::DispText` with the dx array).
fn place(
    window: &mut Window,
    out: &mut Vec<Placed>,
    (col, label, style): (u32, &str, &TextStyle),
    width_of: Option<impl Fn(char) -> u32>,
    shape: &dyn Fn(&mut Window, &str, &TextStyle) -> ShapedLine,
) {
    let Some(width_of) = width_of else {
        out.push(Placed {
            col,
            line: Some(shape(window, label, style)),
            underline: None,
        });
        return;
    };
    // Each character is shaped alone, so its underline would stop at the glyph: the underline is
    // a line across the columns of the character instead.
    let glyph_style = TextStyle {
        underline: false,
        ..*style
    };
    let mut col = col;
    for c in label.chars() {
        let cols = width_of(c);
        let underline = style.underline.then_some((cols, style.text));
        if c.is_whitespace() {
            // A space has nothing to paint but its underline, which joins the one before it:
            // a tab in a wide field has a long run of them.
            match (underline, out.last_mut()) {
                (None, _) => {
                    col += cols;
                    continue;
                }
                (Some(_), Some(last))
                    if last.line.is_none()
                        && last.underline.is_some_and(|(n, color)| {
                            last.col + n == col && color == style.text
                        }) =>
                {
                    last.underline = last.underline.map(|(n, color)| (n + cols, color));
                    col += cols;
                    continue;
                }
                _ => {}
            }
        }
        out.push(Placed {
            col,
            line: (!c.is_whitespace())
                .then(|| shape(window, c.encode_utf8(&mut [0; 4]), &glyph_style)),
            underline,
        });
        col += cols;
    }
}

fn run(font: &gpui::Font, len: usize, color: u32) -> TextRun {
    TextRun {
        len,
        font: font.clone(),
        color: rgb(color).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    }
}

/// A run for text in a style: its color, bold (`bold_font`) and underline.
/// `_DispEOF` (`view/figures/CFigure_Eol.cpp`): [EOF] in the colors of its own entry, the text and
/// the background (the text is opaque), without borrowing from the text around it.
fn eof_style(entry: &ColorEntry) -> TextStyle {
    TextStyle {
        text: rgb_of(entry.text),
        back: Some(rgb_of(entry.back)),
        bold: entry.bold,
        underline: entry.underline,
    }
}

fn styled_run(font: &gpui::Font, bold_font: &gpui::Font, len: usize, style: &TextStyle) -> TextRun {
    TextRun {
        underline: style.underline.then(|| UnderlineStyle {
            thickness: px(1.),
            color: Some(rgb(style.text).into()),
            wavy: false,
        }),
        ..run(if style.bold { bold_font } else { font }, len, style.text)
    }
}

impl Element for EditorElement {
    type RequestLayoutState = ();
    type PrepaintState = Prepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Prepaint {
        let editor = self.editor.read(cx);
        let doc = editor.document.read(cx).doc();
        let layout = &editor.layout;
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let font = style.font();
        let settings = &editor.prefs.settings;
        // The row is the height of the font's line and the line spacing of the type, which can be
        // negative (`CTextMetrics::Update`).
        let line_height = row_height(f32::from(window.line_height()), settings.line_space);
        let colors = settings.colors;
        let rules = &settings.coloring;
        let marks_shown = settings.marks;
        let show_line_numbers = settings.show_line_numbers;

        let shape = |window: &mut Window, text: &str, color: u32| -> ShapedLine {
            let text: SharedString = text.to_owned().into();
            let runs = [run(&font, text.len(), color)];
            window
                .text_system()
                .shape_line(text, font_size, &runs, None)
        };
        let bold_font = gpui::Font {
            weight: gpui::FontWeight::BOLD,
            ..font.clone()
        };
        let shape_number = |window: &mut Window, text: &str, color: u32, bold: bool| {
            let text: SharedString = text.to_owned().into();
            let runs = [run(
                if bold { &bold_font } else { &font },
                text.len(),
                color,
            )];
            window
                .text_system()
                .shape_line(text, font_size, &runs, None)
        };

        let shape_styled = |window: &mut Window, text: &str, style: &TextStyle| -> ShapedLine {
            let text: SharedString = text.to_owned().into();
            let runs = [styled_run(&font, &bold_font, text.len(), style)];
            window
                .text_system()
                .shape_line(text, font_size, &runs, None)
        };

        // One column is the advance of a half-width character and the character spacing of the
        // type (`m_nDxBasis`). The text is placed one character at a time when there is spacing.
        let half = f32::from(shape(window, "0000000000", colors.text).width()) / 10.0;
        let half = if half > 0.0 { half } else { 8.0 };
        let cell = column_width(half, settings.column_space);
        let spaced = settings.column_space > 0;
        let widths = layout.config().width;

        let first_row = (editor.scroll_y / line_height).floor().max(0.0) as usize;
        let y_offset = -(editor.scroll_y - line_height * first_row as f32);
        let top_margin = settings.top_margin() as f32;
        // The horizontal scroll bar takes the bottom of the view (`CreateScrollBar`).
        let hbar_height = if super::hscroll::shown(cx) {
            HSCROLLBAR_HEIGHT
        } else {
            0.0
        };
        let body_height = (f32::from(bounds.size.height) - top_margin - hbar_height).max(0.0);
        let count = (body_height / line_height).ceil() as usize + 1;
        let mut rows: Vec<RowInfo> = layout.rows(doc, first_row, count).unwrap_or_default();

        // `DetectWidthOfLineNumberArea_calculate`: the digits of the last number, at least
        // `nLineNumWidth`, and a blank before them. The last number counts the lines of the text,
        // or the rows of the wrapped view (the empty row after the last line ending is no line).
        let numbers_by_line = settings.outline.line_num_is_crlf;
        let eof_row = usize::from(layout.has_eof_row());
        let last_number = if numbers_by_line {
            doc.line_count().saturating_sub(eof_row)
        } else {
            layout.row_count().saturating_sub(eof_row)
        };
        let digits = lines::line_number_columns(last_number, settings.line_num_width);
        // The text starts `nLineNumberRightSpace` pixels after the line numbers
        // (`CTextArea::DetectWidthOfLineNumberArea`: `nViewAlignLeftNew += GetLeftYohaku()`).
        let gutter_width = settings.line_number_right_space as f32
            + if show_line_numbers {
                cell * (digits as f32 + 1.0) + 4.0
            } else {
                4.0
            };

        let number_area_width = gutter_width - settings.line_number_right_space as f32;
        let box_area = crate::hit::BoxArea::of_cursor(layout, doc, &editor.cursor);
        let selection = editor
            .cursor
            .selection()
            .filter(|_| !editor.cursor.is_box());
        let marked = editor.marked.clone();
        let last_line = doc.line_count().saturating_sub(1);

        // The search colors: a whole-word search numbers the words of its key, and each word has
        // the next color that is shown (`CColor_Found`).
        let search = &editor.search;
        let shown_found = colors.found.len();
        let words = search.plain().filter(|_| search.options().whole_word);
        let mut found = match search
            .matcher()
            .filter(|_| search.mark() && shown_found > 0)
        {
            Some(matcher) => highlight_columns(layout, doc, &rows, &**matcher, &|text| {
                slot_of(words.and_then(|w| w.word_slot(text)), shown_found)
            }),
            None => Vec::new(),
        }
        .into_iter();
        let model = editor.document.read(cx).model();
        let bookmarks = model.bookmarks();
        let diff_marks = model.diff_marks();
        let mut widest = 0;
        let mut painted = Vec::with_capacity(rows.len());
        let caret_row_index = layout
            .pos_to_row(doc, editor.cursor.caret())
            .ok()
            .map(|at| at.row);
        // The colors of the line the rows are in, and the state the next line starts in: lines
        // come one after the other, so the state of the first is asked for and the others follow.
        let mut line_colors: Option<(usize, std::sync::Arc<Vec<matsu_coloring::ColorRun>>)> = None;
        let version = editor.document.read(cx).version();
        let mut carried: Option<(usize, ColorState)> = None;
        for (row_index, row) in rows.iter().enumerate() {
            widest = widest.max(row.end_column);
            let bookmarked = !row.is_eof_row && bookmarks.contains(row.line);
            // the empty row after the last line break is no line (Sakura has no layout for it)
            let modified = !row.is_eof_row && model.is_line_modified(row.line);
            let diff = (!row.is_eof_row)
                .then(|| diff_marks.get(row.line))
                .flatten();
            let style = gutter_style(
                &colors,
                &marks_shown,
                bookmarked,
                modified,
                diff,
                show_line_numbers,
            );
            // The digits are drawn a column apart (`DispLineNumber` uses the dx array of the font
            // with the spacing), so with spacing each digit has its own piece.
            let number = (show_line_numbers && !row.is_eof_row)
                .then(|| lines::line_number(numbers_by_line, row.row, row.line, row.row_in_line))
                .flatten();
            let numbers = if let Some(number) = number {
                let text = format!("{number:>digits$}");
                if spaced {
                    text.chars()
                        .enumerate()
                        .filter(|(_, c)| *c != ' ')
                        .map(|(i, c)| {
                            (
                                i as f32 * cell,
                                shape_number(window, &c.to_string(), style.text, style.bold),
                            )
                        })
                        .collect()
                } else {
                    vec![(0.0, shape_number(window, &text, style.text, style.bold))]
                }
            } else {
                Vec::new()
            };
            let selected = match &box_area {
                Some(area) => area.columns_on(row.row),
                None => {
                    selection.and_then(|s| selected_columns(layout, doc, row, s).ok().flatten())
                }
            };
            let row_found = found.next().unwrap_or_default();
            let row_back =
                syntax::row_background(&colors, row.row, caret_row_index == Some(row.row));

            // The runs of colors of this row: the line is scanned from its start, in the state
            // the lines before it left, as far as the last row of it on the screen.
            let mut eol_color = matsu_coloring::ColorKind::Text;
            let row_runs = if row.is_eof_row || rules.colors_nothing() {
                Vec::new()
            } else {
                if line_colors.as_ref().map(|l| l.0) != Some(row.line)
                    && let Ok(Some(ll)) = layout.line_layout(doc, row.line)
                {
                    let state = match carried.take() {
                        Some((line, state)) if line == row.line => state,
                        _ => editor.color_states.state_at(doc, rules, row.line).0,
                    };
                    let upto = rows[row_index..]
                        .iter()
                        .take_while(|r| r.line == row.line)
                        .map(|r| r.end.col)
                        .max()
                        .unwrap_or(0);
                    // The state of the next line is only needed if a next line is on the screen,
                    // and the states may have it already.
                    let want_next = rows[row_index..]
                        .iter()
                        .any(|r| !r.is_eof_row && r.line > row.line);
                    let next_known = (row.line < editor.color_states.known_lines())
                        .then(|| editor.color_states.state_at(doc, rules, row.line + 1).0);
                    let colored = editor.colored_line(LineColorRequest {
                        rules,
                        version,
                        line: row.line,
                        layout: &ll,
                        state: &state,
                        upto,
                        want_next,
                        next_known,
                        raw_len: super::colors::raw_len_of(doc, row.line),
                    });
                    carried = colored.next.map(|next| (row.line + 1, next));
                    line_colors = Some((row.line, colored.runs));
                }
                match &line_colors {
                    Some((line, runs)) if *line == row.line => {
                        if row.eol.is_some()
                            && let Some(last) = runs.last().filter(|r| r.end > row.end.col)
                        {
                            eol_color = last.kind;
                        }
                        syntax::row_runs(runs, row.start.col, row.end.col)
                    }
                    _ => Vec::new(),
                }
            };

            let styling = syntax::style_row(
                &colors,
                rules,
                &syntax::RowInput {
                    text: row.text(),
                    start_col: row.start_column,
                    cfg: layout.config(),
                    runs: &row_runs,
                    found: &row_found,
                    selected,
                    row_back,
                },
            );
            let mut text = Vec::new();
            let mut selected_text = Vec::new();
            for item in &styling.items {
                let tab_text;
                let mut label_col = item.col;
                let label = match &item.label {
                    Label::Text(text) => text.as_str(),
                    Label::Tab(cols) => {
                        // A tab in a very wide field of the TSV or CSV mode is drawn only where
                        // the window shows it; the head of a long arrow is placed at the true
                        // end of the tab below.
                        tab_text = if wide_long_arrow(&settings.tab_mark, *cols) {
                            let first = (editor.scroll_x / cell).floor().max(0.0) as u32;
                            let shown = (f32::from(bounds.size.width) / cell).ceil() as u32 + 2;
                            let from = item.col.max(first.saturating_sub(1));
                            let to = (item.col + *cols - 1).min(first.saturating_add(shown));
                            label_col = from;
                            "\u{2500}".repeat(to.saturating_sub(from) as usize)
                        } else {
                            settings.tab_mark.text((*cols).min(MAX_TAB_MARK_COLUMNS))
                        };
                        tab_text.as_str()
                    }
                    Label::FullWidthSpace => FULL_WIDTH_SPACE_MARK,
                };
                // The width of each character of the label in columns: the characters of text
                // have their own, and so do those of a custom tab mark; the arrows and the mark of a
                // space are a column each.
                let width_of = |c: char| match item.label {
                    Label::Text(_) => widths.columns(c),
                    // the arrows are a column each, whatever the width rule says of them
                    Label::Tab(_) if !matches!(c, '\u{2192}' | '\u{2500}') => widths.columns(c),
                    _ => 1,
                };
                if let Some(selected_style) = &item.selected {
                    place(
                        window,
                        &mut selected_text,
                        (label_col, label, selected_style),
                        spaced.then_some(width_of),
                        &shape_styled,
                    );
                }
                place(
                    window,
                    &mut text,
                    (label_col, label, &item.style),
                    spaced.then_some(width_of),
                    &shape_styled,
                );
                if let Label::Tab(cols) = item.label
                    && wide_long_arrow(&settings.tab_mark, cols)
                {
                    let head = (item.col + cols - 1, "\u{2192}");
                    if let Some(selected_style) = &item.selected {
                        place(
                            window,
                            &mut selected_text,
                            (head.0, head.1, selected_style),
                            spaced.then_some(|_: char| 1),
                            &shape_styled,
                        );
                    }
                    place(
                        window,
                        &mut text,
                        (head.0, head.1, &item.style),
                        spaced.then_some(|_: char| 1),
                        &shape_styled,
                    );
                }
            }
            let mut marks = Vec::new();
            let mut backs = styling.backs;
            let eof_style = eof_style(&colors.eof_entry);
            let eof_width = EOF_MARK.chars().count() as u32;
            if row.is_eof_row {
                if marks_shown.eof {
                    backs.push((0, eof_width, eof_style.back.unwrap_or_default()));
                    place(
                        window,
                        &mut marks,
                        (0, EOF_MARK, &eof_style),
                        spaced.then_some(|_: char| 1),
                        &shape_styled,
                    );
                }
            } else if let Some(eol) = row.eol {
                if marks_shown.eol {
                    // The mark is in its own color, or in the color of the text it ends.
                    // A search that matches the line break colors its mark (the match covers the
                    // line ending: `highlight_columns` gives the column of the mark).
                    let eol_found = row_found
                        .iter()
                        .find(|r| r.0 == row.end_column)
                        .map(|r| r.2);
                    let around = syntax::entry_of(&colors, rules, eol_color, eol_found);
                    let mark = syntax::space_style(&colors.eol_entry, &around, &colors);
                    if let (Some(_), Some(back)) = (eol_found, mark.style.back) {
                        backs.push((row.end_column, row.end_column + 1, back));
                    }
                    place(
                        window,
                        &mut marks,
                        (row.end_column, eol_mark(eol), &mark.style),
                        spaced.then_some(|_: char| 1),
                        &shape_styled,
                    );
                }
            } else if !row.wrapped && row.line == last_line && marks_shown.eof {
                backs.push((
                    row.end_column,
                    row.end_column + eof_width,
                    eof_style.back.unwrap_or_default(),
                ));
                place(
                    window,
                    &mut marks,
                    (row.end_column, EOF_MARK, &eof_style),
                    spaced.then_some(|_: char| 1),
                    &shape_styled,
                );
            }
            let marked_cols = marked
                .as_ref()
                .filter(|m| m.line == row.start.line)
                .and_then(|m| {
                    let a = matsu_core::buffer::Pos::new(m.line, m.range.start);
                    let b = matsu_core::buffer::Pos::new(m.line, m.range.end);
                    let (a, b) = (
                        layout.pos_to_row(doc, a).ok()?,
                        layout.pos_to_row(doc, b).ok()?,
                    );
                    (a.row == row.row && b.row == row.row).then_some((a.column, b.column))
                });
            let has_number_area = show_line_numbers && !row.is_eof_row;
            let term = if has_number_area {
                match settings.line_term {
                    LineTerm::Line => Some(Term::Line),
                    LineTerm::Char(c) if c != '\0' => Some(Term::Char(Box::new(shape_number(
                        window,
                        &c.to_string(),
                        style.text,
                        style.bold,
                    )))),
                    _ => None,
                }
            } else {
                None
            };
            let number_back = style.background.unwrap_or(colors.line_number_background);
            painted.push(RowPaint {
                selection: selected,
                numbers,
                term,
                number_color: style.text,
                note_from: lines::gutter_note_from(
                    has_number_area && number_back != colors.background,
                    number_area_width,
                ),
                bookmarked,
                gutter_background: style.background,
                diff: diff.filter(|_| show_line_numbers).map(|m| (m, style.text)),
                row_background: (row_back != colors.background).then_some(row_back),
                backs,
                selected_backs: styling.selected_backs,
                underlines: styling.underlines,
                text,
                selected_text,
                marks,
                marked: marked_cols,
            });
        }

        // In a box selection, and in the free cursor mode, the caret can be right of the end of
        // the text.
        let options = matsu_commands::edit_options::get();
        let caret_column = editor
            .cursor
            .box_caret_col()
            .or(editor.cursor.free_col().filter(|_| options.free_cursor))
            .map_or(0, |c| u32::try_from(c).unwrap_or(u32::MAX));
        // While the selected text is dragged, the caret shows where it would drop.
        let caret_at = editor.drop_caret.unwrap_or(editor.cursor.caret());
        let caret = layout.pos_to_row(doc, caret_at).ok().and_then(|at| {
            at.row
                .checked_sub(first_row)
                .filter(|i| *i < painted.len())
                .map(|i| {
                    let overwrite = editor.document.read(cx).overwrite();
                    let shape = caret_shape(
                        options.dos_caret,
                        overwrite,
                        overwrite_caret_cells(doc, layout, caret_at),
                        cell,
                        matsu_platform::caret_width::os_caret_width(),
                    );
                    (i, caret_column.max(at.column), shape)
                })
        });

        // ---- the ruler, the lines and the bracket pair
        let view_width = (f32::from(bounds.size.width) - gutter_width - SCROLLBAR_WIDTH).max(1.0);
        let scroll_x = editor.scroll_x;
        let max_columns = layout.config().max_columns();
        let caret_pos = editor.cursor.caret();
        // Text that is selected, or being selected, has no lines of the caret and no pair
        // (`IsTextSelecting`).
        let selecting = editor.cursor.has_selection() || editor.cursor.selecting_lock();
        let caret_total_column = layout
            .pos_to_row(doc, caret_pos)
            .ok()
            .map(|at| caret_column.max(at.column));

        // The right end of the selection, which the range of the horizontal scroll bar covers when
        // the text does not wrap (`GetRightEdgeForScrollBar`), and the caret of the free cursor.
        let selection_edge = editor
            .cursor
            .selection()
            .into_iter()
            .flat_map(|(from, to)| [from, to])
            .filter_map(|pos| layout.pos_to_row(doc, pos).ok().map(|at| at.column))
            .chain(options.free_cursor.then_some(caret_total_column).flatten())
            // A box selection can end right of the text: its corners have columns of their own.
            .chain(
                [
                    editor.cursor.box_anchor_col(),
                    editor.cursor.box_caret_col(),
                ]
                .into_iter()
                .flatten()
                .map(|c| u32::try_from(c).unwrap_or(u32::MAX)),
            )
            .max()
            .unwrap_or(0);

        let mut lines_paint = LinesPaint::default();
        let vertical = colors.vert_line_entry;
        if vertical.disp {
            lines_paint.vertical = lines::vertical_xs(
                &settings.vert_lines,
                cell,
                scroll_x,
                view_width,
                max_columns,
                settings.vert_line_offset,
                vertical.bold,
            );
            lines_paint.vertical_dotted = vertical.underline;
            // A line in the color of the text's background is drawn by XOR to show.
            lines_paint.vertical_color = if vertical.text == colors.text_entry.back {
                lines::exor_color(rgb_of(colors.text_entry.back), rgb_of(vertical.back))
            } else {
                rgb_of(vertical.text)
            };
        }
        if colors.wrap_entry.disp {
            lines_paint.wrap = lines::wrap_line_x(max_columns, cell, scroll_x, view_width)
                .map(|x| (x, rgb_of(colors.wrap_entry.text)));
        }
        if colors.note_line_entry.disp {
            lines_paint.note = Some((
                lines::note_line_y(settings.note_line_offset, line_height),
                rgb_of(colors.note_line_entry.text),
            ));
        }
        let caret_line = colors.cursor_vline_entry;
        if caret_line.disp
            && !selecting
            && let Some(x) = caret_total_column
                .and_then(|c| lines::cursor_vline_x(c, cell, scroll_x, view_width))
        {
            lines_paint.caret_column.push(x);
            if caret_line.bold
                && let Some(second) = lines::cursor_vline_bold_x(x, view_width)
            {
                lines_paint.caret_column.push(second);
            }
            lines_paint.caret_column_color = rgb_of(caret_line.text);
        }
        if colors.underline_entry.disp
            && !selecting
            && let Some((i, _, _)) = caret
        {
            lines_paint.underline = Some((i, rgb_of(colors.underline_entry.text)));
        }

        let ruler_entry = colors.ruler_entry;
        let ruler_paint = (ruler_entry.disp && settings.ruler_px() > 0).then(|| {
            let height = settings.ruler_px();
            let scale = ruler::scale(height, cell, scroll_x, view_width, max_columns);
            let color = rgb_of(ruler_entry.text);
            let background = rgb_of(ruler_entry.back);
            // The numbers are a little smaller than the ruler is high (`lfHeight = 1 - height`).
            let size = px(height.saturating_sub(1).max(1) as f32);
            let numbers = scale
                .ticks
                .iter()
                .filter_map(|tick| {
                    let number: SharedString = tick.label?.to_string().into();
                    let runs = [run(&font, number.len(), color)];
                    let shaped = window.text_system().shape_line(number, size, &runs, None);
                    Some((tick.x + 2.0, shaped))
                })
                .collect();
            let focused = editor.focus_handle.is_focused(window);
            RulerPaint {
                height: height as f32,
                background,
                color,
                scale,
                numbers,
                caret: caret_total_column
                    .and_then(|c| ruler::caret_mark(height, c, cell, scroll_x, view_width))
                    .map(|mark| (mark, ruler::caret_color(background, focused))),
            }
        });

        let mut brackets = Vec::new();
        let bracket_entry = colors.bracket_entry;
        if bracket_entry.disp && !selecting && editor.focus_handle.is_focused(window) {
            // Every row whose top is above the bottom edge, the first one being cut by the scroll.
            let rows_shown = ((body_height - y_offset) / line_height).ceil().max(1.0) as usize;
            let screen = bracket_marks::Screen {
                first_row,
                last_row: first_row + rows_shown - 1,
                first_col: (scroll_x / cell).floor().max(0.0) as u32,
                last_col: ((scroll_x + view_width) / cell).floor() as u32,
            };
            let bracket_style = TextStyle::of(&bracket_entry, &colors);
            for mark in bracket_marks::marks(doc, layout, caret_pos, screen) {
                let Some(row) = mark
                    .row
                    .checked_sub(first_row)
                    .filter(|i| *i < painted.len())
                else {
                    continue;
                };
                let row_back =
                    syntax::row_background(&colors, mark.row, caret_row_index == Some(mark.row));
                brackets.push(BracketPaint {
                    row,
                    col: mark.col,
                    cols: mark.cols,
                    back: bracket_style.back.unwrap_or(row_back),
                    text: shape_styled(window, &mark.ch.to_string(), &bracket_style),
                });
            }
        }

        // What the screen readers get (`a11y_text.rs`): the rows move, nothing is copied. The
        // strings are made only when a screen reader asks.
        let scale = window.scale_factor();
        // Not the row of the overscan below the screen: its text is not seen, and a caret there is
        // out of the view. A row cut by the edge stays.
        rows.truncate(((body_height - y_offset) / line_height).ceil().max(0.0) as usize);
        let at = |pos| {
            layout
                .pos_to_row(doc, pos)
                .ok()
                .map(|r| a11y_text::Spot { row: r.row, pos })
        };
        let focus = at(editor.cursor.caret());
        let anchor = editor
            .cursor
            .anchor()
            .filter(|_| !editor.cursor.is_box())
            .and_then(at);
        *editor.a11y_text.borrow_mut() = focus.map(|focus| a11y_text::Snapshot {
            rows,
            anchor: anchor.unwrap_or(focus),
            focus,
            columns: LayoutConfig {
                tab_width: layout.config().effective_tab_width(),
                width: layout.config().width,
                tsv: layout.config().tsv.clone(),
                ..LayoutConfig::default()
            },
            left: f32::from(bounds.origin.x) + gutter_width - scroll_x,
            top: f32::from(bounds.origin.y) + top_margin + y_offset,
            cell,
            line_height,
            scale,
        });

        Prepaint {
            rows: painted,
            top_margin,
            ruler_px: settings.ruler_px() as f32,
            ruler: ruler_paint,
            lines: lines_paint,
            brackets,
            caret,
            colors,
            show_line_numbers,
            number_area_width,
            digits,
            bookmark_colored: marks_shown.bookmark,
            cell,
            line_height,
            gutter_width,
            y_offset,
            widest,
            total_rows: layout.row_count(),
            hbar_height,
            selection_edge,
            back_image: editor
                .back_bitmap
                .clone()
                .zip(settings.back_image.clone())
                .filter(|(_, spec)| spec.is_visible())
                .map(|(bitmap, spec)| super::back_paint::BackPaint { bitmap, spec }),
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        p: &mut Prepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.editor.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.editor.clone()),
            cx,
        );

        let editor = self.editor.read(cx);
        let scroll_x = editor.scroll_x;
        let scroll_y = editor.scroll_y;
        let dragging = matches!(editor.drag, super::Drag::Thumb { .. });
        let metrics = Metrics {
            cell_width: p.cell,
            line_height: p.line_height,
            view_width: (f32::from(bounds.size.width) - p.gutter_width - SCROLLBAR_WIDTH).max(1.0),
            view_height: (f32::from(bounds.size.height) - p.top_margin - p.hbar_height).max(1.0),
        };
        let bar = Scrollbar::new(&metrics, p.total_rows, scroll_y);

        let lh = px(p.line_height);
        let body_top = bounds.top() + px(p.top_margin);
        let origin_x = bounds.left() + px(p.gutter_width);
        let text_bounds = Bounds::new(
            point(origin_x, body_top),
            size(
                (bounds.size.width - px(p.gutter_width) - px(SCROLLBAR_WIDTH)).max(px(0.)),
                (bounds.size.height - px(p.top_margin) - px(p.hbar_height)).max(px(0.)),
            ),
        );
        let x_of = |col: u32| origin_x + px(col as f32 * p.cell - scroll_x);

        window.paint_quad(fill(bounds, rgb(p.colors.background)));
        // The background image, under everything that follows (`OnPaint`: `DrawBackImage` first).
        // A part with the background of the text is then left unpainted, so that the image shows.
        let back_frame = super::back_paint::Frame {
            bounds,
            gutter_width: p.gutter_width,
            top_margin: p.top_margin,
            ruler_px: p.ruler_px,
            right_bar: SCROLLBAR_WIDTH,
            bottom_bar: p.hbar_height,
            scroll: (scroll_x, scroll_y),
            scale: window.scale_factor(),
        };
        let paint_back = |window: &mut Window, clip: Bounds<Pixels>| {
            if let Some(back) = &p.back_image {
                super::back_paint::paint(window, back, &back_frame, clip);
            }
        };
        super::back_paint::track(window, cx, p.back_image.as_ref());
        paint_back(window, bounds);
        let gutter_back = if p.show_line_numbers {
            p.colors.line_number_background
        } else {
            p.colors.background
        };
        // `CTextDrawer::DispLineNumber`: the line numbers have no background of their own over
        // the image when it is the background of the text.
        let shows_through = |color: u32| p.back_image.is_some() && color == p.colors.background;
        if !shows_through(gutter_back) {
            window.paint_quad(fill(
                Bounds::new(
                    point(bounds.left(), bounds.top() + px(p.ruler_px)),
                    size(
                        px(p.gutter_width),
                        (bounds.size.height - px(p.ruler_px)).max(px(0.)),
                    ),
                ),
                rgb(gutter_back),
            ));
        }

        for (i, row) in p.rows.iter().enumerate() {
            let y = body_top + px(p.y_offset) + lh * i as f32;
            if let Some(background) = row.gutter_background {
                let cell = Bounds::new(point(bounds.left(), y), size(px(p.gutter_width), lh));
                // The background and the image are given the same device pixels.
                let cell = if shows_through(background) {
                    super::back_paint::snapped(cell, back_frame.scale)
                } else {
                    cell
                };
                if shows_through(background) {
                    // The row's line number area has the background of the text while the rest
                    // of the area does not (`DispLineNumber`: `bTrans`): the image is under it.
                    // Where the area is the text's too, the image is there already.
                    if !shows_through(gutter_back) {
                        window.paint_quad(fill(cell, rgb(p.colors.background)));
                        paint_back(window, cell);
                    }
                } else {
                    window.paint_quad(fill(cell, rgb(background)));
                }
            }
            match gutter_mark(row.bookmarked, p.bookmark_colored, p.show_line_numbers) {
                GutterMark::None | GutterMark::Background => {}
                GutterMark::Line => window.paint_quad(fill(
                    Bounds::new(point(bounds.left() + px(1.), y), size(px(2.), lh)),
                    rgb(p.colors.line_number),
                )),
            }
            if let Some((mark, color)) = row.diff {
                for (dx, dy) in diff_mark::pixels(mark, f32::from(lh).round() as i32) {
                    window.paint_quad(fill(
                        Bounds::new(
                            point(bounds.left() + px(dx as f32), y + px(dy as f32)),
                            size(px(1.), px(1.)),
                        ),
                        rgb(color),
                    ));
                }
            }
            for (dx, number) in &row.numbers {
                number
                    .paint(
                        point(bounds.left() + px(2. + dx), y),
                        lh,
                        TextAlign::Left,
                        None,
                        window,
                        cx,
                    )
                    .ok();
            }
            match &row.term {
                Some(Term::Line) => window.paint_quad(fill(
                    Bounds::new(
                        point(
                            bounds.left() + px(lines::term_line_x(p.number_area_width)),
                            y,
                        ),
                        size(px(1.), lh),
                    ),
                    rgb(row.number_color),
                )),
                Some(Term::Char(mark)) => {
                    mark.paint(
                        point(bounds.left() + px(lines::term_char_x(p.digits, p.cell)), y),
                        lh,
                        TextAlign::Left,
                        None,
                        window,
                        cx,
                    )
                    .ok();
                }
                None => {}
            }
            // The note line goes on through the line number area, over the number.
            if let Some((offset, color)) = p.lines.note {
                window.paint_quad(fill(
                    Bounds::from_corners(
                        point(bounds.left() + px(row.note_from), y + px(offset)),
                        point(bounds.left() + px(p.gutter_width), y + px(offset + 1.)),
                    ),
                    rgb(color),
                ));
            }
        }

        // The dotted vertical lines of one row: one pixel in two, on the same pixel rows in every
        // row of text. `within` keeps the lines inside the columns of a cell (`DispVerticalLines`
        // with a range, as `DrawBracketPair` calls it).
        let paint_dots = |window: &mut Window, y: Pixels, within: Option<(Pixels, Pixels)>| {
            if !p.lines.vertical_dotted {
                return;
            }
            for &x in &p.lines.vertical {
                let x = origin_x + px(x);
                if within.is_some_and(|(left, right)| x < left || x >= right) {
                    continue;
                }
                let row_top = f32::from(y - bounds.top());
                let row_bottom = row_top + p.line_height;
                let mut dot = row_top.ceil();
                if dot % 2.0 != 0.0 {
                    dot += 1.0;
                }
                while dot < row_bottom {
                    window.paint_quad(fill(
                        Bounds::new(point(x, bounds.top() + px(dot)), size(px(1.), px(1.))),
                        rgb(p.lines.vertical_color),
                    ));
                    dot += 2.0;
                }
            }
        };
        window.with_content_mask(
            Some(ContentMask {
                bounds: text_bounds,
            }),
            |window| {
                for (i, row) in p.rows.iter().enumerate() {
                    let y = body_top + px(p.y_offset) + lh * i as f32;
                    if let Some(back) = row.row_background {
                        window.paint_quad(fill(
                            Bounds::from_corners(
                                point(text_bounds.left(), y),
                                point(text_bounds.right(), y + lh),
                            ),
                            rgb(back),
                        ));
                    }
                    for &(from, to, color) in row.backs.iter().filter(|b| !shows_through(b.2)) {
                        window.paint_quad(fill(
                            Bounds::from_corners(point(x_of(from), y), point(x_of(to), y + lh)),
                            rgb(color),
                        ));
                    }
                    if let Some((from, to)) = row.selection {
                        window.paint_quad(fill(
                            Bounds::from_corners(point(x_of(from), y), point(x_of(to), y + lh)),
                            rgb(p.colors.selection),
                        ));
                    }
                    // The selected parts of the pieces have the selection mixed with their colors.
                    for &(from, to, color) in &row.selected_backs {
                        window.paint_quad(fill(
                            Bounds::from_corners(point(x_of(from), y), point(x_of(to), y + lh)),
                            rgb(color),
                        ));
                    }
                    // The note lines go over the backgrounds, under the text.
                    if let Some((offset, color)) = p.lines.note {
                        let line_y = y + px(offset);
                        window.paint_quad(fill(
                            Bounds::from_corners(
                                point(text_bounds.left(), line_y),
                                point(text_bounds.right(), line_y + px(1.)),
                            ),
                            rgb(color),
                        ));
                    }
                    paint_dots(window, y, None);
                    for &(from, to, color) in &row.underlines {
                        window.paint_quad(fill(
                            Bounds::from_corners(
                                point(x_of(from), y + lh - px(3.)),
                                point(x_of(to), y + lh - px(2.)),
                            ),
                            rgb(color),
                        ));
                    }
                    for placed in &row.marks {
                        paint_placed(window, cx, placed, &x_of, y, lh);
                    }
                    // Selected text has its own color when the selection says so: the pieces in
                    // that color are painted inside the selection, the others outside it.
                    let inside = row.selection.map(|(from, to)| {
                        Bounds::from_corners(point(x_of(from), y), point(x_of(to), y + lh))
                    });
                    let masks: Vec<(Bounds<Pixels>, &Vec<Placed>)> = match inside {
                        Some(inside) if !row.selected_text.is_empty() => {
                            let (left, right) = (text_bounds.left(), text_bounds.right());
                            vec![
                                (
                                    Bounds::from_corners(
                                        point(left, y),
                                        point(inside.left().max(left), y + lh),
                                    ),
                                    &row.text,
                                ),
                                (inside, &row.selected_text),
                                (
                                    Bounds::from_corners(
                                        point(inside.right().min(right), y),
                                        point(right, y + lh),
                                    ),
                                    &row.text,
                                ),
                            ]
                        }
                        _ => vec![(text_bounds, &row.text)],
                    };
                    for (mask, pieces) in masks {
                        let mask = mask.intersect(&text_bounds);
                        window.with_content_mask(Some(ContentMask { bounds: mask }), |window| {
                            for placed in pieces {
                                paint_placed(window, cx, placed, &x_of, y, lh);
                            }
                        });
                    }
                    if let Some((from, to)) = row.marked {
                        window.paint_quad(fill(
                            Bounds::from_corners(
                                point(x_of(from), y + lh - px(2.)),
                                point(x_of(to), y + lh - px(1.)),
                            ),
                            rgb(p.colors.text),
                        ));
                    }
                    // The brackets of the pair: the character again, in the color of `BRC`.
                    for bracket in p.brackets.iter().filter(|b| b.row == i) {
                        let cell = Bounds::from_corners(
                            point(x_of(bracket.col), y),
                            point(x_of(bracket.col + bracket.cols), y + lh),
                        );
                        // The rectangle that clears the cell and the image painted in it are
                        // given the same whole device pixels: GPUI paints the edge of a
                        // rectangle that is between two pixels partly, which the image (cut to
                        // whole pixels) would not cover.
                        let cell = if shows_through(bracket.back) {
                            super::back_paint::snapped(cell, back_frame.scale)
                        } else {
                            cell
                        };
                        if shows_through(bracket.back) {
                            // `DrawBracketPair`: the cell gets the image again, under the
                            // character, so that the first one (drawn with the row, in the
                            // colors of the syntax) is gone. (`DrawBackImage` paints the
                            // background of the text first.)
                            window.paint_quad(fill(cell, rgb(p.colors.background)));
                            paint_back(window, cell);
                        } else {
                            window.paint_quad(fill(cell, rgb(bracket.back)));
                        }
                        // `DispNoteLine` and `DispVerticalLines` again over the cell, whatever
                        // its background (the cell was painted over them).
                        if let Some((offset, color)) = p.lines.note {
                            window.paint_quad(fill(
                                Bounds::from_corners(
                                    point(cell.left(), y + px(offset)),
                                    point(cell.right(), y + px(offset + 1.)),
                                ),
                                rgb(color),
                            ));
                        }
                        paint_dots(window, y, Some((cell.left(), cell.right())));
                        bracket
                            .text
                            .paint(
                                point(x_of(bracket.col), y),
                                lh,
                                TextAlign::Left,
                                None,
                                window,
                                cx,
                            )
                            .ok();
                    }
                }
                // The vertical lines at given columns, whole (not row by row), over the text as
                // Sakura draws them; a dotted line was drawn with each row.
                if !p.lines.vertical_dotted {
                    for &x in &p.lines.vertical {
                        window.paint_quad(fill(
                            Bounds::new(
                                point(origin_x + px(x), body_top),
                                size(px(1.), text_bounds.size.height),
                            ),
                            rgb(p.lines.vertical_color),
                        ));
                    }
                }
                // The vertical line at the wrap column (`DispWrapLine`), after those.
                if let Some((x, color)) = p.lines.wrap {
                    window.paint_quad(fill(
                        Bounds::new(
                            point(origin_x + px(x), body_top),
                            size(px(1.), text_bounds.size.height),
                        ),
                        rgb(color),
                    ));
                }
                // The line of the caret column goes first, so that the underline of the caret
                // line is over it (`CaretUnderLineON`).
                for &x in &p.lines.caret_column {
                    window.paint_quad(fill(
                        Bounds::new(
                            point(origin_x + px(x), body_top),
                            size(px(1.), text_bounds.size.height),
                        ),
                        rgb(p.lines.caret_column_color),
                    ));
                }
                if let Some((row_index, color)) = p.lines.underline {
                    let y = body_top
                        + px(p.y_offset)
                        + lh * row_index as f32
                        + px(lines::underline_y(p.line_height));
                    window.paint_quad(fill(
                        Bounds::from_corners(
                            point(text_bounds.left(), y),
                            point(text_bounds.right(), y + px(1.)),
                        ),
                        rgb(color),
                    ));
                }
                if let Some((i, col, shape)) = p.caret
                    && focus_handle.is_focused(window)
                {
                    let y = body_top + px(p.y_offset) + lh * i as f32;
                    // Sakura: a bar in insert mode and a block as wide as the char in overwrite
                    // mode, or the MS-DOS style (`CCaret::ShowEditCaret`, see `caret_shape`). The
                    // block is see-through to show the char; it sits at the bottom of the row.
                    let (width, height, color) = match shape {
                        CaretShape::Bar { width } => (px(width), lh, rgb(p.colors.caret)),
                        CaretShape::Block { cells, half } => (
                            px(cells as f32 * p.cell),
                            if half { lh / 2. } else { lh },
                            gpui::rgba((p.colors.caret << 8) | CARET_BLOCK_ALPHA),
                        ),
                    };
                    window.paint_quad(fill(
                        Bounds::new(point(x_of(col), y + lh - height), size(width, height)),
                        color,
                    ));
                }
            },
        );

        // The space above the first row and the ruler in it. They are over the rows, because the
        // text is scrolled by pixels and the first row may reach up into them.
        if p.top_margin > 0.0 {
            let band = |top: f32, height: f32| {
                Bounds::new(
                    point(bounds.left(), bounds.top() + px(top)),
                    size(bounds.size.width - px(SCROLLBAR_WIDTH), px(height)),
                )
            };
            window.paint_quad(fill(band(0.0, p.top_margin), rgb(p.colors.background)));
            // The image goes on over it, so that the first row does not reach up into the
            // margin (without an image the band is the background).
            paint_back(
                window,
                Bounds::new(
                    bounds.origin,
                    size(bounds.size.width - px(SCROLLBAR_WIDTH), px(p.top_margin)),
                ),
            );
            if p.show_line_numbers && !shows_through(p.colors.line_number_background) {
                window.paint_quad(fill(
                    Bounds::new(
                        point(bounds.left(), bounds.top() + px(p.ruler_px)),
                        size(px(p.gutter_width), px(p.top_margin - p.ruler_px)),
                    ),
                    rgb(p.colors.line_number_background),
                ));
            }
        }
        if let Some(ruler) = &p.ruler {
            let top = bounds.top();
            window.paint_quad(fill(
                Bounds::new(
                    bounds.origin,
                    size(bounds.size.width - px(SCROLLBAR_WIDTH), px(ruler.height)),
                ),
                rgb(ruler.background),
            ));
            let strip = Bounds::from_corners(
                point(origin_x, top),
                point(text_bounds.right(), top + px(ruler.height)),
            );
            window.with_content_mask(Some(ContentMask { bounds: strip }), |window| {
                let scale = &ruler.scale;
                // The scale has a line for each column, and none of them touch: they are one
                // layer, which GPUI draws without working out how each one overlaps the others
                // (that costs more than drawing them does, with a hundred lines every frame).
                window.paint_layer(strip, |window| {
                    window.paint_quad(fill(
                        Bounds::from_corners(
                            point(origin_x + px(scale.base_from), top + px(scale.base_y)),
                            point(origin_x + px(scale.base_to), top + px(scale.base_y + 1.0)),
                        ),
                        rgb(ruler.color),
                    ));
                    for tick in &scale.ticks {
                        let x = origin_x + px(tick.x.round());
                        window.paint_quad(fill(
                            Bounds::from_corners(
                                point(x, top + px(tick.top)),
                                point(x + px(1.), top + px(scale.base_y)),
                            ),
                            rgb(ruler.color),
                        ));
                    }
                    for (x, number) in &ruler.numbers {
                        number
                            .paint(
                                point(origin_x + px(*x), top - px(1.)),
                                px(ruler.height),
                                TextAlign::Left,
                                None,
                                window,
                                cx,
                            )
                            .ok();
                    }
                });
                if let Some(((x0, x1, y0, y1), color)) = ruler.caret {
                    window.paint_quad(fill(
                        Bounds::from_corners(
                            point(origin_x + px(x0), top + px(y0)),
                            point(origin_x + px(x1), top + px(y1)),
                        ),
                        rgb(color),
                    ));
                }
            });
        }

        let track = Bounds::new(
            point(bounds.right() - px(SCROLLBAR_WIDTH), body_top),
            size(
                px(SCROLLBAR_WIDTH),
                (bounds.size.height - px(p.top_margin) - px(p.hbar_height)).max(px(0.)),
            ),
        );
        window.paint_quad(fill(track, rgb(0xf3f3f3)));
        window.paint_quad(
            fill(
                Bounds::new(
                    point(track.left() + px(3.), track.top() + px(bar.thumb_top)),
                    size(px(SCROLLBAR_WIDTH - 6.), px(bar.thumb_height)),
                ),
                if dragging {
                    rgb(0x8c8c8c)
                } else {
                    rgb(0xc2c2c2)
                },
            )
            .corner_radii(px(4.)),
        );

        let hbar_paint = (p.hbar_height > 0.0).then(|| super::hscroll::BarPaint {
            left: f32::from(bounds.left()),
            top: f32::from(bounds.bottom()) - p.hbar_height,
            width: (f32::from(bounds.size.width) - SCROLLBAR_WIDTH).max(0.0),
        });
        let paint_info = PaintInfo {
            origin_x: f32::from(origin_x),
            origin_y: f32::from(body_top),
            scrollbar_left: f32::from(track.left()),
            hbar: hbar_paint,
        };
        let widest = p.widest;
        let selection_edge = p.selection_edge;
        let redraw = self.editor.update(cx, |editor, cx| {
            let mut redraw = false;
            let measured_before = editor.metrics;
            editor.metrics = metrics;
            if measured_before.line_height != metrics.line_height
                || measured_before.cell_width != metrics.cell_width
            {
                // A new font size: the caret may have left the view.
                editor.ensure_caret_visible(cx);
                redraw = true;
            }
            editor.paint = Some(paint_info);
            editor.widest_cols = widest;
            editor.h_selection_edge = selection_edge;
            if hbar_paint.is_some() {
                // `AdjustScrollBars`: with everything in view the view is at the left.
                let before = editor.scroll_x;
                editor.h_scroll_settle();
                redraw |= editor.scroll_x != before;
            }
            // Wrapping at the right edge: the width is known only now.
            if editor.wrap_is_stale() {
                redraw = true;
            }
            if redraw {
                cx.notify();
            }
            redraw
        });
        // `notify` during a paint does not draw again by itself.
        if redraw {
            window.request_animation_frame();
        }
        let editor = self.editor.read(cx);
        if let Some((bar, at)) = editor.h_scrollbar() {
            let dragging = matches!(editor.drag, super::Drag::HThumb { .. });
            paint_hscrollbar(window, &bar, at, dragging);
        }
    }
}

/// The horizontal scroll bar: the arrow buttons and the track, the thumb when there is something
/// to scroll to, and the square right of it where the vertical bar ends (Sakura's size box).
fn paint_hscrollbar(
    window: &mut Window,
    bar: &HScrollbar,
    at: super::hscroll::BarPaint,
    dragging: bool,
) {
    const TRACK: u32 = 0xf3f3f3;
    let (left, top) = (px(at.left), px(at.top));
    let height = px(HSCROLLBAR_HEIGHT);
    window.paint_quad(fill(
        Bounds::new(
            point(left, top),
            size(px(at.width + SCROLLBAR_WIDTH), height),
        ),
        rgb(TRACK),
    ));
    let glyph = if bar.enabled() { 0x606060 } else { 0xbfbfbf };
    // A small triangle in each arrow button, made of strips one pixel wide.
    for (button_left, toward_left) in [(at.left, true), (at.left + at.width - HSCROLL_ARROW, false)]
    {
        let center_x = button_left + HSCROLL_ARROW / 2.0;
        let center_y = at.top + HSCROLLBAR_HEIGHT / 2.0;
        for j in 0..4 {
            // The tip is one strip, and each strip toward the base is two pixels taller.
            let tall = 1.0 + 2.0 * j as f32;
            let x = if toward_left {
                center_x - 2.0 + j as f32
            } else {
                center_x + 1.0 - j as f32
            };
            window.paint_quad(fill(
                Bounds::new(
                    point(px(x), px(center_y - tall / 2.0)),
                    size(px(1.), px(tall)),
                ),
                rgb(glyph),
            ));
        }
    }
    if bar.enabled() {
        window.paint_quad(
            fill(
                Bounds::new(
                    point(left + px(HSCROLL_ARROW + bar.thumb_left), top + px(3.)),
                    size(px(bar.thumb_width), px(HSCROLLBAR_HEIGHT - 6.)),
                ),
                if dragging {
                    rgb(0x8c8c8c)
                } else {
                    rgb(0xc2c2c2)
                },
            )
            .corner_radii(px(4.)),
        );
    }
}

/// The width in columns of the caret in overwrite mode. Sakura (`CCaret::ShowEditCaret`) makes it
/// as wide as the char under the caret, and one half-width column at the end of a line or the
/// document and over a tab.
fn overwrite_caret_cells(
    doc: &matsu_core::buffer::Document,
    layout: &matsu_layout::DocumentLayout,
    caret: matsu_core::buffer::Pos,
) -> u32 {
    let Some(c) = doc
        .rest_of_line(caret.line, caret.col, 8)
        .ok()
        .flatten()
        .and_then(|(rest, _)| rest.chars().next())
    else {
        return 1;
    };
    if c == '\t' {
        return 1;
    }
    // The width of the char itself, not the distance to the next position: at the end of a
    // wrapped row the next position is on the next row.
    layout.config().width.columns(c).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eof_is_drawn_in_a_box_of_its_own_colors() {
        // Sakura's default: cyan text on black (`CShareData::InitTypeConfig`, the entry EOF), as in
        // the box of `sakura-open-ja.png`.
        let colors = crate::settings::ViewSettings::default().colors;
        let style = eof_style(&colors.eof_entry);
        assert_eq!(style.text, 0x00ffff);
        assert_eq!(style.back, Some(0x000000));
        assert!(!style.bold && !style.underline);
        assert_eq!(EOF_MARK.chars().count(), 5);
    }

    #[test]
    fn each_line_ending_has_its_own_mark() {
        assert_eq!(eol_mark(Eol::CrLf), "↵");
        assert_eq!(eol_mark(Eol::Cr), "←");
        assert_eq!(eol_mark(Eol::Lf), "↓");
        assert_eq!(eol_mark(Eol::Nel), "↙");
        assert_eq!(eol_mark(Eol::Ls), "↙");
        assert_eq!(eol_mark(Eol::Ps), "↙");
    }

    use matsu_core::buffer::{Document, Pos};
    use matsu_layout::DocumentLayout;

    fn cells(text: &str, at: Pos) -> u32 {
        let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
        let layout = DocumentLayout::laid_out(&doc, Default::default()).unwrap();
        overwrite_caret_cells(&doc, &layout, at)
    }

    #[test]
    fn the_block_is_as_wide_as_the_char_under_the_caret() {
        // Sakura: CCaret::ShowEditCaret
        assert_eq!(cells("abc", Pos::new(0, 1)), 1);
        assert_eq!(cells("a日c", Pos::new(0, 1)), 2);
        assert_eq!(cells("a😀c", Pos::new(0, 1)), 2);
    }

    #[test]
    fn the_block_over_a_csv_comma_is_the_width_of_the_comma() {
        let doc = Document::from_utf8(b"abc,d\n".to_vec()).unwrap();
        let config = matsu_layout::LayoutConfig {
            tsv: matsu_layout::TsvStops::from_lines(
                matsu_layout::TsvMode::Csv,
                &matsu_layout::WidthRule::default(),
                ["abcdefgh,d"],
            ),
            ..Default::default()
        };
        let layout = DocumentLayout::laid_out(&doc, config).unwrap();
        assert_eq!(overwrite_caret_cells(&doc, &layout, Pos::new(0, 3)), 1);
    }

    #[test]
    fn the_block_is_one_column_at_a_line_end_a_tab_and_the_end_of_the_document() {
        assert_eq!(cells("ab\ncd", Pos::new(0, 2)), 1);
        assert_eq!(cells("a\tb", Pos::new(0, 1)), 1);
        assert_eq!(cells("ab", Pos::new(0, 2)), 1);
        assert_eq!(cells("", Pos::new(0, 0)), 1);
    }

    #[test]
    fn the_block_keeps_its_width_at_the_end_of_a_wrapped_row() {
        let doc = Document::from_utf8("ab日c".as_bytes().to_vec()).unwrap();
        let config = matsu_layout::LayoutConfig {
            wrap: matsu_layout::Wrap::Columns(4),
            ..Default::default()
        };
        let layout = DocumentLayout::laid_out(&doc, config).unwrap();
        assert_eq!(overwrite_caret_cells(&doc, &layout, Pos::new(0, 2)), 2);
    }

    #[test]
    fn a_bookmarked_line_shows_its_mark_the_way_sakura_draws_it() {
        use GutterMark::*;
        assert_eq!(gutter_mark(false, true, true), None);
        assert_eq!(gutter_mark(true, true, true), Background);
        // without the bookmark color a vertical line marks the line
        assert_eq!(gutter_mark(true, false, true), Line);
        assert_eq!(gutter_mark(true, false, false), Line);
        // the color is drawn with the line number, which is not there
        assert_eq!(gutter_mark(true, true, false), None);
    }

    fn colors() -> Colors {
        Colors {
            text: 0x000000,
            background: 0xfffbf0,
            line_number: 0x0000ff,
            line_number_background: 0xefefef,
            caret: 0,
            selection: 0,
            selection_text: 0,
            tab: 0,
            full_width_space: 0,
            eol: 0,
            eof: 0,
            bookmark: 0xfffbf0,
            bookmark_background: 0x0080c0,
            modified: 0x0000ff,
            modified_background: 0xefefef,
            line_number_bold: false,
            bookmark_bold: false,
            modified_bold: true,
            diff_append: diff_color_of(0x0000d2, 0xa2d0ff),
            diff_change: diff_color_of(0x006f00, 0xbdfdc0),
            diff_delete: diff_color_of(0xd56a00, 0xffe9ac),
            ..crate::settings::ViewSettings::default().colors
        }
    }

    fn diff_color_of(text: u32, background: u32) -> DiffColor {
        DiffColor {
            text,
            background,
            bold: false,
            shown: true,
        }
    }

    fn marks(bookmark: bool, modified: bool) -> Marks {
        Marks {
            tab: true,
            full_width_space: true,
            eol: true,
            eof: true,
            bookmark,
            modified,
        }
    }

    #[test]
    fn a_modified_line_number_is_bold_by_default() {
        let c = colors();
        let m = marks(true, true);
        let plain = gutter_style(&c, &m, false, false, None, true);
        assert_eq!(
            (plain.text, plain.background, plain.bold),
            (0x0000ff, None, false)
        );
        // Sakura's default colors differ only in bold
        let modified = gutter_style(&c, &m, false, true, None, true);
        assert_eq!(modified.text, 0x0000ff);
        assert_eq!(modified.background, Some(0xefefef));
        assert!(modified.bold);
    }

    #[test]
    fn a_modified_line_takes_the_colors_of_the_modified_line_entry() {
        let c = Colors {
            modified: 0xff0000,
            modified_background: 0x00ff00,
            ..colors()
        };
        let style = gutter_style(&c, &marks(true, true), false, true, None, true);
        assert_eq!((style.text, style.background), (0xff0000, Some(0x00ff00)));
        // switched off in the settings: the line number as usual
        let style = gutter_style(&c, &marks(true, false), false, true, None, true);
        assert_eq!(
            (style.text, style.background, style.bold),
            (0x0000ff, None, false)
        );
        // without line numbers there is nothing to color
        let style = gutter_style(&c, &marks(true, true), false, true, None, false);
        assert_eq!(style.background, None);
    }

    #[test]
    fn a_bookmark_color_wins_unless_it_is_the_line_numbers_own() {
        let c = Colors {
            modified: 0xff0000,
            modified_background: 0x00ff00,
            ..colors()
        };
        // the bookmark has its own colors: they stay; bold is the modified line's, because the
        // bookmark is as bold as the line number (`cGyouType.IsBoldFont() == cColorType.IsBoldFont()`)
        let style = gutter_style(&c, &marks(true, true), true, true, None, true);
        assert_eq!(
            (style.text, style.background, style.bold),
            (0xfffbf0, Some(0x0080c0), true)
        );
        // a bookmark whose text and background are the line number's own gets the modified ones
        let same = Colors {
            bookmark: c.line_number,
            bookmark_background: c.line_number_background,
            ..c
        };
        let style = gutter_style(&same, &marks(true, true), true, true, None, true);
        assert_eq!(
            (style.text, style.background, style.bold),
            (0xff0000, Some(0x00ff00), true)
        );
        // a bookmark without its color shown: the modified line's colors
        let style = gutter_style(&c, &marks(false, true), true, true, None, true);
        assert_eq!((style.text, style.background), (0xff0000, Some(0x00ff00)));
        // a bookmark and no modification: the bookmark's colors
        let style = gutter_style(&c, &marks(true, true), true, false, None, true);
        assert_eq!((style.text, style.background), (0xfffbf0, Some(0x0080c0)));
    }

    #[test]
    fn a_diff_mark_colors_the_line_number() {
        let c = colors();
        let m = marks(true, true);
        let style = gutter_style(&c, &m, false, false, Some(DiffMark::Change), true);
        assert_eq!((style.text, style.background), (0x006f00, Some(0xbdfdc0)));
        // the end of the file has the color of a deletion
        let style = gutter_style(&c, &m, false, false, Some(DiffMark::DeleteEx), true);
        assert_eq!((style.text, style.background), (0xd56a00, Some(0xffe9ac)));
        // without line numbers there is nothing to color
        let style = gutter_style(&c, &m, false, false, Some(DiffMark::Append), false);
        assert_eq!(style.background, None);
        // a color that is not shown leaves the line number alone
        let hidden = Colors {
            diff_append: DiffColor {
                shown: false,
                ..c.diff_append
            },
            ..c
        };
        let style = gutter_style(&hidden, &m, false, false, Some(DiffMark::Append), true);
        assert_eq!((style.text, style.background), (0x0000ff, None));
    }

    #[test]
    fn a_bookmark_wins_over_a_diff_mark_and_a_modified_line_passes_its_bold_on() {
        let c = colors();
        let m = marks(true, true);
        let style = gutter_style(&c, &m, true, false, Some(DiffMark::Append), true);
        assert_eq!((style.text, style.background), (0xfffbf0, Some(0x0080c0)));
        // a modified line with a diff mark: the colors of the mark, bold from the modified line
        // because the mark is as bold as the line number
        let style = gutter_style(&c, &m, false, true, Some(DiffMark::Append), true);
        assert_eq!(
            (style.text, style.background, style.bold),
            (0x0000d2, Some(0xa2d0ff), true)
        );
        // a diff mark of its own bold keeps it
        let bold = Colors {
            diff_append: DiffColor {
                bold: true,
                ..c.diff_append
            },
            ..c
        };
        let style = gutter_style(
            &bold,
            &marks(true, false),
            false,
            true,
            Some(DiffMark::Append),
            true,
        );
        assert!(style.bold);
    }
}
