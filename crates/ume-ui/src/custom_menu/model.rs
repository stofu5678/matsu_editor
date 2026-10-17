// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/view/CEditView.cpp (CreatePopUpMenuSub), sakura_core/window/CEditWnd.cpp (InitMenu_Function,
//   InitMenu_Special), sakura_core/func/CFuncLookup.cpp (Funccode2Name)
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2001, genta
//   Copyright (C) 2002, YAZAKI, genta, MIK
//   Copyright (C) 2006, fon, ryoji
//   Copyright (C) 2007, ryoji, Moca, genta
//   Copyright (C) 2018-2026, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// zlib License
//
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! A custom menu as a model: the rows of `Entry` that the popup shows, with the same item states as the
//! main menu ([`crate::menu`]): the label with the access key, the shortcut from the key map, and whether
//! the function can run. The model knows nothing of GPUI.
//!
//! Sakura builds the popup in `CEditView::CreatePopUpMenuSub`:
//!
//! * a row with function 0 is a separator;
//! * a row that is the right-click menu or a custom menu is a submenu named after that menu (with the
//!   row's access key) when that menu's "popup" flag is set, and the menu is not already open above it
//!   (otherwise the row is a plain item that runs the function and so opens that menu);
//! * a row from the lists that Sakura fills in at run time (`F_SPECIAL_FIRST` to `F_SPECIAL_LAST`) is
//!   those items;
//! * any other row is an item named by `CFuncLookup::Funccode2Name`.

use ume_commands::keymap::KeyMap;
use ume_commands::sakura::{FUNCTIONS, FunctionKind};
use ume_config::settings::CustomMenus;
use ume_config::settings::custom_menu::INDEX_FOR_RBUTTONUP;

use crate::menu::{Available, Entry, Item, Lists, MenuAction, menu_label};

/// `F_MENU_RBUTTON`, the right-click menu.
pub(crate) const F_MENU_RBUTTON: u16 = 31580;
/// `F_CUSTMENU_BASE`: custom menu `n` is `F_CUSTMENU_BASE + n`.
pub(crate) const F_CUSTMENU_BASE: u16 = 31500;
/// `F_CUSTMENU_LAST` (`F_CUSTMENU_24`).
pub(crate) const F_CUSTMENU_LAST: u16 = 31524;
/// `F_CUSTMENU_LIST`, the row of the main menu that stands for the custom menus.
pub(crate) const F_CUSTMENU_LIST: u16 = 29004;

/// What `Funccode2Name` gives a code that has no name (`F_DISABLE`'s string).
const UNKNOWN_NAME: &str = "-- 不明 --";

/// The custom menu that function `code` opens: 0 for `F_MENU_RBUTTON`, `n` for `F_CUSTMENU_n`.
pub(crate) fn menu_index(code: u16) -> Option<usize> {
    match code {
        F_MENU_RBUTTON => Some(INDEX_FOR_RBUTTONUP),
        F_CUSTMENU_BASE..=F_CUSTMENU_LAST if code > F_CUSTMENU_BASE => {
            Some(usize::from(code - F_CUSTMENU_BASE))
        }
        _ => None,
    }
}

/// The function that opens custom menu `index`.
pub(crate) fn menu_code(index: usize) -> u16 {
    match u16::try_from(index) {
        Ok(0) | Err(_) => F_MENU_RBUTTON,
        Ok(n) => F_CUSTMENU_BASE + n,
    }
}

fn is_special(code: u16) -> bool {
    FUNCTIONS
        .iter()
        .any(|f| f.code == code && f.kind == FunctionKind::MenuSpecial)
}

/// Everything the model needs besides the menu itself.
pub(crate) struct Context<'a> {
    pub menus: &'a CustomMenus,
    pub keymap: &'a KeyMap,
    pub available: &'a dyn Available,
    pub lists: &'a dyn Lists,
}

