//! The panes of a window: Sakura's split view (画面分割).
//!
//! A window shows its document in one pane, or in two or four panes with draggable bars
//! (`F_SPLIT_V`, `F_SPLIT_H`, `F_SPLIT_VH`, and the bars). Every pane is an `Entity<EditorView>`
//! of the same `SharedDocument` (ADR-0013 section 3): it has its own caret, selection, layout and
//! scroll position, and lays out again when another pane edits the text. One pane is active: it
//! has the focus, the title and the status bar come from it, and the commands act on it.
//!
//! The rules (which panes a split shows, who copies whose view, which pane is active) are in
//! [`plan`]; this file does them on the views and draws the bars.
//!
//! Sakura: `sakura_core/window/CSplitterWnd.cpp`, `CEditWnd::CreateEditViewBySplit`, and the
//! synchronized scrolling of `CEditView::SyncScrollV` / `SyncScrollH`
//! (`sakura_core/view/CEditView_Scroll.cpp`).

pub(crate) mod plan;
pub use plan::SplitKind;
#[cfg(test)]
mod tests;

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    AnyElement, App, AppContext as _, Bounds, Context, CursorStyle, Entity, Focusable,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement, Pixels, Styled, Subscription, Window, canvas, div, px, relative, rgb,
};

use crate::app::Workspace;
use crate::view::EditorView;
use plan::{Active, Grid, Plan, do_split, request};

/// `m_bSplitterWndVScroll` and `m_bSplitterWndHScroll` (`sakura_core/env/CShareData.cpp`): the
/// default of both is on. There is no setting for them yet.
const SYNC_SCROLL_V: bool = true;
const SYNC_SCROLL_H: bool = true;

/// The width of a bar (`SPLITTER_FRAME_WIDTH`).
const BAR_WIDTH: f32 = 3.0;

/// The bar being dragged (`m_bDragging`: 1, 2 or 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bar {
    /// The bar between the upper and the lower panes.
    Rows,
    /// The bar between the left and the right panes.
    Cols,
    /// Where the two cross.
    Both,
}

/// The panes of one window.
pub(crate) struct PaneSet {
    /// Sakura's pane numbers: 0 top left, 1 top right, 2 bottom left, 3 bottom right. Only the
    /// panes of `grid` exist; pane 0 always does.
    slots: [Option<Entity<EditorView>>; 4],
    grid: Grid,
    active: usize,
    /// The bars, as fractions of the area (`m_nHSplitPos`, `m_nVSplitPos`).
    h: f32,
    v: f32,
    /// The area of the last paint.
    area: Rc<Cell<Bounds<Pixels>>>,
    dragging: Option<Bar>,
    /// What each pane's subscriptions are (focus, changes); dropped with the pane.
    subscriptions: [Vec<Subscription>; 4],
    /// The scroll position each pane had when we last looked, to see who scrolled.
    scrolled: [(f32, f32); 4],
}

impl PaneSet {
    pub(crate) fn new(first: Entity<EditorView>) -> Self {
        Self {
            slots: [Some(first), None, None, None],
            grid: Grid::ONE,
            active: 0,
            h: 0.0,
            v: 0.0,
            area: Rc::new(Cell::new(Bounds::default())),
            dragging: None,
            subscriptions: Default::default(),
            scrolled: [(0.0, 0.0); 4],
        }
    }

    pub(crate) fn active(&self) -> &Entity<EditorView> {
        self.slots[self.active]
            .as_ref()
            .expect("the active pane exists")
    }

    fn pane(&self, index: usize) -> Option<&Entity<EditorView>> {
        self.slots.get(index).and_then(Option::as_ref)
    }

    /// The panes that are shown, in Sakura's order.
    #[cfg(test)]
    pub(crate) fn shown(&self) -> Vec<Entity<EditorView>> {
        self.grid
            .panes()
            .iter()
            .filter_map(|&i| self.slots[i].clone())
            .collect()
    }

    /// How the window is split now: rows and columns.
    #[cfg(test)]
    pub(crate) fn grid(&self) -> (u8, u8) {
        (self.grid.rows, self.grid.cols)
    }

