//! The right click in the text.
//!
//! Sakura: `CEditView::OnRBUTTONDOWN` moves the caret to the click unless the click is inside the
//! selection (`IsCurrentPositionSelected`), so the selection stays for Cut and Copy.
//! `CEditView::OnRBUTTONUP` runs the function that the key map has for the right click with the modifier
//! keys (`m_pKeyNameArr[MOUSEFUNCTION_RIGHT]`, `F_MENU_RBUTTON` by default), which shows the menu.
//! (`sakura_core/view/CEditView_Mouse.cpp`)

use gpui::{Context, MouseDownEvent, MouseUpEvent, Pixels, Point, Window};
use ume_commands::keymap::{Chord, Key, Modifiers, MouseAction, Resolved};
use ume_core::cursor::Cursor;
use ume_layout::DocumentLayout;

use crate::geometry::column_at;
use crate::hit::BoxArea;
use crate::view::EditorView;

/// `IsCurrentPositionSelected` for the cell at `row` and display column `col`: inside a box selection's
/// columns, or from the start of a range up to, not including, its end.
pub(crate) fn cell_is_selected(
    layout: &DocumentLayout,
    doc: &ume_core::buffer::Document,
    cursor: &Cursor,
    row: usize,
    col: u32,
) -> bool {
    if cursor.is_box() {
        return BoxArea::of_cursor(layout, doc, cursor)
            .and_then(|area| area.columns_on(row))
            .is_some_and(|(from, to)| (from..to).contains(&col));
    }
    let Some((start, end)) = cursor.selection() else {
        return false;
    };
    let cell = |pos| {
        layout
            .pos_to_row(doc, pos)
            .ok()
            .map(|at| (at.row, at.column))
    };
    match (cell(start), cell(end)) {
        (Some(from), Some(to)) => (from..to).contains(&(row, col)),
        _ => false,
    }
}

impl EditorView {
    /// Whether the point of the window is on a selected cell. A point below the last row is not.
    fn point_is_selected(&self, p: Point<Pixels>, cx: &gpui::App) -> bool {
        let Some(paint) = self.paint else {
            return false;
        };
        let x = f32::from(p.x) - paint.origin_x + self.scroll_x;
        let y = f32::from(p.y) - paint.origin_y;
        let row = ((y + self.scroll_y) / self.metrics.line_height).floor();
        if row < 0.0 || row as usize >= self.layout.row_count() {
            return false;
        }
        let (col, _) = column_at(&self.metrics, x);
        cell_is_selected(
            &self.layout,
            self.document.read(cx).doc(),
            &self.cursor,
            row as usize,
            col,
        )
    }

    fn on_scrollbar(&self, p: Point<Pixels>) -> bool {
        self.paint
            .is_none_or(|paint| f32::from(p.x) >= paint.scrollbar_left)
    }

    pub(crate) fn on_right_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if self.on_scrollbar(event.position) || self.point_is_selected(event.position, cx) {
            return;
        }
        // The same as a left click that does not select (`OnLBUTTONDOWN`).
        self.interrupt_composition(cx);
        self.isearch_exit(cx);
        let Some(pos) = self.position_for_point(event.position, cx) else {
            return;
        };
        let extend = event.modifiers.shift;
        self.run_with(cx, |command_cx| command_cx.move_caret(pos, extend))
            .ok();
    }

    pub(crate) fn on_right_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.on_scrollbar(event.position) {
            return;
        }
        let m = event.modifiers;
        let mods = Modifiers {
            shift: m.shift,
            ctrl: m.control,
            alt: m.alt,
            meta: m.platform,
        };
        let chord = Chord::new(Key::Mouse(MouseAction::RightClick), mods);
        let Resolved::Code(code) = self.keymap.resolve(chord) else {
            return;
        };
        // The function (F_MENU_RBUTTON, by default) opens its menu at the pointer.
        self.custom_menu.mouse_at = Some(event.position);
        self.run_function(code, &[], window, cx);
        self.custom_menu.mouse_at = None;
    }
}
