//! Text input through GPUI's input handler: typed characters, and the IME (marked text and the
//! place of the candidate window), as in the PoC.
//!
//! GPUI gives the handler UTF-16 offsets within one text. Here that text is the line of the
//! caret, or the line of the composition while there is one; an offset is converted to a byte
//! column of that line. A selection that spans lines cannot be told in these offsets, so it is
//! reported as the caret.
//!
//! While a composition lasts, its text is in the document (marked, and underlined by the paint).
//! The edits of one composition are one undo step: the block opens with the first marked text and
//! closes when the text is committed or taken back.

use std::ops::Range;

use gpui::{
    Bounds, Context, EntityInputHandler, Pixels, Point, UTF16Selection, Window, point, px, size,
};
use ume_core::buffer::{Pos, col_from_utf16, col_to_utf16};

use super::{EditorView, Marked};

impl EditorView {
    /// The line that the IME's offsets refer to: the composition's, else the caret's.
    fn input_line(&self, cx: &gpui::App) -> (usize, String) {
        let line = self
            .marked
            .as_ref()
            .map_or(self.cursor.caret().line, |m| m.line);
        let text = self
            .document
            .read(cx)
            .doc()
            .line(line)
            .ok()
            .flatten()
            .map(|l| l.text)
            .unwrap_or_default();
        (line, text)
    }

    /// The range that an input call means: the explicit UTF-16 range, else the composition,
    /// else the selection, else the caret.
    fn input_target(&self, range_utf16: Option<Range<usize>>, cx: &gpui::App) -> (Pos, Pos) {
        if let Some(r) = range_utf16 {
            let (line, text) = self.input_line(cx);
            return (
                Pos::new(line, col_from_utf16(&text, r.start)),
                Pos::new(line, col_from_utf16(&text, r.end)),
            );
        }
        if let Some(m) = &self.marked {
            return (
                Pos::new(m.line, m.range.start),
                Pos::new(m.line, m.range.end),
            );
        }
        self.cursor
            .selection()
            .unwrap_or((self.cursor.caret(), self.cursor.caret()))
    }

    /// In overwrite mode, a composition that stays in the document without a commit (an unmark,
    /// or an interruption) overwrites the chars after it, as a commit does. Not when it started
    /// over a selection. Runs inside the composition's undo block.
    fn finish_overwrite(&mut self, cx: &mut Context<Self>) {
        let Some(m) = self.marked.clone() else { return };
        if !self.composing || self.composing_over_selection {
            return;
        }
        let chars = self
            .document
            .read(cx)
            .doc()
            .line(m.line)
            .ok()
            .flatten()
            .and_then(|l| l.text.get(m.range.clone()).map(|t| t.chars().count()))
            .unwrap_or(0);
        self.run_with(cx, |command_cx| {
            if !command_cx.is_overwrite() {
                return;
            }
            let end = Pos::new(m.line, m.range.end);
            if let Ok(to) = command_cx.overwrite_end(end, chars) {
                // the caret stays where the IME put it
                command_cx.replace(end, to, "").ok();
            }
        });
    }

    /// Ends the composition's undo block.
    fn end_composition(&mut self, cx: &mut Context<Self>) {
        if self.composing {
            self.composing = false;
            self.run_with(cx, |command_cx| command_cx.end_block());
        }
    }

    /// Ends a composition that something other than the IME interrupts (a key, a click, a menu or
    /// an edit in another view): its text stays in the document, and its undo block closes, so
    /// that later edits are not undone with it.
    pub(crate) fn interrupt_composition(&mut self, cx: &mut Context<Self>) {
        if self.composing {
            self.record_composition(cx);
        }
        self.finish_overwrite(cx);
        self.marked = None;
        self.end_composition(cx);
    }

    /// Ends a composition after another view changed the document: its range is stale, so no
    /// overwrite is finished (unlike [`interrupt_composition`](Self::interrupt_composition)).
    pub(super) fn drop_composition(&mut self, cx: &mut Context<Self>) {
        self.marked = None;
        self.end_composition(cx);
    }

    /// The pixel position of `pos` in the window, if the view has been painted.
    fn point_of(&self, pos: Pos, cx: &gpui::App) -> Option<Point<Pixels>> {
        let paint = self.paint?;
        let at = self
            .layout
            .pos_to_row(self.document.read(cx).doc(), pos)
            .ok()?;
        Some(point(
            px(paint.origin_x + at.column as f32 * self.metrics.cell_width - self.scroll_x),
            px(paint.origin_y + at.row as f32 * self.metrics.line_height - self.scroll_y),
        ))
    }
}

