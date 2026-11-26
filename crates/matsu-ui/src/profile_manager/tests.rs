//! Tests of the profile manager in GPUI's headless test windows.
//!
//! The tests that touch the disk use a temporary folder as the folder of `matsu_prof.ini` and of the
//! profiles (`Store::in_folder`): `create_makes_the_name_and_writes_the_file`,
//! `rename_moves_the_folder_and_keeps_the_star`, and the others that read or write `matsu_prof.ini`
//! or look at a folder. They name no path of the system and no line ending.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use gpui::{TestAppContext, VisualTestContext};
use matsu_config::profile_manager::{self, ProfileSettings};

use super::startup::{Picked, parse_report, report};
use super::{ProfileManagerDialog, Store, current, in_use, open};

type Answers = Rc<RefCell<Vec<Option<String>>>>;

fn settings(names: &[&str], default_index: i32) -> ProfileSettings {
    ProfileSettings {
        profiles: names.iter().map(|n| (*n).to_owned()).collect(),
        default_index,
        ..ProfileSettings::default()
    }
}

fn file_of(dir: &Path) -> std::path::PathBuf {
    dir.join(profile_manager::FILE_NAME)
}

/// Opens the dialog on a folder that has `matsu_prof.ini` with `start` (no file for `None`).
fn open_dialog(cx: &mut TestAppContext, dir: &Path, start: Option<&ProfileSettings>) -> Answers {
    cx.update(gpui_component::init);
    if let Some(start) = start {
        profile_manager::write(&file_of(dir), start).unwrap();
    }
    let answers: Answers = Rc::default();
    let sink = answers.clone();
    let store = Store::in_folder(dir);
    cx.update(|app| {
        open(
            app,
            store,
            Box::new(move |_, chosen| sink.borrow_mut().push(chosen)),
        )
    });
    cx.run_until_parked();
    answers
}

fn with_dialog<R>(
    cx: &mut TestAppContext,
    f: impl FnOnce(
        &mut ProfileManagerDialog,
        &mut gpui::Window,
        &mut gpui::Context<ProfileManagerDialog>,
    ) -> R,
) -> R {
    let (dialog, handle) = cx.update(|app| current(app)).expect("the dialog is open");
    let result = handle
        .update(cx, |_, window, app| {
            dialog.update(app, |dialog, dcx| f(dialog, window, dcx))
        })
        .unwrap();
    cx.run_until_parked();
    result
}

/// What the field of the name panel holds.
fn entry_text(d: &ProfileManagerDialog, cx: &gpui::App) -> String {
    d.entry
        .as_ref()
        .map(|entry| entry.input.read(cx).value().to_string())
        .unwrap_or_default()
}

fn visual(cx: &mut TestAppContext) -> &mut VisualTestContext {
    let (_, handle) = cx.update(|app| current(app)).unwrap();
    VisualTestContext::from_window(handle, cx).into_mut()
}

fn keys(cx: &mut TestAppContext, keys: &str) {
    visual(cx).simulate_keystrokes(keys);
    cx.run_until_parked();
}

fn is_open(cx: &mut TestAppContext) -> bool {
    cx.update(|app| current(app)).is_some()
}

fn rows(cx: &mut TestAppContext) -> Vec<String> {
    with_dialog(cx, |d, _, _| {
        (0..d.settings().rows())
            .map(|row| d.settings().row_label(row).to_owned())
            .collect()
    })
}

fn selected(cx: &mut TestAppContext) -> usize {
    with_dialog(cx, |d, _, _| d.selected())
}

fn on_disk(dir: &Path) -> Option<ProfileSettings> {
    profile_manager::read(&file_of(dir))
}

/// Whether a box with an OK is waiting; answers it.
fn error_box(cx: &mut TestAppContext) -> bool {
    let shown = visual(cx).has_pending_prompt();
    if shown {
        visual(cx).simulate_prompt_answer("OK");
    }
    cx.run_until_parked();
    shown
}

#[gpui::test]
fn the_dialog_shows_the_file_and_chooses_the_default_row(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&["保存用", "work"], 2)));
    assert_eq!(rows(cx), ["(default)", "保存用", "work"]);
    // `SetData`: the row that starts is chosen
    assert_eq!(selected(cx), 2);
}

#[gpui::test]
fn without_a_file_the_list_is_the_default_row_with_the_star(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let _a = open_dialog(cx, dir.path(), None);
    assert_eq!(rows(cx), ["(default)"]);
    assert_eq!(selected(cx), 0);
    with_dialog(cx, |d, _, _| assert!(d.settings().is_default_row(0)));
}

