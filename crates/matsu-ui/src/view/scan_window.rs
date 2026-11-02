//! Moving in a huge file that the scan has not finished (ADR-0020, chapter 2).
//!
//! The document of a file read by ranges shows only the part the scan has reached. To move to a
//! place beyond it (Ctrl+End, a jump to a line, the scroll bar, scrolling far), the shared document
//! shows a window on the file around the place instead (`SharedDocument::open_window_at`): a
//! read-only document over about 1 MiB that is read from the file there, and nothing between. This
//! file is the view's part of it:
//!
//! * the caret and the top of the view follow a change of the document by the bytes of the file
//!   they stand for ([`FrameMove`]);
//! * the scroll bar covers the whole file by byte position until the scan is done, and dragging
//!   the thumb moves the window;
//! * the window moves with the view when it comes near an edge;
//! * the commands that jump to the end or to a line the scan has not reached use the window.
//!
//! Line numbers in a window are counted from an estimate until the scan has passed it (the
//! gutter and the status bar mark them as "about"); then they are exact, and when the scan is done
//! the complete document replaces the window at the same place.

use gpui::{App, Context};
use matsu_commands::command::{Arg, Registration};
use matsu_core::buffer::{Pos, WINDOW_BYTES};

use super::EditorView;
use crate::geometry::{Metrics, Scrollbar};

/// How close to an edge of the window the view may come before the window moves: a quarter of it.
const EDGE: u64 = (WINDOW_BYTES / 4) as u64;

/// How close to an edge of the document a place may be and still be shown without a new window.
const COMFORT: u64 = (WINDOW_BYTES / 8) as u64;

/// Where the caret and the top of the view go when the document is replaced by a window on
/// another part of the file, or by the whole file.
pub(super) struct FrameMove {
    caret: Pos,
    anchor: Option<Pos>,
    /// The first line of the view (its start) and how far the view was scrolled into its row.
    top: Pos,
    row_offset: f32,
    /// The caret was not in the new document: it goes to the top of the view.
    caret_lost: bool,
}

/// Where the top of the view was put in a new document whose layout is not complete yet.
pub(super) struct FrameTop {
    pos: Pos,
    row_offset: f32,
    /// The scroll that was set: a different one is the user's.
    set: f32,
}

/// The first row of the view.
fn first_row(scroll_y: f32, m: &Metrics) -> usize {
    (scroll_y / m.line_height).floor().max(0.0) as usize
}

impl EditorView {
    /// When the document changed under the view (a window, or the whole file after it): works out
    /// where the caret and the top of the view go in the new one, from the old document, which
    /// the shared document keeps for one change. Call before the layout is made again.
    pub(super) fn take_frame_move(&mut self, cx: &App) -> Option<FrameMove> {
        let shared = self.document.read(cx);
        let seq = shared.frame_seq();
        if seq == self.frame_seen {
            return None;
        }
        let seen = std::mem::replace(&mut self.frame_seen, seq);
        let change = shared.frame_change().filter(|c| c.from_seq == seen)?;
        let (new_doc, new_start) = (shared.doc(), shared.frame_start());
        let new_end = new_start + new_doc.raw_len() as u64;
        // The byte of the file that a position of the old document stands for.
        let byte_of = |p: Pos| {
            change
                .from_doc
                .pos_to_offset(p)
                .ok()
                .map(|off| change.from_start + off as u64)
        };
        let pos_of = |byte: u64| {
            new_doc
                .offset_to_pos(byte.saturating_sub(new_start) as usize)
                .unwrap_or_else(|_| Pos::new(0, 0))
        };
        let inside = |byte: u64| byte >= new_start && byte <= new_end;
        let m = &self.metrics;
        let row = first_row(self.scroll_y, m);
        // The start of the row (not of its line): a wrapped line stays on the row it was.
        let line = self.layout.locate_row(row).line;
        let row_start = self
            .layout
            .row(&change.from_doc, row)
            .map_or(Pos::new(line, 0), |info| info.start);
        let top_byte = byte_of(row_start).unwrap_or(change.from_start);
        let top = pos_of(top_byte);
        let caret_byte = byte_of(self.cursor.caret());
        let kept = caret_byte.filter(|&b| inside(b));
        let anchor = (!self.cursor.is_box())
            .then(|| self.cursor.anchor())
            .flatten()
            .and_then(byte_of)
            .filter(|&b| inside(b))
            .map(pos_of);
        Some(FrameMove {
            caret: kept.map_or(top, pos_of),
            anchor: kept.and(anchor),
            top,
            // The offset into the row only means something while the view is on the same line.
            row_offset: self.scroll_y - row as f32 * m.line_height,
            caret_lost: kept.is_none(),
        })
    }

