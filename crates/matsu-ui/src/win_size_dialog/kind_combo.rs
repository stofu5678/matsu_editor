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
    StatefulInteractiveElement as _, Styled as _, Window, div, px, rgb,
};
use gpui_component::Disableable as _;
use gpui_component::button::Button;

use super::{KINDS, WinSizeDialog};

impl WinSizeDialog {
    /// The row of the kind that is set (the first one for a value that is none of the three).
    fn kind_row(&self) -> usize {
        KINDS
            .iter()
            .position(|(_, kind)| *kind == self.size_type)
            .unwrap_or(0)
    }

    /// Chooses the entry at `row` and closes the list (`CBN_SELCHANGE`).
    pub(super) fn pick_kind(&mut self, row: usize, cx: &mut Context<Self>) {
        if let Some((_, kind)) = KINDS.get(row) {
            self.set_size_type(*kind, cx);
        }
        self.kind_open = false;
        cx.notify();
    }

    fn toggle_kind_list(&mut self, cx: &mut Context<Self>) {
        self.kind_open = !self.kind_open;
        self.kind_cursor = self.kind_row();
        cx.notify();
    }

    /// The row that a key of the list goes to from `from`, without going round.
    fn kind_target(key: &str, from: usize) -> Option<usize> {
        let last = KINDS.len() - 1;
        Some(match key {
            "up" => from.saturating_sub(1),
            "down" => (from + 1).min(last),
            "home" | "pageup" => 0,
            "end" | "pagedown" => last,
            _ => return None,
        })
    }

    /// A key on the combo box with its list closed.
    pub(super) fn on_kind_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if self.kind_open {
            return;
        }
        let modifiers = event.keystroke.modifiers;
        let key = event.keystroke.key.as_str();
        if key == "f4" && !modifiers.modified() || key == "down" && modifiers.alt {
            self.toggle_kind_list(cx);
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
        // A combo box closes its list when it loses the focus.
        if !self.kind_focus.contains_focused(window, cx) {
            self.kind_open = false;
            cx.notify();
            return;
        }
        let modifiers = event.keystroke.modifiers;
        let key = event.keystroke.key.as_str();
        if modifiers.modified() {
            return;
        }
        match key {
            "enter" => self.pick_kind(self.kind_cursor, cx),
            "escape" | "f4" => {
                self.kind_open = false;
                cx.notify();
            }
            _ => {
                let Some(to) = Self::kind_target(key, self.kind_cursor) else {
                    // Tab and the like leave the box: the list is closed and the key goes on
                    self.kind_open = false;
                    cx.notify();
                    return;
                };
                self.kind_cursor = to;
                cx.notify();
            }
        }
        cx.stop_propagation();
    }

    /// The combo box: a button with the entry that is set.
    pub(super) fn kind_combo(&self, usable: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let (text, _) = KINDS[self.kind_row()];
        div()
            .id("win-kinds")
            .track_focus(&self.kind_focus)
            .on_key_down(cx.listener(|this, event, _, cx| this.on_kind_key(event, cx)))
            .w(px(110.))
            .child(
                Button::new("win-kind")
                    .label(format!("{text}  v"))
                    .w_full()
                    .disabled(!usable)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_kind_list(cx))),
            )
    }

    /// The list under the combo box while it is open.
    pub(super) fn kind_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div()
            .id("win-kind-list")
            .role(gpui::Role::ListBox)
            .aria_label("起動時の大きさ")
            .w(px(110.))
            .border_1()
            .border_color(rgb(0xaaaaaa))
            .bg(rgb(0xffffff));
        for (row, (text, _)) in KINDS.iter().enumerate() {
            let at = row == self.kind_cursor;
            list = list.child(
                crate::a11y::list_row(div().id(("win-kind-row", row)), text, at)
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
