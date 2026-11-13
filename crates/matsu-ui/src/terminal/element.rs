//! The element that draws the screen of a terminal (ADR-0028, "描画").
//!
//! Every cell sits at `column x cell width`, whatever the font's advances are, so the grid lines up for the
//! programs that draw with it (box drawing, tables, full-screen editors). The cell width is the advance of a
//! half-width character of the text font; a wide character covers two cells. Text is shaped a piece at a
//! time (cells of one color and weight, cut at blanks): the piece goes on the grid as it is when its advance
//! is the width of its cells (BIZ UDGothic, MS Gothic, Menlo with its Japanese fallback), else each cell is
//! shaped and placed alone, and a glyph that the font draws wider than its cells (box drawing in a Japanese
//! font) is squeezed into them (option 2 of the ADR). The editor's `place` (`view/element.rs`) works the same
//! way for its columns.
//!
//! The screen is copied once per frame, under one short lock ([`matsu_terminal::Terminal::snapshot`]).

use gpui::{
    App, Bounds, ContentMask, DispatchPhase, Element, ElementId, ElementInputHandler, Entity,
    GlobalElementId, InspectorElementId, IntoElement, LayoutId, MouseMoveEvent, Pixels, ShapedLine,
    SharedString, Style, TextAlign, TextRun, Window, fill, point, px, relative, rgb, size,
};
use matsu_terminal::{Cell, CursorShape, Row, SelectionSpan, Snapshot};

use super::palette::{Packed, Palette, blend};
use super::view::{TerminalView, preedit_cells, rows_up};

/// The height of a row, in pixels: the font's size and a little more, so that the box drawing of two rows
/// meets without the gap of the editor's 1.5 lines.
const LINE_HEIGHT_RATIO: f32 = 1.3;

/// The element of the screen of one [`TerminalView`].
pub(super) struct TerminalElement {
    view: Entity<TerminalView>,
}

impl TerminalElement {
    pub(super) fn new(view: Entity<TerminalView>) -> Self {
        Self { view }
    }
}

impl IntoElement for TerminalElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

/// The width of a cell and the height of a row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CellSize {
    pub width: f32,
    pub height: f32,
}

impl CellSize {
    /// How many columns and rows fit `bounds` (at least 2 x 1: the terminal state needs that).
    pub(super) fn grid_in(self, width: f32, height: f32) -> (u16, u16) {
        let count =
            |length: f32, unit: f32| (length / unit).floor().clamp(0.0, f32::from(u16::MAX));
        let columns = count(width, self.width) as u16;
        let rows = count(height, self.height) as u16;
        (columns.max(2), rows.max(1))
    }
}

/// How the pieces of a row are cut: the style that ends a piece when it changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PieceStyle {
    color: Packed,
    bold: bool,
    italic: bool,
}

/// A piece of text: cells of one style next to each other, without blanks.
struct Piece {
    /// `(column, columns covered, the text of the cell)`, in order.
    cells: Vec<(usize, usize, String)>,
    style: PieceStyle,
}

impl Piece {
    fn text(&self) -> String {
        self.cells
            .iter()
            .map(|(_, _, text)| text.as_str())
            .collect()
    }

    fn columns(&self) -> usize {
        self.cells.iter().map(|(_, span, _)| span).sum()
    }
}

/// How many columns a cell covers, or `None` for the right half of a wide character (the left cell covers
/// it).
fn span_of(cell: &Cell) -> Option<usize> {
    if cell.wide_spacer {
        None
    } else if cell.wide {
        Some(2)
    } else {
        Some(1)
    }
}

/// The text of a cell: its character and the zero-width ones that follow.
fn cell_text(cell: &Cell) -> String {
    let mut text = String::with_capacity(4);
    text.push(cell.ch);
    text.extend(cell.zerowidth.iter());
    text
}

fn is_blank(cell: &Cell) -> bool {
    cell.hidden || (cell.ch == ' ' && cell.zerowidth.is_empty()) || cell.leading_wide_spacer
}

