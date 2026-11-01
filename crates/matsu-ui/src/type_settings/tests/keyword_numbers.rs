//! The numbers of the keyword sets in the sheet: they follow the renumbering of the common settings (also the
//! choices that are not saved), and the ten slots are merged one by one.

use std::rc::Rc;

use super::super::pages::child;
use super::super::pages::keyword_select::{self, chooser_name};
use super::dialogs::{color_page, in_child, sheet_values, with_three_sets};
use super::*;

/// The settings file after set A was deleted in the common settings: B, C and D are 0, 1 and 2. The type
/// has the numbers the keyword page wrote for it (A was in its first slot, B in its second).
const WITHOUT_A: &str = "[KeyWords]\r\nnKeyWordSetNum=3\r\n\
    szSN[00]=B\r\nnCASE[00]=1\r\nnKWN[00]=1\r\nszKW[00]=b\t\r\n\
    szSN[01]=C\r\nnCASE[01]=1\r\nnKWN[01]=1\r\nszKW[01]=c\t\r\n\
    szSN[02]=D\r\nnCASE[02]=1\r\nnKWN[02]=1\r\nszKW[02]=d\t\r\n\
    [Types(0)]\r\nnInts=0,10240,0,4,-1,0,0,0,0,0,0,0\r\nnKeywordSelect3=1\r\n";

/// What the keyword page publishes when set A of A to D is deleted.
fn delete_a() {
    crate::common_settings::publish_keyword_renumbering(Rc::new(|n| match n {
        0 => -1,
        n if n > 0 => n - 1,
        n => n,
    }));
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
    crate::common_settings::forget_keyword_renumbering();
    std::fs::write(&ini.path, with_bom(WITHOUT_A)).unwrap();
    delete_a();
    with_dialog(cx, |d, _, dcx| d.after_keyword_edit(dcx));
    // B is 0 and D is 2 now; the empty slot stays empty.
    assert_eq!(
        keyword_select::sets_of(&sheet_values(cx))[..4],
        [0, 0, 2, -1]
    );
    assert_eq!(keyword_select::names_of(&sheet_values(cx)), ["B", "C", "D"]);
    // A second watcher of the same closing does not renumber again.
    with_dialog(cx, |d, _, dcx| d.after_keyword_edit(dcx));
    assert_eq!(
        keyword_select::sets_of(&sheet_values(cx))[..4],
        [0, 0, 2, -1]
    );
    cancel(cx);
}

#[gpui::test]
fn a_deleted_set_becomes_none_in_the_sheet(cx: &mut TestAppContext) {
    let ini = color_page(cx, &with_three_sets());
    crate::common_settings::forget_keyword_renumbering();
    std::fs::write(&ini.path, with_bom(WITHOUT_A)).unwrap();
    delete_a();
    // The slots hold A, B and C (0, 1 and 2).
    with_dialog(cx, |d, _, dcx| d.after_keyword_edit(dcx));
    assert_eq!(
        keyword_select::sets_of(&sheet_values(cx))[..4],
        [-1, 0, 1, -1],
        "A is gone, B and C moved up"
    );
    cancel(cx);
}

#[gpui::test]
fn a_choice_made_before_the_deletion_is_saved_after_it(cx: &mut TestAppContext) {
    // Slot 1 holds A. The user chooses B for it (not saved), then deletes A in the common settings,
    // which makes slot 1 none in the file. The choice of the user must still be saved.
    let ini = color_page(cx, &with_three_sets());
    with_dialog(cx, |d, _, dcx| d.pick_first_set(1, dcx));
    crate::common_settings::forget_keyword_renumbering();
    std::fs::write(&ini.path, with_bom(WITHOUT_A)).unwrap();
    delete_a();
    with_dialog(cx, |d, _, dcx| d.after_keyword_edit(dcx));
    ok(cx);
    let ints = ints(&ini, 0);
    assert_eq!((ints[4], ints[5]), (0, 0), "B, B");
    assert_eq!(
        ini.get("Types(0)", "nKeywordSelect3").as_deref(),
        Some("1"),
        "the slot nobody changed has the number the keyword page wrote"
    );
}

#[gpui::test]
fn a_dialog_that_is_open_is_closed_when_the_sets_were_renumbered(cx: &mut TestAppContext) {
    let _ini = color_page(cx, &with_three_sets());
    let _dialog = with_dialog(cx, |d, _, dcx| d.keyword_select(dcx)).expect("opens");
    let handle = cx.update(|app| child::open_window(app)).unwrap();
    assert!(cx.update(|app| app.windows().contains(&handle)));
    crate::common_settings::forget_keyword_renumbering();
    with_dialog(cx, |d, _, dcx| d.after_keyword_edit(dcx));
    cx.run_until_parked();
    assert!(
        !cx.update(|app| app.windows().contains(&handle)),
        "its numbers would be old"
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