    #[cfg(test)]
    pub(crate) fn active_index(&self) -> usize {
        self.active
    }

    /// The pane with Sakura's number `index`, if it is shown.
    #[cfg(test)]
    pub(crate) fn pane_at(&self, index: usize) -> Option<&Entity<EditorView>> {
        if self.grid.shows(index) {
            self.pane(index)
        } else {
            None
        }
    }

    fn index_of(&self, pane: &Entity<EditorView>) -> Option<usize> {
        self.slots.iter().position(|s| s.as_ref() == Some(pane))
    }

    /// The size of the area in pixels; a default before the first paint.
    fn area_size(&self) -> (f32, f32) {
        let b = self.area.get();
        let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
        if w > 0.0 && h > 0.0 {
            (w, h)
        } else {
            (1000.0, 600.0)
        }
    }
}

impl Workspace {
    /// Splits the window, or removes the split, the way `F_SPLIT_V`, `F_SPLIT_H` and
    /// `F_SPLIT_VH` do (`CSplitterWnd::VSplitOnOff` and the others).
    pub fn split(&mut self, kind: SplitKind, window: &mut Window, cx: &mut Context<Self>) {
        let set = &self.panes;
        let (w, h) = set.area_size();
        let (rh, rv) = request(kind, set.grid, set.h * w, set.v * h, w, h);
        self.do_split(rh, rv, window, cx);
    }

    /// `CSplitterWnd::DoSplit`: the bars at `h` and `v` pixels (0: none).
    fn do_split(&mut self, h: f32, v: f32, window: &mut Window, cx: &mut Context<Self>) {
        let (w, height) = self.panes.area_size();
        let plan = do_split(self.panes.grid, self.panes.active, h, v, w, height);
        self.apply(plan, w, height, window, cx);
    }

    fn apply(
        &mut self,
        plan: Plan,
        w: f32,
        height: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // A macro that runs belongs to its pane: the pane stays until the macro ends.
        let busy = (0..4).find(|&i| {
            !plan.grid.shows(i)
                && self.panes.slots[i]
                    .as_ref()
                    .is_some_and(|p| p.read(cx).is_macro_running())
        });
        if busy.is_some() {
            let active = self.panes.active().clone();
            active.update(cx, |view, cx| {
                view.notice = Some("マクロの実行中は、その画面の分割を解除できない".into());
                cx.notify();
            });
            return;
        }
        let old_active = self.panes.active;
        // The panes that come to be shown start like the pane the user is in.
        for &i in plan.grid.panes() {
            if self.panes.slots[i].is_none() {
                self.make_pane(i, window, cx);
            }
        }
        for &(from, to) in &plan.copies {
            let status = self.panes.slots[from]
                .as_ref()
                .map(|p| p.read(cx).view_status());
            if let (Some(status), Some(pane)) = (status, self.panes.slots[to].clone()) {
                pane.update(cx, |view, cx| {
                    view.apply_view_status(status);
                    cx.notify();
                });
            }
        }
        if plan.stagger {
            self.stagger_pane_2(plan.v, height, cx);
        }
        let new_active = match plan.active {
            Active::Pane(n) => n,
            Active::ByCaret => self.pane_2_has_the_caret(cx),
        };
        self.panes.grid = plan.grid;
        self.panes.h = if w > 0.0 { plan.h / w } else { 0.0 };
        self.panes.v = if height > 0.0 { plan.v / height } else { 0.0 };
        // The window's own state moves with the user to the pane that is active afterwards; the
        // panes that go away are dropped after that.
        self.move_window_state(old_active, new_active, window, cx);
        for i in 0..4 {
            if !plan.grid.shows(i) {
                self.panes.slots[i] = None;
                self.panes.subscriptions[i].clear();
            }
        }
        self.panes.active = new_active;
        self.note_scroll_positions(cx);
        if let Some(pane) = self.panes.pane(new_active) {
            window.focus(&pane.read(cx).focus_handle(cx), cx);
        }
        cx.notify();
    }

