//! Where the window puts the outline pane, and the splitter that sizes it.
//!
//! Sakura: `CEditWnd::OnSize2` (`sakura_core/window/CEditWnd.cpp`) puts the docked dialog at the
//! edge of the area between the bars: a pane at the left or the right takes the full height of
//! that area, a pane at the top or the bottom the full width, and the minimap and the text are
//! what is left. `CDlgFuncList::OnLButtonDown/Move/Up` size it with the bar on its inner edge
//! (`m_bStretching`), and `DOCK_MIN_SIZE` keeps both parts from vanishing.
//!
//! Floating: Sakura's floating form is a window of its own. GPUI has no window that stays above
//! another one of the application on every OS, and the pane takes the keys and the focus of its
//! window, so here the floating pane is drawn over the text, at the right, with the same bar to
//! size it (type B of ADR-0018: the nearest form).

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    AnyElement, Bounds, Context, CursorStyle, Entity, InteractiveElement as _, IntoElement as _,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement as _, Pixels,
    Styled as _, Window, canvas, div, px, relative, rgb,
};

use super::dock::{DOCK_MIN_SIZE, DockSide};
use super::pane::OutlinePane;
use crate::app::Workspace;
use crate::minimap::panel::MiniMap;

/// The width of the bar between the pane and the text.
const BAR: f32 = 5.0;

/// The part of the window that the pane and the text share, as of the last paint, and whether the
/// bar is being dragged.
#[derive(Default)]
pub(crate) struct OutlineLayout {
    area: Rc<Cell<Bounds<Pixels>>>,
    dragging: bool,
}

/// `GetIdealStretch`: the size of a pane whose kept size is `kept` (0: not set, a third of the
/// area), inside an area of `max`; the text keeps at least [`DOCK_MIN_SIZE`].
pub(crate) fn ideal_size(kept: i32, max: f32) -> f32 {
    let size = if kept <= 0 { max / 3.0 } else { kept as f32 };
    let size = size.min(max - BAR - DOCK_MIN_SIZE);
    size.max(DOCK_MIN_SIZE)
}

/// The size of the pane that a pointer at `pointer` gives, for the area `area`, with the pane at
/// `side`: the bar follows the pointer.
pub(crate) fn size_for_pointer(
    side: DockSide,
    area: Bounds<Pixels>,
    pointer: gpui::Point<Pixels>,
) -> f32 {
    let bar = BAR / 2.0;
    let size = match side.size_side() {
        DockSide::Left => f32::from(pointer.x - area.left()) - bar,
        DockSide::Right => f32::from(area.right() - pointer.x) - bar,
        DockSide::Top => f32::from(pointer.y - area.top()) - bar,
        _ => f32::from(area.bottom() - pointer.y) - bar,
    };
    let max = if side.size_side().is_vertical() {
        f32::from(area.size.width)
    } else {
        f32::from(area.size.height)
    };
    size.min(max - BAR - DOCK_MIN_SIZE).max(DOCK_MIN_SIZE)
}