    /// Puts the caret and the top of the view where [`take_frame_move`](Self::take_frame_move) said,
    /// after the layout of the new document is made.
    pub(super) fn apply_frame_move(&mut self, moved: FrameMove, cx: &App) {
        let doc = self.document.read(cx).doc();
        self.cursor.move_to(moved.caret, false);
        if let Some(anchor) = moved.anchor.filter(|a| *a != moved.caret) {
            self.cursor.select(anchor, moved.caret);
        }
        if moved.caret_lost {
            self.cursor.set_free_col(None);
        }
        let row = self
            .layout
            .pos_to_row(doc, moved.top)
            .map_or(0, |at| at.row);
        self.scroll_past_end = false;
        self.scroll_y = row as f32 * self.metrics.line_height + moved.row_offset;
        // The layout of the new document is made a part at a time, and the rows of a wrapped
        // line are not known before: the top of the view is put again as it goes on.
        self.frame_top = (!self.layout.is_complete()).then_some(FrameTop {
            pos: moved.top,
            row_offset: moved.row_offset,
            set: self.scroll_y,
        });
        self.prefs.reveal_caret = false;
    }

    /// While the layout of a new document is made (a window, or the whole file), keeps the top of
    /// the view on the line it was moved to; the user's own scroll ends it.
    pub(crate) fn hold_frame_top(&mut self, cx: &App) {
        let Some(top) = self.frame_top.take() else {
            return;
        };
        if (self.scroll_y - top.set).abs() > 0.5 {
            return;
        }
        let doc = self.document.read(cx).doc();
        let row = self.layout.pos_to_row(doc, top.pos).map_or(0, |at| at.row);
        self.scroll_y = row as f32 * self.metrics.line_height + top.row_offset;
        if !self.layout.is_complete() {
            self.frame_top = Some(FrameTop {
                set: self.scroll_y,
                ..top
            });
        }
    }

    /// The number of lines for the status bar: in a window, the lines of the file as far as they
    /// are known or estimated.
    pub(super) fn status_line_count(&self, cx: &App) -> usize {
        let shared = self.document.read(cx);
        let local = shared.doc().line_count().max(1);
        let Some(window) = shared.scan_window() else {
            return local;
        };
        let known = usize::try_from(window.lines_before).unwrap_or(usize::MAX) + local;
        let estimated = shared
            .scan_extent()
            .map_or(0, |(_, lines)| usize::try_from(lines).unwrap_or(usize::MAX));
        known.max(estimated)
    }

    /// The byte of the file where the line of row `row` starts.
    fn row_byte(&self, row: usize, cx: &App) -> u64 {
        let shared = self.document.read(cx);
        let line = self.layout.locate_row(row).line;
        let offset = shared
            .doc()
            .pos_to_offset(Pos::new(line, 0))
            .unwrap_or_default();
        shared.frame_start() + offset as u64
    }

    /// The bytes of the file at the top row and at the bottom row of the view.
    pub(crate) fn visible_bytes(&self, m: &Metrics, cx: &App) -> (u64, u64) {
        let top = first_row(self.scroll_y, m);
        let rows = self.layout.row_count().max(1);
        let bottom = (top + m.visible_rows()).min(rows - 1);
        (self.row_byte(top, cx), self.row_byte(bottom, cx))
    }

    // ---- the scroll bar ----

    /// The vertical scroll bar while the scan runs: the track is the whole file, the thumb sits
    /// where the top of the view is in it, by bytes (the lines of the rest are not known yet).
    pub(crate) fn scan_scrollbar(&self, m: &Metrics, cx: &App) -> Option<Scrollbar> {
        let (total, lines) = self.document.read(cx).scan_extent()?;
        let (top, _) = self.visible_bytes(m, cx);
        let visible = m.visible_rows() as f32 / lines.max(1) as f32;
        Some(Scrollbar::spanning(
            m,
            visible,
            (top as f64 / total.max(1) as f64) as f32,
        ))
    }

