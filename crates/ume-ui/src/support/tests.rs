//! Tests of the support group in GPUI's headless test window: the keys and function codes reach
//! the commands, the completion list takes its keys, the command list runs a function, and the
//! version dialog opens and closes. The rules of the candidates are tested in
//! `ume_commands::text::complete`.
//!
//! Expectations cite `sakura_core/cmd/CViewCommander_Support.cpp`,
//! `sakura_core/view/CEditView_CmdHokan.cpp` and `sakura_core/env/CHokanMgr.cpp`.

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use ume_commands::sakura::FUNCTIONS;
use ume_core::buffer::{Document, Pos};
use ume_core::model::DocumentModel;

use crate::document::SharedDocument;
use crate::view::EditorView;

const F_HOKAN: u16 = 31430;
const F_MENU_ALLFUNC: u16 = 31445;
const F_ABOUT: u16 = 31455;

struct Fixture {
    document: Entity<SharedDocument>,
    view: Entity<EditorView>,
}

/// A view over `text` with the caret at the end of the text.
fn setup<'a>(cx: &'a mut TestAppContext, text: &str) -> (Fixture, &'a mut VisualTestContext) {
    cx.update(gpui_component::init);
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let shared = document.clone();
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    vcx.simulate_keystrokes("ctrl-end");
    (Fixture { document, view }, vcx)
}

impl Fixture {
    fn text(&self, cx: &mut VisualTestContext) -> String {
        self.document.read_with(cx, |shared, _| {
            let doc = shared.doc();
            doc.text_between(Pos::new(0, 0), Pos::new(doc.line_count(), 0))
                .unwrap()
        })
    }

    fn caret(&self, cx: &mut VisualTestContext) -> Pos {
        self.view.read_with(cx, |v, _| v.cursor.caret())
    }

    fn notice(&self, cx: &mut VisualTestContext) -> Option<String> {
        self.view.read_with(cx, |v, _| v.notice.clone())
    }

    fn run(&self, cx: &mut VisualTestContext, code: u16) {
        self.view
            .update_in(cx, |v, window, cx| v.run_function(code, &[], window, cx));
    }

    /// The candidates of the list and the chosen row, if the list is shown.
    fn list(&self, cx: &mut VisualTestContext) -> Option<(Vec<String>, usize)> {
        self.view.read_with(cx, |v, _| {
            v.support
                .completion
                .as_ref()
                .map(|c| (c.items.clone(), c.selected))
        })
    }

    fn command_list_open(&self, cx: &mut VisualTestContext) -> bool {
        self.view
            .read_with(cx, |v, _| v.support.command_list.is_some())
    }

    fn about_open(&self, cx: &mut VisualTestContext) -> bool {
        self.view.read_with(cx, |v, _| v.support.about.is_some())
    }
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

// ---- F_HOKAN ----

#[gpui::test]
fn the_function_codes_are_the_ones_of_the_table(_: &mut TestAppContext) {
    for (code, name) in [
        (F_HOKAN, "F_HOKAN"),
        (F_MENU_ALLFUNC, "F_MENU_ALLFUNC"),
        (F_ABOUT, "F_ABOUT"),
    ] {
        assert_eq!(
            FUNCTIONS.iter().find(|f| f.name == name).unwrap().code,
            code
        );
    }
}

#[gpui::test]
fn ctrl_space_shows_the_candidates_with_the_first_one_chosen(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "apple apricot avocado\nap");
    cx.simulate_keystrokes("ctrl-space");
    assert_eq!(f.list(cx), Some((strings(&["apple", "apricot"]), 0)));
    // showing the list changes nothing in the text
    assert_eq!(f.text(cx), "apple apricot avocado\nap");
}

#[gpui::test]
fn the_keys_choose_and_enter_inserts_the_word_in_place_of_the_key(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "apple apricot avocado\nap");
    cx.simulate_keystrokes("ctrl-space down");
    assert_eq!(f.list(cx).unwrap().1, 1);
    // Down at the last candidate stays; Up goes back
    cx.simulate_keystrokes("down");
    assert_eq!(f.list(cx).unwrap().1, 1);
    cx.simulate_keystrokes("up up");
    assert_eq!(f.list(cx).unwrap().1, 0);
    cx.simulate_keystrokes("end enter");
    // "ap" is part of the result: "ap" and "apricot" make "apricot", not "apapricot"
    assert_eq!(f.text(cx), "apple apricot avocado\napricot");
    assert_eq!(f.caret(cx), Pos::new(1, 7));
    assert_eq!(f.list(cx), None);
}

