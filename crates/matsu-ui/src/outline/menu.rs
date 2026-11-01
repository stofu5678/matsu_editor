//! The right-click menu of the outline pane (`CDlgFuncList::DoMenu`), as a model.
//!
//! The labels are `STR_DLGFNCLST_MENU_*` of `sakura_core/sakura_rc.rc`; the access key mark `&`
//! is left out, since GPUI's menus have none. The numbers in the comments are the item ids of
//! `DoMenu`.

use super::dock::DockSide;

/// What an item does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MenuCommand {
    /// 450: analyze again.
    Update,
    /// 451: the result to the clipboard.
    Copy,
    /// 500.
    ExpandAll,
    /// 501.
    CollapseAll,
    /// 510: the bookmark of the selected item.
    BookmarkDelete,
    /// 511.
    BookmarkDeleteAll,
    /// 100 + side.
    Side(DockSide),
    /// 200.
    Sync,
    /// 300 (0) and 301 (1): `nOutlineDockSet`.
    Inherit(i32),
    /// 305.
    Unify,
    /// 452.
    Close,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MenuEntry {
    Item {
        label: &'static str,
        command: MenuCommand,
        checked: bool,
        disabled: bool,
    },
    Separator,
    Submenu {
        label: &'static str,
        entries: Vec<MenuEntry>,
    },
}

/// What the menu looks at.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MenuState {
    /// The pane shows a tree (`VIEWTYPE_TREE`; the file tree is one).
    pub tree: bool,
    /// The pane shows the list of bookmarks (`OUTLINE_BOOKMARK`).
    pub bookmarks: bool,
    /// The copy button is enabled.
    pub copy_enabled: bool,
    /// An item is selected.
    pub has_selection: bool,
    /// The side the settings in use give (`ProfDockSide`).
    pub side: DockSide,
    /// `bOutlineDockSync`.
    pub sync: bool,
    /// `nOutlineDockSet`.
    pub dock_set: i32,
}

fn item(label: &'static str, command: MenuCommand) -> MenuEntry {
    MenuEntry::Item {
        label,
        command,
        checked: false,
        disabled: false,
    }
}

/// The items of the right click, in Sakura's order.
pub(crate) fn entries(state: MenuState) -> Vec<MenuEntry> {
    let mut entries = vec![item("更新(U)", MenuCommand::Update)];
    entries.push(MenuEntry::Item {
        label: "コピー(C)",
        command: MenuCommand::Copy,
        checked: false,
        disabled: !state.copy_enabled,
    });
    if state.tree {
        entries.push(MenuEntry::Separator);
        entries.push(item("すべて展開(E)", MenuCommand::ExpandAll));
        entries.push(item("すべて縮小(O)", MenuCommand::CollapseAll));
    } else if state.bookmarks {
        entries.push(MenuEntry::Separator);
        entries.push(MenuEntry::Item {
            label: "ブックマーク削除(D)",
            command: MenuCommand::BookmarkDelete,
            checked: false,
            disabled: !state.has_selection,
        });
        entries.push(item(
            "ブックマーク全削除(A)",
            MenuCommand::BookmarkDeleteAll,
        ));
    }
    entries.push(MenuEntry::Separator);
    entries.push(MenuEntry::Submenu {
        label: "ウィンドウの位置(W)",
        entries: position_entries(state),
    });
    entries.push(MenuEntry::Separator);
    entries.push(item("閉じる(X)", MenuCommand::Close));
    entries
}

/// The submenu ウィンドウの位置: the sides, the sync and the inheritance.
pub(crate) fn position_entries(state: MenuState) -> Vec<MenuEntry> {
    let side = |label, side: DockSide| MenuEntry::Item {
        label,
        command: MenuCommand::Side(side),
        checked: state.side == side,
        disabled: false,
    };
    let inherit = |label, set: i32| MenuEntry::Item {
        label,
        command: MenuCommand::Inherit(set),
        checked: state.dock_set == set,
        disabled: false,
    };
    vec![
        side("左ドッキング(L)", DockSide::Left),
        side("右ドッキング(R)", DockSide::Right),
        side("上ドッキング(T)", DockSide::Top),
        side("下ドッキング(B)", DockSide::Bottom),
        side("フローティング(F)", DockSide::Float),
        side("ドッキング禁止(I)", DockSide::Undockable),
        MenuEntry::Separator,
        MenuEntry::Item {
            label: "ドッキング配置を同期(S)",
            command: MenuCommand::Sync,
            checked: state.sync,
            disabled: false,
        },
        MenuEntry::Separator,
        inherit("ドッキング配置を共通継承(C)", 0),
        inherit("ドッキング配置をタイプ別継承(Y)", 1),
        MenuEntry::Separator,
        item("継承情報を統一(U)", MenuCommand::Unify),
    ]
}

/// `STR_DLGFNCLST_UNIFY`: the question before 継承情報を統一.
pub(crate) const UNIFY_MESSAGE: &str = "現在の画面のドッキング配置情報をすべての共通設定・タイプ別設定にコピーして統一します。\n（現在開いている他画面の状態も統一します。）\n";
