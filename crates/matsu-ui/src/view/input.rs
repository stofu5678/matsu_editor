//! Text input through GPUI's input handler: typed characters, and the IME (marked text and the
//! place of the candidate window), as in the PoC.
//!
//! GPUI gives the handler UTF-16 offsets within one text. Here that text is the line of the
//! caret, or the line of the composition while there is one; an offset is converted to a byte
//! column of that line. For a very long line (ADR-0020, 9.10) the text is a window of about
//! `INPUT_WINDOW` bytes on each side of the composition (else of the caret), so that the line is
//! not decoded as a whole; the offsets are relative to the window, and the window stays where the
//! composition began while it lasts. A selection that spans lines cannot be told in these offsets, so it is
//! reported as the caret.
//!
//! While a composition lasts, its text is in the document (marked, and underlined by the paint).
//! The edits of one composition are one undo step: the block opens with the first marked text and
//! closes when the text is committed or taken back.

use std::ops::Range;

use gpui::{
    Bounds, Context, EntityInputHandler, Pixels, Point, UTF16Selection, Window, point, px, size,
};
use matsu_core::buffer::{Pos, col_from_utf16, col_to_utf16};

use super::{EditorView, Marked};

/// Bytes on each side of the anchor that the IME sees of a very long line.
const INPUT_WINDOW: usize = 2048;

/// The text that the IME's UTF-16 offsets refer to: all of a line, or a window of a very long one
/// (`base` is the column where the window starts).
struct InputText {
    line: usize,
    base: usize,
    text: String,
}

impl InputText {
    /// The UTF-16 offset of line column `col`, clamped to the window.
    fn utf16(&self, col: usize) -> usize {
        col_to_utf16(
            &self.text,
            col.saturating_sub(self.base).min(self.text.len()),
        )
    }

    /// The line column of a UTF-16 offset in the window.
    fn col(&self, units: usize) -> usize {
        self.base + col_from_utf16(&self.text, units)
    }
}

impl EditorView {
    /// The text that the IME's offsets refer to: the composition's line, else the caret's.
    fn input_line(&self, cx: &gpui::App) -> InputText {
        let line = self
            .marked
            .as_ref()
            .map_or(self.cursor.caret().line, |m| m.line);
        let doc = self.document.read(cx).doc();
        if doc.is_long_line(line) {
            let anchor = match &self.marked {
                Some(m) => m.range.start,
                None if self.cursor.caret().line == line => self.cursor.caret().col,
                None => 0,
            };
            let end = self
                .marked
                .as_ref()
                .map_or(anchor, |m| m.range.end.max(anchor));
            let slice = doc
                .line_slice(
                    line,
                    anchor.saturating_sub(INPUT_WINDOW),
                    end.saturating_add(INPUT_WINDOW),
                )
                .ok()
                .flatten();
            if let Some(slice) = slice {
                return InputText {
                    line,
                    base: slice.start_col,
                    text: slice.text,
                };
            }
        }
        let text = doc
            .line(line)
            .ok()
            .flatten()
            .map(|l| l.text)
            .unwrap_or_default();
        InputText {
            line,
            base: 0,
            text,
        }
    }

