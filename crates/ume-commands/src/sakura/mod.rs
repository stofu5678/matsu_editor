//! Sakura Editor's command, key binding and main menu tables.
//!
//! `functions.rs`, `key_bindings.rs` and `main_menu.rs` are generated from Sakura's source by
//! `cargo xtask sakura-tables` (see `xtask/src/main.rs`); this file only defines their row
//! types. Rows refer to commands by Sakura's numeric function code, which is unique among
//! the entries of [`FUNCTIONS`] except for the `Reserved` placeholders.

mod functions;
mod key_bindings;
mod main_menu;

pub use functions::{FUNCTION_GROUPS, FUNCTIONS};
pub use key_bindings::KEY_BINDINGS;
pub use main_menu::{KEY_PARENTHESES, MAIN_MENU};

/// What a function code is for, decided by its range in `Funccode_x.hsrc`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionKind {
    /// Codes 0-10: "none", separator, and other placeholders. Several names may share a code.
    Reserved,
    /// 29001-29006: items that only exist in the main menu (window list, recent files, ...).
    MenuSpecial,
    /// 30000-32767: commands that menus, keys and toolbars can call.
    Command,
    /// 32768-39999: commands that only macros call (they have no menu entry).
    MacroOnly,
    /// 40000-49151: macro functions that return a value.
    MacroFunction,
    /// 0xC000 and above (`F_ISEARCH_*`): internal helpers of the incremental search. They are
    /// not macro functions and have no menu entry.
    Internal,
    /// Not a command: a string ID of a menu label (`sakura_rc.h`), such as "File" for the menu bar.
    MenuLabel,
}

/// One function code of Sakura.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Function {
    /// Sakura's numeric code.
    pub code: u16,
    /// Sakura's name, for example `F_FILESAVE`.
    pub name: &'static str,
    /// Our command ID: the name without `F_`, in lowercase `snake_case` where the name has word
    /// boundaries (`F_WordCut` is `word_cut`, `F_FILESAVE` is `filesave`).
    pub id: &'static str,
    pub kind: FunctionKind,
    /// Indexes into [`FUNCTION_GROUPS`]: the groups of the customize dialogs that list this
    /// code (usually one; a few codes are listed in two groups; none for macro-only codes).
    pub groups: &'static [u8],
    /// Japanese label from Sakura's string table (the raw text; `&&` is a literal `&`).
    pub label: Option<&'static str>,
}

/// Bit values of the index into [`KeyBinding::functions`].
pub mod modifier {
    pub const SHIFT: usize = 1;
    pub const CTRL: usize = 2;
    pub const ALT: usize = 4;
}

/// One key (or mouse action) with the function codes of its 8 modifier combinations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyBinding {
    /// Windows virtual-key code, or Sakura's own `VKEX_*` code (0x100 and above) for mouse actions.
    pub key_code: u16,
    /// Display name in Japanese (for example `Del` or `ダブルクリック`).
    pub key_name: &'static str,
    /// Function code (0 = none) per modifier combination; the index is a sum of the
    /// [`modifier`] bits (0 = none, 1 = Shift, 2 = Ctrl, 3 = Shift+Ctrl, 4 = Alt, ...).
    pub functions: [u16; 8],
}

/// The kind of a main menu item (`EMainMenuType` in Sakura's `CommonSetting.h`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuKind {
    /// A menu or submenu; `code` is a `MenuLabel` ID.
    Node,
    /// A command.
    Leaf,
    /// A separator line (`code` is 1).
    Separator,
    /// A list that the editor fills in at run time (window list, recent files, ...).
    Special,
}

/// One item of the default main menu. See [`MAIN_MENU`] for how the tree is stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuItem {
    pub level: u8,
    pub kind: MenuKind,
    /// Function code, or a `MenuLabel` ID for nodes.
    pub code: u16,
    /// Access key (the letter after `&` in Windows menus).
    pub access_key: Option<char>,
    pub label: Option<&'static str>,
}

#[cfg(test)]
mod tests;