#[gpui::test]
fn a_default_of_none_chooses_the_first_row(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&["a"], -1)));
    assert_eq!(selected(cx), 0);
    with_dialog(cx, |d, _, _| assert!(!d.clear_usable()));
}

#[gpui::test]
fn create_makes_the_name_and_writes_the_file(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&["a"], 1)));
    with_dialog(cx, |d, window, dcx| d.create("執筆用", window, dcx));
    assert!(!error_box(cx));
    assert_eq!(rows(cx), ["(default)", "a", "執筆用"]);
    // the chosen row stays, as in `CreateProf`
    assert_eq!(selected(cx), 1);
    let file = on_disk(dir.path()).unwrap();
    assert_eq!(file.profiles, ["a", "執筆用"]);
    assert_eq!(file.default_index, 1);
}

#[gpui::test]
fn a_bad_name_is_told_and_changes_nothing(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&["Work"], 1)));
    for bad in ["a/b", "x:y", "..", "work", "(default)"] {
        with_dialog(cx, |d, window, dcx| d.create(bad, window, dcx));
        assert!(error_box(cx), "{bad:?}");
    }
    // nothing typed: nothing is said
    with_dialog(cx, |d, window, dcx| d.create("", window, dcx));
    assert!(!error_box(cx));
    assert_eq!(rows(cx), ["(default)", "Work"]);
    assert_eq!(on_disk(dir.path()).unwrap().profiles, ["Work"]);
}

#[gpui::test]
fn a_file_with_the_name_stops_create_but_a_folder_does_not(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("taken"), b"x").unwrap();
    std::fs::create_dir(dir.path().join("used")).unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&[], 0)));
    with_dialog(cx, |d, window, dcx| d.create("taken", window, dcx));
    assert!(error_box(cx));
    assert_eq!(rows(cx), ["(default)"]);
    // the folder of a profile that was deleted from the list can be made a profile again
    with_dialog(cx, |d, window, dcx| d.create("used", window, dcx));
    assert!(!error_box(cx));
    assert_eq!(rows(cx), ["(default)", "used"]);
}

#[gpui::test]
fn rename_moves_the_folder_and_keeps_the_star(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("a")).unwrap();
    std::fs::write(dir.path().join("a").join("matsu.ini"), b"x").unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&["a", "b"], 1)));
    with_dialog(cx, |d, window, dcx| {
        d.select(1, dcx);
        d.rename(1, "新", window, dcx);
    });
    assert!(!error_box(cx));
    assert_eq!(rows(cx), ["(default)", "新", "b"]);
    assert!(!dir.path().join("a").exists());
    assert!(dir.path().join("新").join("matsu.ini").is_file());
    let file = on_disk(dir.path()).unwrap();
    assert_eq!(file.profiles, ["新", "b"]);
    assert_eq!(file.default_index, 1);
}

#[gpui::test]
fn rename_without_a_folder_only_changes_the_list(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&["a"], -1)));
    with_dialog(cx, |d, window, dcx| d.rename(1, "b", window, dcx));
    assert!(!error_box(cx));
    assert_eq!(on_disk(dir.path()).unwrap().profiles, ["b"]);
    assert!(!dir.path().join("b").exists());
}

#[gpui::test]
fn rename_checks_the_name_and_says_nothing_for_the_same_name(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&["a", "b"], -1)));
    // unchanged, empty: nothing happens, nothing is said
    with_dialog(cx, |d, window, dcx| {
        d.rename(1, "a", window, dcx);
        d.rename(1, "", window, dcx);
    });
    assert!(!error_box(cx));
    // a name of another row (any case), an invalid one
    for bad in ["B", "a*", "(default)"] {
        with_dialog(cx, |d, window, dcx| d.rename(1, bad, window, dcx));
        assert!(error_box(cx), "{bad:?}");
    }
    // the case of the row's own name may change
    with_dialog(cx, |d, window, dcx| d.rename(1, "A", window, dcx));
    assert!(!error_box(cx));
    assert_eq!(rows(cx), ["(default)", "A", "b"]);
}

