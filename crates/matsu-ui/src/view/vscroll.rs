//! The events of the vertical scroll bar: an arrow button at each end, the track, and the thumb.
//!
//! Sakura: the bar is the system's (`CEditView::CreateScrollBar`, `SBS_VERT`), so it has arrows and
//! repeats; `CEditView::OnVScroll` (`sakura_core/view/CEditView_Scroll.cpp`) is what each part does:
//! an arrow moves `nRepeatedScrollLineNum` rows (3 by default, the common setting of the held up
//! and down keys), the track moves a page, and the thumb sets the top row.
//!
//! The bar is painted by the element of the view; this file is the events. While a huge file is
//! scanned the thumb is put by byte positions (`scan_window.rs`), and the same parts work on it.

use gpui::Context;

use super::scroll_repeat::Axis;
use super::{Drag, EditorView, ThumbStart};
use crate::geometry::{BarPart, Scrollbar, clamp_scroll};
use crate::scroll_pos::VScroll;

impl EditorView {
    /// The bar as it is now, and whether it is the bar of a file that is still scanned.
    fn v_bar(&self, cx: &gpui::App) -> (Scrollbar, bool) {
        match self.scan_scrollbar(&self.metrics, cx) {
            Some(bar) => (bar, true),
            None => (
                Scrollbar::new(&self.metrics, self.layout.row_count(), self.vscroll),
                false,
            ),
        }
    }

    /// The part of the bar at `pos` pixels from its top end, if the view has painted.
    pub(crate) fn v_part_at(&self, pos: f32, cx: &gpui::App) -> Option<BarPart> {
        self.paint?;
        let (bar, _) = self.v_bar(cx);
        Some(bar.part_at(&self.metrics, pos))
    }

    /// A press at `window_y` on the bar.
    pub(crate) fn v_scroll_press(
        &mut self,
        window_y: f32,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        let Some(paint) = self.paint else {
            return;
        };
        let (bar, scan) = self.v_bar(cx);
        let pos = window_y - paint.vbar_top;
        let part = bar.part_at(&self.metrics, pos);
        if part == BarPart::Thumb {
            let rows = self.layout.row_count();
            self.set_drag(Drag::Thumb {
                start_y: window_y,
                // (for the bar of a scan: where the thumb was)
                start: if scan {
                    ThumbStart::ScanBar(bar.thumb_top)
                } else {
                    ThumbStart::Top(clamp_scroll(&self.metrics, rows, self.vscroll))
                },
            });
        } else {
            self.v_scroll_step(part, cx);
            self.start_scroll_repeat(Axis::Vertical, part, pos, window, cx);
        }
        cx.notify();
    }

    /// What an arrow or a click on the track does, once (`SB_LINEUP`, `SB_PAGEUP`, `SB_PAGEDOWN`
    /// and `SB_LINEDOWN` of `OnVScroll`). The top stays between the first row and the last.
    pub(crate) fn v_scroll_step(&mut self, part: BarPart, cx: &gpui::App) {
        let m = self.metrics;
        let lines = crate::common_settings::scroll_options(cx)
            .repeated_lines
            .max(1) as isize;
        let page = (m.visible_rows() as isize - 1).max(0);
        // An arrow moves from the row at the top, not from a part of one; the track pages from
        // where the view is.
        let from_row = VScroll::at_row(self.vscroll.nearest_row(m.line_height));
        let to = match part {
            BarPart::LineBack => from_row.shifted_rows(-lines),
            BarPart::LineForward => from_row.shifted_rows(lines),
            // Clicking the track pages up or down, as on Windows.
            BarPart::PageBack => self.vscroll.shifted_rows(-page),
            BarPart::PageForward => self.vscroll.shifted_rows(page),
            BarPart::Thumb => return,
        };
        self.scroll_past_end = false;
        self.vscroll = clamp_scroll(&m, self.layout.row_count(), to);
    }
}
