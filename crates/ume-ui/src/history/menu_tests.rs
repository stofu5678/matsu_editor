//! Tests of the recent files in the File menu.

use ume_commands::keymap::KeyMap;
use ume_config::history::{FileEntry, History};

use super::menu::{RecentLists, label};
use crate::menu::{Entry, Item, MenuAction, Registered, TopMenu, main_menu_with};

fn entry(path: &str, code: i32, favorite: bool) -> FileEntry {
    let mut e = FileEntry::new(path);
    e.encoding_code = code;
    e.favorite = favorite;
    e
}

#[test]
fn a_label_has_the_key_the_star_the_path_and_the_encoding() {
    assert_eq!(label(0, &entry("/a/b.txt", 0, false)), "1 /a/b.txt");
    assert_eq!(label(1, &entry("/a/b.txt", 99, true)), "2 ★ /a/b.txt");
    assert_eq!(
        label(8, &entry("C:\\メモ & 日記.txt", 4, false)),
        "9 C:\\メモ & 日記.txt  [UTF-8]"
    );
    assert_eq!(label(9, &entry("x", 3, false)), "A x  [UTF-16]");
    assert_eq!(label(35, &entry("x", 0, false)), "0 x");
    assert_eq!(label(36, &entry("x", 0, false)), "1 x");
}

fn history_of(count: usize, view: usize) -> History {
    let mut h = History::new(view, view, false);
    for n in 0..count {
        h.add_file(FileEntry::new(format!("/d/{n}.txt")));
    }
    h
}

fn recent_items(menus: &[TopMenu]) -> Vec<Item> {
    fn walk(entries: &[Entry], out: &mut Vec<Item>) {
        for e in entries {
            match e {
                Entry::Item(i) if matches!(i.action, MenuAction::RecentFile(_)) => {
                    out.push(i.clone());
                }
                Entry::Submenu { entries, .. } => walk(entries, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    for m in menus {
        walk(&m.entries, &mut out);
    }
    out
}

/// The items of the submenu of the File menu whose label starts with `label`.
fn submenu<'a>(menus: &'a [TopMenu], label: &str) -> &'a [Entry] {
    menus[0]
        .entries
        .iter()
        .find_map(|e| match e {
            Entry::Submenu { label: l, entries } if l.starts_with(label) => {
                Some(entries.as_slice())
            }
            _ => None,
        })
        .expect("the submenu")
}

#[test]
fn the_file_menu_lists_the_files_that_are_shown() {
    let lists = RecentLists::new(Some(history_of(5, 3)));
    let menus = main_menu_with(&KeyMap::default(), &Registered, &lists);
    let items = recent_items(&menus);
    assert_eq!(items.len(), 3, "the view count, not all five");
    assert_eq!(items[0].label, "1 /d/4.txt");
    assert_eq!(items[0].action, MenuAction::RecentFile(0));
    assert_eq!(items[2].action, MenuAction::RecentFile(2));
    assert!(items.iter().all(|i| i.enabled));
    // They are in the submenu of the recent files, in the File menu.
    let entries = submenu(&menus, "最近使ったファイル");
    assert_eq!(entries.len(), 3);
}

#[test]
fn an_empty_history_leaves_the_disabled_placeholder() {
    for lists in [
        RecentLists::new(None),
        RecentLists::new(Some(history_of(0, 3))),
    ] {
        let menus = main_menu_with(&KeyMap::default(), &Registered, &lists);
        assert!(recent_items(&menus).is_empty());
        let entries = submenu(&menus, "最近使ったファイル");
        assert!(matches!(
            entries,
            [Entry::Item(i)] if i.action == MenuAction::Placeholder && !i.enabled
        ));
    }
}

#[test]
fn the_folders_stay_a_placeholder() {
    // Choosing a folder would open the open dialog there; the dialog of the OS cannot start in a
    // given folder.
    let lists = RecentLists::new(Some(history_of(3, 3)));
    let menus = main_menu_with(&KeyMap::default(), &Registered, &lists);
    let entries = submenu(&menus, "最近使ったフォルダー");
    assert!(matches!(
        entries,
        [Entry::Item(i)] if i.action == MenuAction::Placeholder
    ));
}