#[gpui::test]
fn rename_that_cannot_move_the_folder_is_told(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("a")).unwrap();
    // a folder of the new name is there already: `MoveFile` does not replace it
    std::fs::create_dir(dir.path().join("b")).unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&["a"], -1)));
    with_dialog(cx, |d, window, dcx| d.rename(1, "b", window, dcx));
    assert!(error_box(cx));
    assert_eq!(rows(cx), ["(default)", "a"]);
    assert!(dir.path().join("a").is_dir());
    // nothing was written
    assert_eq!(on_disk(dir.path()).unwrap().profiles, ["a"]);
}

#[gpui::test]
fn rename_of_a_name_that_is_a_file_checks_the_new_name_is_free(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    // the old name is a file (not a profile folder) and so is the new one
    std::fs::write(dir.path().join("a"), b"x").unwrap();
    std::fs::write(dir.path().join("b"), b"x").unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&["a"], -1)));
    with_dialog(cx, |d, window, dcx| d.rename(1, "b", window, dcx));
    assert!(error_box(cx));
    assert_eq!(rows(cx), ["(default)", "a"]);
}

#[gpui::test]
fn delete_leaves_the_folder_and_the_star_goes_with_its_row(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("a")).unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&["a", "b", "c"], 3)));
    with_dialog(cx, |d, window, dcx| {
        d.select(1, dcx);
        d.delete(window, dcx);
    });
    assert_eq!(rows(cx), ["(default)", "b", "c"]);
    assert_eq!(selected(cx), 1);
    // the folder of the profile stays
    assert!(dir.path().join("a").is_dir());
    let file = on_disk(dir.path()).unwrap();
    assert_eq!(file.profiles, ["b", "c"]);
    assert_eq!(file.default_index, 2);
    // the row with the star goes: no default; the last row goes: the one above is chosen
    with_dialog(cx, |d, window, dcx| {
        d.select(2, dcx);
        d.delete(window, dcx);
    });
    assert_eq!(selected(cx), 1);
    assert_eq!(on_disk(dir.path()).unwrap().default_index, -1);
    // 「(default)」 cannot be deleted
    with_dialog(cx, |d, window, dcx| {
        d.select(0, dcx);
        d.delete(window, dcx);
    });
    assert_eq!(rows(cx), ["(default)", "b"]);
}

#[gpui::test]
fn the_default_can_be_set_and_cleared(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&["a", "b"], -1)));
    with_dialog(cx, |d, window, dcx| {
        assert!(!d.clear_usable());
        d.select(2, dcx);
        d.make_default(window, dcx);
        assert!(d.clear_usable());
    });
    assert_eq!(on_disk(dir.path()).unwrap().default_index, 2);
    with_dialog(cx, |d, window, dcx| d.clear_default(window, dcx));
    assert_eq!(on_disk(dir.path()).unwrap().default_index, -1);
}

#[gpui::test]
fn start_answers_the_chosen_profile_and_closes(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let answers = open_dialog(cx, dir.path(), Some(&settings(&["a", "b"], -1)));
    with_dialog(cx, |d, window, dcx| {
        d.select(2, dcx);
        d.start(window, dcx);
    });
    assert!(!is_open(cx));
    assert_eq!(answers.borrow().as_slice(), [Some("b".to_owned())]);
    // the check box was off: the default stays as it was
    assert_eq!(on_disk(dir.path()).unwrap().default_index, -1);
}

#[gpui::test]
fn start_of_the_default_row_answers_the_empty_name(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let answers = open_dialog(cx, dir.path(), None);
    with_dialog(cx, |d, window, dcx| d.start(window, dcx));
    assert_eq!(answers.borrow().as_slice(), [Some(String::new())]);
    // nothing was changed, so no file is made
    assert!(on_disk(dir.path()).is_none());
}

#[gpui::test]
fn start_with_the_check_box_makes_the_row_the_default(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let answers = open_dialog(cx, dir.path(), Some(&settings(&["a", "b"], 1)));
    with_dialog(cx, |d, window, dcx| {
        d.select(2, dcx);
        d.set_default_start(true, dcx);
        d.start(window, dcx);
    });
    assert_eq!(answers.borrow().as_slice(), [Some("b".to_owned())]);
    let file = on_disk(dir.path()).unwrap();
    assert_eq!(file.default_index, 2);
    assert!(file.default_select);
}

