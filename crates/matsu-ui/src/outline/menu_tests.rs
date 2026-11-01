//! Tests of the right-click menu model: the items and their order are those of
//! `CDlgFuncList::DoMenu`.

use super::dock::DockSide;
use super::menu::{MenuCommand, MenuEntry, MenuState, entries, position_entries};

fn state() -> MenuState {
    MenuState {
        tree: false,
        bookmarks: false,
        copy_enabled: true,
        has_selection: false,
        side: DockSide::Right,
        sync: true,
        dock_set: 0,
    }
}

fn labels(entries: &[MenuEntry]) -> Vec<&'static str> {
    entries
        .iter()
        .map(|e| match e {
            MenuEntry::Item { label, .. } | MenuEntry::Submenu { label, .. } => *label,
            MenuEntry::Separator => "-",
        })
        .collect()
}

#[test]
fn a_list_has_update_copy_position_and_close() {
    assert_eq!(
        labels(&entries(state())),
        [
            "更新(U)",
            "コピー(C)",
            "-",
            "ウィンドウの位置(W)",
            "-",
            "閉じる(X)"
        ]
    );
}

#[test]
fn a_tree_adds_expand_and_collapse() {
    let tree = MenuState {
        tree: true,
        ..state()
    };
    assert_eq!(
        labels(&entries(tree)),
        [
            "更新(U)",
            "コピー(C)",
            "-",
            "すべて展開(E)",
            "すべて縮小(O)",
            "-",
            "ウィンドウの位置(W)",
            "-",
            "閉じる(X)"
        ]
    );
}

#[test]
fn the_bookmark_list_adds_the_deletes_and_grays_the_one_without_a_selection() {
    let bookmarks = MenuState {
        bookmarks: true,
        ..state()
    };
    let list = entries(bookmarks);
    assert_eq!(
        labels(&list),
        [
            "更新(U)",
            "コピー(C)",
            "-",
            "ブックマーク削除(D)",
            "ブックマーク全削除(A)",
            "-",
            "ウィンドウの位置(W)",
            "-",
            "閉じる(X)"
        ]
    );
    let disabled = |list: &[MenuEntry], command| {
        list.iter().any(
            |e| matches!(e, MenuEntry::Item { command: c, disabled: true, .. } if *c == command),
        )
    };
    assert!(disabled(&list, MenuCommand::BookmarkDelete));
    let selected = entries(MenuState {
        has_selection: true,
        ..bookmarks
    });
    assert!(!disabled(&selected, MenuCommand::BookmarkDelete));
}

#[test]
fn the_copy_item_is_gray_when_the_button_is() {
    let list = entries(MenuState {
        copy_enabled: false,
        ..state()
    });
    assert!(matches!(
        list[1],
        MenuEntry::Item {
            command: MenuCommand::Copy,
            disabled: true,
            ..
        }
    ));
}

#[test]
fn the_position_menu_checks_the_side_the_sync_and_the_inheritance() {
    let sub = position_entries(MenuState {
        side: DockSide::Bottom,
        sync: false,
        dock_set: 1,
        ..state()
    });
    assert_eq!(
        labels(&sub),
        [
            "左ドッキング(L)",
            "右ドッキング(R)",
            "上ドッキング(T)",
            "下ドッキング(B)",
            "フローティング(F)",
            "ドッキング禁止(I)",
            "-",
            "ドッキング配置を同期(S)",
            "-",
            "ドッキング配置を共通継承(C)",
            "ドッキング配置をタイプ別継承(Y)",
            "-",
            "継承情報を統一(U)"
        ]
    );
    let checked: Vec<MenuCommand> = sub
        .iter()
        .filter_map(|e| match e {
            MenuEntry::Item {
                command,
                checked: true,
                ..
            } => Some(*command),
            _ => None,
        })
        .collect();
    assert_eq!(
        checked,
        [MenuCommand::Side(DockSide::Bottom), MenuCommand::Inherit(1)]
    );
}

#[test]
fn the_position_menu_is_a_submenu_of_the_right_click() {
    let list = entries(state());
    let Some(MenuEntry::Submenu { entries: sub, .. }) =
        list.iter().find(|e| matches!(e, MenuEntry::Submenu { .. }))
    else {
        panic!("the submenu");
    };
    assert_eq!(sub, &position_entries(state()));
}
