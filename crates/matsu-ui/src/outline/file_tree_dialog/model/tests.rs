//! Tests of the tree of the dialog 「ファイルツリー設定」 and its buttons.

use matsu_config::settings::file_tree::{FileTreeItem, ItemType};

use super::{Tree, node_label};

fn folder(label: &str, depth: i32) -> FileTreeItem {
    FileTreeItem {
        item_type: ItemType::Folder,
        label: label.to_owned(),
        depth,
        ..FileTreeItem::default()
    }
}

fn file(path: &str, depth: i32) -> FileTreeItem {
    FileTreeItem {
        item_type: ItemType::File,
        target_path: path.to_owned(),
        depth,
        ..FileTreeItem::default()
    }
}

fn grep(path: &str, depth: i32) -> FileTreeItem {
    FileTreeItem {
        target_path: path.to_owned(),
        target_file: "*.*".to_owned(),
        depth,
        ..FileTreeItem::default()
    }
}

/// The labels of every item, depth first, with the depth: `"A0 b1 c1 D0"`.
fn shape(tree: &Tree) -> String {
    let (items, complete) = tree.to_items(0);
    assert!(complete);
    items
        .iter()
        .map(|item| format!("{}{}", super::node_label(item), item.depth))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A tree with two folders: `top0 (a1 (inner1 (deep2)) b1) c0 (d1) e0`, nothing selected.
fn sample() -> Tree {
    let mut tree = Tree::from_items(&[
        folder("top", 0),
        file("a", 1),
        folder("inner", 1),
        file("deep", 2),
        file("b", 1),
        folder("c", 0),
        file("d", 1),
        file("e", 0),
    ]);
    for path in [vec![0], vec![0, 1], vec![1]] {
        tree.select(&path);
    }
    tree
}

#[test]
fn the_items_become_a_tree_by_their_depth() {
    let tree = sample();
    assert_eq!(shape(&tree), "top0 a1 inner1 deep2 b1 c0 d1 e0");
    assert_eq!(tree.count(), 8);
    let (items, _) = tree.to_items(0);
    assert_eq!(items.len(), 8);
    assert_eq!(tree.node(&[0, 1, 0]).unwrap().item.target_path, "deep");
    assert_eq!(tree.node(&[2]).unwrap().item.target_path, "e");
}

#[test]
fn the_first_item_is_selected_and_nothing_is_open() {
    let tree = Tree::from_items(&[folder("top", 0), file("a", 1)]);
    assert_eq!(tree.selected(), Some(&vec![0]));
    assert_eq!(tree.rows().len(), 1, "the folder is closed");
    assert!(Tree::from_items(&[]).selected().is_none());
}

#[test]
fn an_item_deeper_than_its_parent_goes_under_the_last_folder() {
    // depth 3 after a folder at depth 0 is a child of the folder, as in `SetData`
    let tree = Tree::from_items(&[folder("top", 0), file("x", 3), file("y", 0)]);
    assert_eq!(shape(&tree), "top0 x1 y0");
    // and a file takes no children: the next item with a deeper depth is its sibling
    let tree = Tree::from_items(&[file("a", 0), file("b", 1)]);
    assert_eq!(shape(&tree), "a0 b0");
}

#[test]
fn the_rows_follow_the_open_folders() {
    let mut tree = sample();
    let labels = |tree: &Tree| {
        tree.rows()
            .iter()
            .map(|r| format!("{}{}", r.label, r.depth))
            .collect::<Vec<_>>()
            .join(" ")
    };
    // (`sample` selected the rows on its way, which opened `top` and its parents)
    assert_eq!(labels(&tree), "top0 a1 inner1 b1 c0 e0");
    tree.toggle(&[1]);
    assert_eq!(labels(&tree), "top0 a1 inner1 b1 c0 d1 e0");
    tree.select(&[0, 0]);
    tree.toggle(&[0]);
    assert_eq!(labels(&tree), "top0 c0 d1 e0");
    assert_eq!(
        tree.selected(),
        Some(&vec![0]),
        "the closed folder takes the selection"
    );
}

#[test]
fn labels_follow_the_rules_of_sakura() {
    let mut item = grep(".", 0);
    assert_eq!(node_label(&item), "<カレントフォルダー>");
    item.target_path = ".\\".to_owned();
    assert_eq!(node_label(&item), "<カレントフォルダー>");
    item.target_path = "./".to_owned();
    assert_eq!(node_label(&item), "<カレントフォルダー>");
    item.target_path = "C:\\work".to_owned();
    assert_eq!(node_label(&item), "C:\\work");
    item.label = "仕事".to_owned();
    assert_eq!(node_label(&item), "仕事");
    // a label is kept even for the current folder
    item.target_path = ".".to_owned();
    assert_eq!(node_label(&item), "仕事");
    assert_eq!(node_label(&folder("", 0)), "Folder");
    assert_eq!(node_label(&folder("資料", 0)), "資料");
}

#[test]
fn add_puts_the_item_last_into_the_selected_folder_or_its_list() {
    let mut tree = Tree::from_items(&[folder("top", 0), file("a", 1), file("z", 0)]);
    tree.select(&[0]);
    tree.add(file("new", 0));
    assert_eq!(shape(&tree), "top0 a1 new1 z0");
    assert_eq!(
        tree.selected(),
        Some(&vec![0, 1]),
        "the new item is selected"
    );
    // below a file: last in the list of the file
    tree.select(&[0, 0]);
    tree.add(file("n2", 0));
    assert_eq!(shape(&tree), "top0 a1 new1 n21 z0");
    // below a file of the top list: last in the top list
    tree.select(&[1]);
    tree.add(file("n3", 0));
    assert_eq!(shape(&tree), "top0 a1 new1 n21 z0 n30");
}

#[test]
fn add_with_nothing_selected_goes_last_into_the_top_list() {
    let mut tree = Tree::default();
    tree.add(file("one", 0));
    tree.delete();
    assert!(tree.selected().is_none());
    tree.add(file("two", 0));
    tree.delete();
    tree.add(file("three", 0));
    assert_eq!(shape(&tree), "three0");
}

#[test]
fn insert_below_goes_first_into_a_folder_and_after_a_file() {
    let mut tree = Tree::from_items(&[folder("top", 0), file("a", 1), file("z", 0)]);
    tree.select(&[0]);
    tree.insert_below(file("new", 0));
    assert_eq!(shape(&tree), "top0 new1 a1 z0");
    tree.select(&[0, 1]);
    tree.insert_below(file("n2", 0));
    assert_eq!(shape(&tree), "top0 new1 a1 n21 z0");
    assert_eq!(tree.selected(), Some(&vec![0, 2]));
}

#[test]
fn insert_above_goes_right_before_the_selected_item_at_every_level() {
    let mut tree = Tree::from_items(&[file("a", 0), file("b", 0), folder("top", 0), file("c", 1)]);
    // in the top list too (Sakura puts it first there; see the comment of `insert_above`)
    tree.select(&[1]);
    tree.insert_above(file("n1", 0));
    assert_eq!(shape(&tree), "a0 n10 b0 top0 c1");
    tree.select(&[3, 0]);
    tree.insert_above(file("n2", 0));
    assert_eq!(shape(&tree), "a0 n10 b0 top0 n21 c1");
    assert_eq!(tree.selected(), Some(&vec![3, 0]));
    // before a folder, not into it
    tree.select(&[3]);
    tree.insert_above(file("n3", 0));
    assert_eq!(shape(&tree), "a0 n10 b0 n30 top0 n21 c1");
}

#[test]
fn insert_above_with_nothing_selected_goes_first() {
    let mut tree = Tree::from_items(&[file("a", 0)]);
    tree.delete();
    tree.insert_above(file("x", 0));
    tree.delete();
    tree.insert_above(file("y", 0));
    assert_eq!(shape(&tree), "y0");
}

#[test]
fn files_go_first_into_a_folder_in_order_and_last_below_a_file() {
    let mut tree = Tree::from_items(&[folder("top", 0), file("a", 1), file("z", 0)]);
    tree.select(&[0]);
    tree.add_files(vec![file("f1", 0), file("f2", 0)]);
    assert_eq!(shape(&tree), "top0 f11 f21 a1 z0");
    assert_eq!(
        tree.selected(),
        Some(&vec![0, 0]),
        "the first of them is selected"
    );
    // below a file they go last in its list (Sakura: `TVI_LAST`)
    tree.select(&[0, 2]);
    tree.add_files(vec![file("g1", 0), file("g2", 0)]);
    assert_eq!(shape(&tree), "top0 f11 f21 a1 g11 g21 z0");
    assert_eq!(tree.selected(), Some(&vec![0, 3]));
}

#[test]
fn files_with_nothing_selected_go_last() {
    let mut tree = Tree::from_items(&[file("a", 0)]);
    tree.delete();
    tree.add_files(vec![file("f1", 0), file("f2", 0)]);
    assert_eq!(shape(&tree), "f10 f20");
    assert_eq!(tree.selected(), Some(&vec![0]));
}

#[test]
fn update_changes_the_selected_item_and_keeps_the_children() {
    let mut tree = sample();
    tree.select(&[0]);
    assert!(tree.update(folder("renamed", 0)));
    assert_eq!(shape(&tree), "renamed0 a1 inner1 deep2 b1 c0 d1 e0");
    tree.delete();
    tree.delete();
    tree.delete();
    assert!(!tree.update(file("x", 0)), "nothing is selected");
}

#[test]
fn delete_takes_the_children_and_selects_a_neighbor() {
    let mut tree = sample();
    tree.select(&[0, 1]);
    assert!(tree.selected_has_children());
    assert!(tree.delete());
    assert_eq!(shape(&tree), "top0 a1 b1 c0 d1 e0");
    // the next one of the list
    assert_eq!(tree.selected(), Some(&vec![0, 1]));
    // the last one: the one before
    assert!(tree.delete());
    assert_eq!(tree.selected(), Some(&vec![0, 0]));
    // the only one: the parent
    assert!(tree.delete());
    assert_eq!(tree.selected(), Some(&vec![0]));
    assert_eq!(shape(&tree), "top0 c0 d1 e0");
    // the last of the top list
    tree.select(&[2]);
    tree.delete();
    assert_eq!(tree.selected(), Some(&vec![1]));
}

#[test]
fn deleting_everything_leaves_nothing_selected() {
    let mut tree = Tree::from_items(&[file("a", 0), file("b", 0)]);
    assert!(tree.delete());
    assert!(tree.delete());
    assert!(tree.is_empty());
    assert!(tree.selected().is_none());
    assert!(!tree.delete());
}

#[test]
fn up_and_down_change_places_within_the_list_and_follow_the_item() {
    let mut tree = sample();
    tree.select(&[0, 2]);
    assert!(tree.move_up());
    assert_eq!(shape(&tree), "top0 a1 b1 inner1 deep2 c0 d1 e0");
    assert_eq!(
        tree.selected(),
        Some(&vec![0, 1]),
        "the item stays selected"
    );
    assert!(tree.move_up());
    assert!(!tree.move_up(), "the first of its list");
    assert_eq!(shape(&tree), "top0 b1 a1 inner1 deep2 c0 d1 e0");
    assert!(tree.move_down());
    assert!(tree.move_down());
    assert!(!tree.move_down(), "the last of its list");
    assert_eq!(shape(&tree), "top0 a1 inner1 deep2 b1 c0 d1 e0");
    assert_eq!(tree.selected(), Some(&vec![0, 2]));
}

#[test]
fn a_folder_moves_with_its_children() {
    let mut tree = sample();
    tree.select(&[0]);
    assert!(tree.move_down());
    assert_eq!(shape(&tree), "c0 d1 top0 a1 inner1 deep2 b1 e0");
    assert_eq!(tree.selected(), Some(&vec![1]));
}

#[test]
fn right_puts_the_item_last_into_the_folder_before_it() {
    let mut tree = sample();
    tree.select(&[2]);
    // `e` follows the folder `c`
    assert!(tree.move_right());
    assert_eq!(shape(&tree), "top0 a1 inner1 deep2 b1 c0 d1 e1");
    assert_eq!(tree.selected(), Some(&vec![1, 1]));
    // the one before `e` is `d`, a file: nothing happens
    assert!(!tree.move_right());
    // the first of a list: nothing happens
    tree.select(&[0, 0]);
    assert!(!tree.move_right());
    // the one before `b` is the folder `inner`: `b` goes into it
    tree.select(&[0, 2]);
    assert!(tree.move_right());
    assert_eq!(shape(&tree), "top0 a1 inner1 deep2 b2 c0 d1 e1");
    assert_eq!(tree.selected(), Some(&vec![0, 1, 1]));
}

#[test]
fn right_works_when_the_item_before_is_a_folder() {
    let mut tree = Tree::from_items(&[folder("f", 0), file("a", 1), file("b", 0)]);
    tree.select(&[1]);
    assert!(tree.move_right());
    assert_eq!(shape(&tree), "f0 a1 b1");
    assert_eq!(tree.selected(), Some(&vec![0, 1]));
}

#[test]
fn left_puts_the_item_right_after_its_parent() {
    let mut tree = sample();
    tree.select(&[0, 0]);
    assert!(tree.move_left());
    assert_eq!(shape(&tree), "top0 inner1 deep2 b1 a0 c0 d1 e0");
    assert_eq!(tree.selected(), Some(&vec![1]));
    // not from the top list
    assert!(!tree.move_left());
    // from the second level: after the folder it is in
    tree.select(&[0, 0, 0]);
    assert!(tree.move_left());
    assert_eq!(shape(&tree), "top0 inner1 deep1 b1 a0 c0 d1 e0");
    assert_eq!(tree.selected(), Some(&vec![0, 1]));
}

#[test]
fn replace_changes_every_path_and_leaves_the_rest() {
    let mut tree = Tree::from_items(&[
        grep("C:\\old\\src", 0),
        folder("old", 0),
        file("C:\\old\\a.txt", 1),
        file("D:\\other", 0),
    ]);
    tree.replace_paths("C:\\old", "E:\\new");
    let (items, _) = tree.to_items(0);
    assert_eq!(items[0].target_path, "E:\\new\\src");
    assert_eq!(items[1].label, "old", "labels are not paths");
    assert_eq!(items[2].target_path, "E:\\new\\a.txt");
    assert_eq!(items[3].target_path, "D:\\other");
}

#[test]
fn replace_takes_every_place_in_a_path_and_does_not_look_in_the_new_text() {
    let mut tree = Tree::from_items(&[file("aaa", 0)]);
    tree.replace_paths("a", "aa");
    assert_eq!(tree.to_items(0).0[0].target_path, "aaaaaa");
}

#[test]
fn replace_with_an_empty_text_to_look_for_changes_nothing() {
    // Sakura's `CNativeW::Replace` never ends with it
    let mut tree = Tree::from_items(&[file("abc", 0)]);
    tree.replace_paths("", "x");
    assert_eq!(tree.to_items(0).0[0].target_path, "abc");
}

#[test]
fn replace_can_delete_text_and_handles_japanese_paths() {
    let mut tree = Tree::from_items(&[file("C:\\ユーザー\\太郎\\文書", 0)]);
    tree.replace_paths("太郎", "");
    assert_eq!(tree.to_items(0).0[0].target_path, "C:\\ユーザー\\\\文書");
}

#[test]
fn a_replaced_path_that_does_not_fit_is_left_alone() {
    // `SFilePath` holds `_MAX_PATH` (260) characters with the end of the text
    let long = "x".repeat(250);
    let mut tree = Tree::from_items(&[file(&long, 0), file("short", 0)]);
    tree.replace_paths("x", "xx");
    let (items, _) = tree.to_items(0);
    assert_eq!(items[0].target_path, long);
    tree.replace_paths("short", "s".repeat(259).as_str());
    assert_eq!(tree.to_items(0).0[1].target_path.len(), 259);
    tree.replace_paths("s", "ss");
    assert_eq!(tree.to_items(0).0[1].target_path.len(), 259);
}

#[test]
fn to_items_stops_at_the_limit() {
    let tree = Tree::from_items(&[folder("f", 0), file("a", 1), file("b", 1), file("c", 0)]);
    let (items, complete) = tree.to_items(3);
    assert!(!complete);
    assert_eq!(items.len(), 3);
    let (items, complete) = tree.to_items(4);
    assert!(complete, "all four fit");
    assert_eq!(items.len(), 4);
    let (items, complete) = tree.to_items(0);
    assert!(complete);
    assert_eq!(items.len(), 4);
}

#[test]
fn the_depth_is_the_depth_in_the_tree() {
    let mut tree = Tree::from_items(&[folder("f", 0), file("a", 1)]);
    tree.select(&[0]);
    tree.add(file("n", 7));
    let (items, _) = tree.to_items(0);
    assert_eq!(items.iter().map(|i| i.depth).collect::<Vec<_>>(), [0, 1, 1]);
}

#[test]
fn only_folders_have_children_in_the_output() {
    // an item of another kind never gets the items that follow it as children
    let tree = Tree::from_items(&[
        FileTreeItem {
            item_type: ItemType::Other(9),
            depth: 0,
            ..FileTreeItem::default()
        },
        file("a", 1),
    ]);
    assert_eq!(tree.count(), 2);
    assert_eq!(tree.rows().len(), 2);
}