#[gpui::test]
fn close_answers_nothing_and_keeps_only_a_changed_check_box(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let answers = open_dialog(cx, dir.path(), Some(&settings(&["a"], 1)));
    with_dialog(cx, |d, window, dcx| {
        d.select(0, dcx);
        d.set_default_start(true, dcx);
        d.close(window, dcx);
    });
    assert!(!is_open(cx));
    assert_eq!(answers.borrow().as_slice(), [None]);
    let file = on_disk(dir.path()).unwrap();
    assert!(file.default_select);
    // the row was not made the default by closing
    assert_eq!(file.default_index, 1);
}

#[gpui::test]
fn close_without_a_change_writes_nothing(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let answers = open_dialog(cx, dir.path(), None);
    with_dialog(cx, |d, window, dcx| d.close(window, dcx));
    assert_eq!(answers.borrow().as_slice(), [None]);
    assert!(on_disk(dir.path()).is_none());
}

#[gpui::test]
fn a_second_open_does_not_make_a_second_window(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let first = open_dialog(cx, dir.path(), None);
    let second = open_dialog(cx, dir.path(), None);
    assert_eq!(cx.update(|app| app.windows().len()), 1);
    with_dialog(cx, |d, window, dcx| d.close(window, dcx));
    assert_eq!(first.borrow().len(), 1);
    assert!(second.borrow().is_empty(), "the second one was dropped");
}

#[gpui::test]
fn the_arrow_keys_move_the_choice_and_enter_starts(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let answers = open_dialog(cx, dir.path(), Some(&settings(&["a", "b"], 0)));
    keys(cx, "down down down");
    assert_eq!(selected(cx), 2, "stops at the last row");
    keys(cx, "up");
    assert_eq!(selected(cx), 1);
    keys(cx, "home");
    assert_eq!(selected(cx), 0);
    keys(cx, "end");
    assert_eq!(selected(cx), 2);
    keys(cx, "enter");
    assert!(!is_open(cx));
    assert_eq!(answers.borrow().as_slice(), [Some("b".to_owned())]);
}

#[gpui::test]
fn escape_closes_like_the_button(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let answers = open_dialog(cx, dir.path(), None);
    keys(cx, "escape");
    assert!(!is_open(cx));
    assert_eq!(answers.borrow().as_slice(), [None]);
}

#[gpui::test]
fn the_access_keys_of_the_labels_work(cx: &mut TestAppContext) {
    // `IDD_PROFILEMGR`: N 新規作成, R 名前変更, D 削除, F デフォルト設定, C デフォルト解除,
    // E the check box, S 起動, X 閉じる
    let dir = tempfile::tempdir().unwrap();
    let answers = open_dialog(cx, dir.path(), Some(&settings(&["a", "b"], -1)));
    keys(cx, "alt-e");
    with_dialog(cx, |d, _, _| assert!(d.settings().default_select));
    keys(cx, "alt-e");
    with_dialog(cx, |d, _, _| assert!(!d.settings().default_select));
    // 名前変更, 削除 and デフォルト解除 cannot be used here: their letters do nothing
    keys(cx, "alt-r alt-d alt-c");
    assert_eq!(rows(cx), ["(default)", "a", "b"]);
    with_dialog(cx, |d, _, _| assert!(!d.asking()));
    keys(cx, "down alt-f");
    assert_eq!(on_disk(dir.path()).unwrap().default_index, 1);
    keys(cx, "alt-c");
    assert_eq!(on_disk(dir.path()).unwrap().default_index, -1);
    keys(cx, "alt-d");
    assert_eq!(rows(cx), ["(default)", "b"]);
    keys(cx, "alt-x");
    assert!(!is_open(cx));
    assert_eq!(answers.borrow().as_slice(), [None]);
}

#[gpui::test]
fn alt_s_starts(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let answers = open_dialog(cx, dir.path(), Some(&settings(&["a"], 1)));
    keys(cx, "alt-s");
    assert_eq!(answers.borrow().as_slice(), [Some("a".to_owned())]);
}

#[gpui::test]
fn the_name_panel_takes_a_typed_name_with_enter(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let answers = open_dialog(cx, dir.path(), Some(&settings(&["a"], -1)));
    keys(cx, "alt-n");
    with_dialog(cx, |d, _, _| assert!(d.asking()));
    visual(cx).simulate_input("新しい");
    keys(cx, "enter");
    cx.run_until_parked();
    with_dialog(cx, |d, _, _| assert!(!d.asking()));
    assert_eq!(rows(cx), ["(default)", "a", "新しい"]);
    assert_eq!(on_disk(dir.path()).unwrap().profiles, ["a", "新しい"]);
    // Enter in the field of the panel is its OK; it did not start the profile
    assert!(is_open(cx));
    assert!(answers.borrow().is_empty());
}