    /// The thumb of the scan's scroll bar was dragged to `thumb_top` pixels from the top of the
    /// track: the view moves to the same share of the file.
    pub(crate) fn drag_scan_thumb(
        &mut self,
        bar: &Scrollbar,
        thumb_top: f32,
        cx: &mut Context<Self>,
    ) {
        let Some((total, _)) = self.document.read(cx).scan_extent() else {
            return;
        };
        let byte = (f64::from(bar.position_of(thumb_top)) * total as f64) as u64;
        if self.cover_scan_byte(byte, cx) {
            self.scroll_scan_byte_to_top(byte, cx);
        }
        cx.notify();
    }

    /// Scrolls so that the line with byte `byte` of the file is at the top of the view.
    fn scroll_scan_byte_to_top(&mut self, byte: u64, cx: &App) {
        let shared = self.document.read(cx);
        let doc = shared.doc();
        let pos = doc
            .offset_to_pos(byte.saturating_sub(shared.frame_start()) as usize)
            .unwrap_or_else(|_| Pos::new(0, 0));
        let row = self.layout.pos_to_row(doc, pos).map_or(0, |at| at.row);
        self.scroll_past_end = false;
        self.scroll_y = row as f32 * self.metrics.line_height;
        self.clamp_scroll();
        // The user scrolled: the view does not go back to the caret while the layout is made.
        self.prefs.reveal_caret = false;
        // The layout of a new document is not complete (wrapped lines have more rows than it
        // counts yet): the top is put again as it goes on.
        self.frame_top = (!self.layout.is_complete()).then_some(FrameTop {
            pos,
            row_offset: 0.0,
            set: self.scroll_y,
        });
    }

    // ---- the window ----

    /// Makes sure the document has the place at byte `byte` of the file well inside it (not near an
    /// edge), by showing a window on the file there if it has not. Returns whether it has.
    fn cover_scan_byte(&mut self, byte: u64, cx: &mut Context<Self>) -> bool {
        let shared = self.document.read(cx);
        let Some((total, _)) = shared.scan_extent() else {
            return false;
        };
        let start = shared.frame_start();
        let end = start + shared.doc().raw_len() as u64;
        let comfortable = byte >= start
            && byte <= end
            && (start == 0 || byte >= start + COMFORT)
            && (end >= total || byte + COMFORT <= end);
        comfortable || self.open_scan_window(byte, cx)
    }

    /// Shows a window on the file around `byte`, and brings the view and the caret along.
    fn open_scan_window(&mut self, byte: u64, cx: &mut Context<Self>) -> bool {
        let opened = self.document.update(cx, |shared, cx| {
            shared.open_window_at(byte, cx).then(|| shared.version())
        });
        let Some(version) = opened else {
            return false;
        };
        self.document_changed(version, cx);
        true
    }

    /// While the view is near an edge of the window, asks (after this frame) for a window around
    /// the view, so that scrolling goes on through the file.
    pub(crate) fn queue_scan_window_shift(&mut self, focused: bool, cx: &mut Context<Self>) {
        // Only the pane that has the focus moves the window: the panes of a document share it, and
        // two that asked for different places would move it back and forth.
        // A selection is held in the positions of this window: a new window would drop it.
        if self.scan_shift_queued || !focused || self.cursor.has_selection() {
            return;
        }
        let shared = self.document.read(cx);
        let (Some(window), Some((total, _))) = (shared.scan_window(), shared.scan_extent()) else {
            return;
        };
        let (top, bottom) = self.visible_bytes(&self.metrics, cx);
        // A view that fills more than half of the window (very long lines) cannot be given the
        // room at both edges that a new window would have: it would be moved without end.
        if bottom.saturating_sub(top) >= (window.end - window.start) / 2 {
            return;
        }
        let near_top = window.start > 0 && top < window.start + EDGE;
        let near_bottom = window.end < total && bottom + EDGE > window.end;
        if !(near_top || near_bottom) || self.scan_shift_gave_up == Some(top) {
            return;
        }
        self.scan_shift_queued = true;
        let this = cx.entity();
        cx.defer(move |cx| {
            this.update(cx, |view, cx| view.shift_scan_window(cx));
        });
    }

