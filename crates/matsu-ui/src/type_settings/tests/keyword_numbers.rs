//! The numbers of the keyword sets in the sheet: they follow the sets when the common settings delete or
//! change one, and the ten slots are merged one by one.

use super::super::pages::keyword_select::{self, SetKey, chooser_name, renumbering};
use super::dialogs::{color_page, in_child, sheet_values, with_three_sets};
use super::*;

/// The sets with the given names, each with its name in lower case as its only word.
fn keys(names: &[&str]) -> Vec<SetKey> {
    names
        .iter()
        .map(|n| ((*n).to_owned(), true, vec![n.to_lowercase()]))
        .collect()
}

/// The settings file after set A was deleted in the common settings: B, C and D are 0, 1 and 2.
const WITHOUT_A: &str = "[KeyWords]\r\nnKeyWordSetNum=3\r\n\
    szSN[00]=B\r\nnCASE[00]=1\r\nnKWN[00]=1\r\nszKW[00]=b\t\r\n\
    szSN[01]=C\r\nnCASE[01]=1\r\nnKWN[01]=1\r\nszKW[01]=c\t\r\n\
    szSN[02]=D\r\nnCASE[02]=1\r\nnKWN[02]=1\r\nszKW[02]=d\t\r\n\
    [Types(0)]\r\nnInts=0,10240,0,4,0,1,0,0,0,0,0,0\r\n";

#[test]
fn the_sets_are_matched_by_what_they_are() {
    // A is deleted: B, C and D move up, A is none.
    assert_eq!(
        renumbering(&keys(&["A", "B", "C", "D"]), &keys(&["B", "C", "D"])),
        [-1, 0, 1, 2]
    );
    // Nothing happened.
    assert_eq!(renumbering(&keys(&["A", "B"]), &keys(&["A", "B"])), [0, 1]);
    // A set added in front, and two equal sets keep their order.
    assert_eq!(
        renumbering(&keys(&["A", "A", "B"]), &keys(&["N", "A", "A", "B"])),
        [1, 2, 3]
    );
    // The words of B are edited (same name), and C is renamed (same words).
    let before = vec![
        ("B".to_owned(), true, vec!["b".to_owned()]),
        ("C".to_owned(), true, vec!["c".to_owned()]),
    ];
    let after = vec![
        ("C2".to_owned(), true, vec!["c".to_owned()]),
        ("B".to_owned(), true, vec!["b".to_owned(), "bb".to_owned()]),
    ];
    assert_eq!(renumbering(&before, &after), [1, 0]);
}

#[gpui::test]
fn an_unsaved_choice_follows_the_sets_when_one_is_deleted(cx: &mut TestAppContext) {
    let ini = color_page(cx, &with_three_sets());
    // The first slot is changed from A to B and the third slot to D; nothing is saved yet.
    with_dialog(cx, |d, _, dcx| d.pick_first_set(1, dcx));
    let dialog = with_dialog(cx, |d, _, dcx| d.keyword_select(dcx)).expect("opens");
    in_child(cx, &dialog, |d, window, dcx| {
        d.choose(2, 3, dcx);
        d.ok(window, dcx);
    });
    assert_eq!(
        keyword_select::sets_of(&sheet_values(cx))[..4],
        [1, 1, 3, -1],
        "B, B and D"
    );
    let before = with_dialog(cx, |d, _, dcx| d.keyword_snapshot(dcx));
    std::fs::write(&ini.path, with_bom(WITHOUT_A)).unwrap();
    with_dialog(cx, |d, _, dcx| d.after_keyword_edit(&before, dcx));
    // B is 0 and D is 2 now; the empty slot stays empty.
    assert_eq!(
        keyword_select::sets_of(&sheet_values(cx))[..4],
        [0, 0, 2, -1]
    );
    assert_eq!(keyword_select::names_of(&sheet_values(cx)), ["B", "C", "D"]);
    cancel(cx);
}

#[gpui::test]
fn a_deleted_set_becomes_none_in_the_sheet(cx: &mut TestAppContext) {
    let ini = color_page(cx, &with_three_sets());
    // The slots hold A, B and C (0, 1 and 2).
    let before = with_dialog(cx, |d, _, dcx| d.keyword_snapshot(dcx));
    std::fs::write(&ini.path, with_bom(WITHOUT_A)).unwrap();
    with_dialog(cx, |d, _, dcx| d.after_keyword_edit(&before, dcx));
    assert_eq!(
        keyword_select::sets_of(&sheet_values(cx))[..4],
        [-1, 0, 1, -1],
        "A is gone, B and C moved up"
    );
    cancel(cx);
}

#[test]
fn a_change_to_one_slot_keeps_another_process_change_to_another_slot() {
    let old = TypeSettings::basic();
    // The user changes slot 6; another process changes slot 3 meanwhile.
    let mut new = old.clone();
    new.keyword_set_idx[5] = 7;
    let mut base = old.clone();
    base.keyword_set_idx[2] = 4;
    let merged = super::super::values::merge(&base, &old, &new);
    assert_eq!(merged.keyword_set_idx[2], 4, "the other process's slot");
    assert_eq!(merged.keyword_set_idx[5], 7, "the user's slot");
    assert_eq!(merged.keyword_set_idx[0], old.keyword_set_idx[0]);
}

#[test]
fn the_lists_of_sets_have_names_for_a_screen_reader() {
    let names = ["A".to_owned(), "B".to_owned()];
    assert_eq!(chooser_name(0, 1, &names), "強調キーワード1: B");
    assert_eq!(chooser_name(9, 0, &names), "強調キーワード10: A");
    // No set, and a set the list does not have, read as none.
    assert_eq!(chooser_name(2, -1, &names), "強調キーワード3: なし");
    assert_eq!(chooser_name(3, 5, &names), "強調キーワード4: なし");
}