/// The colors a cell is drawn with: its foreground and background after the inverse video, the dim
/// attribute and the hidden one. `block_cursor` swaps them once more (a block cursor shows the cell inverted).
/// A `selected` cell has the selection's background and its own foreground (not swapped: the text of an
/// inverse cell stays readable on it).
fn resolve(cell: &Cell, palette: &Palette, block_cursor: bool, selected: bool) -> (Packed, Packed) {
    let mut foreground = palette.foreground_of(cell.fg);
    let mut background = palette.background_of(cell.bg);
    if selected {
        background = palette.selection();
        // Text of the very color of the selection would vanish in it.
        if foreground == background {
            foreground = palette.default_foreground();
        }
    } else if cell.inverse != block_cursor {
        std::mem::swap(&mut foreground, &mut background);
    }
    if cell.dim {
        foreground = blend(foreground, background, 128);
    }
    if cell.hidden {
        foreground = background;
    }
    (foreground, background)
}

/// What a row has besides its cells: where the block cursor is, if it is on this row, and the columns the
/// selection covers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RowMarks {
    /// The column of the block cursor (a cell the cursor inverts is not shown as selected).
    cursor: Option<usize>,
    /// The first and the last column of the selection on this row, both included.
    selected: Option<(usize, usize)>,
}

impl RowMarks {
    /// Whether the cell at `column`, `span` columns wide, is selected: a wide character is selected when
    /// either of its halves is.
    fn selects(&self, column: usize, span: usize) -> bool {
        self.cursor != Some(column)
            && self
                .selected
                .is_some_and(|(first, last)| column <= last && column + span > first)
    }
}

/// The columns of the view's row `row` (of `columns`) that the selection covers.
fn selected_columns(
    selection: Option<&SelectionSpan>,
    row: usize,
    columns: usize,
) -> Option<(usize, usize)> {
    let span = selection?;
    let row = i32::try_from(row).ok()?;
    if row < span.start_row || row > span.end_row {
        return None;
    }
    let first = if row == span.start_row {
        span.start_column
    } else {
        0
    };
    let last = if row == span.end_row {
        span.end_column
    } else {
        columns.saturating_sub(1)
    };
    (first <= last).then_some((first, last))
}

/// Cuts a row into the pieces of text to shape.
fn pieces_of(row: &Row, palette: &Palette, marks: RowMarks) -> Vec<Piece> {
    let mut pieces: Vec<Piece> = Vec::new();
    let mut current: Option<Piece> = None;
    for (column, cell) in row.cells.iter().enumerate() {
        let Some(span) = span_of(cell) else {
            continue;
        };
        if is_blank(cell) {
            pieces.extend(current.take());
            continue;
        }
        let (color, _) = resolve(
            cell,
            palette,
            marks.cursor == Some(column),
            marks.selects(column, span),
        );
        let style = PieceStyle {
            color,
            bold: cell.bold,
            italic: cell.italic,
        };
        if current.as_ref().is_some_and(|piece| piece.style != style) {
            pieces.extend(current.take());
        }
        current
            .get_or_insert_with(|| Piece {
                cells: Vec::new(),
                style,
            })
            .cells
            .push((column, span, cell_text(cell)));
    }
    pieces.extend(current);
    pieces
}

/// A line of a row to draw as a thin box: underline and strikeout runs, as `(first column, columns,
/// color)`.
fn runs_of(
    row: &Row,
    palette: &Palette,
    marks: RowMarks,
    wanted: impl Fn(&Cell) -> bool,
) -> Vec<(usize, usize, Packed)> {
    let mut runs: Vec<(usize, usize, Packed)> = Vec::new();
    for (column, cell) in row.cells.iter().enumerate() {
        let Some(span) = span_of(cell) else {
            continue;
        };
        if !wanted(cell) {
            continue;
        }
        let color = match cell.underline_color {
            Some(color) if cell.underline => palette.foreground_of(color),
            _ => {
                resolve(
                    cell,
                    palette,
                    marks.cursor == Some(column),
                    marks.selects(column, span),
                )
                .0
            }
        };
        match runs.last_mut() {
            Some((first, columns, last)) if *first + *columns == column && *last == color => {
                *columns += span;
            }
            _ => runs.push((column, span, color)),
        }
    }
    runs
}