    /// Pane 2 shows the text from where pane 0 ends (`SetViewTopLine( top + m_nViewRowNum )` of
    /// `DoSplit`).
    fn stagger_pane_2(&mut self, bar: f32, height: f32, cx: &mut Context<Self>) {
        let (Some(first), Some(second)) =
            (self.panes.pane(0).cloned(), self.panes.pane(2).cloned())
        else {
            return;
        };
        let (top, rows) = {
            let view = first.read(cx);
            let m = view.metrics();
            // pane 0 loses what pane 2 will take: the part of the area below the bar
            let shown = ((m.view_height - (height - bar)) / m.line_height).floor();
            let top = (view.scroll_y() / m.line_height).round().max(0.0) as usize;
            (top, shown.max(1.0) as usize)
        };
        second.update(cx, |view, cx| {
            view.move_view_top_line((top + rows) as isize, cx)
        });
    }

    /// `pcViewArr[2]->GetTextArea().GetViewTopLine() < pcViewArr[2]->GetCaret().GetCaretLayoutPos().y`:
    /// the caret is below the first row of pane 2, so pane 2 becomes active.
    fn pane_2_has_the_caret(&self, cx: &App) -> usize {
        let Some(pane) = self.panes.pane(2) else {
            return 0;
        };
        let view = pane.read(cx);
        let top = (view.scroll_y() / view.metrics().line_height)
            .round()
            .max(0.0) as usize;
        if top < view.caret_row(cx).row { 2 } else { 0 }
    }

    /// A pane for slot `index`, made like the active pane.
    fn make_pane(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let source = self.panes.active().clone();
        let seed = source.read(cx).pane_seed();
        let document = source.read(cx).document().clone();
        let host = cx.weak_entity();
        let pane = cx.new(|cx| EditorView::new_pane(document, seed, host, cx));
        self.adopt(index, pane, window, cx);
    }

    /// Puts `pane` into slot `index` and listens to it.
    pub(crate) fn adopt(
        &mut self,
        index: usize,
        pane: Entity<EditorView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let handle = pane.read(cx).focus_handle(cx);
        let focus = cx.on_focus_in(&handle, window, {
            let pane = pane.clone();
            move |this, window, cx| this.pane_got_focus(&pane, window, cx)
        });
        let watch = cx.observe(&pane, {
            let pane = pane.clone();
            move |this, _, cx| this.pane_changed(&pane, cx)
        });
        self.panes.subscriptions[index] = vec![focus, watch];
        self.panes.slots[index] = Some(pane);
    }

    /// The user went to another pane (a click, or a key that moves the focus).
    fn pane_got_focus(
        &mut self,
        pane: &Entity<EditorView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self.panes.index_of(pane) {
            self.activate(index, window, cx);
        }
    }

    /// Makes the pane `index` the active one.
    fn activate(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index == self.panes.active || !self.panes.grid.shows(index) {
            return;
        }
        let old = self.panes.active;
        self.move_window_state(old, index, window, cx);
        self.panes.active = index;
        cx.notify();
    }

    /// The search pattern, the key macro and the status bar belong to the window, so they go to
    /// the pane that is active.
    fn move_window_state(
        &mut self,
        from: usize,
        to: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if from == to {
            return;
        }
        let (Some(old), Some(new)) = (self.panes.slots[from].clone(), self.panes.slots[to].clone())
        else {
            return;
        };
        // A macro that runs belongs to the view that runs it.
        let with_macros = !old.read(cx).is_macro_running() && !new.read(cx).is_macro_running();
        let state = old.update(cx, |view, _| view.take_window_state(with_macros));
        new.update(cx, |view, cx| {
            view.put_window_state(state);
            // an open search panel shows the pattern it had
            view.show_pattern_in_panel(window, cx);
            cx.notify();
        });
    }

    fn note_scroll_positions(&mut self, cx: &App) {
        for i in 0..4 {
            if let Some(pane) = &self.panes.slots[i] {
                self.panes.scrolled[i] = pane.read(cx).scroll();
            }
        }
    }