#[gpui::test]
fn right_inserts_the_chosen_word_too(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "apple apricot\nap");
    cx.simulate_keystrokes("ctrl-space right");
    assert_eq!(f.text(cx), "apple apricot\napple");
}

#[gpui::test]
fn tab_does_not_insert_with_sakuras_default_settings(cx: &mut TestAppContext) {
    // bHokanKey_TAB is off in `CShareData.cpp`, so Tab is a plain Tab: the list closes (the
    // command is not one that keeps it) and the key does what it does without the list
    let (f, cx) = setup(cx, "apple apricot\nap");
    cx.simulate_keystrokes("ctrl-space tab");
    assert_eq!(f.list(cx), None);
    assert_eq!(f.text(cx), "apple apricot\nap\t");
}

#[gpui::test]
fn the_insertion_is_one_undo_step(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "apple apricot\nap");
    cx.simulate_keystrokes("ctrl-space enter");
    assert_eq!(f.text(cx), "apple apricot\napple");
    cx.simulate_keystrokes("ctrl-z");
    assert_eq!(f.text(cx), "apple apricot\nap");
    assert_eq!(f.caret(cx), Pos::new(1, 2));
}

#[gpui::test]
fn escape_closes_the_list_and_leaves_the_text(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "apple apricot\nap");
    cx.simulate_keystrokes("ctrl-space escape");
    assert_eq!(f.list(cx), None);
    assert_eq!(f.text(cx), "apple apricot\nap");
}

#[gpui::test]
fn one_candidate_is_inserted_without_a_list(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "hello world\nhe");
    cx.simulate_keystrokes("ctrl-space");
    assert_eq!(f.list(cx), None);
    assert_eq!(f.text(cx), "hello world\nhello");
    assert_eq!(f.caret(cx), Pos::new(1, 5));
}

#[gpui::test]
fn no_candidate_shows_nothing_and_changes_nothing(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "hello\nzz");
    cx.simulate_keystrokes("ctrl-space");
    assert_eq!(f.list(cx), None);
    assert_eq!(f.text(cx), "hello\nzz");
}

#[gpui::test]
fn without_a_word_before_the_caret_the_status_bar_says_so(cx: &mut TestAppContext) {
    // the status message of Command_HOKAN (STR_SUPPORT_*): the caret is after a space
    let (f, cx) = setup(cx, "hello ");
    cx.simulate_keystrokes("ctrl-space");
    assert_eq!(f.list(cx), None);
    assert_eq!(f.notice(cx).as_deref(), Some("補完対象がありません"));
    assert_eq!(f.text(cx), "hello ");
}

#[gpui::test]
fn at_the_start_of_a_line_there_is_no_word_either(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "hello\n");
    cx.simulate_keystrokes("ctrl-space");
    assert_eq!(f.list(cx), None);
    assert_eq!(f.notice(cx).as_deref(), Some("補完対象がありません"));
}

#[gpui::test]
fn typing_narrows_the_list_and_backspace_widens_it(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "apple apricot avocado\nap");
    cx.simulate_keystrokes("ctrl-space");
    cx.simulate_input("r");
    assert_eq!(f.list(cx), Some((strings(&["apricot"]), 0)));
    cx.simulate_keystrokes("backspace");
    assert_eq!(f.list(cx), Some((strings(&["apple", "apricot"]), 0)));
    // no candidate left: the list closes
    cx.simulate_input("z");
    assert_eq!(f.list(cx), None);
}

#[gpui::test]
fn the_list_closes_when_the_word_is_gone(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "apple apricot\nap");
    cx.simulate_keystrokes("ctrl-space");
    cx.simulate_input(" ");
    assert_eq!(f.list(cx), None);
}

#[gpui::test]
fn another_command_closes_the_list(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "apple apricot\nap");
    cx.simulate_keystrokes("ctrl-space");
    assert!(f.list(cx).is_some());
    cx.simulate_keystrokes("ctrl-a");
    assert_eq!(f.list(cx), None);
    assert_eq!(f.text(cx), "apple apricot\nap");
}