/// The cells with a background of their own, as runs `(first column, columns, color)`.
fn backgrounds_of(row: &Row, palette: &Palette, marks: RowMarks) -> Vec<(usize, usize, Packed)> {
    let default = palette.default_background();
    let mut runs: Vec<(usize, usize, Packed)> = Vec::new();
    for (column, cell) in row.cells.iter().enumerate() {
        let Some(span) = span_of(cell) else {
            continue;
        };
        let (_, background) = resolve(
            cell,
            palette,
            marks.cursor == Some(column),
            marks.selects(column, span),
        );
        if background == default {
            continue;
        }
        match runs.last_mut() {
            Some((first, columns, last)) if *first + *columns == column && *last == background => {
                *columns += span;
            }
            _ => runs.push((column, span, background)),
        }
    }
    runs
}

fn text_run(font: &gpui::Font, len: usize, color: Packed) -> TextRun {
    TextRun {
        len,
        font: font.clone(),
        color: rgb(color).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    }
}

/// The fonts of a terminal: the regular one and the bold and italic variants.
struct Fonts {
    regular: gpui::Font,
    bold: gpui::Font,
    italic: gpui::Font,
    bold_italic: gpui::Font,
}

impl Fonts {
    fn new(regular: gpui::Font) -> Self {
        let with = |weight, style| gpui::Font {
            weight,
            style,
            ..regular.clone()
        };
        Self {
            bold: with(gpui::FontWeight::BOLD, gpui::FontStyle::Normal),
            italic: with(regular.weight, gpui::FontStyle::Italic),
            bold_italic: with(gpui::FontWeight::BOLD, gpui::FontStyle::Italic),
            regular,
        }
    }

    fn of(&self, style: PieceStyle) -> &gpui::Font {
        match (style.bold, style.italic) {
            (false, false) => &self.regular,
            (true, false) => &self.bold,
            (false, true) => &self.italic,
            (true, true) => &self.bold_italic,
        }
    }
}

fn shape(
    window: &mut Window,
    text: &str,
    font_size: Pixels,
    font: &gpui::Font,
    color: Packed,
) -> ShapedLine {
    let text: SharedString = text.to_owned().into();
    let runs = [text_run(font, text.len(), color)];
    window
        .text_system()
        .shape_line(text, font_size, &runs, None)
}

/// Whether a shaped piece goes on the columns as it is: its advance is the width of its cells.
fn fits(advance: f32, columns: usize, cell: f32) -> bool {
    (advance - columns as f32 * cell).abs() < 0.5
}

/// The size to shape a glyph at so that it fits `columns` cells: the font size, made smaller when the glyph
/// is wider than its cells.
fn squeezed_size(font_size: f32, advance: f32, columns: usize, cell: f32) -> f32 {
    let room = columns as f32 * cell;
    if advance > room + 0.5 {
        font_size * room / advance
    } else {
        font_size
    }
}

/// What the element works out before it paints: the cell size and the copy of the screen.
pub(super) struct Prepaint {
    cell: CellSize,
    snapshot: Snapshot,
    palette: Palette,
    focused: bool,
    /// The text the IME is composing, drawn at the cursor.
    preedit: Option<String>,
}

