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
    Styled as _, Window, canvas, div, px, rgb,
};

use super::dock::{self, DOCK_MIN_SIZE, DockSide, FloatRect};
use super::drop;
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
    /// The bar of a docked pane, or the left bar of the floating one, is held.
    dragging: bool,
    /// The bottom bar of the floating pane is held.
    dragging_bottom: bool,
    /// The title of the pane is held (`CDlgFuncList::Track`).
    track: Option<Track>,
}

/// The drag of the title of the pane.
struct Track {
    /// Where the title was taken hold of.
    grab: gpui::Point<Pixels>,
    /// The pointer has moved far enough for the drag to begin.
    started: bool,
    /// The place and the rectangle (in the coordinates of the window) the pane takes if it is let
    /// go now: the frame is drawn there.
    drop: Option<(DockSide, drop::Rect)>,
}

/// `GetIdealStretch`'s clamp for the floating pane: the pane (with its bars) as the area allows
/// it. `stored` is the position and the size that were set; without a size the pane is at the
/// right, with the width of a pane docked there and 70% of the height.
pub(crate) fn fit_float(
    stored: Option<FloatRect>,
    area_w: f32,
    area_h: f32,
    default_pane_w: f32,
) -> FloatRect {
    let least = DOCK_MIN_SIZE + BAR;
    let (x, y, w, h) = match stored.filter(FloatRect::has_size) {
        Some(r) => (r.x as f32, r.y as f32, r.w as f32, r.h as f32),
        None => {
            let w = default_pane_w + BAR;
            (area_w - w, 0.0, w, area_h * 0.7)
        }
    };
    let w = w.min(area_w).max(least.min(area_w));
    let h = h.min(area_h).max(least.min(area_h));
    let x = x.min(area_w - w).max(0.0);
    let y = y.min(area_h - h).max(0.0);
    FloatRect {
        x: x.round() as i32,
        y: y.round() as i32,
        w: w.round() as i32,
        h: h.round() as i32,
    }
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
            self.outline_layout.dragging_bottom = false;
            self.outline_layout.track = None;
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
        pane.read(cx).note_drawn(side);
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
                    cx.notify();
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
        // The place the pane takes if the title that is dragged is let go now (`Track` draws the
        // rectangle of `GetDropRect`).
        let frame = self
            .outline_layout
            .track
            .as_ref()
            .filter(|track| track.started)
            .and_then(|track| track.drop)
            .map(|(side, r)| {
                let thickness = if side.is_docked() {
                    drop::FRAME_DOCK
                } else {
                    drop::FRAME_FLOAT
                };
                div()
                    .id("outline-drop-frame")
                    .debug_selector(|| "outline-drop-frame".to_owned())
                    .absolute()
                    .left(px(r.left - f32::from(measured.origin.x)))
                    .top(px(r.top - f32::from(measured.origin.y)))
                    .w(px(r.width()))
                    .h(px(r.height()))
                    .border(px(thickness))
                    .border_color(rgb(0x5a7fd0))
                    .opacity(0.8)
            });
        let container = match side {
            DockSide::Float | DockSide::Undockable => {
                // over the text: a bar at its left edge and one at its bottom edge size it
                // (it hides what it covers from the mouse: a click is the pane's)
                let rect = fit_float(
                    pane.read(cx).float_rect(),
                    max_of(true),
                    max_of(false),
                    size,
                );
                pane.read(cx).set_float_shown(rect);
                let bottom = div()
                    .id("outline-float-bottom")
                    .debug_selector(|| "outline-float-bottom".to_owned())
                    .flex_none()
                    .w_full()
                    .h(px(BAR))
                    .bg(rgb(0xd8d8d8))
                    .cursor(CursorStyle::ResizeUpDown)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, _, cx| {
                            this.outline_layout.dragging_bottom = true;
                            cx.notify();
                            cx.stop_propagation();
                        }),
                    );
                container.child(text.size_full()).child(
                    div()
                        .id("outline-float")
                        .debug_selector(|| "outline-float".to_owned())
                        .absolute()
                        .occlude()
                        .left(px(rect.x as f32))
                        .top(px(rect.y as f32))
                        .w(px(rect.w as f32))
                        .h(px(rect.h as f32))
                        .flex()
                        .flex_col()
                        .shadow_lg()
                        .child(
                            div()
                                .flex()
                                .flex_1()
                                .min_h(px(0.))
                                .child(bar.w(px(BAR)).h_full().cursor(CursorStyle::ResizeLeftRight))
                                .child(
                                    div()
                                        .id("outline-pane-box")
                                        .flex_none()
                                        .debug_selector(|| "outline-pane-box".to_owned())
                                        .w(px(rect.w as f32 - BAR))
                                        .h_full()
                                        .child(pane.clone()),
                                ),
                        )
                        .child(bottom),
                )
            }
            DockSide::Left => container
                .flex_row()
                .child(pane_box(true))
                .child(bar.w(px(BAR)).h_full().cursor(CursorStyle::ResizeLeftRight))
                .child(text.h_full()),
            DockSide::Right => container
                .flex_row()
                .child(text.h_full())
                .child(bar.w(px(BAR)).h_full().cursor(CursorStyle::ResizeLeftRight))
                .child(pane_box(true)),
            DockSide::Top => container
                .flex_col()
                .child(pane_box(false))
                .child(bar.h(px(BAR)).w_full().cursor(CursorStyle::ResizeUpDown))
                .child(text.w_full()),
            DockSide::Bottom => container
                .flex_col()
                .child(text.w_full())
                .child(bar.h(px(BAR)).w_full().cursor(CursorStyle::ResizeUpDown))
                .child(pane_box(false)),
        };
        // While the title is held the whole area takes the pointer, as `SetCapture` does, so the
        // pane under it does not keep the release.
        let capture = (self.outline_layout.track.is_some()
            || self.outline_layout.dragging
            || self.outline_layout.dragging_bottom)
            .then(|| {
            div()
                .id("outline-capture")
                .absolute()
                .size_full()
                .occlude()
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
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(|this, _: &MouseDownEvent, _, cx| {
                        this.outline_track_cancel(cx);
                    }),
                )
        });
        container
            .children(capture)
            .children(frame)
            .into_any_element()
    }

    /// The pointer moves while a bar or the title is held.
    fn outline_drag_to(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.outline_layout.track.is_some() {
            self.outline_track_to(event, cx);
            return;
        }
        if !self.outline_layout.dragging && !self.outline_layout.dragging_bottom {
            return;
        }
        if !event.dragging() {
            self.outline_layout.dragging = false;
            self.outline_layout.dragging_bottom = false;
            cx.notify();
            return;
        }
        self.outline_resize(event.position, cx);
    }

    /// A bar is dragged: the pane takes the size that puts the bar under the pointer.
    fn outline_resize(&mut self, pointer: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        let Some(pane) = self.outline.clone() else {
            return;
        };
        let side = pane.read(cx).place(cx).side;
        if !side.is_docked() {
            self.float_resize(&pane, pointer, cx);
            return;
        }
        let size = size_for_pointer(side, self.outline_layout.area.get(), pointer);
        pane.update(cx, |pane, cx| pane.resize(size.round() as i32, cx));
    }

    /// The bars of the floating pane: the left one moves the left edge, the bottom one the bottom
    /// edge; the pane keeps [`DOCK_MIN_SIZE`] and stays inside the area.
    fn float_resize(
        &mut self,
        pane: &Entity<OutlinePane>,
        pointer: gpui::Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let Some(mut rect) = pane.read(cx).float_shown() else {
            return;
        };
        let area = self.outline_layout.area.get();
        let least = (DOCK_MIN_SIZE + BAR) as i32;
        if self.outline_layout.dragging {
            let right = rect.x + rect.w;
            let x = (f32::from(pointer.x - area.left()) - BAR / 2.0).round() as i32;
            rect.x = x.clamp(0, (right - least).max(0));
            rect.w = right - rect.x;
        }
        if self.outline_layout.dragging_bottom {
            let bottom = (f32::from(pointer.y - area.top()) + BAR / 2.0).round() as i32;
            let most = (f32::from(area.size.height).round() as i32 - rect.y).max(least);
            rect.h = (bottom - rect.y).clamp(least, most);
        }
        pane.update(cx, |pane, cx| pane.set_float(rect, cx));
    }

    /// A bar is let go: the size is kept.
    fn outline_end_drag(&mut self, event: &MouseUpEvent, cx: &mut Context<Self>) {
        if self.outline_layout.track.is_some() {
            self.outline_track_end(event, cx);
            return;
        }
        if !self.outline_layout.dragging && !self.outline_layout.dragging_bottom {
            return;
        }
        // The release can be outside the area, where no move reached us.
        self.outline_resize(event.position, cx);
        self.outline_layout.dragging = false;
        self.outline_layout.dragging_bottom = false;
        cx.notify();
        if let Some(pane) = self.outline.clone() {
            pane.update(cx, |pane, cx| pane.finish_resize(cx));
        }
    }

    /// The title of the pane is taken hold of (`OnNcLButtonDown`): the drag begins when the pointer
    /// has moved a little.
    pub(crate) fn outline_track_begin(&mut self, at: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        self.outline_layout.track = Some(Track {
            grab: at,
            started: false,
            drop: None,
        });
        cx.notify();
    }

    /// Esc or the right button: the drag is given up (`Track`). Returns whether there was one.
    pub(crate) fn outline_track_cancel(&mut self, cx: &mut Context<Self>) -> bool {
        let was = self.outline_layout.track.take().is_some_and(|t| t.started);
        cx.notify();
        was
    }

    /// What `GetDropRect` needs to know now; `None` while the pane has not been drawn.
    fn drop_context(&self, cx: &Context<Self>) -> Option<drop::DropContext> {
        let pane = self.outline.as_ref()?.read(cx);
        let place = pane.place(cx);
        let area = self.outline_layout.area.get();
        let (ax, ay) = (f32::from(area.origin.x), f32::from(area.origin.y));
        let (aw, ah) = (f32::from(area.size.width), f32::from(area.size.height));
        let rect = drop::Rect::new(ax, ay, aw, ah);
        let pane_rect = match place.side {
            DockSide::Left => {
                let s = ideal_size(place.left, aw) + BAR;
                drop::Rect::new(ax, ay, s, ah)
            }
            DockSide::Right => {
                let s = ideal_size(place.right, aw) + BAR;
                drop::Rect::new(ax + aw - s, ay, s, ah)
            }
            DockSide::Top => {
                let s = ideal_size(place.top, ah) + BAR;
                drop::Rect::new(ax, ay, aw, s)
            }
            DockSide::Bottom => {
                let s = ideal_size(place.bottom, ah) + BAR;
                drop::Rect::new(ax, ay + ah - s, aw, s)
            }
            DockSide::Float | DockSide::Undockable => {
                let r = pane.float_shown()?;
                drop::Rect::new(ax + r.x as f32, ay + r.y as f32, r.w as f32, r.h as f32)
            }
        };
        let remembered = dock::current(cx).float_if_remembered();
        let float_size = match remembered {
            Some(r) => (r.w as f32, r.h as f32),
            None => (ideal_size(place.right, aw) + BAR, ah * 0.7),
        };
        Some(drop::DropContext {
            area: rect,
            pane: pane_rect,
            side: place.side,
            kept: place,
            float_size,
            float_size_remembered: remembered.is_some(),
        })
    }

    fn outline_track_to(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if !event.dragging() {
            self.outline_layout.track = None;
            cx.notify();
            return;
        }
        let Some(ctx) = self.drop_context(cx) else {
            return;
        };
        let Some(track) = self.outline_layout.track.as_mut() else {
            return;
        };
        let moved = event.position - track.grab;
        if !track.started
            && f32::from(moved.x).abs() < drop::DRAG_START
            && f32::from(moved.y).abs() < drop::DRAG_START
        {
            return;
        }
        track.started = true;
        // a pane that cannot dock (ドッキング禁止) is only moved
        let force = event.modifiers.control || ctx.side == DockSide::Undockable;
        track.drop = Some(drop::drop_rect(
            &ctx,
            point_of(track.grab),
            point_of(event.position),
            force,
        ));
        cx.notify();
    }

    /// The title is let go: the pane takes the place the frame shows (`Track`, `WM_LBUTTONUP`).
    fn outline_track_end(&mut self, event: &MouseUpEvent, cx: &mut Context<Self>) {
        let Some(track) = self.outline_layout.track.take() else {
            return;
        };
        // the frame and the area that took the pointer go
        cx.notify();
        if !track.started {
            return;
        }
        let (Some(ctx), Some(pane)) = (self.drop_context(cx), self.outline.clone()) else {
            return;
        };
        let force = event.modifiers.control || ctx.side == DockSide::Undockable;
        let (side, rect) =
            drop::drop_rect(&ctx, point_of(track.grab), point_of(event.position), force);
        if side.is_docked() && side == ctx.side && rect == ctx.pane {
            // let go on the pane where it is: nothing moves
            return;
        }
        let area = self.outline_layout.area.get();
        let rel = FloatRect {
            x: (rect.left - f32::from(area.origin.x)).round() as i32,
            y: (rect.top - f32::from(area.origin.y)).round() as i32,
            w: rect.width().round() as i32,
            h: rect.height().round() as i32,
        };
        pane.update(cx, |pane, cx| pane.drop_at(side, rel, cx));
    }
}

fn point_of(p: gpui::Point<Pixels>) -> drop::Point {
    drop::Point {
        x: f32::from(p.x),
        y: f32::from(p.y),
    }
}
