//! Holding an arrow or the track of a scroll bar down: it moves at once, waits a little, and then
//! goes on at a steady pace for as long as the pointer is still on what was pressed.
//!
//! Sakura's bars are the system's bars (`WC_SCROLLBAR`, `CEditView::CreateScrollBar`), which
//! repeat. The system's bar waits 200 ms and then repeats every 50 ms (`SCROLL_FIRST_DELAY` and
//! `SCROLL_REPEAT_DELAY` in the scroll bars of ReactOS and Wine, the open implementations of it).
//! Windows has no call that returns the two values, and the bars here are drawn by the view, so
//! they are fixed.
//!
//! A track press stops when the thumb comes under the pointer, as the system bar does: the part
//! under the pointer is not the part that was pressed any more.

use std::time::Duration;

use gpui::Context;

use super::{Drag, EditorView};
use crate::geometry::BarPart;

/// The wait before the first repeat.
pub(crate) const FIRST_DELAY: Duration = Duration::from_millis(200);
/// The wait between the repeats after that.
pub(crate) const INTERVAL: Duration = Duration::from_millis(50);

/// The wait before the `n`th repeat (0 for the first), after the press, which moved by itself.
pub(crate) fn delay_before(n: u32) -> Duration {
    if n == 0 { FIRST_DELAY } else { INTERVAL }
}

/// Which scroll bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Axis {
    Vertical,
    Horizontal,
}

/// A press that repeats.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScrollRepeat {
    pub axis: Axis,
    /// What was pressed: an arrow or a side of the track.
    pub part: BarPart,
    /// Where the pointer is now, in pixels from the start of the bar along it; `None` when it is
    /// off the bar (beside it or past either end).
    pub pointer: Option<f32>,
    /// Tells this press from the one before, for a task that is still waiting.
    pub id: u64,
}

/// Whether a repeat goes on: the pointer is still on what was pressed.
pub(crate) fn goes_on(pressed: BarPart, under_pointer: BarPart) -> bool {
    pressed == under_pointer
}

impl EditorView {
    /// Starts repeating `part` of the bar. The press itself has moved already.
    pub(crate) fn start_scroll_repeat(
        &mut self,
        axis: Axis,
        part: BarPart,
        pointer: f32,
        cx: &mut Context<Self>,
    ) {
        let id = self.scroll_repeat.map_or(0, |r| r.id) + 1;
        self.drag = Drag::Repeat;
        self.scroll_repeat = Some(ScrollRepeat {
            axis,
            part,
            pointer: Some(pointer),
            id,
        });
        self.scroll_repeat_task = Some(cx.spawn(async move |this, cx| {
            let mut n = 0;
            loop {
                cx.background_executor().timer(delay_before(n)).await;
                n += 1;
                match this.update(cx, |view, cx| view.scroll_repeat_tick(id, cx)) {
                    Ok(true) => {}
                    _ => break,
                }
            }
        }));
    }

    /// The pointer moves while the press is held.
    pub(crate) fn scroll_repeat_pointer(&mut self, x: f32, y: f32) {
        let Some(paint) = self.paint else {
            return;
        };
        if let Some(repeat) = &mut self.scroll_repeat {
            // The whole bar is the place: a pointer beside it or past its ends is off it.
            let (along, across, len, thick, across_start) = match repeat.axis {
                Axis::Vertical => (
                    y - paint.vbar_top,
                    x,
                    paint.vbar_len,
                    crate::geometry::SCROLLBAR_WIDTH,
                    paint.scrollbar_left,
                ),
                Axis::Horizontal => {
                    let Some(bar) = paint.hbar else {
                        repeat.pointer = None;
                        return;
                    };
                    (
                        x - bar.left,
                        y,
                        bar.width,
                        crate::geometry::HSCROLLBAR_HEIGHT,
                        bar.top,
                    )
                }
            };
            let on_bar = along >= 0.0
                && along < len
                && across >= across_start
                && across < across_start + thick;
            repeat.pointer = on_bar.then_some(along);
        }
    }

    /// One repeat. Returns whether the press is still held (the task ends if not).
    fn scroll_repeat_tick(&mut self, id: u64, cx: &mut Context<Self>) -> bool {
        let Some(repeat) = self.scroll_repeat else {
            return false;
        };
        if repeat.id != id || self.drag != Drag::Repeat {
            return false;
        }
        let under = repeat.pointer.and_then(|pointer| match repeat.axis {
            Axis::Vertical => self.v_part_at(pointer, cx),
            Axis::Horizontal => self.h_part_at(pointer),
        });
        if under.is_some_and(|under| goes_on(repeat.part, under)) {
            match repeat.axis {
                Axis::Vertical => self.v_scroll_step(repeat.part, cx),
                Axis::Horizontal => self.h_scroll_step(repeat.part),
            }
            cx.notify();
        }
        true
    }

    /// The button is released: the repeat ends.
    pub(crate) fn end_scroll_repeat(&mut self) {
        self.scroll_repeat = None;
        self.scroll_repeat_task = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_waits_longer_before_the_first_repeat_than_between_the_others() {
        assert_eq!(delay_before(0), Duration::from_millis(200));
        assert_eq!(delay_before(1), Duration::from_millis(50));
        assert_eq!(delay_before(40), Duration::from_millis(50));
    }

    #[test]
    fn it_goes_on_only_while_the_pointer_is_on_what_was_pressed() {
        assert!(goes_on(BarPart::PageForward, BarPart::PageForward));
        // The thumb has come under the pointer.
        assert!(!goes_on(BarPart::PageForward, BarPart::Thumb));
        assert!(!goes_on(BarPart::LineBack, BarPart::PageBack));
    }
}
