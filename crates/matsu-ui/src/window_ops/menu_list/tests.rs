//! Tests of the window list of the main menu: the items for two windows, and choosing one.

use gpui::{AnyWindowHandle, TestAppContext, VisualTestContext};

use super::{F_WINDOW_LIST, WindowListMenu, select};
use crate::document::SharedDocument;
use crate::file_ops::windows::{editors, open_editor_window};
use crate::menu::{Item, Lists, MenuAction};

fn open(cx: &mut TestAppContext) -> AnyWindowHandle {
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    cx.run_until_parked();
    handle
}

fn items(cx: &mut TestAppContext) -> Vec<Item> {
    cx.update(|cx| WindowListMenu::collect(cx).items(F_WINDOW_LIST))
}

fn window_of(item: &Item) -> (u64, bool) {
    match item.action {
        MenuAction::Window { id, current } => (id, current),
        ref other => panic!("not a window item: {other:?}"),
    }
}

#[gpui::test]
fn the_items_are_the_windows_in_the_order_they_were_opened(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    assert!(items(cx).is_empty());
    let first = open(cx);
    let second = open(cx);
    let list = items(cx);
    assert_eq!(list.len(), 2);
    // `GetMenuFullLabel_WinList`: the access keys 1 and 2, and the name of a document with no file.
    assert!(list[0].label.starts_with("1 (無題)"), "{}", list[0].label);
    assert!(list[1].label.starts_with("2 (無題)"), "{}", list[1].label);
    assert!(list.iter().all(|item| item.enabled));
    // The action of an item names the document of its window.
    let ids = cx.update(|cx| {
        editors(cx)
            .into_iter()
            .map(|(handle, view)| (handle, view.entity_id().as_u64()))
            .collect::<Vec<_>>()
    });
    assert_eq!(
        ids,
        vec![
            (first, window_of(&list[0]).0),
            (second, window_of(&list[1]).0)
        ]
    );
    // Another list gets nothing.
    assert!(cx.update(|cx| WindowListMenu::collect(cx).items(29002).is_empty()));
}

#[gpui::test]
fn a_changed_document_shows_its_mark(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let first = open(cx);
    let _second = open(cx);
    VisualTestContext::from_window(first, cx)
        .into_mut()
        .simulate_input("a");
    cx.run_until_parked();
    let list = items(cx);
    assert!(list[0].label.contains(" *"), "{}", list[0].label);
    assert!(!list[1].label.contains(" *"), "{}", list[1].label);
}

#[gpui::test]
fn choosing_an_item_brings_that_window_to_the_front(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let first = open(cx);
    let second = open(cx);
    second
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    let list = items(cx);
    // The active window has the check mark.
    assert_eq!(cx.update(|cx| cx.active_window()), Some(second));
    assert!(!window_of(&list[0]).1);
    assert!(window_of(&list[1]).1);

    let id = window_of(&list[0]).0;
    cx.update(|cx| select(cx, id));
    cx.run_until_parked();
    assert_eq!(cx.update(|cx| cx.active_window()), Some(first));
    let list = items(cx);
    assert!(window_of(&list[0]).1);
    assert!(!window_of(&list[1]).1);

    // An id that no document has does nothing.
    cx.update(|cx| select(cx, u64::MAX));
    cx.run_until_parked();
    assert_eq!(cx.update(|cx| cx.active_window()), Some(first));
}

/// The names of every item in `items`, submenus included.
fn names_of(items: &[gpui::OwnedMenuItem], out: &mut Vec<String>) {
    for item in items {
        match item {
            gpui::OwnedMenuItem::Action { name, .. } => out.push(name.clone()),
            gpui::OwnedMenuItem::Submenu(menu) => names_of(&menu.items, out),
            _ => {}
        }
    }
}

/// The names of every item of the menus the application made for the editor.
fn menu_names(cx: &mut TestAppContext) -> Vec<String> {
    cx.update(|cx| {
        let menus = crate::app::KindMenus::of(cx, crate::tabs::content::MenuKind::Editor).unwrap();
        let mut out = Vec::new();
        for menu in &menus {
            names_of(&menu.items, &mut out);
        }
        out
    })
}

#[gpui::test]
fn the_menus_follow_the_windows_that_open(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::app::set_key_map(cx, matsu_commands::keymap::KeyMap::default());
        crate::app::refresh_menus(cx);
    });
    // The Window menu has 「(なし)」 while there is no window.
    assert!(menu_names(cx).iter().any(|n| n == "(なし)"));
    open(cx);
    let names = menu_names(cx);
    assert!(names.iter().any(|n| n.starts_with("1 (無題)")), "{names:?}");
    assert!(
        !names.iter().any(|n| n.starts_with("2 (無題)")),
        "{names:?}"
    );
    open(cx);
    let names = menu_names(cx);
    assert!(names.iter().any(|n| n.starts_with("2 (無題)")), "{names:?}");
}

#[gpui::test]
fn the_digit_of_an_item_is_its_access_key(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::app::set_key_map(cx, matsu_commands::keymap::KeyMap::default());
        crate::app::refresh_menus(cx);
    });
    open(cx);
    open(cx);
    let keys = cx.update(|cx| {
        let menus = crate::app::KindMenus::of(cx, crate::tabs::content::MenuKind::Editor).unwrap();
        let mut keys = Vec::new();
        fn walk(items: &[gpui::OwnedMenuItem], keys: &mut Vec<(String, Option<char>)>) {
            for item in items {
                match item {
                    gpui::OwnedMenuItem::Action { name, .. }
                        if name.contains(" (無題)")
                            && name.chars().next().is_some_and(|c| c.is_ascii_digit()) =>
                    {
                        keys.push((name.clone(), crate::menu_bar::access_key_of(item)));
                    }
                    gpui::OwnedMenuItem::Submenu(menu) => walk(&menu.items, keys),
                    _ => {}
                }
            }
        }
        for menu in &menus {
            walk(&menu.items, &mut keys);
        }
        keys
    });
    let keys: Vec<_> = keys.into_iter().map(|(_, key)| key).collect();
    assert_eq!(keys, vec![Some('1'), Some('2')]);
}
