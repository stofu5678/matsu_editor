//! Mouse and wheel: click to move the caret, drag to select, double click to select a word, the
//! scroll bar, and scrolling.

use gpui::{
    Context, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point,
    ScrollWheelEvent, Window,
};
use ume_core::buffer::Pos;

use super::{Drag, EditorView};
use crate::geometry::{Scrollbar, clamp_scroll_y};
use crate::hit::position_at;
use crate::word::word_range;

impl EditorView {
    /// The document position under a point of the window, if the view has been painted.
    pub(crate) fn position_for_point(&self, p: Point<Pixels>, cx: &gpui::App) -> Option<Pos> {
        let paint = self.paint?;
        let x = f32::from(p.x) - paint.origin_x + self.scroll_x;
        let y = f32::from(p.y) - paint.origin_y;
        position_at(
            &self.layout,
            self.document.read(cx).doc(),
            &self.metrics,
            self.scroll_y,
            x,
            y,
        )
        .ok()
    }

    pub(crate) fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        self.hide_key_help(cx);
        let Some(paint) = self.paint else {
            return;
        };
        let rows = self.layout.row_count();
        if f32::from(event.position.x) >= paint.scrollbar_left {
            let bar = Scrollbar::new(&self.metrics, rows, self.scroll_y);
            let y = f32::from(event.position.y) - paint.origin_y;
            if y >= bar.thumb_top && y <= bar.thumb_top + bar.thumb_height {
                self.drag = Drag::Thumb {
                    start_y: f32::from(event.position.y),
                    start_scroll: clamp_scroll_y(&self.metrics, rows, self.scroll_y),
                };
            } else {
                // Clicking the track pages up or down, as on Windows.
                let page = self.metrics.line_height * (self.metrics.visible_rows() as f32 - 1.0);
                let page = if y < bar.thumb_top { -page } else { page };
                self.scroll_past_end = false;
                self.scroll_y = clamp_scroll_y(&self.metrics, rows, self.scroll_y + page);
            }
            cx.notify();
            return;
        }
        // Before the hit test: finishing an overwrite composition can remove chars and move the
        // text under the pointer.
        self.interrupt_composition(cx);
        // A click ends the incremental search (`CEditView::OnLBUTTONDOWN`).
        self.isearch_exit(cx);
        let Some(pos) = self.position_for_point(event.position, cx) else {
            return;
        };
        let extend = event.modifiers.shift;
        let anchor = if extend {
            self.cursor.anchor().unwrap_or(self.cursor.caret())
        } else if event.click_count == 2 {
            self.select_word(pos, cx)
        } else {
            pos
        };
        if event.click_count != 2 || extend {
            self.run_with(cx, |command_cx| command_cx.move_caret(pos, extend))
                .ok();
        }
        self.drag = Drag::Select { anchor };
    }

    /// Selects the word at `pos` and returns where it starts.
    fn select_word(&mut self, pos: Pos, cx: &mut Context<Self>) -> Pos {
        let text = self
            .document
            .read(cx)
            .doc()
            .line(pos.line)
            .ok()
            .flatten()
            .map(|l| l.text)
            .unwrap_or_default();
        let (start, end) = word_range(&text, pos.col);
        let (from, to) = (Pos::new(pos.line, start), Pos::new(pos.line, end));
        self.run_with(cx, |command_cx| command_cx.select_range(from, to))
            .ok();
        from
    }

    pub(crate) fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.key_help_pointer_moved(event.position, event.pressed_button.is_some(), window, cx);
        if event.pressed_button != Some(MouseButton::Left) {
            self.drag = Drag::None;
            return;
        }
        match self.drag {
            Drag::None => {}
            Drag::Select { anchor } => {
                // Above or below the view the pointer maps to the first or last row, and
                // moving the caret there scrolls.
                if let Some(pos) = self.position_for_point(event.position, cx) {
                    self.run_with(cx, |command_cx| {
                        if pos == anchor {
                            command_cx.move_caret(pos, false)
                        } else {
                            command_cx.select_range(anchor, pos)
                        }
                    })
                    .ok();
                }
            }
            Drag::Thumb {
                start_y,
                start_scroll,
            } => {
                let rows = self.layout.row_count();
                let bar = Scrollbar::new(&self.metrics, rows, self.scroll_y);
                let dy = f32::from(event.position.y) - start_y;
                self.scroll_past_end = false;
                self.scroll_y = bar.scroll_for_drag(&self.metrics, rows, start_scroll, dy);
                cx.notify();
            }
        }
    }

    pub(crate) fn on_mouse_up(
        &mut self,
        _: &MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The selection by the mouse is done: the tip may come for what is selected.
        self.key_help_restart(window, cx);
        if self.drag != Drag::None {
            self.drag = Drag::None;
            cx.notify();
        }
    }

    pub(crate) fn on_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.hide_key_help(cx);
        self.handle_wheel(event, window, cx);
    }

    /// Scrolls the text as the wheel does: a positive `dy` moves the text up.
    pub fn scroll_by(&mut self, dx: f32, dy: f32) {
        let rows = self.layout.row_count();
        self.scroll_past_end = false;
        self.scroll_y = clamp_scroll_y(&self.metrics, rows, self.scroll_y - dy);
        // The text can scroll right until the widest row of the last paint ends at the edge.
        let widest = (self.widest_cols as f32 + 1.0) * self.metrics.cell_width;
        let max_x = (widest - self.metrics.view_width).max(0.0);
        let wanted = (self.scroll_x - dx).max(0.0);
        self.scroll_x = wanted.min(max_x.max(self.scroll_x));
    }
}