impl Context<'_> {
    /// `CFuncLookup::Funccode2Name`, for the codes a custom menu can hold.
    fn function_name(&self, code: u16) -> String {
        if let Some(index) = menu_index(code) {
            return self.menus.name(index).unwrap_or_default();
        }
        FUNCTIONS
            .iter()
            .find(|f| f.code == code)
            .and_then(|f| f.label)
            .unwrap_or(UNKNOWN_NAME)
            .to_owned()
    }

    /// Whether the function can run: a function without a command is grayed out, and so is a row that
    /// opens a menu with no items (`InitMenu_Function`).
    fn is_enabled(&self, code: u16) -> bool {
        if let Some(index) = menu_index(code)
            && self.menus.get(index).is_none_or(|m| m.items.is_empty())
        {
            return false;
        }
        self.available.is_available(code)
    }

    fn item(&self, code: u16, key: Option<char>) -> Item {
        Item {
            label: menu_label(&self.function_name(code), key),
            action: MenuAction::Function(code),
            shortcut: self.keymap.shortcut_for(code),
            enabled: self.is_enabled(code),
        }
    }
}

/// The rows of custom menu `index` (0 is the right-click menu). Empty for a number out of range or a menu
/// with no items.
pub(crate) fn entries(index: usize, cx: &Context<'_>) -> Vec<Entry> {
    fill(index, &mut Vec::new(), cx)
}

/// `CreatePopUpMenuSub`. `open` is the chain of the menus above this one (`pParentMenus`), as function
/// codes, so that a menu is not shown inside itself.
fn fill(index: usize, open: &mut Vec<u16>, cx: &Context<'_>) -> Vec<Entry> {
    let Some(menu) = cx.menus.get(index) else {
        return Vec::new();
    };
    open.push(menu_code(index));
    let mut out = Vec::new();
    for row in &menu.items {
        if row.is_separator() {
            out.push(Entry::Separator);
            continue;
        }
        let code = row.function;
        if let Some(target) = menu_index(code)
            && cx.menus.get(target).is_some_and(|m| m.popup)
            && !open.contains(&code)
        {
            let label = menu_label(&cx.function_name(code), row.key);
            let entries = fill(target, open, cx);
            // Sakura makes the submenu of a menu with no items too, which opens nothing; a grayed item
            // says the same.
            out.push(if entries.is_empty() {
                Entry::Item(Item {
                    label,
                    action: MenuAction::Placeholder,
                    shortcut: None,
                    enabled: false,
                })
            } else {
                Entry::Submenu { label, entries }
            });
        } else if is_special(code) {
            // `InitMenu_Special`: the items of the list, or "(なし)" when it is empty.
            let items = cx.lists.items(code);
            if items.is_empty() {
                out.push(Entry::Item(Item {
                    label: "(なし)".to_owned(),
                    action: MenuAction::Placeholder,
                    shortcut: None,
                    enabled: false,
                }));
            } else {
                out.extend(items.into_iter().map(Entry::Item));
            }
        } else {
            out.push(Entry::Item(cx.item(code, row.key)));
        }
    }
    open.pop();
    out
}

/// The items of the main menu's row for the custom menus (`F_CUSTMENU_LIST`): every menu that has items, the
/// right-click menu first (`CEditWnd::InitMenu_Special`).
pub(crate) struct CustomMenuList<'a>(pub &'a CustomMenus);

impl Lists for CustomMenuList<'_> {
    fn items(&self, code: u16) -> Vec<Item> {
        if code != F_CUSTMENU_LIST {
            return Vec::new();
        }
        (0..ume_config::settings::custom_menu::MAX_CUSTOM_MENU)
            .filter(|&i| self.0.get(i).is_some_and(|m| !m.items.is_empty()))
            .map(|i| Item {
                label: self.0.name(i).unwrap_or_default(),
                action: MenuAction::Function(menu_code(i)),
                shortcut: None,
                enabled: true,
            })
            .collect()
    }
}
