//! Alt+letter in the tag jump list and in 「タグファイルの作成」 (`IsDialogMessage`, `IDD_TAGJUMPLIST`,
//! `IDD_TAG_MAKE`): the letters of the labels act while the focus is in a box, where the letter is not
//! typed.

use std::time::Duration;

use gpui::TestAppContext;
use matsu_commands::command::Arg;
use matsu_core::buffer::Pos;

use super::super::list::{SHORT_DELAY, TYPING_DELAY};
use super::{Win, make_dialog_open, newest, open, project, prompt, settle, use_ctags_in, windows};

#[gpui::test]
fn the_check_boxes_of_the_keyword_form_change_what_matches(cx: &mut TestAppContext) {
    let (_dir, main) = project("x\n");
    let w = open(cx, &main);
    w.run_with(cx, "F_TAGJUMP_KEYWORD", &[Arg::Text("OBA".into())]);
    settle(cx, SHORT_DELAY);
    assert!(w.list(cx).unwrap().0.is_empty());
    // 大文字小文字の同一視(F) and 文字列の途中にマッチ(A), from the box of the keyword
    w.cx(cx).simulate_keystrokes("alt-f");
    settle(cx, SHORT_DELAY);
    assert!(w.list(cx).unwrap().0.is_empty(), "not a beginning");
    w.cx(cx).simulate_keystrokes("alt-a");
    settle(cx, SHORT_DELAY);
    assert_eq!(w.list(cx).unwrap().0, ["foobar"]);
    // the letters were not typed into the keyword, and Shift does not matter
    w.cx(cx).simulate_keystrokes("alt-shift-f");
    settle(cx, SHORT_DELAY);
    assert!(w.list(cx).unwrap().0.is_empty());
}

#[gpui::test]
fn alt_k_goes_to_the_keyword_and_alt_j_jumps_and_alt_x_cancels(cx: &mut TestAppContext) {
    let (_dir, main) = project("x\n");
    let w = open(cx, &main);
    w.run_with(cx, "F_TAGJUMP_KEYWORD", &[Arg::Text("zzz".into())]);
    settle(cx, SHORT_DELAY);
    // キーワード(K) selects the text of the box, so what is typed replaces it
    w.cx(cx).simulate_keystrokes("alt-k");
    w.cx(cx).simulate_input("foob");
    settle(cx, TYPING_DELAY);
    assert_eq!(w.list(cx).unwrap().0, ["foobar"]);
    // ジャンプ(J)
    w.cx(cx).simulate_keystrokes("alt-j");
    cx.run_until_parked();
    assert!(w.list(cx).is_none());
    assert_eq!(windows(cx), 2);
    assert_eq!(newest(cx).caret(cx), Pos::new(4, 0));
    // キャンセル(X)
    let w = open(cx, &main);
    w.run_with(cx, "F_TAGJUMP_KEYWORD", &[Arg::Text("foo".into())]);
    settle(cx, SHORT_DELAY);
    w.cx(cx).simulate_keystrokes("alt-x");
    cx.run_until_parked();
    assert!(w.list(cx).is_none());
}

#[gpui::test]
fn the_page_buttons_have_their_letters_only_while_they_work(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mut tags = String::from("!_TAG_FILE_SORTED\t0\n");
    for i in 0..120 {
        tags += &format!("item{i:03}\tlib.c\t{};\"\tf\n", i + 1);
    }
    std::fs::write(dir.path().join("tags"), tags).unwrap();
    std::fs::write(dir.path().join("lib.c"), "x\n".repeat(200)).unwrap();
    let main = dir.path().join("main.c");
    std::fs::write(&main, "x\n").unwrap();
    let w = open(cx, &main);
    w.run_with(cx, "F_TAGJUMP_KEYWORD", &[Arg::Text("item".into())]);
    settle(cx, SHORT_DELAY);
    let top = |w: &Win, cx: &mut TestAppContext| w.list(cx).unwrap().3;
    // 前へ(P) does not work on the first page
    w.cx(cx).simulate_keystrokes("alt-p");
    cx.run_until_parked();
    assert_eq!(top(&w, cx), 0);
    // 次へ(N)
    w.cx(cx).simulate_keystrokes("alt-n");
    cx.run_until_parked();
    assert_eq!(top(&w, cx), 50);
    w.cx(cx).simulate_keystrokes("alt-p");
    cx.run_until_parked();
    assert_eq!(top(&w, cx), 0);
    settle(cx, Duration::from_millis(1));
}

