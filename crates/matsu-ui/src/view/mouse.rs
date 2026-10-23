//! Mouse and wheel: the clicks and drags that select, the scroll bar, and scrolling.
//!
//! Sakura: `CEditView::OnLBUTTONDOWN`, `OnLBUTTONDBLCLK`, `OnMOUSEMOVE` and `OnLBUTTONUP` in
//! `sakura_core/view/CEditView_Mouse.cpp`. What a press does depends on which click of a series it
//! is and on the keys held:
//!
//! * a click puts the caret there, Shift extends the selection, Ctrl selects the word and the drag
//!   goes by words, Alt starts a box selection;
//! * a click on a URL selects it (`bSelectClickedURL`), a double click on one opens it;
//! * the double, triple and quadruple click run the functions of the key assignment (the rows
//!   ダブルクリック, トリプルクリック and クアドラプルクリック; `F_SELECTWORD`, `F_SELECTLINE` and
//!   `F_SELECTALL` by default);
//! * a press in the line number column selects the row, and the drag selects rows;
//! * a press in the selection waits for the pointer to move, and then drags the text to move it,
//!   or with Ctrl to copy it (`bUseOLE_DragDrop`, `bUseOLE_DropSource`);
//! * the side buttons run the functions of their rows.
//!
//! The selections themselves are made by `matsu_commands::context` (`mouse.rs`, `mouse_drop.rs`),
//! where they are tested without a window; this file turns the events into calls.

use gpui::{
    Context, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, NavigationDirection,
    Pixels, Point, ScrollWheelEvent, Window,
};
use matsu_commands::context::DragSelection;
use matsu_commands::keymap::{Chord, Key, Modifiers, MouseAction, Resolved};
use matsu_core::buffer::Pos;

use super::{Drag, EditorView};
use crate::geometry::{Scrollbar, clamp_scroll_y, row_at};
use crate::hit::position_at;

/// `F_SELECTWORD`, the default of the double click.
const F_SELECTWORD: u16 = 30400;
/// `F_SELECTLINE`, the default of the triple click.
const F_SELECTLINE: u16 = 30403;

/// How far the pointer has to move (in pixels, in each direction) from a press in the selection
/// before the text is dragged. Windows' `DragDetect` uses `SM_CXDRAG` (4 by default).
const DRAG_START_DISTANCE: f32 = 4.0;

/// Which click of a series a press is, given the one before and the count that GPUI counted:
/// 1 single, 2 double, 3 triple, 4 quadruple. After the quadruple click the series starts over
/// (Sakura clears `m_dwTripleClickCheck` there, so the next press is a plain one).
pub(crate) fn next_click_phase(previous: u8, click_count: usize) -> u8 {
    if click_count <= 1 {
        return 1;
    }
    match previous {
        1 => 2,
        2 => 3,
        3 => 4,
        _ => 1,
    }
}

