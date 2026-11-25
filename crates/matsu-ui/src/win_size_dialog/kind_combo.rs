//! The kind of window of 大きさ: `IDC_COMBO_WINTYPE`, a drop-down list (`CBS_DROPDOWNLIST`,
//! `WS_GROUP | WS_TABSTOP`, `IDD_WINPOSSIZE` in `sakura_core/sakura_rc.rc`) of 普通, 最大化 and
//! (最小化) (`CDlgWinSize::SetData`), greyed out unless 直接指定 is chosen (`EnableWinSizeControls`).
//!
//! It is drawn as the combo boxes of the print settings are (`print_setting/dialog.rs`: `combo`,
//! `on_combo_key`): one Tab stop, a button that shows the entry and opens the list. With the list
//! closed, Up, Down, Home, End, PageUp and PageDown choose another entry at once without going
//! round; F4, Alt+Down and a click open the list; in the open list the arrows move, Enter takes
//! the row and Escape closes it. A copy of that code and not a shared part: the print settings'
//! list is four lists of one dialog that share its state (`List`, `list_cursor`, the scroll
//! handle, the capture of the keys), and moving them out is not a plain move.
//! `set_size_type` stays the one writer of the choice.

use gpui::prelude::FluentBuilder as _;
use gpui::{
    Context, InteractiveElement as _, IntoElement, KeyDownEvent, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, deferred, div, px, rgb,
};
use gpui_component::Disableable as _;
use gpui_component::button::Button;

use super::{KINDS, WinSizeDialog};

impl WinSizeDialog {
    /// The row of the kind that is set. A value that is none of the three is shown as the first
    /// entry, and that is what `values` then gives back: `CDlgWinSize::SetData` (lines 178-189)
    /// selects entry 0 for anything but `SIZE_MINIMIZED` and `SIZE_MAXIMIZED`, and `GetData`
    /// (lines 221-231) writes what the entry says, so the display and the saved value agree.
    pub(super) fn kind_row(&self) -> usize {
        KINDS
            .iter()
            .position(|(_, kind)| *kind == self.size_type)
            .unwrap_or(0)
    }

    /// The combo box can be used only with 直接指定 (`EnableWinSizeControls`).
    pub(super) fn kind_usable(&self) -> bool {
        self.size_mode == super::MODE_SET
    }

    /// Chooses the entry at `row` (one of `KINDS`) and closes the list (`CBN_SELCHANGE`).
    pub(super) fn pick_kind(&mut self, row: usize, cx: &mut Context<Self>) {
        self.set_size_type(KINDS[row].1, cx);
        self.close_kind_list(cx);
    }

    /// Closes the list that is open: the one place that does it (a row was taken, Escape, F4, a
    /// key that leaves the box, the focus left the box, or the size left 直接指定).
    pub(super) fn close_kind_list(&mut self, cx: &mut Context<Self>) {
        self.kind_open = false;
        cx.notify();
    }

    fn toggle_kind_list(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.kind_open {
            self.close_kind_list(cx);
            return;
        }
        self.kind_open = true;
        // A click on the button does not move the focus (the button does not take it on
        // mouse down), and a list opened while the focus is elsewhere would be closed again
        // by `render` as a list that lost the focus: the focus goes into the box, as the
        // print settings' `open_list` takes it.
        window.focus(&self.kind_focus, cx);
        self.kind_cursor = self.kind_row();
        cx.notify();
    }

    /// The row that a key of the list goes to from `from`, without going round.
    fn kind_target(key: &str, from: usize) -> Option<usize> {
        let last = KINDS.len() - 1;
        debug_assert!(from <= last, "a row of the combo box that is not one");
        Some(match key {
            "up" => from.saturating_sub(1),
            "down" => (from + 1).min(last),
            "home" | "pageup" => 0,
            "end" | "pagedown" => last,
            _ => return None,
        })
    }