impl Element for TerminalElement {
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
        let (look, focus) = {
            let view = self.view.read(cx);
            (view.look().clone(), view.focus_handle().clone())
        };
        let font = look.font.font();
        let font_size = px(look.size);
        // One column is the advance of a half-width character, as in the editor.
        let half = f32::from(shape(window, "0000000000", font_size, &font, 0).width()) / 10.0;
        let cell = CellSize {
            width: if half > 0.0 { half } else { 8.0 },
            height: (look.size * LINE_HEIGHT_RATIO).round().max(1.0),
        };
        let (columns, rows) =
            cell.grid_in(f32::from(bounds.size.width), f32::from(bounds.size.height));
        self.view.update(cx, |view, _| {
            view.fit(columns, rows, cell);
            view.set_layout(bounds, cell);
        });
        let (snapshot, preedit) = {
            let view = self.view.read(cx);
            (view.screen(), view.preedit().map(str::to_owned))
        };
        let mut palette = look.palette;
        palette.set_overrides(&snapshot.color_overrides);
        Prepaint {
            cell,
            snapshot,
            palette,
            focused: focus.is_focused(window),
            preedit,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        prepaint: &mut Prepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Prepaint {
            cell,
            snapshot,
            palette,
            focused,
            preedit,
        } = prepaint;
        let (cell, focused) = (*cell, *focused);
        let (font, font_size, focus) = {
            let view = self.view.read(cx);
            (
                view.look().font.font(),
                view.look().size,
                view.focus_handle().clone(),
            )
        };
        // The text the platform delivers (typed, committed by an IME, composed) goes to the view.
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.view.clone()),
            cx,
        );
        // A selection follows the pointer past the edge of the screen: an element's own move listener only
        // hears the pointer while it is over the element, so the view listens on the window.
        let view = self.view.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if phase == DispatchPhase::Bubble {
                view.update(cx, |view, cx| view.on_mouse_move(event, window, cx));
            }
        });
        let fonts = Fonts::new(font);
        let size_px = px(font_size);
        let x_of = |column: usize| bounds.left() + px(column as f32 * cell.width);
        let width_of = |columns: usize| px(columns as f32 * cell.width);
        let line = px(cell.height);
        let cursor = snapshot.info.cursor;
        let viewport_row = i64::from(cursor.row) + snapshot.info.display_offset as i64;
        let block = focused && cursor.shape == CursorShape::Block;

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            window.paint_quad(fill(bounds, rgb(palette.default_background())));
            for (index, row) in snapshot.rows.iter().enumerate() {
                let y = bounds.top() + px(index as f32 * cell.height);
                if y > bounds.bottom() {
                    break;
                }
                let on_cursor_row = i64::try_from(index).ok() == Some(viewport_row);
                let marks = RowMarks {
                    cursor: (on_cursor_row && block).then_some(cursor.column),
                    selected: selected_columns(
                        snapshot.selection.as_ref(),
                        index,
                        snapshot.info.columns,
                    ),
                };

                for (first, columns, color) in backgrounds_of(row, palette, marks) {
                    window.paint_quad(fill(
                        Bounds::new(point(x_of(first), y), size(width_of(columns), line)),
                        rgb(color),
                    ));
                }

                for piece in pieces_of(row, palette, marks) {
                    let font = fonts.of(piece.style);
                    let whole = shape(window, &piece.text(), size_px, font, piece.style.color);
                    if fits(f32::from(whole.width()), piece.columns(), cell.width) {
                        let first = piece.cells[0].0;
                        whole
                            .paint(
                                point(x_of(first), y),
                                line,
                                TextAlign::Left,
                                None,
                                window,
                                cx,
                            )
                            .ok();
                        continue;
                    }
                    // The font's advances do not fit the cells (a fallback font, a glyph that is
                    // wider): every cell is placed on its own column, and squeezed if it must be.
                    for (column, columns, text) in &piece.cells {
                        let mut shaped = shape(window, text, size_px, font, piece.style.color);
                        let advance = f32::from(shaped.width());
                        let squeezed = squeezed_size(font_size, advance, *columns, cell.width);
                        if squeezed < font_size {
                            shaped = shape(window, text, px(squeezed), font, piece.style.color);
                        }
                        shaped
                            .paint(
                                point(x_of(*column), y),
                                line,
                                TextAlign::Left,
                                None,
                                window,
                                cx,
                            )
                            .ok();
                    }
                }

                for (first, columns, color) in runs_of(row, palette, marks, |c| c.underline) {
                    window.paint_quad(fill(
                        Bounds::new(
                            point(x_of(first), y + line - px(2.)),
                            size(width_of(columns), px(1.)),
                        ),
                        rgb(color),
                    ));
                }
                for (first, columns, color) in runs_of(row, palette, marks, |c| c.strikeout) {
                    window.paint_quad(fill(
                        Bounds::new(
                            point(x_of(first), y + line / 2.),
                            size(width_of(columns), px(1.)),
                        ),
                        rgb(color),
                    ));
                }
            }

            if on_screen(viewport_row, snapshot.rows.len()) && cursor.shape != CursorShape::Hidden {
                paint_cursor(
                    window,
                    snapshot,
                    palette,
                    (bounds, cell),
                    viewport_row as usize,
                    if focused || cursor.shape != CursorShape::Block {
                        cursor.shape
                    } else {
                        CursorShape::HollowBlock
                    },
                );
            }

            if let Some(text) = preedit.as_deref()
                && on_screen(viewport_row, snapshot.rows.len())
            {
                let cells = preedit_cells(text, cursor.column, snapshot.info.columns);
                // A preedit that wraps below the last row is drawn higher, over the rows above.
                let up = rows_up(&cells, viewport_row as usize, snapshot.rows.len());
                for piece in &cells {
                    let row = (viewport_row as usize + piece.row).saturating_sub(up);
                    let at = point(
                        x_of(piece.column),
                        bounds.top() + px(row as f32 * cell.height),
                    );
                    let width = width_of(piece.span);
                    let foreground = palette.default_foreground();
                    // Over the grid: the cells of the program under it are covered, the text is underlined.
                    window.paint_quad(fill(
                        Bounds::new(at, size(width, line)),
                        rgb(palette.default_background()),
                    ));
                    let mut shaped =
                        shape(window, &piece.text, size_px, &fonts.regular, foreground);
                    let squeezed =
                        squeezed_size(font_size, f32::from(shaped.width()), piece.span, cell.width);
                    if squeezed < font_size {
                        shaped = shape(
                            window,
                            &piece.text,
                            px(squeezed),
                            &fonts.regular,
                            foreground,
                        );
                    }
                    shaped
                        .paint(at, line, TextAlign::Left, None, window, cx)
                        .ok();
                    window.paint_quad(fill(
                        Bounds::new(point(at.x, at.y + line - px(2.)), size(width, px(1.))),
                        rgb(foreground),
                    ));
                }
            }
        });
    }
}

