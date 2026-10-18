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
    assert_eq!(
        label(0, &entry("/a/b.txt", 0, false), "/a/b.txt"),
        "1 /a/b.txt"
    );
    assert_eq!(
        label(1, &entry("/a/b.txt", 99, true), "/a/b.txt"),
        "2 ★ /a/b.txt"
    );
    assert_eq!(
        label(
            8,
            &entry("C:\\メモ & 日記.txt", 4, false),
            "C:\\メモ & 日記.txt"
        ),
        "9 C:\\メモ & 日記.txt  [UTF-8]"
    );
    assert_eq!(label(9, &entry("x", 3, false), "x"), "A x  [UTF-16]");
    assert_eq!(label(35, &entry("x", 0, false), "x"), "0 x");
    assert_eq!(label(36, &entry("x", 0, false), "x"), "1 x");
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
    assert_eq!(items[0].action, MenuAction::RecentFile("/d/4.txt".into()));
    assert_eq!(items[2].action, MenuAction::RecentFile("/d/2.txt".into()));
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
fn the_folders_are_listed_with_their_access_keys_and_stars() {
    let mut h = History::new(3, 2, false);
    for path in ["/a", "/b", "/c"] {
        h.add_folder(path);
    }
    h.folders.set_favorite(0, true);
    let lists = RecentLists::new(Some(h));
    let menus = main_menu_with(&KeyMap::default(), &Registered, &lists);
    let entries = submenu(&menus, "最近使ったフォルダー");
    let shown: Vec<_> = entries
        .iter()
        .map(|e| match e {
            Entry::Item(i) => (i.label.clone(), i.action.clone(), i.enabled),
            other => panic!("{other:?}"),
        })
        .collect();
    // the view count (two folders), newest first
    assert_eq!(
        shown,
        [
            (
                "1 ★ /c".to_owned(),
                MenuAction::RecentFolder("/c".into()),
                true
            ),
            (
                "2 /b".to_owned(),
                MenuAction::RecentFolder("/b".into()),
                true
            ),
        ]
    );
}

#[test]
fn no_folders_is_a_disabled_placeholder() {
    let lists = RecentLists::new(Some(history_of(3, 3)));
    let menus = main_menu_with(&KeyMap::default(), &Registered, &lists);
    // the files have folders, but the history here has none of its own
    let entries = submenu(&menus, "最近使ったフォルダー");
    assert!(
        matches!(entries, [Entry::Item(i)] if i.action == MenuAction::RecentFolder("/d/".into()))
    );
    let empty = RecentLists::new(Some(History::new(3, 3, false)));
    let menus = main_menu_with(&KeyMap::default(), &Registered, &empty);
    assert!(matches!(
        submenu(&menus, "最近使ったフォルダー"),
        [Entry::Item(i)] if i.action == MenuAction::Placeholder && !i.enabled
    ));
}

#[test]
fn the_menus_show_the_paths_with_the_file_name_display() {
    use crate::file_names::{FileNames, NoFolders};

    let rows = [vec!["/d/".to_owned(), "データ:".to_owned()]];
    let names = FileNames::new(true, 100, &rows, &NoFolders, false);
    let mut h = History::new(3, 2, false);
    h.add_file(entry("/d/a.txt", 0, false));
    h.add_file(entry("/e/日本語/long.txt", 4, false));
    h.files.set_favorite(0, true);
    h.add_folder("/d/sub");
    h.add_folder("/e");
    let lists = RecentLists::new(Some(h)).with_names(names);
    let menus = main_menu_with(&KeyMap::default(), &Registered, &lists);
    let files: Vec<_> = recent_items(&menus).into_iter().map(|i| i.label).collect();
    // The label keeps the star and the encoding; the path is the shown one.
    assert_eq!(files, ["1 ★ /e/日本語/long.txt  [UTF-8]", "2 データ:a.txt"]);
    let folders: Vec<_> = submenu(&menus, "最近使ったフォルダー")
        .iter()
        .map(|e| match e {
            Entry::Item(i) => (i.label.clone(), i.action.clone()),
            other => panic!("{other:?}"),
        })
        .collect();
    // What the item does still uses the path itself.
    assert_eq!(
        folders,
        [
            ("1 /e".to_owned(), MenuAction::RecentFolder("/e".into())),
            (
                "2 データ:sub".to_owned(),
                MenuAction::RecentFolder("/d/sub".into())
            ),
        ]
    );
}