#[gpui::test]
fn left_closes_the_list_and_moves_the_caret(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "apple apricot\nap");
    cx.simulate_keystrokes("ctrl-space left");
    assert_eq!(f.list(cx), None);
    assert_eq!(f.caret(cx), Pos::new(1, 1));
}

#[gpui::test]
fn japanese_words_are_completed(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "補完候補 補完機能 入力\n補完");
    cx.simulate_keystrokes("ctrl-space");
    assert_eq!(f.list(cx), Some((strings(&["補完候補", "補完機能"]), 0)));
    cx.simulate_keystrokes("down enter");
    assert_eq!(f.text(cx), "補完候補 補完機能 入力\n補完機能");
}

#[gpui::test]
fn the_function_runs_from_a_menu_too(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "apple apricot\nap");
    f.run(cx, F_HOKAN);
    assert_eq!(f.list(cx), Some((strings(&["apple", "apricot"]), 0)));
    // running it again makes the list again (the list stays for F_HOKAN)
    f.run(cx, F_HOKAN);
    assert_eq!(f.list(cx), Some((strings(&["apple", "apricot"]), 0)));
}

// ---- F_MENU_ALLFUNC ----

#[gpui::test]
fn shift_f1_opens_the_command_list_and_escape_closes_it(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "text");
    cx.simulate_keystrokes("shift-f1");
    assert!(f.command_list_open(cx));
    cx.simulate_keystrokes("escape");
    assert!(!f.command_list_open(cx));
}

#[gpui::test]
fn the_command_list_runs_the_chosen_function(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_MENU_ALLFUNC);
    assert!(f.command_list_open(cx));
    // the groups in Sakura's order: file, edit, cursor movement, ...; "選択系" is the fourth
    // group; its functions are F_SELECTWORD, F_SELECTALL, ...
    cx.simulate_keystrokes("down down down right down");
    cx.simulate_keystrokes("enter");
    assert!(!f.command_list_open(cx));
    assert!(f.view.read_with(cx, |v, _| v.cursor.has_selection()));
}

#[gpui::test]
fn the_keys_do_not_reach_the_text_while_the_command_list_is_open(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_MENU_ALLFUNC);
    cx.simulate_input("x");
    cx.simulate_keystrokes("left");
    assert_eq!(f.text(cx), "abc");
    assert_eq!(f.caret(cx), Pos::new(0, 3));
    cx.simulate_keystrokes("escape");
    // and the focus is back in the text
    cx.simulate_input("x");
    assert_eq!(f.text(cx), "abcx");
}

#[gpui::test]
fn the_command_list_can_open_the_version_dialog(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_MENU_ALLFUNC);
    // the last group before "その他" is 支援; its last function is F_ABOUT
    cx.simulate_keystrokes("end up right end enter");
    assert!(!f.command_list_open(cx));
    assert!(f.about_open(cx));
}

// ---- F_ABOUT ----

#[gpui::test]
fn ctrl_shift_f1_opens_the_version_dialog_and_enter_closes_it(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    cx.simulate_keystrokes("ctrl-shift-f1");
    assert!(f.about_open(cx));
    // typing does not reach the text
    cx.simulate_input("x");
    assert_eq!(f.text(cx), "abc");
    cx.simulate_keystrokes("enter");
    assert!(!f.about_open(cx));
    cx.simulate_input("x");
    assert_eq!(f.text(cx), "abcx");
}

#[gpui::test]
fn the_version_dialog_closes_with_escape(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_ABOUT);
    assert!(f.about_open(cx));
    cx.simulate_keystrokes("escape");
    assert!(!f.about_open(cx));
}

#[gpui::test]
fn the_copy_button_puts_the_version_text_on_the_clipboard(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_ABOUT);
    let dialog = f
        .view
        .read_with(cx, |v, _| v.support.about.clone())
        .expect("the dialog");
    dialog.update(cx, |d, cx| d.copy(cx));
    let copied = cx.read_from_clipboard().and_then(|c| c.text());
    let expected = dialog.read_with(cx, |d, _| d.info().version_text());
    assert_eq!(copied, Some(expected));
    assert!(copied_starts_with_name(&dialog, cx));
}

fn copied_starts_with_name(
    dialog: &Entity<super::about::AboutDialog>,
    cx: &mut VisualTestContext,
) -> bool {
    dialog.read_with(cx, |d, _| {
        d.info().version_text().starts_with("UME Editor Ver. ")
    })
}
