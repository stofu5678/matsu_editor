//! The pane of the minimap in a window, and the mouse on it.
//!
//! Sakura's minimap is a window of its own at the right of the text (`CEditWnd::LayoutMiniMap`,
//! `OnSize`: left of the outline when that is docked at the right). It has its own top row: the
//! text of the window does not drag it along. What follows the window is the mark on the rows the
//! window shows (`COLORIDX_PAGEVIEW`, `MiniMapRedraw`). A click moves the caret to that row (the
//! start of it) and the window scrolls to show it; Shift extends the selection; with the button
//! down, the pointer near the top or the bottom scrolls the minimap
//! (`CEditView::OnMOUSEMOVE`, `OnLBUTTONDOWN`, `OnLBUTTONUP`, all with `m_bMiniMap`).

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    Bounds, Context, Entity, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Pixels, Render, ScrollWheelEvent, Styled, Subscription, Window, div, px,
};

use super::{current, geometry};
use crate::geometry::SCROLLBAR_WIDTH;
use crate::view::EditorView;

/// How many rows a notch of the wheel scrolls: Sakura multiplies the lines per notch by 10 in the
/// minimap (`OnMOUSEWHEEL2`: `nRollLineNum *= 10` when `m_bMiniMap`).
const WHEEL_ROWS_FACTOR: f32 = 10.0;

/// What the mouse is doing between button down and button up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Drag {
    None,
    /// `m_bMiniMapMouseDown`: the button went down on the text.
    Text,
    /// The thumb of the scroll bar.
    Thumb {
        start_y: f32,
        start_top: usize,
    },
}

/// The minimap of one window: it shows the document of the pane that is active.
pub(crate) struct MiniMap {
    /// The pane the minimap draws, and the subscription that repaints it when that changes.
    source: Option<Entity<EditorView>>,
    watching: Option<Subscription>,
    /// The first row shown (`GetViewTopLine` of the minimap).
    pub(super) top_row: usize,
    pub(super) drag: Drag,
    /// The area of the last paint, in window coordinates.
    pub(super) area: Rc<Cell<Bounds<Pixels>>>,
    _settings: Subscription,
}