fn on_screen(row: i64, rows: usize) -> bool {
    row >= 0 && (row as usize) < rows
}

/// The cursor, other than the block of a focused terminal (that one is the inverted cell, drawn with the
/// cell): the outline of a block, a bar at the left of the cell, or a line under it.
fn paint_cursor(
    window: &mut Window,
    snapshot: &Snapshot,
    palette: &Palette,
    (bounds, cell): (Bounds<Pixels>, CellSize),
    row: usize,
    shape: CursorShape,
) {
    let cursor = snapshot.info.cursor;
    let columns = snapshot
        .rows
        .get(row)
        .and_then(|r| r.cells.get(cursor.column))
        .map_or(1, |c| if c.wide { 2 } else { 1 });
    let origin = point(
        bounds.left() + px(cursor.column as f32 * cell.width),
        bounds.top() + px(row as f32 * cell.height),
    );
    let (width, height) = (px(columns as f32 * cell.width), px(cell.height));
    let color = rgb(palette.default_foreground());
    match shape {
        CursorShape::Block | CursorShape::Hidden => {}
        CursorShape::Beam => {
            window.paint_quad(fill(Bounds::new(origin, size(px(2.), height)), color));
        }
        CursorShape::Underline => {
            window.paint_quad(fill(
                Bounds::new(
                    point(origin.x, origin.y + height - px(2.)),
                    size(width, px(2.)),
                ),
                color,
            ));
        }
        CursorShape::HollowBlock => {
            let edge = px(1.);
            for rect in [
                Bounds::new(origin, size(width, edge)),
                Bounds::new(point(origin.x, origin.y + height - edge), size(width, edge)),
                Bounds::new(origin, size(edge, height)),
                Bounds::new(point(origin.x + width - edge, origin.y), size(edge, height)),
            ] {
                window.paint_quad(fill(rect, color));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matsu_terminal::{Color, Rgb};

    fn cell(ch: char) -> Cell {
        Cell {
            ch,
            zerowidth: Vec::new(),
            fg: Color::Default,
            bg: Color::Default,
            underline_color: None,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            strikeout: false,
            inverse: false,
            hidden: false,
            wide: false,
            wide_spacer: false,
            leading_wide_spacer: false,
            hyperlink: None,
        }
    }

    fn row(cells: Vec<Cell>) -> Row {
        Row {
            cells,
            wrapped: false,
        }
    }

    fn palette() -> Palette {
        Palette::new(0x000000, 0xffffff)
    }

    #[test]
    fn the_grid_has_at_least_two_columns_and_one_row() {
        let cell = CellSize {
            width: 8.0,
            height: 16.0,
        };
        assert_eq!(cell.grid_in(800.0, 480.0), (100, 30));
        assert_eq!(cell.grid_in(799.0, 479.0), (99, 29));
        assert_eq!(cell.grid_in(3.0, 3.0), (2, 1));
        assert_eq!(cell.grid_in(0.0, 0.0), (2, 1));
    }

    #[test]
    fn pieces_are_cut_at_blanks_and_style_changes() {
        let mut bold = cell('c');
        bold.bold = true;
        let cells = vec![cell('a'), cell('b'), cell(' '), bold, cell('d')];
        let pieces = pieces_of(&row(cells), &palette(), RowMarks::default());
        let texts: Vec<_> = pieces.iter().map(Piece::text).collect();
        assert_eq!(texts, ["ab", "c", "d"]);
        assert_eq!(pieces[0].cells[0].0, 0);
        assert_eq!(pieces[1].cells[0].0, 3, "a piece keeps its column");
        assert_eq!(pieces[2].cells[0].0, 4);
    }

    #[test]
    fn a_wide_character_covers_two_columns_and_its_spacer_is_skipped() {
        let mut wide = cell('あ');
        wide.wide = true;
        let mut spacer = cell(' ');
        spacer.wide_spacer = true;
        let cells = vec![cell('a'), wide, spacer, cell('b')];
        let pieces = pieces_of(&row(cells), &palette(), RowMarks::default());
        assert_eq!(pieces.len(), 1);
        assert_eq!(pieces[0].text(), "aあb");
        assert_eq!(pieces[0].columns(), 4);
        assert_eq!(
            pieces[0].cells[2].0, 3,
            "b is in the column after the spacer"
        );
    }

    #[test]
    fn combining_characters_stay_with_their_base() {
        let mut base = cell('e');
        base.zerowidth.push('\u{301}');
        let pieces = pieces_of(&row(vec![base]), &palette(), RowMarks::default());
        assert_eq!(pieces[0].text(), "e\u{301}");
        assert_eq!(pieces[0].columns(), 1);
    }

    #[test]
    fn inverse_dim_and_hidden_change_the_colors() {
        let palette = palette();
        let plain = resolve(&cell('a'), &palette, false, false);
        assert_eq!(plain, (0x000000, 0xffffff));
        let mut inverse = cell('a');
        inverse.inverse = true;
        assert_eq!(
            resolve(&inverse, &palette, false, false),
            (0xffffff, 0x000000)
        );
        // a block cursor inverts the cell once more
        assert_eq!(
            resolve(&inverse, &palette, true, false),
            (0x000000, 0xffffff)
        );
        assert_eq!(
            resolve(&cell('a'), &palette, true, false),
            (0xffffff, 0x000000)
        );
        let mut dim = cell('a');
        dim.dim = true;
        assert_eq!(resolve(&dim, &palette, false, false).0, 0x7f7f7f);
        let mut hidden = cell('a');
        hidden.hidden = true;
        let (fg, bg) = resolve(&hidden, &palette, false, false);
        assert_eq!(fg, bg);
        // a hidden cell has no piece of text
        assert!(pieces_of(&row(vec![hidden]), &palette, RowMarks::default()).is_empty());
    }

    #[test]
    fn colors_are_the_palettes_the_cubes_and_true_color() {
        let palette = palette();
        let mut c = cell('a');
        c.fg = Color::Indexed(196);
        c.bg = Color::Rgb(Rgb { r: 1, g: 2, b: 3 });
        assert_eq!(resolve(&c, &palette, false, false), (0xff0000, 0x010203));
        c.fg = Color::Indexed(1);
        assert_eq!(resolve(&c, &palette, false, false).0, palette.indexed(1));
    }

    #[test]
    fn backgrounds_are_runs_of_the_cells_that_differ_from_the_default() {
        let mut red = cell(' ');
        red.bg = Color::Indexed(1);
        let mut wide = cell('あ');
        wide.wide = true;
        wide.bg = Color::Indexed(1);
        let mut spacer = cell(' ');
        spacer.wide_spacer = true;
        spacer.bg = Color::Indexed(1);
        let cells = vec![cell('a'), red.clone(), wide, spacer, cell('b'), red];
        let runs = backgrounds_of(&row(cells), &palette(), RowMarks::default());
        let color = palette().indexed(1);
        assert_eq!(runs, vec![(1, 3, color), (5, 1, color)]);
    }

    #[test]
    fn underline_and_strikeout_runs_cover_blanks_too() {
        let mut under = cell(' ');
        under.underline = true;
        let mut under_a = cell('a');
        under_a.underline = true;
        let cells = vec![under_a, under, cell('b')];
        assert_eq!(
            runs_of(&row(cells), &palette(), RowMarks::default(), |c| c
                .underline),
            vec![(0, 2, 0x000000)]
        );
    }

    #[test]
    fn a_piece_fits_when_its_advance_is_the_width_of_its_cells() {
        assert!(fits(72.0, 10, 7.2));
        assert!(!fits(70.0, 10, 7.2));
        // a full-width glyph (2 cells of 7.2) that the font draws 14.4 wide fits; one of 28.8 is squeezed
        assert_eq!(squeezed_size(14.0, 14.4, 2, 7.2), 14.0);
        let squeezed = squeezed_size(14.0, 28.8, 2, 7.2);
        assert!((squeezed - 7.0).abs() < 1e-4, "{squeezed}");
    }

    fn span(start: (i32, usize), end: (i32, usize)) -> SelectionSpan {
        SelectionSpan {
            start_row: start.0,
            start_column: start.1,
            end_row: end.0,
            end_column: end.1,
        }
    }

    #[test]
    fn the_selection_covers_part_of_its_first_and_last_rows_and_all_between() {
        let one = span((2, 3), (2, 5));
        assert_eq!(selected_columns(Some(&one), 2, 10), Some((3, 5)));
        assert_eq!(selected_columns(Some(&one), 1, 10), None);
        assert_eq!(selected_columns(Some(&one), 3, 10), None);
        let many = span((1, 4), (3, 2));
        assert_eq!(selected_columns(Some(&many), 0, 10), None);
        assert_eq!(selected_columns(Some(&many), 1, 10), Some((4, 9)));
        assert_eq!(selected_columns(Some(&many), 2, 10), Some((0, 9)));
        assert_eq!(selected_columns(Some(&many), 3, 10), Some((0, 2)));
        assert_eq!(selected_columns(Some(&many), 4, 10), None);
        assert_eq!(selected_columns(None, 1, 10), None);
        // a selection that began above the view covers the rows from its top
        let above = span((-5, 7), (1, 1));
        assert_eq!(selected_columns(Some(&above), 0, 10), Some((0, 9)));
    }

    #[test]
    fn a_wide_character_is_selected_when_either_half_is() {
        let marks = RowMarks {
            cursor: None,
            selected: Some((3, 3)),
        };
        assert!(marks.selects(3, 1));
        // a wide character in columns 2 and 3, and one in 3 and 4
        assert!(marks.selects(2, 2));
        assert!(marks.selects(3, 2));
        assert!(!marks.selects(1, 2));
        assert!(!marks.selects(4, 1));
    }

    #[test]
    fn selected_cells_have_the_selection_background_and_the_cursors_cell_stays_inverted() {
        let palette = palette();
        let mut inverse = cell('a');
        inverse.inverse = true;
        let selection = palette.selection();
        // the text keeps its own foreground, not the swapped one
        assert_eq!(
            resolve(&inverse, &palette, false, true),
            (0x000000, selection)
        );
        // text of the selection's own color is drawn in the default foreground
        let mut same = cell('a');
        same.fg = Color::Rgb(Rgb {
            r: (selection >> 16) as u8,
            g: (selection >> 8) as u8,
            b: selection as u8,
        });
        assert_eq!(resolve(&same, &palette, false, true).0, 0x000000);
        let cells = vec![cell('a'), cell('b'), cell('c')];
        let marks = RowMarks {
            cursor: None,
            selected: Some((1, 2)),
        };
        assert_eq!(
            backgrounds_of(&row(cells.clone()), &palette, marks),
            vec![(1, 2, selection)]
        );
        // the cell of the block cursor is shown by the cursor, not as selected
        let with_cursor = RowMarks {
            cursor: Some(1),
            selected: Some((1, 2)),
        };
        assert_eq!(
            backgrounds_of(&row(cells), &palette, with_cursor),
            // the inverted cell (the foreground is the background now), then the selected one
            vec![(1, 1, 0x000000), (2, 1, selection)]
        );
    }
}