    /// Synchronized scrolling: when a pane scrolls vertically, the pane beside it (the other of a
    /// left-right pair) scrolls to the same top row; when it scrolls horizontally, the pane above
    /// or below it takes the same left column.
    ///
    /// `CEditView::SyncScrollV` uses pane `index ^ 1` and `SyncScrollH` pane `index ^ 2`
    /// (`sakura_core/view/CEditView_Scroll.cpp`), if the pane is shown.
    fn pane_changed(&mut self, pane: &Entity<EditorView>, cx: &mut Context<Self>) {
        let Some(index) = self.panes.index_of(pane) else {
            return;
        };
        let (x, y) = pane.read(cx).scroll();
        let past_end = pane.read(cx).scroll_past_end();
        // Panes can have other font sizes (the zoom is per pane): the top row and the left
        // column are what is kept in step, not the pixels.
        let metrics = *pane.read(cx).metrics();
        let top_row = y / metrics.line_height;
        let left_col = x / metrics.cell_width;
        let (last_x, last_y) = self.panes.scrolled[index];
        self.panes.scrolled[index] = (x, y);
        if SYNC_SCROLL_V && y != last_y {
            self.sync_scroll(index ^ 1, None, Some((top_row, past_end)), cx);
        }
        if SYNC_SCROLL_H && x != last_x {
            self.sync_scroll(index ^ 2, Some(left_col), None, cx);
        }
    }

    fn sync_scroll(
        &mut self,
        partner: usize,
        left_col: Option<f32>,
        top_row: Option<(f32, bool)>,
        cx: &mut Context<Self>,
    ) {
        if !self.panes.grid.shows(partner) {
            return;
        }
        let Some(other) = self.panes.slots[partner].clone() else {
            return;
        };
        // Remember what the partner will have, so that its change is not synchronized back.
        let after = other.update(cx, |view, cx| {
            let m = *view.metrics();
            view.set_scroll(
                left_col.map(|col| col * m.cell_width),
                top_row.map(|(row, past_end)| (row * m.line_height, past_end)),
            );
            cx.notify();
            view.scroll()
        });
        self.panes.scrolled[partner] = after;
    }

    // ---- the bars ----

    fn start_drag(
        &mut self,
        bar: Bar,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.click_count >= 2 {
            // OnLButtonDblClk: a double click removes the split on that bar.
            let (w, h) = self.panes.area_size();
            let keep_h = if self.panes.grid.cols > 1 {
                self.panes.h * w
            } else {
                0.0
            };
            let keep_v = if self.panes.grid.rows > 1 {
                self.panes.v * h
            } else {
                0.0
            };
            match bar {
                Bar::Rows => self.do_split(keep_h, 0.0, window, cx),
                Bar::Cols => self.do_split(0.0, keep_v, window, cx),
                Bar::Both => self.do_split(0.0, 0.0, window, cx),
            }
            cx.stop_propagation();
            return;
        }
        self.panes.dragging = Some(bar);
        cx.stop_propagation();
    }

    fn drag_to(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.panes.dragging.is_none() {
            return;
        }
        if !event.dragging() {
            self.panes.dragging = None;
            return;
        }
        self.move_bar(event.position, cx);
    }

    /// Moves the bar being dragged to `position`, within the area.
    fn move_bar(&mut self, position: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        let Some(bar) = self.panes.dragging else {
            return;
        };
        let area = self.panes.area.get();
        let (w, h) = self.panes.area_size();
        // OnMouseMove keeps the tracker inside the area (1 pixel from the top left, 6 from the
        // bottom right).
        let x = f32::from(position.x - area.origin.x).clamp(1.0, (w - 6.0).max(1.0));
        let y = f32::from(position.y - area.origin.y).clamp(1.0, (h - 6.0).max(1.0));
        if matches!(bar, Bar::Cols | Bar::Both) {
            self.panes.h = x / w;
        }
        if matches!(bar, Bar::Rows | Bar::Both) {
            self.panes.v = y / h;
        }
        cx.notify();
    }