#[gpui::test]
fn the_direct_form_has_no_keyword_and_its_letters_are_not_taken(cx: &mut TestAppContext) {
    let (_dir, main) = project("dup\n");
    let w = open(cx, &main);
    w.go_to(cx, Pos::new(0, 1));
    w.run(cx, "F_DIRECT_TAGJUMP");
    cx.run_until_parked();
    assert!(w.list(cx).is_some(), "two hits open the list");
    // キーワード(K) and the check boxes are not in this form
    for letter in ["k", "f", "a"] {
        w.cx(cx).simulate_keystrokes(&format!("alt-{letter}"));
    }
    assert!(w.list(cx).is_some());
    w.cx(cx).simulate_keystrokes("alt-x");
    cx.run_until_parked();
    assert!(w.list(cx).is_none());
}

#[gpui::test]
fn the_dialog_of_the_tags_file_has_o_b_and_x(cx: &mut TestAppContext) {
    let (dir, main) = project("x\n");
    std::fs::remove_file(dir.path().join("tags")).unwrap();
    let w = open(cx, &main);
    use_ctags_in(cx, Vec::new());
    // キャンセル(X)
    w.run(cx, "F_TAGS_MAKE");
    assert!(make_dialog_open(&w, cx));
    w.cx(cx).simulate_keystrokes("alt-x");
    cx.run_until_parked();
    assert!(!make_dialog_open(&w, cx));
    // 上(B): the folder above the one in the box (the letter is not typed into the box)
    w.run(cx, "F_TAGS_MAKE");
    w.cx(cx).simulate_keystrokes("alt-b");
    assert!(make_dialog_open(&w, cx));
    // 作成(O): there is no ctags, and the refusal is a box
    cx.executor().allow_parking();
    w.cx(cx).simulate_keystrokes("alt-o");
    for _ in 0..6000 {
        cx.run_until_parked();
        if prompt(&w, cx).is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let message = prompt(&w, cx).expect("the refusal is shown");
    assert!(message.contains("ctags"), "{message}");
    w.cx(cx).simulate_prompt_answer("OK");
    cx.run_until_parked();
    assert!(!make_dialog_open(&w, cx));
}

#[gpui::test]
fn the_help_of_the_tag_jump_list_and_of_the_tags_file_dialog_is_alt_h_and_f1(
    cx: &mut TestAppContext,
) {
    use crate::support::help::testing::{clear, page_of_function, take};
    let (dir, main) = project("x\n");
    let w = open(cx, &main);
    // `CDlgTagJumpList`: `MyWinHelp( ..., F_TAGJUMP_LIST )`; the dialog stays open
    w.run_with(cx, "F_TAGJUMP_KEYWORD", &[Arg::Text("foo".into())]);
    settle(cx, SHORT_DELAY);
    clear();
    w.cx(cx).simulate_keystrokes("alt-h");
    assert_eq!(take(), [page_of_function("F_TAGJUMP_LIST")]);
    w.cx(cx).simulate_keystrokes("f1");
    assert_eq!(take(), [page_of_function("F_TAGJUMP_LIST")]);
    assert!(w.list(cx).is_some());
    w.cx(cx).simulate_keystrokes("alt-x");
    cx.run_until_parked();
    // `CDlgTagsMake`: `F_TAGS_MAKE`
    std::fs::remove_file(dir.path().join("tags")).unwrap();
    w.run(cx, "F_TAGS_MAKE");
    assert!(make_dialog_open(&w, cx));
    w.cx(cx).simulate_keystrokes("alt-h");
    assert_eq!(take(), [page_of_function("F_TAGS_MAKE")]);
    w.cx(cx).simulate_keystrokes("f1");
    assert_eq!(take(), [page_of_function("F_TAGS_MAKE")]);
    assert!(make_dialog_open(&w, cx));
    w.cx(cx).simulate_keystrokes("alt-x");
    cx.run_until_parked();
    assert!(!make_dialog_open(&w, cx));
}