impl Workspace {
    /// The text area of the window with the outline pane (if there is one) at the side the
    /// settings say.
    pub(crate) fn render_text_area(
        &mut self,
        body: AnyElement,
        minimap: Option<Entity<MiniMap>>,
        outline: Option<Entity<OutlinePane>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // The text and the minimap: Sakura puts the minimap between the text and a pane at the
        // right.
        let text = div()
            .id("outline-text")
            .flex_1()
            .flex()
            .min_w(px(0.))
            .min_h(px(0.))
            .overflow_hidden()
            .debug_selector(|| "outline-text".to_owned())
            .child(div().flex_1().h_full().overflow_hidden().child(body))
            .children(minimap);
        let Some(pane) = outline else {
            self.outline_layout.dragging = false;
            return text.into_any_element();
        };
        let place = pane.read(cx).place(cx);
        let side = place.side;
        let area = self.outline_layout.area.clone();
        let measured = area.get();
        let max_of = |vertical: bool| {
            let known = if vertical {
                f32::from(measured.size.width)
            } else {
                f32::from(measured.size.height)
            };
            if known > 0.0 {
                known
            } else if vertical {
                f32::from(window.viewport_size().width)
            } else {
                f32::from(window.viewport_size().height)
            }
        };
        let size_side = side.size_side();
        let size = ideal_size(place.size_of(size_side), max_of(size_side.is_vertical()));
        let measure = canvas(
            move |bounds, window, _| {
                // a size the last paint did not know (the window changed): draw again with it
                if area.get() != bounds {
                    area.set(bounds);
                    window.refresh();
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();
        let container = div()
            .id("outline-area")
            .relative()
            .flex_1()
            .min_h(px(0.))
            .min_w(px(0.))
            .flex()
            .overflow_hidden()
            .on_mouse_move(
                cx.listener(|this, e: &MouseMoveEvent, _, cx| this.outline_drag_to(e, cx)),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, e: &MouseUpEvent, _, cx| this.outline_end_drag(e, cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, e: &MouseUpEvent, _, cx| this.outline_end_drag(e, cx)),
            )
            .child(measure);
        let bar = div()
            .id("outline-splitter")
            .flex_none()
            .bg(rgb(0xd8d8d8))
            .debug_selector(|| "outline-splitter".to_owned())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.outline_layout.dragging = true;
                    cx.stop_propagation();
                }),
            );
        let pane_box = |vertical: bool| {
            let boxed = div()
                .id("outline-pane-box")
                .flex_none()
                .debug_selector(|| "outline-pane-box".to_owned())
                .child(pane.clone());
            if vertical {
                boxed.w(px(size)).h_full()
            } else {
                boxed.h(px(size)).w_full()
            }
        };
        match side {
            DockSide::Float | DockSide::Undockable => container
                .child(text.size_full())
                .child(
                    // over the text, at the right: a bar at its left edge sizes it
                    // (it hides what it covers from the mouse: a click is the pane's)
                    div()
                        .absolute()
                        .occlude()
                        .top(px(0.))
                        .right(px(0.))
                        .w(px(size + BAR))
                        .h(relative(0.7))
                        .flex()
                        .shadow_lg()
                        .child(bar.w(px(BAR)).h_full().cursor(CursorStyle::ResizeLeftRight))
                        .child(pane_box(true)),
                )
                .into_any_element(),
            DockSide::Left => container
                .flex_row()
                .child(pane_box(true))
                .child(bar.w(px(BAR)).h_full().cursor(CursorStyle::ResizeLeftRight))
                .child(text.h_full())
                .into_any_element(),
            DockSide::Right => container
                .flex_row()
                .child(text.h_full())
                .child(bar.w(px(BAR)).h_full().cursor(CursorStyle::ResizeLeftRight))
                .child(pane_box(true))
                .into_any_element(),
            DockSide::Top => container
                .flex_col()
                .child(pane_box(false))
                .child(bar.h(px(BAR)).w_full().cursor(CursorStyle::ResizeUpDown))
                .child(text.w_full())
                .into_any_element(),
            DockSide::Bottom => container
                .flex_col()
                .child(text.w_full())
                .child(bar.h(px(BAR)).w_full().cursor(CursorStyle::ResizeUpDown))
                .child(pane_box(false))
                .into_any_element(),
        }
    }

    /// The bar is dragged: the pane takes the size that puts the bar under the pointer.
    fn outline_drag_to(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if !self.outline_layout.dragging {
            return;
        }
        if !event.dragging() {
            self.outline_layout.dragging = false;
            return;
        }
        self.outline_resize(event.position, cx);
    }

    fn outline_resize(&mut self, pointer: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        let Some(pane) = self.outline.clone() else {
            return;
        };
        let side = pane.read(cx).place(cx).side;
        let size = size_for_pointer(side, self.outline_layout.area.get(), pointer);
        pane.update(cx, |pane, cx| pane.resize(size.round() as i32, cx));
    }

    /// The bar is let go: the size is kept.
    fn outline_end_drag(&mut self, event: &MouseUpEvent, cx: &mut Context<Self>) {
        if !self.outline_layout.dragging {
            return;
        }
        // The release can be outside the area, where no move reached us.
        self.outline_resize(event.position, cx);
        self.outline_layout.dragging = false;
        if let Some(pane) = self.outline.clone() {
            pane.update(cx, |pane, cx| pane.finish_resize(cx));
        }
    }
}