    /// OnLButtonUp: the split with the bars where the user dropped them.
    fn end_drag(&mut self, event: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.panes.dragging.is_none() {
            return;
        }
        // The release can be outside the area, where no move reached us.
        self.move_bar(event.position, cx);
        self.panes.dragging = None;
        let (w, h) = self.panes.area_size();
        let set = &self.panes;
        let x = if set.grid.cols > 1 { set.h * w } else { 0.0 };
        let y = if set.grid.rows > 1 { set.v * h } else { 0.0 };
        self.do_split(x, y, window, cx);
    }

    // ---- drawing ----

    /// The text area: the pane itself when the window is not split, else the panes and the bars.
    pub(crate) fn render_panes(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let set = &self.panes;
        let area = set.area.clone();
        let measure = canvas(move |bounds, _, _| area.set(bounds), |_, _, _, _| {})
            .absolute()
            .size_full();
        if !set.grid.is_split() {
            // The area is measured here too: the first split needs the size of the window.
            return div()
                .relative()
                .size_full()
                .child(measure)
                .child(set.active().clone())
                .into_any_element();
        }
        let (rows, cols) = (set.grid.rows > 1, set.grid.cols > 1);
        let (h, v) = (set.h, set.v);
        let mut root = div()
            .id("panes")
            .relative()
            .size_full()
            .overflow_hidden()
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| this.drag_to(e, cx)))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, e, window, cx| this.end_drag(e, window, cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, e, window, cx| this.end_drag(e, window, cx)),
            )
            .child(measure);
        for &i in set.grid.panes() {
            let Some(pane) = set.pane(i) else { continue };
            let (col, row) = (i % 2, i / 2);
            let left = if cols && col == 1 { h } else { 0.0 };
            let top = if rows && row == 1 { v } else { 0.0 };
            let width = match (cols, col) {
                (false, _) => 1.0,
                (true, 0) => h,
                (true, _) => 1.0 - h,
            };
            let height = match (rows, row) {
                (false, _) => 1.0,
                (true, 0) => v,
                (true, _) => 1.0 - v,
            };
            root = root.child(
                div()
                    .id(("pane", i))
                    .absolute()
                    .left(relative(left))
                    .top(relative(top))
                    .w(relative(width))
                    .h(relative(height))
                    .overflow_hidden()
                    // The panels of a pane (search, replace) are not under its focus handle: a
                    // click anywhere in the pane makes it the active one.
                    .capture_any_mouse_down(
                        cx.listener(move |this, _, window, cx| this.activate(i, window, cx)),
                    )
                    .child(pane.clone()),
            );
        }
        let bar_color = rgb(0xc8c8c8);
        if rows {
            root = root.child(
                div()
                    .id("pane-bar-rows")
                    .absolute()
                    .left_0()
                    .w_full()
                    .top(relative(v))
                    .mt(px(-BAR_WIDTH / 2.0))
                    .h(px(BAR_WIDTH))
                    .bg(bar_color)
                    .cursor(CursorStyle::ResizeUpDown)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, e, window, cx| {
                            this.start_drag(Bar::Rows, e, window, cx)
                        }),
                    ),
            );
        }
        if cols {
            root = root.child(
                div()
                    .id("pane-bar-cols")
                    .absolute()
                    .top_0()
                    .h_full()
                    .left(relative(h))
                    .ml(px(-BAR_WIDTH / 2.0))
                    .w(px(BAR_WIDTH))
                    .bg(bar_color)
                    .cursor(CursorStyle::ResizeLeftRight)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, e, window, cx| {
                            this.start_drag(Bar::Cols, e, window, cx)
                        }),
                    ),
            );
        }
        if rows && cols {
            root = root.child(
                div()
                    .id("pane-bar-both")
                    .absolute()
                    .left(relative(h))
                    .top(relative(v))
                    .ml(px(-BAR_WIDTH * 2.0))
                    .mt(px(-BAR_WIDTH * 2.0))
                    .size(px(BAR_WIDTH * 4.0))
                    .cursor(CursorStyle::ResizeUpLeftDownRight)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, e, window, cx| {
                            this.start_drag(Bar::Both, e, window, cx)
                        }),
                    ),
            );
        }
        root.into_any_element()
    }
}