impl EntityInputHandler for EditorView {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        let (_, text) = self.input_line(cx);
        let start = col_from_utf16(&text, range_utf16.start);
        let end = col_from_utf16(&text, range_utf16.end).max(start);
        *actual_range = Some(col_to_utf16(&text, start)..col_to_utf16(&text, end));
        text.get(start..end).map(str::to_owned)
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let (line, text) = self.input_line(cx);
        let caret = self.cursor.caret();
        let utf16 = |col| col_to_utf16(&text, col);
        let selection = self
            .cursor
            .selection()
            .filter(|(a, b)| a.line == line && b.line == line);
        Some(match selection {
            Some((a, b)) => UTF16Selection {
                range: utf16(a.col)..utf16(b.col),
                reversed: caret == a,
            },
            None => {
                let c = if caret.line == line {
                    utf16(caret.col)
                } else {
                    0
                };
                UTF16Selection {
                    range: c..c,
                    reversed: false,
                }
            }
        })
    }

    fn marked_text_range(&self, _: &mut Window, cx: &mut Context<Self>) -> Option<Range<usize>> {
        let m = self.marked.as_ref()?;
        let (_, text) = self.input_line(cx);
        Some(col_to_utf16(&text, m.range.start)..col_to_utf16(&text, m.range.end))
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.record_composition(cx);
        self.finish_overwrite(cx);
        self.marked = None;
        self.end_composition(cx);
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let was_composing = self.composing;
        let over_selection = was_composing && self.composing_over_selection;
        let mut chars = text.chars();
        if let (false, None, Some(c), None) =
            (was_composing, &range_utf16, chars.next(), chars.next())
        {
            // One typed character: the `input_char` command, as for a key.
            self.type_char(c, cx);
            return;
        }
        self.record_typed_text(text);
        let (from, to) = self.input_target(range_utf16, cx);
        self.marked = None;
        self.composing = false;
        self.run_with(cx, |command_cx| {
            if !was_composing {
                command_cx.begin_block();
            }
            // Overwrite mode: the committed text replaces as many chars after it as it has, as
            // Sakura does char by char (`WM_IME_CHAR` runs `F_IME_CHAR` there, and
            // `Command_IME_CHAR` calls `DelCharForOverwrite` only when nothing is selected), so a
            // composition that started over a selection overwrites nothing more.
            let to = if command_cx.is_overwrite()
                && ((was_composing && !over_selection) || from == to)
            {
                command_cx
                    // a line break is inserted, not overwritten (Enter in `Command_WCHAR`)
                    .overwrite_end(
                        to,
                        text.chars().filter(|c| !matches!(c, '\r' | '\n')).count(),
                    )
                    .unwrap_or(to)
            } else {
                to
            };
            if let Ok(edit) = command_cx.replace(from, to, text) {
                command_cx.move_caret(edit.end, false).ok();
            }
            command_cx.end_block();
        });
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (from, to) = self.input_target(range_utf16, cx);
        // Text being composed has no line break.
        let text = text.replace(['\r', '\n'], "");
        let opened = !self.composing;
        if opened {
            self.composing_over_selection = from != to;
        }
        self.composing = true;
        let edit = self.run_with(cx, |command_cx| {
            if opened {
                command_cx.begin_block();
            }
            let edit = command_cx.replace(from, to, &text).ok()?;
            // The caret goes where the IME asks, else to the end of the composition.
            let caret = new_selected_range_utf16.as_ref().map_or(edit.end, |r| {
                Pos::new(
                    edit.start.line,
                    edit.start.col + col_from_utf16(&text, r.end),
                )
            });
            command_cx.move_caret(caret, false).ok();
            Some(edit)
        });
        // The composition starts where the replaced range started. `from` and `to` can be on
        // different lines, so the smaller of their columns is not the start.
        self.marked = edit.filter(|_| !text.is_empty()).map(|edit| Marked {
            line: edit.start.line,
            range: edit.start.col..edit.start.col + text.len(),
        });
        if text.is_empty() {
            self.end_composition(cx);
        }
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let (line, text) = self.input_line(cx);
        let start = col_from_utf16(&text, range_utf16.start);
        let end = col_from_utf16(&text, range_utf16.end).max(start);
        let a = self.point_of(Pos::new(line, start), cx)?;
        let b = self.point_of(Pos::new(line, end), cx)?;
        let width = (b.x - a.x).max(px(self.metrics.cell_width));
        Some(Bounds::new(a, size(width, px(self.metrics.line_height))))
    }

    fn character_index_for_point(
        &mut self,
        p: Point<Pixels>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<usize> {
        let (line, text) = self.input_line(cx);
        let pos = self
            .position_for_point(p, cx)
            .filter(|pos| pos.line == line)?;
        Some(col_to_utf16(&text, pos.col))
    }
}
