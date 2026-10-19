//! What the buttons between the lists do to the items of the menu that is chosen: the handlers of
//! `CPropCustmenu::DispatchEvent` (`sakura_core/prop/CPropComCustmenu.cpp`).

use ume_config::settings::custom_menu::{CustomMenuItem, MAX_CUSTOM_MENU_ITEMS};

/// The items of a menu with the selected item and the selected function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Edit {
    pub items: Vec<CustomMenuItem>,
    /// The selected row of the menu's list (`None` is `LB_ERR`).
    pub sel: Option<usize>,
    /// The selected row of the function list.
    pub func_sel: Option<usize>,
    /// How many functions the function list has.
    pub func_count: usize,
}

impl Edit {
    /// `MAX_CUSTOM_MENU_ITEMS <= m_nCustMenuItemNumArr`: no more items can be put in.
    pub(super) fn is_full(&self) -> bool {
        self.items.len() >= MAX_CUSTOM_MENU_ITEMS
    }

    /// `List_SetCurSel(hwndLIST_FUNC, nIdx4 + 1)`: the function list moves on; past its end nothing is
    /// selected.
    fn next_function(&self, func: usize) -> Option<usize> {
        Some(func + 1).filter(|&f| f < self.func_count)
    }

    /// 「---」 (`IDC_BUTTON_INSERTSEPARATOR`): a separator before the selected row (the top when none is
    /// selected); it becomes the selected one.
    pub(super) fn insert_separator(&mut self) {
        if self.is_full() {
            return;
        }
        let at = self.sel.unwrap_or(0).min(self.items.len());
        self.items.insert(at, CustomMenuItem::SEPARATOR);
        self.sel = Some(at);
    }

    /// 「削除」 (`IDC_BUTTON_DELETE`): removes the selected row; the one now at its place (or the last) is
    /// selected.
    pub(super) fn delete(&mut self) {
        let Some(at) = self.sel.filter(|&i| i < self.items.len()) else {
            return;
        };
        self.items.remove(at);
        self.sel = if self.items.is_empty() {
            None
        } else {
            Some(at.min(self.items.len() - 1))
        };
    }

    /// 「→」 (`IDC_BUTTON_INSERT`): the selected function before the selected row (the top when none is
    /// selected), with no access key. It becomes the selected row and the function list moves on.
    pub(super) fn insert_function(&mut self, code: u16) {
        let Some(func) = self.func_sel else {
            return;
        };
        if self.is_full() {
            return;
        }
        let at = self.sel.unwrap_or(0).min(self.items.len());
        self.items.insert(
            at,
            CustomMenuItem {
                function: code,
                key: None,
            },
        );
        self.sel = Some(at);
        self.func_sel = self.next_function(func);
    }

    /// 「>>」 (`IDC_BUTTON_ADD`): the selected function at the end; it becomes the selected row and the
    /// function list moves on.
    pub(super) fn add_function(&mut self, code: u16) {
        let Some(func) = self.func_sel else {
            return;
        };
        if self.is_full() {
            return;
        }
        self.items.push(CustomMenuItem {
            function: code,
            key: None,
        });
        self.sel = Some(self.items.len() - 1);
        self.func_sel = self.next_function(func);
    }

    /// 「↑」 (`IDC_BUTTON_UP`).
    pub(super) fn up(&mut self) {
        if let Some(at) = self.sel.filter(|&i| i > 0 && i < self.items.len()) {
            self.items.swap(at, at - 1);
            self.sel = Some(at - 1);
        }
    }

    /// 「↓」 (`IDC_BUTTON_DOWN`).
    pub(super) fn down(&mut self) {
        if let Some(at) = self.sel.filter(|&i| i + 1 < self.items.len()) {
            self.items.swap(at, at + 1);
            self.sel = Some(at + 1);
        }
    }

    /// The access key of the selected row (`LBN_DBLCLK`): not for a separator. `None` clears it.
    pub(super) fn set_key(&mut self, key: Option<char>) {
        if let Some(item) = self
            .sel
            .and_then(|i| self.items.get_mut(i))
            .filter(|item| !item.is_separator())
        {
            item.key = key;
        }
    }
}
