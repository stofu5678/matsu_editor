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
use crate::segments::{SegmentKind, row_segments};

const TEXT: u32 = 0x1e1e1e;
const LINE_NUMBER: u32 = 0x8a8a8a;
const MARK: u32 = 0xb4b4b4;
const SELECTION: u32 = 0xb4d5fe;
const CARET: u32 = 0x0050c8;
const GUTTER: u32 = 0xf0f0f0;

/// The mark for a tab: Sakura draws an arrow (`CFigure_Tab.cpp`, the default `TAB_ARROW`).
const TAB_MARK: &str = "→";
/// The mark for an ideographic space: a box (`CFigure_ZenSpace.cpp`).
const FULL_WIDTH_SPACE_MARK: &str = "□";
/// The mark for a line break, by kind. Sakura draws an arrow bent to the lower left for CRLF, an
/// arrow to the left for CR and an arrow down for LF; NEL, LS and PS look like LF for now
/// (`_DrawEOL` in `sakura_core/view/figures/CFigure_Eol.cpp`).
fn eol_mark(eol: Eol) -> &'static str {
    match eol {
        Eol::CrLf => "↵",
        Eol::Cr => "←",
        Eol::Lf | Eol::Nel | Eol::Ls | Eol::Ps => "↓",
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
    selection: Option<(u32, u32)>,
    numbers: Option<ShapedLine>,
    text: Vec<Placed>,
    marks: Vec<Placed>,
    /// The columns of the IME composition on this row.
    marked: Option<(u32, u32)>,
}

pub(super) struct Prepaint {
    rows: Vec<RowPaint>,
    /// The index in `rows` and the column of the caret.
    caret: Option<(usize, u32)>,
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

        let shape = |window: &mut Window, text: &str, color: u32| -> ShapedLine {
            let text: SharedString = text.to_owned().into();
            let runs = [run(&font, text.len(), color)];
            window
                .text_system()
                .shape_line(text, font_size, &runs, None)
        };

        // One column is the advance of a half-width character.
        let cell = f32::from(shape(window, "0000000000", TEXT).width()) / 10.0;
        let cell = if cell > 0.0 { cell } else { 8.0 };

        let first_row = (editor.scroll_y / line_height).floor().max(0.0) as usize;
        let y_offset = -(editor.scroll_y - line_height * first_row as f32);
        let count = (f32::from(bounds.size.height) / line_height).ceil() as usize + 1;
        let rows: Vec<RowInfo> = layout.rows(doc, first_row, count).unwrap_or_default();

        let digits = doc.line_count().max(1).to_string().len().max(3);
        let gutter_width = cell * (digits as f32 + 1.0) + 4.0;

        let selection = editor.cursor.selection();
        let marked = editor.marked.clone();
        let last_line = doc.line_count().saturating_sub(1);

        let mut widest = 0;
        let mut painted = Vec::with_capacity(rows.len());
        for row in &rows {
            widest = widest.max(row.end_column);
            let numbers = (row.row_in_line == 0 && !row.is_eof_row).then(|| {
                shape(
                    window,
                    &format!("{:>width$}", row.line + 1, width = digits),
                    LINE_NUMBER,
                )
            });
            let mut text = Vec::new();
            let mut marks = Vec::new();
            for seg in row_segments(row.text(), row.start_column, layout.config()) {
                match seg.kind {
                    SegmentKind::Text => text.push(Placed {
                        col: seg.col,
                        line: shape(window, &seg.text, TEXT),
                    }),
                    SegmentKind::Tab => marks.push(Placed {
                        col: seg.col,
                        line: shape(window, TAB_MARK, MARK),
                    }),
                    SegmentKind::FullWidthSpace => marks.push(Placed {
                        col: seg.col,
                        line: shape(window, FULL_WIDTH_SPACE_MARK, MARK),
                    }),
                }
            }
            if row.is_eof_row {
                marks.push(Placed {
                    col: 0,
                    line: shape(window, EOF_MARK, MARK),
                });
            } else if let Some(eol) = row.eol {
                marks.push(Placed {
                    col: row.end_column,
                    line: shape(window, eol_mark(eol), MARK),
                });
            } else if !row.wrapped && row.line == last_line {
                marks.push(Placed {
                    col: row.end_column,
                    line: shape(window, EOF_MARK, MARK),
                });
            }
            let selected =
                selection.and_then(|s| selected_columns(layout, doc, row, s).ok().flatten());
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
                selection: selected,
                numbers,
                text,
                marks,
                marked: marked_cols,
            });
        }

        let caret = layout
            .pos_to_row(doc, editor.cursor.caret())
            .ok()
            .and_then(|at| {
                at.row
                    .checked_sub(first_row)
                    .filter(|i| *i < painted.len())
                    .map(|i| (i, at.column))
            });

        Prepaint {
            rows: painted,
            caret,
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

        window.paint_quad(fill(bounds, rgb(0xffffff)));
        window.paint_quad(fill(
            Bounds::new(bounds.origin, size(px(p.gutter_width), bounds.size.height)),
            rgb(GUTTER),
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
                    if let Some((from, to)) = row.selection {
                        window.paint_quad(fill(
                            Bounds::from_corners(point(x_of(from), y), point(x_of(to), y + lh)),
                            rgb(SELECTION),
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
                    for placed in &row.text {
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
                    if let Some((from, to)) = row.marked {
                        window.paint_quad(fill(
                            Bounds::from_corners(
                                point(x_of(from), y + lh - px(2.)),
                                point(x_of(to), y + lh - px(1.)),
                            ),
                            rgb(TEXT),
                        ));
                    }
                }
                if let Some((i, col)) = p.caret
                    && focus_handle.is_focused(window)
                {
                    let y = bounds.top() + px(p.y_offset) + lh * i as f32;
                    window.paint_quad(fill(
                        Bounds::new(point(x_of(col), y), size(px(2.), lh)),
                        rgb(CARET),
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
        self.editor.update(cx, |editor, _| {
            editor.metrics = metrics;
            editor.paint = Some(paint_info);
            editor.widest_cols = widest;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_line_ending_has_its_own_mark() {
        assert_eq!(eol_mark(Eol::CrLf), "↵");
        assert_eq!(eol_mark(Eol::Cr), "←");
        assert_eq!(eol_mark(Eol::Lf), "↓");
        assert_eq!(eol_mark(Eol::Ls), "↓");
    }
}