#[gpui::test]
fn escape_in_the_name_panel_closes_only_the_panel(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let answers = open_dialog(cx, dir.path(), Some(&settings(&["a"], -1)));
    keys(cx, "alt-n");
    visual(cx).simulate_input("x");
    keys(cx, "escape");
    with_dialog(cx, |d, _, _| assert!(!d.asking()));
    assert!(is_open(cx));
    assert!(answers.borrow().is_empty());
    assert_eq!(rows(cx), ["(default)", "a"]);
    // the keys of the list work again
    keys(cx, "down");
    assert_eq!(selected(cx), 1);
}

#[gpui::test]
fn the_rename_panel_starts_with_the_name(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&["元の名前"], 1)));
    keys(cx, "alt-r");
    with_dialog(cx, |d, _, dcx| assert_eq!(entry_text(d, dcx), "元の名前"));
    visual(cx).simulate_input("2");
    keys(cx, "enter");
    cx.run_until_parked();
    // the field starts selected, so the typed text replaced the name
    assert_eq!(rows(cx), ["(default)", "2"]);
}

#[gpui::test]
fn the_name_panel_limits_the_length(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let _a = open_dialog(cx, dir.path(), None);
    keys(cx, "alt-n");
    // the field starts empty and focused: typed text goes in, up to the length that Sakura's field takes
    visual(cx).simulate_input(&"あ".repeat(300));
    with_dialog(cx, |d, _, dcx| {
        assert_eq!(entry_text(d, dcx).encode_utf16().count(), 259);
    });
}

/// Changes the file as another process (a second dialog, the picker of a start-up) would.
fn change_elsewhere(dir: &Path, change: impl FnOnce(&mut ProfileSettings)) {
    let mut list = on_disk(dir).unwrap_or_default();
    change(&mut list);
    profile_manager::write(&file_of(dir), &list).unwrap();
}

#[gpui::test]
fn a_change_of_another_window_is_not_written_over(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&["a"], 1)));
    // another process makes "work" while this window is open with the old list
    change_elsewhere(dir.path(), |list| {
        list.add("work");
    });
    // the check box and 閉じる write the file: "work" stays
    with_dialog(cx, |d, window, dcx| {
        d.set_default_start(true, dcx);
        d.close(window, dcx);
    });
    let file = on_disk(dir.path()).unwrap();
    assert_eq!(file.profiles, ["a", "work"]);
    assert!(file.default_select);
}

#[gpui::test]
fn every_action_starts_from_the_file_and_shows_the_result(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&["a", "b"], -1)));
    change_elsewhere(dir.path(), |list| {
        list.add("work");
    });
    with_dialog(cx, |d, window, dcx| d.create("new", window, dcx));
    assert_eq!(
        on_disk(dir.path()).unwrap().profiles,
        ["a", "b", "work", "new"]
    );
    assert_eq!(rows(cx), ["(default)", "a", "b", "work", "new"]);
    // a profile that another window deleted is not found by the action: nothing is written
    change_elsewhere(dir.path(), |list| {
        list.remove(2);
    });
    with_dialog(cx, |d, window, dcx| {
        d.select(2, dcx);
        d.make_default(window, dcx);
    });
    assert!(!error_box(cx));
    assert_eq!(on_disk(dir.path()).unwrap().profiles, ["a", "work", "new"]);
    assert_eq!(on_disk(dir.path()).unwrap().default_index, -1);
    assert_eq!(rows(cx), ["(default)", "a", "work", "new"]);
}

/// Makes the file impossible to write: a folder has its name.
fn break_the_file(dir: &Path) {
    let file = file_of(dir);
    let _ = std::fs::remove_file(&file);
    std::fs::create_dir(&file).unwrap();
}

#[gpui::test]
fn an_action_that_cannot_write_says_so_and_changes_nothing(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("a")).unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&["a", "b"], 1)));
    break_the_file(dir.path());
    with_dialog(cx, |d, window, dcx| {
        d.create("c", window, dcx);
    });
    assert!(error_box(cx), "create");
    with_dialog(cx, |d, window, dcx| {
        d.select(2, dcx);
        d.delete(window, dcx);
    });
    assert!(error_box(cx), "delete");
    with_dialog(cx, |d, window, dcx| d.make_default(window, dcx));
    assert!(error_box(cx), "make_default");
    with_dialog(cx, |d, window, dcx| d.clear_default(window, dcx));
    assert!(error_box(cx), "clear_default");
    // the folder is not moved when the list cannot be written
    with_dialog(cx, |d, window, dcx| d.rename(1, "z", window, dcx));
    assert!(error_box(cx), "rename");
    assert!(dir.path().join("a").is_dir());
    assert!(!dir.path().join("z").exists());
}

