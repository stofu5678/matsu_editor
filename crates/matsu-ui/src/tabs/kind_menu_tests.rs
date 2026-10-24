//! The menus follow the kind of the active tab (ADR-0024): the menu bar of a window shows the menus
//! of the kind of its active tab, and the state of the native menu (macOS) is found for it.
//! Only the editor kind exists; the tests give a tab the test-only `MenuKind::Probe`.

use gpui::{AnyWindowHandle, TestAppContext};
use matsu_commands::keymap::KeyMap;

use super::content::MenuKind;
use super::tests::{run, window, workspace};
use crate::app::{KindMenus, active_menu_context};

/// macOS puts the application menu before the menus of every kind; the tests leave it out.
const APP_MENU: &str = "MATSU Editor";

/// Makes the menus of every kind, as the application does at startup.
fn make_menus(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::app::set_key_map(cx, KeyMap::default());
        crate::app::refresh_menus(cx);
    });
}

/// The names of the menus of the bar of the window.
fn bar_names(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Vec<String> {
    let ws = workspace(cx, handle);
    let bar = ws.read_with(cx, |ws, _| ws.menu_bar.clone());
    bar.read_with(cx, |bar, cx| bar.menu_names(cx))
        .into_iter()
        .filter(|name| name != APP_MENU)
        .collect()
}

/// Gives tab `index` of the window the kind `kind` and makes it the active tab.
fn show_tab_of_kind(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    index: usize,
    kind: Option<MenuKind>,
) {
    let ws = workspace(cx, handle);
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| {
                let id = ws.tabs.list[index].id;
                match kind {
                    Some(kind) => ws.tabs.kinds.insert(id, kind),
                    None => ws.tabs.kinds.remove(&id),
                };
                ws.activate_tab(index, window, cx);
            })
        })
        .unwrap();
    cx.run_until_parked();
}

#[gpui::test]
fn the_application_makes_the_menus_of_every_kind(cx: &mut TestAppContext) {
    make_menus(cx);
    cx.update(|cx| {
        let names = |kind| {
            KindMenus::of(cx, kind)
                .unwrap()
                .iter()
                .map(|m| m.name.to_string())
                .filter(|name| name != APP_MENU)
                .collect::<Vec<_>>()
        };
        assert!(
            names(MenuKind::Editor)
                .iter()
                .any(|n| n.contains("ファイル")),
            "{:?}",
            names(MenuKind::Editor)
        );
        assert_eq!(names(MenuKind::Probe), vec!["Probe".to_owned()]);
    });
}

#[gpui::test]
fn the_menu_bar_follows_the_kind_of_the_active_tab(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    make_menus(cx);
    run(cx, handle, "F_FILENEW");
    // The new tab is the active one; go back to the first.
    show_tab_of_kind(cx, handle, 0, None);
    let editor_menus = bar_names(cx, handle);
    assert!(editor_menus.iter().any(|n| n.contains("ファイル")));

    // The second tab is of the other kind: the bar changes with the tab, and back.
    show_tab_of_kind(cx, handle, 1, Some(MenuKind::Probe));
    assert_eq!(bar_names(cx, handle), vec!["Probe".to_owned()]);
    show_tab_of_kind(cx, handle, 0, None);
    assert_eq!(bar_names(cx, handle), editor_menus);
}

#[gpui::test]
fn another_window_keeps_its_menus(cx: &mut TestAppContext) {
    let first = window(cx, true);
    make_menus(cx);
    run(cx, first, "F_FILENEW");
    let second = cx
        .update(|app| {
            crate::file_ops::windows::open_editor_window(
                app,
                crate::document::SharedDocument::empty(),
                None,
            )
        })
        .unwrap();
    cx.run_until_parked();
    show_tab_of_kind(cx, first, 0, None);
    let editor_menus = bar_names(cx, second);

    show_tab_of_kind(cx, first, 1, Some(MenuKind::Probe));
    assert_eq!(bar_names(cx, first), vec!["Probe".to_owned()]);
    assert_eq!(bar_names(cx, second), editor_menus);
}

#[gpui::test]
fn the_native_menu_asks_for_the_kind_and_state_of_the_active_window(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    make_menus(cx);
    run(cx, handle, "F_FILENEW");
    show_tab_of_kind(cx, handle, 0, None);
    handle
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    let (kind, states) = cx.update(active_menu_context);
    assert_eq!(kind, MenuKind::Editor);
    assert!(states.is_some());

    // A kind that is not an editor has no state of the editor: its items stay as they are made.
    show_tab_of_kind(cx, handle, 1, Some(MenuKind::Probe));
    let (kind, states) = cx.update(active_menu_context);
    assert_eq!(kind, MenuKind::Probe);
    assert!(states.is_none());
}
