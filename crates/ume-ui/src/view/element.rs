//! The element that paints the rows in view: line numbers, selection, text, Sakura's marks, the
//! caret, the IME underline and the scroll bar. It shapes only the rows on the screen.
//!
//! Every position comes from the layout in columns and is turned into pixels with the width of a
//! half-width character, so the caret and the selection stay on the grid whatever the font does.
//! Sakura draws the marks that this paints in `sakura_core/view/figures/` (`CFigure_Tab.cpp`,
//! `CFigure_ZenSpace.cpp`, `CFigure_Eol.cpp`).

use gpui::{
    App, Bounds, ContentMask, Element, ElementId, ElementInputHandler, Entity, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, ShapedLine, SharedString, Style, TextAlign,
    TextRun, Window, fill, point, px, relative, rgb, size,
};
use ume_core::buffer::Eol;
use ume_layout::RowInfo;

use super::{EditorView, PaintInfo};
use crate::geometry::{Metrics, SCROLLBAR_WIDTH, Scrollbar};
use crate::hit::selected_columns;
use crate::search::highlight::highlight_columns;
use crate::segments::{SegmentKind, row_segments};
use crate::settings::Colors;

/// The caret of overwrite mode: the caret color with some transparency.
const CARET_BLOCK_ALPHA: u32 = 0x70;
/// The background of the matches of the search pattern (the settings have no color for it yet).
const FOUND: u32 = 0xffe680;

/// The mark for a tab: Sakura draws an arrow (`CFigure_Tab.cpp`, the default `TAB_ARROW`).
const TAB_MARK: &str = "→";
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

/// A shaped piece and the column where it goes.
struct Placed {
    col: u32,
    line: ShapedLine,
}

struct RowPaint {
    /// The columns of the matches of the search pattern on this row.
    found: Vec<(u32, u32)>,
    selection: Option<(u32, u32)>,
    numbers: Option<ShapedLine>,
    text: Vec<Placed>,
    /// The same pieces in the color of selected text, when that differs from the text color and
    /// the row has a selection.
    selected_text: Vec<Placed>,
    marks: Vec<Placed>,
    /// The columns of the IME composition on this row.
    marked: Option<(u32, u32)>,
}

pub(super) struct Prepaint {
    rows: Vec<RowPaint>,
    /// The index in `rows`, the column and the width of the caret: `None` for the bar of insert
    /// mode, the columns of the char under the caret for the block of overwrite mode.
    caret: Option<(usize, u32, Option<u32>)>,
    colors: Colors,
    show_line_numbers: bool,
    cell: f32,
    line_height: f32,
    gutter_width: f32,
    y_offset: f32,
    widest: u32,
    total_rows: usize,
}