    /// A key on the combo box with its list closed, as `print_setting::on_combo_key` does it: F4
    /// and Alt+Down open the list, and so does Space when the box itself has the focus (the
    /// focus is on the box after a click opened the list: the button inside it takes Space as a
    /// click when it has the focus). The arrows, Home, End, PageUp and PageDown choose at once.
    pub(super) fn on_kind_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.kind_open || !self.kind_usable() {
            return;
        }
        let modifiers = event.keystroke.modifiers;
        let key = event.keystroke.key.as_str();
        let own_focus = self.kind_focus.is_focused(window);
        if key == "f4" && !modifiers.modified()
            || key == "down" && modifiers.alt
            || key == "space" && own_focus && !modifiers.modified()
        {
            self.toggle_kind_list(window, cx);
            cx.stop_propagation();
            return;
        }
        if modifiers.modified() {
            return;
        }
        if let Some(to) = Self::kind_target(key, self.kind_row()) {
            if to != self.kind_row() {
                self.pick_kind(to, cx);
            }
            cx.stop_propagation();
        }
    }

    /// The keys of the open list, before the control that has the focus sees them (Enter would
    /// press the button, Escape would close the dialog).
    pub(super) fn on_kind_capture_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.kind_open {
            return;
        }
        // A combo box closes its list when it loses the focus (`render` also closes it when the
        // focus has moved by Shift+Tab or a click) or cannot be used any more.
        if !self.kind_usable() || !self.kind_focus.contains_focused(window, cx) {
            self.close_kind_list(cx);
            return;
        }
        let modifiers = event.keystroke.modifiers;
        let key = event.keystroke.key.as_str();
        // Alt+Down and Alt+Up close the open list, as F4 does
        if modifiers.alt && matches!(key, "down" | "up") {
            self.close_kind_list(cx);
            cx.stop_propagation();
            return;
        }
        if modifiers.modified() {
            return;
        }
        match key {
            // Space takes the row as Enter does, whichever of the box and its button has the
            // focus (the button would take it as a click and close the list without choosing)
            "enter" | "space" => self.pick_kind(self.kind_cursor, cx),
            "escape" | "f4" => self.close_kind_list(cx),
            _ => {
                let Some(to) = Self::kind_target(key, self.kind_cursor) else {
                    // Tab and the like leave the box: the list is closed and the key goes on
                    self.close_kind_list(cx);
                    return;
                };
                self.kind_cursor = to;
                cx.notify();
            }
        }
        cx.stop_propagation();
    }

    /// The combo box: a button with the entry that is set, and under it the list while it is open.
    /// The list is inside the box that tracks the focus, so a click on a row keeps the focus in it.
    pub(super) fn kind_combo(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (text, _) = KINDS[self.kind_row()];
        div()
            .id("win-kinds")
            .debug_selector(|| "win-kinds".to_owned())
            .track_focus(&self.kind_focus)
            .on_key_down(cx.listener(|this, event, window, cx| this.on_kind_key(event, window, cx)))
            .relative()
            .w(px(110.))
            .child(
                Button::new("win-kind")
                    .label(format!("{text}  v"))
                    .w_full()
                    .disabled(!self.kind_usable())
                    .on_click(cx.listener(|this, _, window, cx| this.toggle_kind_list(window, cx))),
            )
            .when(self.kind_open, |combo| {
                // drawn over what is under the box (deferred, so that nothing after it covers
                // it), so that opening the list moves nothing in the fixed window
                combo.child(deferred(self.kind_list(cx)).with_priority(1))
            })
    }

    /// The list under the combo box while it is open, anchored to the bottom of the box.
    fn kind_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div()
            .id("win-kind-list")
            .role(gpui::Role::ListBox)
            .aria_label("起動時の大きさ")
            .absolute()
            .top_full()
            .left_0()
            .w(px(110.))
            .occlude()
            .border_1()
            .border_color(rgb(0xaaaaaa))
            .bg(rgb(0xffffff));
        for (row, (text, _)) in KINDS.iter().enumerate() {
            let at = row == self.kind_cursor;
            list = list.child(
                crate::a11y::list_row(div().id(("win-kind-row", row)), text, at)
                    .debug_selector(move || format!("win-kind-row-{row}"))
                    .px_1()
                    .cursor_pointer()
                    .when(at, |item| item.bg(rgb(0xc8dcf5)))
                    .hover(|style| style.bg(rgb(0xdde8f5)))
                    .on_click(cx.listener(move |this, _, _, cx| this.pick_kind(row, cx)))
                    .child(*text),
            );
        }
        list
    }
}
