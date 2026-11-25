//! The kind of window of 大きさ: `IDC_COMBO_WINTYPE`, a drop-down list (`CBS_DROPDOWNLIST`,
//! `WS_GROUP | WS_TABSTOP`, `IDD_WINPOSSIZE` in `sakura_core/sakura_rc.rc`) of 普通, 最大化 and
//! (最小化) (`CDlgWinSize::SetData`), greyed out unless 直接指定 is chosen (`EnableWinSizeControls`).
//!
//! The box, its keys and its list are the shared `crate::drop_down::DropDown`; this file is what
//! the dialog does with them. `set_size_type` stays the one writer of the choice.

use gpui::{Context, KeyDownEvent, Window, px};

use super::{KINDS, WinSizeDialog};
use crate::drop_down::{Hooks, Keyed, Look};

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
    fn pick_kind(&mut self, row: usize, cx: &mut Context<Self>) {
        self.set_size_type(KINDS[row].1, cx);
        self.kind.close();
        cx.notify();
    }

    /// What a key of the box did, as the drop-down tells it.
    fn kind_keyed(&mut self, keyed: Keyed, cx: &mut Context<Self>) {
        match keyed {
            Keyed::Ignored => {}
            Keyed::Redraw => cx.notify(),
            Keyed::Handled => {
                cx.notify();
                cx.stop_propagation();
            }
            Keyed::Choose(row) => {
                self.pick_kind(row, cx);
                cx.stop_propagation();
            }
        }
    }

    /// A key on the combo box with its list closed.
    fn on_kind_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let (selected, usable) = (self.kind_row(), self.kind_usable());
        let keyed = self
            .kind
            .on_key(event, window, cx, selected, KINDS.len(), usable);
        self.kind_keyed(keyed, cx);
    }

    /// The keys of the open list, before the control that has the focus sees them.
    pub(super) fn on_kind_capture_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let usable = self.kind_usable();
        let keyed = self
            .kind
            .on_capture_key(event, window, cx, KINDS.len(), usable);
        self.kind_keyed(keyed, cx);
    }

    /// The combo box: a button with the entry that is set, and under it the list while it is open.
    pub(super) fn kind_combo(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let look = Look {
            id: "win-kind",
            name: "起動時の大きさ",
            width: px(110.),
        };
        let hooks = Hooks {
            toggle: |this: &mut Self, window, cx| {
                let selected = this.kind_row();
                this.kind.toggle(selected, window, cx);
                cx.notify();
            },
            pick: Self::pick_kind,
            key: Self::on_kind_key,
        };
        self.kind.view(
            &look,
            self.kind_row(),
            KINDS.iter().map(|(text, _)| *text),
            self.kind_usable(),
            &hooks,
            cx,
        )
    }
}
