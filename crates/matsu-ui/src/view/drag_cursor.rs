//! The shape of the mouse pointer over the view, and the part of a drag of the selected text that
//! has no button in it: scrolling near the edge and Esc.
//!
//! Sakura (`sakura_core/view/CEditView_Mouse.cpp`):
//! * `OnMOUSEMOVE` picks the pointer when no selection is being made. In the line number column
//!   it is the arrow that points to the right (`IDC_CURSOR_RVARROW`), over the ruler the arrow,
//!   over the selected text the arrow (when the text can be dragged out), over a URL the hand
//!   (`SetHandCursor`), and elsewhere the I-beam. While a selection is being made it is the
//!   I-beam.
//! * While the selected text is dragged (`m_bDragMode`) the caret follows the pointer
//!   (`MoveCursorToClientPoint`), and a caret near the edge of the view scrolls it
//!   (`CCaret::MoveCursor`, `m_bDragMode`: at most one scroll per 30 ms up and 80 ms down).
//! * Esc ends an OLE drag without a drop (`DoDragDrop` of the system); `DragLeave` puts the caret
//!   back where it was.
//!
//! Type B: the arrow that points to the right needs a picture of the cursor, which GPUI cannot
//! load, so the plain arrow stands in. The pointer of an OLE drop (the arrow with a box or a plus)
//! is the system's; here the dragged text shows [`CursorStyle::DragCopy`] when Ctrl is held and
//! the arrow otherwise. The cursors of the incremental search (`IDC_CURSOR_ISEARCH_F` and `_B`)
//! are pictures as well: the I-beam stays.

use std::time::Duration;

use gpui::{Context, CursorStyle, Modifiers};

use super::{Drag, EditorView};

/// The wait between two scrolls of a drag toward the top (`dwPerTiming = 30` of
/// `CCaret::MoveCursor`).
const SCROLL_UP_INTERVAL: Duration = Duration::from_millis(30);
/// The wait between the other scrolls of a drag (`dwPerTiming = 80`).
const SCROLL_INTERVAL: Duration = Duration::from_millis(80);

/// Which way a drag near the edge scrolls the rows: -1 up, 1 down, 0 none. `row` is the pointer's
/// row counted from the top of the view (negative above it), `visible` the rows that fit. The first
/// and the last row of the view scroll, and what is beyond them; Sakura's caret margin is not used
/// because the caret of a drag is only a mark.
pub(crate) fn edge_row_step(row: f32, visible: usize) -> i32 {
    if row < 1.0 {
        -1
    } else if row >= visible as f32 - 1.0 && visible > 2 {
        1
    } else {
        0
    }
}

/// Which way a drag near the side scrolls the columns: -1 left, 1 right, 0 none. `x` is the pointer
/// from the left of the text area (negative over the line numbers), `width` the width of the area
/// and `cell` the width of one column.
pub(crate) fn edge_col_step(x: f32, width: f32, cell: f32) -> i32 {
    if x < 0.0 {
        -1
    } else if x >= width - cell {
        1
    } else {
        0
    }
}

impl EditorView {
    /// The pointer for the point of the window, by what is there (`OnMOUSEMOVE`).
    pub(crate) fn cursor_style_at(
        &self,
        p: gpui::Point<gpui::Pixels>,
        m: Modifiers,
        cx: &gpui::App,
    ) -> CursorStyle {
        // The text of a drag: the arrow, or the copy mark while Ctrl is held (`TranslateDropEffect`).
        if matches!(self.drag, Drag::Drop) {
            return if m.control {
                CursorStyle::DragCopy
            } else {
                CursorStyle::Arrow
            };
        }
        // Selecting: the I-beam (the block after `IsMouseSelecting` in `OnMOUSEMOVE`).
        if !matches!(self.drag, Drag::None | Drag::Press { .. }) {
            return CursorStyle::IBeam;
        }
        let Some(paint) = self.paint else {
            return CursorStyle::IBeam;
        };
        let (x, y) = (f32::from(p.x), f32::from(p.y));
        if y < paint.origin_y || x < paint.origin_x || x >= paint.scrollbar_left {
            // The ruler, the line number column and the scroll bars.
            return CursorStyle::Arrow;
        }
        if paint.hbar.is_some_and(|bar| y >= bar.top) {
            return CursorStyle::Arrow;
        }
        let options = matsu_commands::edit_options::get();
        if options.drag_drop && options.drop_source && self.point_is_selected(p, cx) {
            return CursorStyle::Arrow;
        }
        if self
            .position_for_point(p, cx)
            .is_some_and(|pos| self.url_range_at(pos, cx).is_some())
        {
            return CursorStyle::PointingHand;
        }
        CursorStyle::IBeam
    }

    /// The pointer or a modifier key changed, or the drag did: redraws if the shape that the
    /// render would now give differs from the one it gave last. The shape itself is never kept
    /// as the truth: [`render`](gpui::Render) derives it from the drag state, the pointer of the
    /// window and the modifiers the window holds.
    pub(crate) fn cursor_changed(&mut self, window: &gpui::Window, cx: &mut Context<Self>) {
        let now = self.cursor_style_at(window.mouse_position(), window.modifiers(), cx);
        if now != self.shown_cursor {
            cx.notify();
        }
    }