impl IntoElement for EditorElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
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
        let line_height = f32::from(window.line_height());
        let font = style.font();
        let settings = &editor.prefs.settings;
        let colors = settings.colors;
        let marks_shown = settings.marks;
        let show_line_numbers = settings.show_line_numbers;

        let shape = |window: &mut Window, text: &str, color: u32| -> ShapedLine {
            let text: SharedString = text.to_owned().into();
            let runs = [run(&font, text.len(), color)];
            window
                .text_system()
                .shape_line(text, font_size, &runs, None)
        };

        // One column is the advance of a half-width character.
        let cell = f32::from(shape(window, "0000000000", colors.text).width()) / 10.0;
        let cell = if cell > 0.0 { cell } else { 8.0 };

        let first_row = (editor.scroll_y / line_height).floor().max(0.0) as usize;
        let y_offset = -(editor.scroll_y - line_height * first_row as f32);
        let count = (f32::from(bounds.size.height) / line_height).ceil() as usize + 1;
        let rows: Vec<RowInfo> = layout.rows(doc, first_row, count).unwrap_or_default();

        let digits = doc.line_count().max(1).to_string().len().max(3);
        let gutter_width = if show_line_numbers {
            cell * (digits as f32 + 1.0) + 4.0
        } else {
            4.0
        };

        let box_area = crate::hit::BoxArea::of_cursor(layout, doc, &editor.cursor);
        let selection = editor
            .cursor
            .selection()
            .filter(|_| !editor.cursor.is_box());
        let marked = editor.marked.clone();
        let last_line = doc.line_count().saturating_sub(1);

        let mut found = match editor.search.matcher().filter(|_| editor.search.mark()) {
            Some(matcher) => highlight_columns(layout, doc, &rows, &**matcher),
            None => Vec::new(),
        }
        .into_iter();
        let mut widest = 0;
        let mut painted = Vec::with_capacity(rows.len());
        for row in &rows {
            widest = widest.max(row.end_column);
            let numbers =
                (show_line_numbers && row.row_in_line == 0 && !row.is_eof_row).then(|| {
                    shape(
                        window,
                        &format!("{:>width$}", row.line + 1, width = digits),
                        colors.line_number,
                    )
                });
            let selected = match &box_area {
                Some(area) => area.columns_on(row.row),
                None => {
                    selection.and_then(|s| selected_columns(layout, doc, row, s).ok().flatten())
                }
            };
            let own_selection_color = selected.is_some() && colors.selection_text != colors.text;
            let mut text = Vec::new();
            let mut selected_text = Vec::new();
            let mut marks = Vec::new();
            for seg in row_segments(row.text(), row.start_column, layout.config()) {
                match seg.kind {
                    SegmentKind::Text => {
                        if own_selection_color {
                            selected_text.push(Placed {
                                col: seg.col,
                                line: shape(window, &seg.text, colors.selection_text),
                            });
                        }
                        text.push(Placed {
                            col: seg.col,
                            line: shape(window, &seg.text, colors.text),
                        });
                    }
                    SegmentKind::Tab if marks_shown.tab => marks.push(Placed {
                        col: seg.col,
                        line: shape(window, TAB_MARK, colors.tab),
                    }),
                    SegmentKind::FullWidthSpace if marks_shown.full_width_space => {
                        marks.push(Placed {
                            col: seg.col,
                            line: shape(window, FULL_WIDTH_SPACE_MARK, colors.full_width_space),
                        })
                    }
                    SegmentKind::Tab | SegmentKind::FullWidthSpace => {}
                }
            }
            if row.is_eof_row {
                if marks_shown.eof {
                    marks.push(Placed {
                        col: 0,
                        line: shape(window, EOF_MARK, colors.eof),
                    });
                }
            } else if let Some(eol) = row.eol {
                if marks_shown.eol {
                    marks.push(Placed {
                        col: row.end_column,
                        line: shape(window, eol_mark(eol), colors.eol),
                    });
                }
            } else if !row.wrapped && row.line == last_line && marks_shown.eof {
                marks.push(Placed {
                    col: row.end_column,
                    line: shape(window, EOF_MARK, colors.eof),
                });
            }
            let marked_cols = marked
                .as_ref()
                .filter(|m| m.line == row.start.line)
                .and_then(|m| {
                    let a = ume_core::buffer::Pos::new(m.line, m.range.start);
                    let b = ume_core::buffer::Pos::new(m.line, m.range.end);
                    let (a, b) = (
                        layout.pos_to_row(doc, a).ok()?,
                        layout.pos_to_row(doc, b).ok()?,
                    );
                    (a.row == row.row && b.row == row.row).then_some((a.column, b.column))
                });
            painted.push(RowPaint {
                found: found.next().unwrap_or_default(),
                selection: selected,
                numbers,
                text,
                selected_text,
                marks,
                marked: marked_cols,
            });
        }

        // In a box selection the caret can be right of the end of the text.
        let caret_column = editor
            .cursor
            .box_caret_col()
            .map_or(0, |c| u32::try_from(c).unwrap_or(u32::MAX));
        let caret = layout
            .pos_to_row(doc, editor.cursor.caret())
            .ok()
            .and_then(|at| {
                at.row
                    .checked_sub(first_row)
                    .filter(|i| *i < painted.len())
                    .map(|i| {
                        let block = editor
                            .document
                            .read(cx)
                            .overwrite()
                            .then(|| overwrite_caret_cells(doc, layout, editor.cursor.caret()));
                        (i, caret_column.max(at.column), block)
                    })
            });

        Prepaint {
            rows: painted,
            caret,
            colors,
            show_line_numbers,
            cell,
            line_height,
            gutter_width,
            y_offset,
            widest,
            total_rows: layout.row_count(),
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
            view_height: f32::from(bounds.size.height),
        };
        let bar = Scrollbar::new(&metrics, p.total_rows, scroll_y);

        let lh = px(p.line_height);
        let origin_x = bounds.left() + px(p.gutter_width);
        let text_bounds = Bounds::new(
            point(origin_x, bounds.top()),
            size(
                (bounds.size.width - px(p.gutter_width) - px(SCROLLBAR_WIDTH)).max(px(0.)),
                bounds.size.height,
            ),
        );
        let x_of = |col: u32| origin_x + px(col as f32 * p.cell - scroll_x);

        window.paint_quad(fill(bounds, rgb(p.colors.background)));
        window.paint_quad(fill(
            Bounds::new(bounds.origin, size(px(p.gutter_width), bounds.size.height)),
            rgb(if p.show_line_numbers {
                p.colors.line_number_background
            } else {
                p.colors.background
            }),
        ));

        for (i, row) in p.rows.iter().enumerate() {
            let y = bounds.top() + px(p.y_offset) + lh * i as f32;
            if let Some(numbers) = &row.numbers {
                numbers
                    .paint(
                        point(bounds.left() + px(2.), y),
                        lh,
                        TextAlign::Left,
                        None,
                        window,
                        cx,
                    )
                    .ok();
            }
        }

        window.with_content_mask(
            Some(ContentMask {
                bounds: text_bounds,
            }),
            |window| {
                for (i, row) in p.rows.iter().enumerate() {
                    let y = bounds.top() + px(p.y_offset) + lh * i as f32;
                    for &(from, to) in &row.found {
                        window.paint_quad(fill(
                            Bounds::from_corners(point(x_of(from), y), point(x_of(to), y + lh)),
                            rgb(FOUND),
                        ));
                    }
                    if let Some((from, to)) = row.selection {
                        window.paint_quad(fill(
                            Bounds::from_corners(point(x_of(from), y), point(x_of(to), y + lh)),
                            rgb(p.colors.selection),
                        ));
                    }
                    for placed in &row.marks {
                        placed
                            .line
                            .paint(
                                point(x_of(placed.col), y),
                                lh,
                                TextAlign::Left,
                                None,
                                window,
                                cx,
                            )
                            .ok();
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
                                placed
                                    .line
                                    .paint(
                                        point(x_of(placed.col), y),
                                        lh,
                                        TextAlign::Left,
                                        None,
                                        window,
                                        cx,
                                    )
                                    .ok();
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
                }
                if let Some((i, col, block)) = p.caret
                    && focus_handle.is_focused(window)
                {
                    let y = bounds.top() + px(p.y_offset) + lh * i as f32;
                    // Sakura: a 2px bar in insert mode, a block as wide as the char in overwrite
                    // mode (`CCaret::ShowEditCaret`). The block is see-through to show the char.
                    let (width, color) = match block {
                        None => (px(2.), rgb(p.colors.caret)),
                        Some(cells) => (
                            px(cells as f32 * p.cell),
                            gpui::rgba((p.colors.caret << 8) | CARET_BLOCK_ALPHA),
                        ),
                    };
                    window.paint_quad(fill(
                        Bounds::new(point(x_of(col), y), size(width, lh)),
                        color,
                    ));
                }
            },
        );

        let track = Bounds::new(
            point(bounds.right() - px(SCROLLBAR_WIDTH), bounds.top()),
            size(px(SCROLLBAR_WIDTH), bounds.size.height),
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

        let paint_info = PaintInfo {
            origin_x: f32::from(origin_x),
            origin_y: f32::from(bounds.top()),
            scrollbar_left: f32::from(track.left()),
        };
        let widest = p.widest;
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
    }
}