/// What the system opens for the text of a URL (`OnLBUTTONDBLCLK`): a mail address gets `mailto:`,
/// and the URLs written without their `h` or `ht` (`ttp://`, the way text avoids a link) get it
/// back.
fn url_to_open(text: &str) -> String {
    if matsu_coloring::mail_len(text.as_bytes(), None) == text.len() {
        format!("mailto:{text}")
    } else if text
        .get(..6)
        .is_some_and(|h| h.eq_ignore_ascii_case("ttp://"))
    {
        format!("h{text}")
    } else if text
        .get(..5)
        .is_some_and(|h| h.eq_ignore_ascii_case("tp://"))
    {
        format!("ht{text}")
    } else {
        text.to_owned()
    }
}

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

    /// The layout row under the point (the last one below the text, the first one above), as the
    /// position where it starts, and the display column of the nearest edge between cells: where a
    /// box selection puts its corner.
    fn box_corner_for_point(&self, p: Point<Pixels>, cx: &gpui::App) -> Option<(Pos, usize)> {
        let paint = self.paint?;
        let x = (f32::from(p.x) - paint.origin_x + self.scroll_x).max(0.0);
        let y = f32::from(p.y) - paint.origin_y;
        let row = row_at(&self.metrics, self.scroll_y, y, self.layout.row_count());
        let start = self
            .layout
            .row_to_pos(self.document.read(cx).doc(), row, 0)
            .ok()?
            .pos;
        Some((start, (x / self.metrics.cell_width + 0.5).floor() as usize))
    }

    /// The function the key map has for a click of the mouse with the keys held (the rows
    /// ダブルクリック, トリプルクリック, ... of the key assignment); 0 for none.
    pub(crate) fn mouse_function(&self, action: MouseAction, m: gpui::Modifiers) -> u16 {
        let mods = Modifiers {
            shift: m.shift,
            ctrl: m.control,
            alt: m.alt,
            meta: m.platform,
        };
        match self.keymap.resolve(Chord::new(Key::Mouse(action), mods)) {
            Resolved::Code(code) => code,
            _ => 0,
        }
    }

    pub(crate) fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        self.autoscroll_button_pressed(MouseButton::Left);
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
        // The ruler and the gap under it do nothing: `OnLBUTTONDOWN` returns for `y < AreaTop`
        // (the comment there says "ルーラクリック").
        if f32::from(event.position.y) < paint.origin_y {
            self.drag = Drag::None;
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
        // The line number column and the margin left of the text (`x < GetAreaLeft()`). A press
        // there is never part of a double or triple click (`CheckTripleClick`).
        let in_gutter = f32::from(event.position.x) < paint.origin_x;
        let phase = if in_gutter {
            1
        } else {
            next_click_phase(self.click_phase, event.click_count)
        };
        self.click_phase = phase;
        let m = event.modifiers;
        match phase {
            4 => {
                let code = self.mouse_function(MouseAction::QuadrupleClick, m);
                if code == 0 {
                    // Not assigned: a double click again (`OnLBUTTONDBLCLK`).
                    self.double_click(pos, true, m, window, cx);
                } else {
                    // The series ends here: the next press (count 5) is a plain click.
                    self.drag = Drag::None;
                    self.run_at_click(pos, code, window, cx);
                }
                return;
            }
            3 => {
                let code = self.mouse_function(MouseAction::TripleClick, m);
                if code != 0 {
                    self.triple_click(pos, code, window, cx);
                    return;
                }
                // Without a function it is a plain click (`tripleClickMode = 0`).
            }
            2 => {
                self.double_click(pos, false, m, window, cx);
                return;
            }
            _ => {}
        }
        self.single_click(event, pos, in_gutter, cx);
    }

    /// Moves the caret to the click, drops what was selected and runs the function (a double,
    /// triple or quadruple click that does not select by itself).
    fn run_at_click(&mut self, pos: Pos, code: u16, window: &mut Window, cx: &mut Context<Self>) {
        self.run_with(cx, |command_cx| command_cx.move_caret(pos, false))
            .ok();
        self.run_function(code, &[], window, cx);
    }

    /// `OnLBUTTONDBLCLK`: opens the URL under the click; otherwise runs the function of the
    /// double click (the key assignment, `F_SELECTWORD` by default), and when that selects a word,
    /// the drag that follows goes by words. `after_quad` is a quadruple click without a function of
    /// its own, which is a double click that does not open URLs.
    fn double_click(
        &mut self,
        pos: Pos,
        after_quad: bool,
        m: gpui::Modifiers,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !after_quad && self.open_url_at(pos, cx) {
            self.drag = Drag::None;
            self.click_phase = 1;
            return;
        }
        let code = self.mouse_function(MouseAction::DoubleClick, m);
        // The press before this one put the caret here; do it again for a double click that comes
        // alone, or after a quadruple click.
        if self.cursor.caret() != pos || self.cursor.has_selection() {
            self.run_with(cx, |command_cx| command_cx.move_caret(pos, false))
                .ok();
        }
        if code != 0 {
            self.run_function(code, &[], window, cx);
        }
        // The next press may be the third click (`m_dwTripleClickCheck = GetTickCount()`).
        self.click_phase = 2;
        self.drag = if code == F_SELECTWORD {
            let caret = self.cursor.caret();
            Drag::Select {
                bgn: self.cursor.selection().unwrap_or((caret, caret)),
                words: true,
            }
        } else {
            Drag::None
        };
    }

    /// The third click of a series with a function assigned (`tripleClickMode` in
    /// `OnLBUTTONDOWN`): moves the caret to the click and runs the function. `F_SELECTLINE` also
    /// starts a drag by logical lines (the "2.5 click").
    fn triple_click(&mut self, pos: Pos, code: u16, window: &mut Window, cx: &mut Context<Self>) {
        self.run_at_click(pos, code, window, cx);
        self.drag = Drag::None;
        if code != F_SELECTLINE {
            return;
        }
        self.drag = match self.cursor.selection() {
            Some(bgn) => Drag::Lines { bgn, logical: true },
            // Nothing to select (the empty row at the end): a plain click.
            None => Drag::Select {
                bgn: (self.cursor.caret(), self.cursor.caret()),
                words: false,
            },
        };
    }

    /// The first click of a series (`OnLBUTTONDOWN` after the triple click).
    fn single_click(
        &mut self,
        event: &MouseDownEvent,
        pos: Pos,
        in_gutter: bool,
        cx: &mut Context<Self>,
    ) {
        let m = event.modifiers;
        let options = matsu_commands::edit_options::get();
        // A press in the selection waits to see whether the text is dragged (`DragDetect`).
        if options.drag_drop
            && options.drop_source
            && !in_gutter
            && self.point_is_selected(event.position, cx)
        {
            self.drag = Drag::Press {
                down: event.position,
            };
            return;
        }
        // Alt: a box selection. In the gutter it starts at the left edge of the row.
        if m.alt {
            self.run_with(cx, |command_cx| command_cx.mouse_begin_box(pos))
                .ok();
            self.drag = Drag::Box;
            return;
        }
        // The line number column: the row, and the rows to the pointer while it is dragged along
        // the column. Shift takes the press as a click at the start of the row.
        if in_gutter && !m.shift {
            let bgn = self
                .run_with(cx, |command_cx| command_cx.mouse_begin_units(pos, false))
                .ok()
                .flatten();
            self.drag = match bgn {
                Some(bgn) => Drag::Lines {
                    bgn,
                    logical: false,
                },
                None => Drag::Select {
                    bgn: (pos, pos),
                    words: false,
                },
            };
            return;
        }
        let (bgn, words) = self.press_selection(event, pos, cx);
        self.drag = Drag::Select { bgn, words };
        // A click on a URL selects it, if the setting says so (`m_bSelectClickedURL`).
        if !in_gutter
            && options.select_clicked_url
            && let Some(url) = self.url_range_at(pos, cx)
        {
            self.run_with(cx, |command_cx| command_cx.select_range(url.0, url.1))
                .ok();
            self.drag = Drag::Select {
                bgn: url,
                words: false,
            };
        }
    }

    /// The selection that a press makes by the keys held: Shift extends from the anchor, Ctrl
    /// selects the word (and the drag goes by words), and the caret moves to the click otherwise.
    /// Returns the range the drag starts from and whether it goes by words.
    fn press_selection(
        &mut self,
        event: &MouseDownEvent,
        pos: Pos,
        cx: &mut Context<Self>,
    ) -> ((Pos, Pos), bool) {
        let m = event.modifiers;
        if m.shift {
            // A box is dropped before the selection grows from the caret.
            if self.cursor.is_box() && self.cursor.has_selection() {
                let caret = self.cursor.caret();
                self.run_with(cx, |command_cx| command_cx.move_caret(caret, false))
                    .ok();
            }
            let anchor = self.cursor.anchor().unwrap_or(self.cursor.caret());
            self.run_with(cx, |command_cx| command_cx.move_caret(pos, true))
                .ok();
            if m.control {
                // Shift and Ctrl: the selection grows by words from the word at the anchor.
                let word = self
                    .run_with(cx, |command_cx| command_cx.word_range_at(anchor))
                    .ok()
                    .flatten()
                    .unwrap_or((anchor, anchor));
                let bgn = (word.0.min(anchor), word.1.max(anchor));
                self.run_with(cx, |command_cx| {
                    command_cx.mouse_drag_to(DragSelection { bgn, words: true }, pos)
                })
                .ok();
                return (bgn, true);
            }
            return ((anchor, anchor), false);
        }
        self.run_with(cx, |command_cx| command_cx.move_caret(pos, false))
            .ok();
        if m.control {
            // Ctrl: the word at the click (`Command_SELECTWORD`), and the drag goes by words.
            let selected = self
                .run_with(cx, |command_cx| command_cx.select_word())
                .unwrap_or(false);
            let bgn = if selected {
                self.cursor.selection().unwrap_or((pos, pos))
            } else {
                (pos, pos)
            };
            return (bgn, true);
        }
        ((pos, pos), false)
    }

    /// The range of the URL at `pos` in its line, if the type shows URLs (`IsCurrentPositionURL`;
    /// the URLs of regular expression keywords are not looked for).
    fn url_range_at(&self, pos: Pos, cx: &gpui::App) -> Option<(Pos, Pos)> {
        if !self.prefs.settings.coloring.shows_url() {
            return None;
        }
        let line = self.document.read(cx).doc().line(pos.line).ok().flatten()?;
        let range = matsu_coloring::url_at(&line.text, pos.col)?;
        Some((
            Pos::new(pos.line, range.start),
            Pos::new(pos.line, range.end),
        ))
    }

    /// Opens the URL or mail address at `pos` with the system, as the double click does. Returns
    /// whether there was one.
    fn open_url_at(&mut self, pos: Pos, cx: &mut Context<Self>) -> bool {
        let Some((from, to)) = self.url_range_at(pos, cx) else {
            return false;
        };
        let Ok(text) = self.document.read(cx).doc().text_between(from, to) else {
            return false;
        };
        cx.open_url(&url_to_open(&text));
        true
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
            self.drop_caret = None;
            return;
        }
        match self.drag {
            Drag::None => {}
            Drag::Select { bgn, words } => {
                // Above or below the view the pointer maps to the first or last row, and
                // moving the caret there scrolls.
                if let Some(pos) = self.position_for_point(event.position, cx) {
                    self.run_with(cx, |command_cx| {
                        command_cx.mouse_drag_to(DragSelection { bgn, words }, pos)
                    })
                    .ok();
                }
            }
            Drag::Lines { bgn, logical } => {
                let Some(pos) = self.position_for_point(event.position, cx) else {
                    return;
                };
                // A drag that began in the line number column goes by rows only while the pointer
                // is in it; in the text it selects by characters from the row it began on
                // (`ptMouse.x < GetAreaLeft() || m_dwTripleClickCheck`).
                let in_text = self
                    .paint
                    .is_some_and(|paint| f32::from(event.position.x) >= paint.origin_x);
                self.run_with(cx, |command_cx| {
                    if logical || !in_text {
                        command_cx.mouse_drag_units_to(bgn, pos, logical)
                    } else {
                        command_cx.mouse_drag_to(DragSelection { bgn, words: false }, pos)
                    }
                })
                .ok();
            }
            Drag::Box => {
                if let Some((row, col)) = self.box_corner_for_point(event.position, cx) {
                    self.run_with(cx, |command_cx| command_cx.mouse_box_to(row, col))
                        .ok();
                }
            }
            Drag::Press { down } => {
                let dx = f32::from(event.position.x - down.x);
                let dy = f32::from(event.position.y - down.y);
                if dx.abs().max(dy.abs()) > DRAG_START_DISTANCE {
                    self.drag = Drag::Drop;
                    self.move_drop_caret(event.position, cx);
                }
            }
            Drag::Drop => self.move_drop_caret(event.position, cx),
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

    /// The pointer moves while the selected text is dragged: the drop caret follows it.
    fn move_drop_caret(&mut self, p: Point<Pixels>, cx: &mut Context<Self>) {
        if let Some(pos) = self.position_for_point(p, cx) {
            self.drop_caret = Some(pos);
            cx.notify();
        }
    }

    pub(crate) fn on_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The selection by the mouse is done: the tip may come for what is selected.
        self.key_help_restart(window, cx);
        match self.drag {
            Drag::Press { .. } => {
                // The text was not dragged: it was a click in the selection, which puts the caret
                // there and drops the selection (`DragDetect` failed in `OnLBUTTONDOWN`).
                if let Some(pos) = self.position_for_point(event.position, cx) {
                    self.run_with(cx, |command_cx| command_cx.move_caret(pos, false))
                        .ok();
                }
            }
            Drag::Drop => {
                if let Some(at) = self.drop_caret {
                    // Ctrl copies (`TranslateDropEffect`).
                    let copy = event.modifiers.control;
                    self.run_with(cx, |command_cx| command_cx.mouse_drop_selection(at, copy))
                        .ok();
                }
            }
            Drag::Box => {
                self.run_with(cx, |command_cx| command_cx.mouse_end_empty_box())
                    .ok();
            }
            _ => {}
        }
        self.drop_caret = None;
        if self.drag != Drag::None {
            self.drag = Drag::None;
            cx.notify();
        }
    }

    /// The left button is released outside the view. Dragged text that is let go there has no
    /// place to go (another window or program is not supported): the drop is cancelled and the
    /// text stays where it was, as when the target of an OLE drop refuses it. A press in the
    /// selection that never moved is cancelled too. Other drags end as usual.
    pub(crate) fn on_mouse_up_outside(
        &mut self,
        event: &MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(self.drag, Drag::Drop | Drag::Press { .. }) {
            self.drag = Drag::None;
            self.drop_caret = None;
            cx.notify();
            return;
        }
        self.on_mouse_up(event, window, cx);
    }

    /// A side button of the mouse goes down: the auto scroll ends (`OnXLBUTTONDOWN`,
    /// `OnXRBUTTONDOWN`).
    pub(crate) fn on_side_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        self.autoscroll_button_pressed(event.button);
    }

    /// A side button is released: runs the function that the key map has for it with the keys held
    /// (`OnXLBUTTONUP`, `OnXRBUTTONUP`; the back button is the left side button).
    pub(crate) fn on_side_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The button was the key of a wheel combination and the wheel turned: that was its use.
        if self.wheel_used_buttons.take(event.button) {
            return;
        }
        let action = match event.button {
            MouseButton::Navigate(NavigationDirection::Back) => MouseAction::LeftSideClick,
            MouseButton::Navigate(NavigationDirection::Forward) => MouseAction::RightSideClick,
            _ => return,
        };
        let code = self.mouse_function(action, event.modifiers);
        if code != 0 {
            self.run_function_at(code, event.position, window, cx);
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