    /// Starts the task that goes on scrolling while the pointer stays at the edge of the view
    /// during a drag of the selected text (the pointer sends nothing then). The view owns the
    /// task; it ends with the drag, and dropping it (a new drag, [`end_drag_scroll`]) stops it.
    ///
    /// [`end_drag_scroll`]: Self::end_drag_scroll
    pub(crate) fn start_drag_scroll(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.drag_scroll_task = Some(cx.spawn_in(window, async move |this, cx| {
            let mut wait = SCROLL_UP_INTERVAL;
            loop {
                cx.background_executor().timer(wait).await;
                let next = this.update_in(cx, |view, window, cx| {
                    if view.drag != Drag::Drop {
                        return None;
                    }
                    let p = window.mouse_position();
                    let (wait, scrolled) = view.drag_scroll(p, cx);
                    // The drop caret follows only a real scroll: a resting drag redraws nothing.
                    if scrolled {
                        view.move_drop_caret(p, cx);
                    }
                    Some(wait)
                });
                match next {
                    Ok(Some(next)) => wait = next,
                    _ => break,
                }
            }
        }));
    }

    /// The drag of the selected text is over: the scrolling task and its wait end.
    pub(crate) fn end_drag_scroll(&mut self) {
        self.drag_scroll_task = None;
    }

    /// The pointer of a drag of the selected text is near the edge of the view: scrolls it by a
    /// row (and a column). Returns how long to wait before the next step (Sakura's `dwPerTiming`:
    /// shorter when the view goes toward the top) and whether the view moved.
    fn drag_scroll(
        &mut self,
        p: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) -> (Duration, bool) {
        let Some(paint) = self.paint else {
            return (SCROLL_INTERVAL, false);
        };
        let row = (f32::from(p.y) - paint.origin_y) / self.metrics.line_height;
        let rows = edge_row_step(row, self.metrics.visible_rows());
        let width = paint.scrollbar_left - paint.origin_x;
        let cols = edge_col_step(
            f32::from(p.x) - paint.origin_x,
            width,
            self.metrics.cell_width,
        );
        let can_up = rows < 0 && self.scroll_y > 0.0;
        let can_down = rows > 0 && self.can_scroll_down();
        let can_left = cols < 0 && self.scroll_x > 0.0;
        let can_right = cols > 0;
        if !(can_up || can_down || can_left || can_right) {
            return (SCROLL_UP_INTERVAL, false);
        }
        let dy = if can_up || can_down {
            -rows as f32 * self.metrics.line_height
        } else {
            0.0
        };
        let dx = if can_left || can_right {
            -cols as f32 * self.metrics.cell_width
        } else {
            0.0
        };
        let (before_x, before_y) = (self.scroll_x, self.scroll_y);
        self.scroll_by(dx, dy);
        let moved = (self.scroll_x, self.scroll_y) != (before_x, before_y);
        if moved {
            cx.notify();
        }
        let wait = if can_up {
            SCROLL_UP_INTERVAL
        } else {
            SCROLL_INTERVAL
        };
        (wait, moved)
    }

    /// There are rows below the last one on the view.
    fn can_scroll_down(&self) -> bool {
        let rows = self.layout.row_count();
        crate::geometry::clamp_scroll_y(
            &self.metrics,
            rows,
            self.scroll_y + self.metrics.line_height,
        ) > self.scroll_y
    }

    /// Esc during a drag of the selected text (or a press in it that may become one): the drag
    /// ends and nothing is dropped. Returns whether there was one.
    pub(crate) fn cancel_drag(&mut self, cx: &mut Context<Self>) -> bool {
        if !matches!(self.drag, Drag::Drop | Drag::Press { .. }) {
            return false;
        }
        self.drag = Drag::None;
        self.drop_caret = None;
        self.end_drag_scroll();
        cx.notify();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_and_last_rows_and_beyond_scroll() {
        assert_eq!(edge_row_step(-3.0, 10), -1);
        assert_eq!(edge_row_step(0.5, 10), -1);
        assert_eq!(edge_row_step(1.5, 10), 0);
        assert_eq!(edge_row_step(8.5, 10), 0);
        assert_eq!(edge_row_step(9.0, 10), 1);
        assert_eq!(edge_row_step(14.0, 10), 1);
        // a view of one or two rows would scroll all the time
        assert_eq!(edge_row_step(1.5, 2), 0);
    }

    #[test]
    fn the_side_scrolls_over_the_numbers_and_in_the_last_column() {
        assert_eq!(edge_col_step(-5.0, 800.0, 8.0), -1);
        assert_eq!(edge_col_step(100.0, 800.0, 8.0), 0);
        assert_eq!(edge_col_step(792.0, 800.0, 8.0), 1);
    }
}
