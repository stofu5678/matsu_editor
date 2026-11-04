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
use super::{Drag, EditorView};
use crate::geometry::{BarPart, Scrollbar, clamp_scroll_y};

impl EditorView {
    /// The bar as it is now, and whether it is the bar of a file that is still scanned.
    fn v_bar(&self, cx: &gpui::App) -> (Scrollbar, bool) {
        match self.scan_scrollbar(&self.metrics, cx) {
            Some(bar) => (bar, true),
            None => (
                Scrollbar::new(&self.metrics, self.layout.row_count(), self.scroll_y),
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
    pub(crate) fn v_scroll_press(&mut self, window_y: f32, cx: &mut Context<Self>) {
        let Some(paint) = self.paint else {
            return;
        };
        let (bar, scan) = self.v_bar(cx);
        self.scan_thumb_drag = scan;
        let pos = window_y - paint.vbar_top;
        let part = bar.part_at(&self.metrics, pos);
        if part == BarPart::Thumb {
            let rows = self.layout.row_count();
            self.drag = Drag::Thumb {
                start_y: window_y,
                // (for the bar of a scan: where the thumb was)
                start_scroll: if scan {
                    bar.thumb_top
                } else {
                    clamp_scroll_y(&self.metrics, rows, self.scroll_y)
                },
            };
        } else {
            self.v_scroll_step(part, cx);
            self.start_scroll_repeat(Axis::Vertical, part, pos, cx);
        }
        cx.notify();
    }

    /// What an arrow or a click on the track does, once (`SB_LINEUP`, `SB_PAGEUP`, `SB_PAGEDOWN`
    /// and `SB_LINEDOWN` of `OnVScroll`). The top stays between the first row and the last.
    pub(crate) fn v_scroll_step(&mut self, part: BarPart, cx: &gpui::App) {
        let m = self.metrics;
        let lines = crate::common_settings::scroll_options(cx)
            .repeated_lines
            .max(1) as f32;
        let delta = match part {
            BarPart::LineBack => -lines * m.line_height,
            BarPart::LineForward => lines * m.line_height,
            // Clicking the track pages up or down, as on Windows.
            BarPart::PageBack => -m.line_height * (m.visible_rows() as f32 - 1.0),
            BarPart::PageForward => m.line_height * (m.visible_rows() as f32 - 1.0),
            BarPart::Thumb => return,
        };
        // An arrow moves from the row at the top, not from a part of one.
        let from = match part {
            BarPart::LineBack | BarPart::LineForward => {
                (self.scroll_y / m.line_height).round() * m.line_height
            }
            _ => self.scroll_y,
        };
        self.scroll_past_end = false;
        self.scroll_y = clamp_scroll_y(&m, self.layout.row_count(), from + delta);
    }
}