    /// The range that an input call means: the explicit UTF-16 range, else the composition,
    /// else the selection, else the caret.
    fn input_target(&self, range_utf16: Option<Range<usize>>, cx: &gpui::App) -> (Pos, Pos) {
        if let Some(r) = range_utf16 {
            let t = self.input_line(cx);
            return (
                Pos::new(t.line, t.col(r.start)),
                Pos::new(t.line, t.col(r.end)),
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
        let marked = {
            let doc = self.document.read(cx).doc();
            if doc.is_long_line(m.line) {
                doc.line_slice(m.line, m.range.start, m.range.end)
                    .ok()
                    .flatten()
                    .map(|s| s.text)
                    .unwrap_or_default()
            } else {
                doc.line(m.line)
                    .ok()
                    .flatten()
                    .and_then(|l| l.text.get(m.range.clone()).map(str::to_owned))
                    .unwrap_or_default()
            }
        };
        self.run_with(cx, |command_cx| {
            if !command_cx.is_overwrite() {
                return;
            }
            let end = Pos::new(m.line, m.range.end);
            // The composition is in the text already: what follows it is replaced by the spaces
            // that keep the columns (`m_bOverWriteFixMode`), or by nothing.
            if let Ok(plan) = command_cx.overwrite_replacement(end, &marked) {
                // the caret stays where the IME put it
                let pad = " ".repeat(plan.pad);
                command_cx.replace(end, plan.end, &pad).ok();
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
    pub(crate) fn point_of(&self, pos: Pos, cx: &gpui::App) -> Option<Point<Pixels>> {
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
        let t = self.input_line(cx);
        let start = col_from_utf16(&t.text, range_utf16.start);
        let end = col_from_utf16(&t.text, range_utf16.end).max(start);
        *actual_range = Some(col_to_utf16(&t.text, start)..col_to_utf16(&t.text, end));
        t.text.get(start..end).map(str::to_owned)
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let t = self.input_line(cx);
        let line = t.line;
        let caret = self.cursor.caret();
        let utf16 = |col| t.utf16(col);
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
        let t = self.input_line(cx);
        Some(t.utf16(m.range.start)..t.utf16(m.range.end))
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
        // Typed text (`Command_WCHAR`) ends the auto scroll.
        self.autoscroll_exit();
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
        if self.isearch.is_some() {
            // `F_INSTEXT_W` in the incremental search: the text goes to the pattern, and a macro
            // does not record it
            self.isearch_add_str(text, cx);
            return;
        }
        self.record_typed_text(text);
        let (mut from, to) = self.input_target(range_utf16.clone(), cx);
        // A composition cancelled with an empty text (the backends do it through this call) takes
        // the spaces of the free cursor gap with it.
        if text.is_empty()
            && let Some(m) = self.marked.as_ref().filter(|m| m.pad > 0)
            && from == Pos::new(m.line, m.range.start)
        {
            from.col = from.col.saturating_sub(m.pad);
        }
        self.marked = None;
        self.composing = false;
        self.run_with(cx, |command_cx| {
            let (mut from, mut to) = (from, to);
            if !was_composing {
                command_cx.begin_block();
                // The free cursor mode: text committed right of the end of the line follows
                // spaces, in the same undo block (`InsertData_CEditView`).
                if !text.is_empty()
                    && range_utf16.is_none()
                    && from == to
                    && command_cx.pad_free_caret().is_ok()
                {
                    (from, to) = (command_cx.caret(), command_cx.caret());
                }
            }
            // Overwrite mode: the committed text replaces as many chars after it as it has, as
            // Sakura does char by char (`WM_IME_CHAR` runs `F_IME_CHAR` there, and
            // `Command_IME_CHAR` calls `DelCharForOverwrite` only when nothing is selected), so a
            // composition that started over a selection overwrites nothing more.
            let overwrites =
                command_cx.is_overwrite() && ((was_composing && !over_selection) || from == to);
            let plan = if overwrites {
                // a line break is inserted, not overwritten (Enter in `Command_WCHAR`)
                command_cx.overwrite_replacement(to, text).ok()
            } else {
                None
            };
            let (to, text, pad) = match &plan {
                Some(plan) => (plan.end, plan.text.as_str(), plan.pad),
                None => (to, text, 0),
            };
            if let Ok(edit) = command_cx.replace(from, to, text) {
                // the spaces that keep the columns come after the caret
                let caret = Pos::new(edit.end.line, edit.end.col - pad);
                command_cx.move_caret(caret, false).ok();
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
        // A composition starts: it ends the auto scroll like any typed text.
        self.autoscroll_exit();
        if self.isearch.is_some() {
            return; // the composition is not shown; its commit goes to the pattern
        }
        let (mut from, to) = self.input_target(range_utf16.clone(), cx);
        // Text being composed has no line break.
        let text = text.replace(['\r', '\n'], "");
        let opened = !self.composing;
        // A composition that is cancelled takes the spaces of the free cursor gap with it.
        let old_pad = self.marked.as_ref().map_or(0, |m| m.pad);
        let marked_start = self
            .marked
            .as_ref()
            .map(|m| Pos::new(m.line, m.range.start));
        if text.is_empty() && old_pad > 0 && Some(from) == marked_start {
            from.col = from.col.saturating_sub(old_pad);
        }
        if opened {
            self.composing_over_selection = from != to;
        }
        self.composing = true;
        let mut pad = old_pad;
        let edit = self.run_with(cx, |command_cx| {
            let (mut from, mut to) = (from, to);
            if opened {
                command_cx.begin_block();
                // The free cursor mode: the composition starts after spaces that fill the gap.
                if !text.is_empty() && range_utf16.is_none() && from == to {
                    let before = command_cx.caret();
                    if command_cx.pad_free_caret().is_ok() {
                        (from, to) = (command_cx.caret(), command_cx.caret());
                        pad = from.col - before.col;
                    }
                }
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
            pad,
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
        let t = self.input_line(cx);
        let start = t.col(range_utf16.start);
        let end = t.col(range_utf16.end).max(start);
        let a = self.point_of(Pos::new(t.line, start), cx)?;
        let b = self.point_of(Pos::new(t.line, end), cx)?;
        let width = (b.x - a.x).max(px(self.metrics.cell_width));
        Some(Bounds::new(a, size(width, px(self.metrics.line_height))))
    }

    fn character_index_for_point(
        &mut self,
        p: Point<Pixels>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<usize> {
        let t = self.input_line(cx);
        let pos = self
            .position_for_point(p, cx)
            .filter(|pos| pos.line == t.line)?;
        Some(t.utf16(pos.col))
    }
}