#[gpui::test]
fn a_profile_in_use_cannot_be_renamed_or_deleted(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&["a", "b"], -1)));
    // a second holder in this process: the mark of the profile "a"
    let held = in_use::hold(&dir.path().join("a")).unwrap();
    with_dialog(cx, |d, window, dcx| d.rename(1, "z", window, dcx));
    assert!(error_box(cx), "rename");
    with_dialog(cx, |d, window, dcx| {
        d.select(1, dcx);
        d.delete(window, dcx);
    });
    assert!(error_box(cx), "delete");
    assert_eq!(rows(cx), ["(default)", "a", "b"]);
    assert!(dir.path().join("a").is_dir());
    // "b" is not in use
    with_dialog(cx, |d, window, dcx| d.rename(2, "c", window, dcx));
    assert!(!error_box(cx));
    // the mark goes with the holder
    drop(held);
    with_dialog(cx, |d, window, dcx| d.rename(1, "z", window, dcx));
    assert!(!error_box(cx));
    assert!(dir.path().join("z").is_dir());
    assert_eq!(rows(cx), ["(default)", "z", "c"]);
}

#[test]
fn the_mark_is_held_by_one_and_goes_with_the_lock() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("p");
    // a folder that was never marked is free, and the file of a holder that is gone holds nothing
    assert!(!in_use::is_held(&folder));
    let first = in_use::hold(&folder).unwrap();
    assert!(in_use::is_held(&folder));
    assert!(in_use::hold(&folder).is_err(), "a second holder is refused");
    drop(first);
    assert!(!in_use::is_held(&folder), "the lock went with its holder");
    assert!(in_use::hold(&folder).is_ok());
}

#[gpui::test]
fn a_case_only_rename_moves_the_folder(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("work")).unwrap();
    std::fs::write(dir.path().join("work").join("matsu.ini"), b"x").unwrap();
    let _a = open_dialog(cx, dir.path(), Some(&settings(&["work"], -1)));
    with_dialog(cx, |d, window, dcx| d.rename(1, "Work", window, dcx));
    assert!(!error_box(cx));
    assert_eq!(rows(cx), ["(default)", "Work"]);
    // the folder has the new case, with its content (on a file system that ignores case, `exists`
    // cannot tell: the names that the folder lists can)
    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.contains(&"Work".to_owned()), "{names:?}");
    assert!(!names.contains(&"work".to_owned()), "{names:?}");
    assert!(dir.path().join("Work").join("matsu.ini").is_file());
    // and nothing was left under the temporary name
    assert!(
        names.iter().all(|n| !n.contains(".matsu-rename-")),
        "{names:?}"
    );
}

#[test]
fn a_case_only_rename_of_one_folder_is_not_taken_for_a_clash() {
    // On a file system that ignores case `work` and `Work` are one folder; on another they are two.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("work")).unwrap();
    let store = Store::in_folder(dir.path());
    let plan = store.plan_rename("work", "Work");
    let ignores_case = dir.path().join("WORK").exists();
    if ignores_case {
        assert!(matches!(plan, Ok(Some(_))), "the same folder: {plan:?}");
    } else {
        // the new name is free: a plain move
        assert!(matches!(plan, Ok(Some(_))), "{plan:?}");
        std::fs::create_dir(dir.path().join("Work")).unwrap();
        assert_eq!(
            store.plan_rename("work", "Work").unwrap_err(),
            super::store::MoveError::Exists
        );
    }
}

#[test]
fn the_picking_process_reports_its_answer_on_one_line() {
    for picked in [
        Picked::Profile("執筆用".to_owned()),
        Picked::Profile(String::new()),
        Picked::Cancelled,
    ] {
        assert_eq!(parse_report(report(&picked).as_bytes()), Some(picked));
    }
    // noise before the answer is skipped; no answer is none
    assert_eq!(
        parse_report(b"log line\nprofile:a b\n"),
        Some(Picked::Profile("a b".to_owned()))
    );
    assert_eq!(parse_report(b""), None);
    assert_eq!(parse_report(b"crashed\n"), None);
}