    fn shift_scan_window(&mut self, cx: &mut Context<Self>) {
        self.scan_shift_queued = false;
        if self.document.read(cx).scan_window().is_none() {
            return;
        }
        let (top, bottom) = self.visible_bytes(&self.metrics, cx);
        let before = self
            .document
            .read(cx)
            .scan_window()
            .map(|w| (w.start, w.end));
        let opened = self.open_scan_window(top / 2 + bottom / 2, cx);
        let after = self
            .document
            .read(cx)
            .scan_window()
            .map(|w| (w.start, w.end));
        if !opened || after == before {
            // A line too long to cut a window from, or one that keeps the window where it was:
            // wait for the scan, and do not ask again for this place.
            self.scan_shift_gave_up = Some(top);
        }
        cx.notify();
    }

    // ---- the jumps ----

    /// A jump to the end of the file, to its start while a window is shown, or to a line the scan
    /// has not reached: shows the place now, from a window on the file there, instead of waiting for
    /// the scan (`defer_for_scan`). Returns whether the command was handled. A macro waits for
    /// the scan as before, and so do the moves that extend a selection, which a window cannot
    /// hold.
    pub(crate) fn jump_in_scan(
        &mut self,
        reg: &'static Registration,
        args: &[Arg],
        cx: &mut Context<Self>,
    ) -> bool {
        if self.is_macro_running() {
            return false;
        }
        let shared = self.document.read(cx);
        let Some((total, _)) = shared.scan_extent() else {
            return false;
        };
        let windowed = shared.scan_window().is_some();
        enum Goal {
            Start,
            End,
            Line(u64),
        }
        let goal = match reg.sakura {
            "F_GOFILEEND" => Goal::End,
            "F_GOFILETOP" if windowed => Goal::Start,
            "F_JUMP" => {
                let (line, option) = super::scan::jump_args(args);
                let Some(n) = line.and_then(|n| u64::try_from(n).ok()) else {
                    return false;
                };
                // Only the plain count of lines (option 1) is the number of a line the scan can
                // place; the last line of the part shown may be cut short.
                let beyond = windowed || n as usize >= shared.doc().line_count();
                if option != Some(1) || n == 0 || !beyond {
                    return false;
                }
                Goal::Line(n)
            }
            _ => return false,
        };
        let (byte, exact) = match goal {
            Goal::Start => (0, true),
            Goal::End => (total, true),
            Goal::Line(n) => match shared.scan_line_byte(n - 1) {
                Some(found) => found,
                None => return false,
            },
        };
        if !self.cover_scan_byte(byte, cx) {
            // No window can be cut there: the jump waits for the scan after all.
            return false;
        }
        // Recorded in its place among the commands, as the jump that waits is. It replaces a
        // move that waited for the scan.
        self.record_call(reg.sakura, args);
        self.pending_scan_jump = None;
        let shared = self.document.read(cx);
        let doc = shared.doc();
        let pos = match goal {
            Goal::End => doc.end_pos().unwrap_or_else(|_| Pos::new(0, 0)),
            Goal::Line(n) if !exact => {
                // Counted from the estimate the window starts at: the gutter shows `n` there.
                let before = shared.scan_window().map_or(0, |w| w.lines_before);
                let local = (n - 1).saturating_sub(before) as usize;
                Pos::new(local.min(doc.line_count().saturating_sub(1)), 0)
            }
            _ => doc
                .offset_to_pos(byte.saturating_sub(shared.frame_start()) as usize)
                .unwrap_or_else(|_| Pos::new(0, 0)),
        };
        self.cursor.move_to(pos, false);
        self.cursor.set_free_col(None);
        match goal {
            Goal::Line(_) => {
                let offset = self.metrics.visible_rows() / 2;
                self.scroll_caret_row_to(offset, cx);
            }
            _ => self.ensure_caret_visible(cx),
        }
        // The place of the jump stays on the screen while the layout of the window is made
        // (a wrapped line has more rows than the layout counts at first).
        self.frame_top = None;
        self.prefs.reveal_caret = true;
        self.notice = None;
        cx.notify();
        true
    }
}