impl MiniMap {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        Self {
            source: None,
            watching: None,
            top_row: 0,
            drag: Drag::None,
            area: Rc::new(Cell::new(Bounds::default())),
            // The width and the font follow the settings.
            _settings: cx.observe_global::<super::MiniMapSettings>(|_, cx| cx.notify()),
        }
    }

    /// The minimap draws the document of `view` from now on (the pane that is active). Called by
    /// the window before it draws, so that a change of the active pane or tab shows at once.
    pub(crate) fn follow(&mut self, view: &Entity<EditorView>, cx: &mut Context<Self>) {
        if self.source.as_ref() == Some(view) {
            return;
        }
        self.source = Some(view.clone());
        self.watching = Some(cx.observe(view, |_, _, cx| cx.notify()));
        self.drag = Drag::None;
        cx.notify();
    }

    /// The minimap is hidden: it lets go of the pane, so that a closed document is not kept alive
    /// and a hidden minimap is not repainted.
    pub(crate) fn forget(&mut self) {
        self.source = None;
        self.watching = None;
        self.drag = Drag::None;
    }

    /// The pane the minimap draws.
    pub(crate) fn source(&self) -> Option<&Entity<EditorView>> {
        self.source.as_ref()
    }

    /// The height of a row, how many rows fit and how many the layout has.
    fn numbers(&self, cx: &gpui::App) -> Option<(f32, usize, usize)> {
        let source = self.source.as_ref()?;
        let row_height = geometry::row_height(current(cx).font_size);
        let height = f32::from(self.area.get().size.height);
        Some((
            row_height,
            geometry::view_rows(height, row_height),
            source.read(cx).minimap_total_rows(),
        ))
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(source) = self.source.clone() else {
            return;
        };
        let Some((_, view_rows, total)) = self.numbers(cx) else {
            return;
        };
        let area = self.area.get();
        let x = f32::from(event.position.x - area.left());
        let y = f32::from(event.position.y - area.top());
        if x >= f32::from(area.size.width) - SCROLLBAR_WIDTH {
            let (thumb_top, thumb_height) =
                geometry::thumb(self.top_row, view_rows, total, f32::from(area.size.height));
            if y >= thumb_top && y <= thumb_top + thumb_height {
                self.drag = Drag::Thumb {
                    start_y: f32::from(event.position.y),
                    start_top: self.top_row,
                };
            } else {
                // Clicking the track pages up or down, as on Windows.
                let page = view_rows.saturating_sub(1) as f32;
                let rows = if y < thumb_top { -page } else { page };
                self.top_row = geometry::top_after_wheel(self.top_row, rows, view_rows, total);
            }
            cx.notify();
            return;
        }
        // The caret goes to the row, and the typing goes on in the text.
        window.focus(&source.read(cx).focus_handle.clone(), cx);
        self.drag = Drag::Text;
        self.drag_to(y, event.modifiers.shift, cx);
    }

    /// `OnMOUSEMOVE` of the minimap with the button down: scrolls the minimap when the pointer is
    /// near its edge, and moves the caret of the window to the row under the pointer (with
    /// `extend` the selection grows to it).
    fn drag_to(&mut self, y: f32, extend: bool, cx: &mut Context<Self>) {
        let Some(source) = self.source.clone() else {
            return;
        };
        let Some((row_height, view_rows, total)) = self.numbers(cx) else {
            return;
        };
        let pointer_row = geometry::row_at(self.top_row, y, row_height);
        self.top_row = geometry::top_after_drag(self.top_row, view_rows, total, pointer_row);
        let row = geometry::row_at(self.top_row, y, row_height);
        source.update(cx, |view, cx| view.minimap_go_to_row(row, extend, cx));
        cx.notify();
    }

    /// The pointer moved with the window's mouse events (also outside the pane: Sakura captures
    /// the mouse while the button is down).
    pub(super) fn mouse_moved(
        &mut self,
        position: gpui::Point<Pixels>,
        left_down: bool,
        shift: bool,
        cx: &mut Context<Self>,
    ) {
        if !left_down {
            self.drag = Drag::None;
            return;
        }
        let area = self.area.get();
        match self.drag {
            Drag::None => {}
            Drag::Text => self.drag_to(f32::from(position.y - area.top()), shift, cx),
            Drag::Thumb { start_y, start_top } => {
                let Some((_, view_rows, total)) = self.numbers(cx) else {
                    return;
                };
                self.top_row = geometry::top_for_thumb_drag(
                    start_top,
                    f32::from(position.y) - start_y,
                    view_rows,
                    total,
                    f32::from(area.size.height),
                );
                cx.notify();
            }
        }
    }

    /// `OnLBUTTONUP`: `ReleaseCapture`.
    pub(super) fn mouse_released(&mut self, cx: &mut Context<Self>) {
        if self.drag != Drag::None {
            self.drag = Drag::None;
            cx.notify();
        }
    }

    fn wheel(&mut self, event: &ScrollWheelEvent, cx: &mut Context<Self>) {
        let Some((row_height, view_rows, total)) = self.numbers(cx) else {
            return;
        };
        // A positive delta is the wheel turned up: toward the top of the text.
        let delta = f32::from(event.delta.pixel_delta(px(1.0)).y);
        let rows = if event.delta.precise() {
            // a trackpad: the rows under the fingers
            -delta / row_height
        } else {
            -delta * WHEEL_ROWS_FACTOR
        };
        let top = geometry::top_after_wheel(self.top_row, rows, view_rows, total);
        if top != self.top_row {
            self.top_row = top;
            cx.notify();
        }
    }
}

impl Render for MiniMap {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = current(cx).pane_width();
        div()
            .id("minimap")
            .h_full()
            .w(px(width))
            .flex_none()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_scroll_wheel(cx.listener(|this, event, _, cx| this.wheel(event, cx)))
            .child(super::paint::minimap_canvas(cx.entity()))
    }
}