/// The width in columns of the caret in overwrite mode. Sakura (`CCaret::ShowEditCaret`) makes it
/// as wide as the char under the caret, and one half-width column at the end of a line or the
/// document and over a tab.
fn overwrite_caret_cells(
    doc: &ume_core::buffer::Document,
    layout: &ume_layout::DocumentLayout,
    caret: ume_core::buffer::Pos,
) -> u32 {
    let Some(c) = doc
        .line(caret.line)
        .ok()
        .flatten()
        .and_then(|l| l.text.get(caret.col..)?.chars().next())
    else {
        return 1;
    };
    if c == '\t' {
        return 1;
    }
    // The width of the char itself, not the distance to the next position: at the end of a
    // wrapped row the next position is on the next row.
    layout.config().columns_of(c, 0).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_line_ending_has_its_own_mark() {
        assert_eq!(eol_mark(Eol::CrLf), "↵");
        assert_eq!(eol_mark(Eol::Cr), "←");
        assert_eq!(eol_mark(Eol::Lf), "↓");
        assert_eq!(eol_mark(Eol::Nel), "↙");
        assert_eq!(eol_mark(Eol::Ls), "↙");
        assert_eq!(eol_mark(Eol::Ps), "↙");
    }

    use ume_core::buffer::{Document, Pos};
    use ume_layout::DocumentLayout;

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
    fn the_block_is_one_column_at_a_line_end_a_tab_and_the_end_of_the_document() {
        assert_eq!(cells("ab\ncd", Pos::new(0, 2)), 1);
        assert_eq!(cells("a\tb", Pos::new(0, 1)), 1);
        assert_eq!(cells("ab", Pos::new(0, 2)), 1);
        assert_eq!(cells("", Pos::new(0, 0)), 1);
    }

    #[test]
    fn the_block_keeps_its_width_at_the_end_of_a_wrapped_row() {
        let doc = Document::from_utf8("ab日c".as_bytes().to_vec()).unwrap();
        let config = ume_layout::LayoutConfig {
            wrap: ume_layout::Wrap::Columns(4),
            ..Default::default()
        };
        let layout = DocumentLayout::laid_out(&doc, config).unwrap();
        assert_eq!(overwrite_caret_cells(&doc, &layout, Pos::new(0, 2)), 2);
    }
}
