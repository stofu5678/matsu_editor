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
use crate::settings::UserSettings;
use crate::view::EditorView;

const F_HOKAN: u16 = 31430;
const F_MENU_ALLFUNC: u16 = 31445;
const F_CTRL_CODE_DIALOG: u16 = 30792;
const F_ABOUT: u16 = 31455;

struct Fixture {
    document: Entity<SharedDocument>,
    view: Entity<EditorView>,
}

/// A view over `text` with the caret at the end of the text.
fn setup<'a>(cx: &'a mut TestAppContext, text: &str) -> (Fixture, &'a mut VisualTestContext) {
    setup_with(cx, text, None)
}

/// A view over `text` in an application with the settings `user`.
fn setup_with<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
    user: Option<UserSettings>,
) -> (Fixture, &'a mut VisualTestContext) {
    cx.update(gpui_component::init);
    if let Some(user) = user {
        cx.update(|app| app.set_global(user));
    }
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

/// The keys of the page 支援 that choose a candidate, for the test.
fn choose_with(cx: &mut VisualTestContext, enter: bool, tab: bool, right: bool) {
    cx.update(|_, app| {
        app.set_global(crate::support::prefs::HelperPrefs {
            hokan_key_return: enter,
            hokan_key_tab: tab,
            hokan_key_right: right,
            ..Default::default()
        });
    });
}

#[gpui::test]
fn tab_inserts_when_the_page_turns_it_on(cx: &mut TestAppContext) {
    // 「候補決定キー」 of the page 支援
    let (f, cx) = setup(cx, "apple apricot\nap");
    choose_with(cx, true, true, true);
    cx.simulate_keystrokes("ctrl-space tab");
    assert_eq!(f.text(cx), "apple apricot\napple");
    assert_eq!(f.list(cx), None);
}

#[gpui::test]
fn enter_and_right_go_to_the_editor_when_the_page_turns_them_off(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "apple apricot\nap");
    choose_with(cx, false, false, false);
    cx.simulate_keystrokes("ctrl-space right");
    // Right is not a key of the list: the list closes and the caret stays at the end
    assert_eq!(f.list(cx), None);
    assert_eq!(f.text(cx), "apple apricot\nap");
    cx.simulate_keystrokes("ctrl-space enter");
    assert_eq!(f.list(cx), None);
    // Enter is a line break again
    assert_eq!(f.text(cx), "apple apricot\nap\n");
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
fn a_completion_is_recorded_in_a_key_macro_as_what_it_does(cx: &mut TestAppContext) {
    // Sakura's HandleCommand records the F_WordDeleteToStart and the F_INSTEXT_W of DoHokan, not
    // F_HOKAN (a key macro replays the result, not the list)
    let (f, cx) = setup(cx, "apple apricot\nap");
    cx.simulate_keystrokes("ctrl-shift-m ctrl-space down enter ctrl-shift-m");
    assert_eq!(f.text(cx), "apple apricot\napricot");
    cx.simulate_keystrokes("enter");
    cx.simulate_input("ap");
    cx.simulate_keystrokes("ctrl-shift-l");
    assert_eq!(f.text(cx), "apple apricot\napricot\napricot");
    assert_eq!(f.list(cx), None);
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

// ---- F_HOKAN: the settings of the document type ----

/// A folder that is removed at the end of the test.
struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ume-ui-hokan-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Settings read from a file `ume.ini` with `types_0` as its `[Types(0)]` section, in a private
/// folder that has the files `files` (name and bytes).
fn settings_with(types_0: &str, files: &[(&str, &[u8])]) -> (TempDir, UserSettings) {
    let dir = TempDir::new();
    for (name, bytes) in files {
        std::fs::write(dir.0.join(name), bytes).unwrap();
    }
    let path = dir.0.join("ume.ini");
    let profile = ume_config::Profile::from_bytes(format!("[Types(0)]\r\n{types_0}").as_bytes());
    let user = UserSettings::from_profile(profile, Some(path));
    (dir, user)
}

#[gpui::test]
fn the_word_file_of_the_type_comes_first_and_its_path_starts_at_the_settings_folder(
    cx: &mut TestAppContext,
) {
    // `CHokanMgr::Search`: the word file, then the document. A word that is in both is once.
    let (_dir, user) = settings_with(
        "szHokanFile=words.kwd\r\n",
        &[("words.kwd", b"; a comment\r\napple\r\nzebra\r\napricot\r\n")],
    );
    let (f, cx) = setup_with(cx, "apex apple\nap", Some(user));
    f.run(cx, F_HOKAN);
    assert_eq!(
        f.list(cx),
        Some((strings(&["apple", "apricot", "apex"]), 0))
    );
}

#[gpui::test]
fn a_word_file_that_changed_is_read_again(cx: &mut TestAppContext) {
    let (dir, user) = settings_with("szHokanFile=words.kwd\r\n", &[("words.kwd", b"apple\r\n")]);
    let (f, cx) = setup_with(cx, "ap", Some(user));
    f.run(cx, F_HOKAN);
    // one candidate: inserted
    assert_eq!(f.text(cx), "apple");
    std::fs::write(dir.0.join("words.kwd"), b"apple\r\napricot juice\r\n").unwrap();
    cx.simulate_input(" ap");
    f.run(cx, F_HOKAN);
    assert_eq!(f.list(cx), Some((strings(&["apple", "apricot juice"]), 0)));
}

#[gpui::test]
fn a_word_file_that_is_not_there_gives_no_candidates_but_the_document_still_does(
    cx: &mut TestAppContext,
) {
    let (_dir, user) = settings_with("szHokanFile=missing.kwd\r\n", &[]);
    let (f, cx) = setup_with(cx, "apex apple\nap", Some(user));
    f.run(cx, F_HOKAN);
    assert_eq!(f.list(cx), Some((strings(&["apex", "apple"]), 0)));
}

#[gpui::test]
fn the_words_of_the_document_can_be_left_out(cx: &mut TestAppContext) {
    let (_dir, user) = settings_with(
        "szHokanFile=words.kwd\r\nbUseHokanByFile=0\r\n",
        &[("words.kwd", b"apple\r\napricot\r\n")],
    );
    let (f, cx) = setup_with(cx, "apex\nap", Some(user));
    f.run(cx, F_HOKAN);
    assert_eq!(f.list(cx), Some((strings(&["apple", "apricot"]), 0)));
}

#[gpui::test]
fn the_case_of_letters_is_ignored_when_the_type_says_so(cx: &mut TestAppContext) {
    let (_dir, user) = settings_with("bHokanLoHiCase=1\r\n", &[]);
    let (f, cx) = setup_with(cx, "Apple apricot\nap", Some(user));
    f.run(cx, F_HOKAN);
    assert_eq!(f.list(cx), Some((strings(&["Apple", "apricot"]), 0)));
}

#[gpui::test]
fn the_case_of_letters_matters_by_default(cx: &mut TestAppContext) {
    // `m_bHokanLoHiCase` is off in `_DefaultConfig`: the only candidate is inserted at once.
    let (f, cx) = setup(cx, "Apple apricot\nap");
    f.run(cx, F_HOKAN);
    assert_eq!(f.list(cx), None);
    assert_eq!(f.text(cx), "Apple apricot\napricot");
}

#[gpui::test]
fn the_highlighted_keywords_of_the_type_are_offered_after_the_words_of_the_document(
    cx: &mut TestAppContext,
) {
    // The keyword set 0 (C/C++) in slot 3 (`nKeywordSelect3`).
    let (_dir, user) = settings_with("bUseHokanByKeyword=1\r\nnKeywordSelect3=0\r\n", &[]);
    let (f, cx) = setup_with(cx, "whilst\nwh", Some(user));
    f.run(cx, F_HOKAN);
    let (items, _) = f.list(cx).expect("a list");
    assert_eq!(items[0], "whilst", "the document comes first");
    assert!(items.contains(&"while".to_owned()), "{items:?}");
    assert_eq!(
        items.iter().filter(|w| *w == "while").count(),
        1,
        "a keyword is once"
    );
}

#[gpui::test]
fn the_keywords_are_not_offered_by_default(cx: &mut TestAppContext) {
    // `m_bUseHokanByKeyword` is off in `_DefaultConfig`.
    let (f, cx) = setup(cx, "whilst\nwh");
    f.run(cx, F_HOKAN);
    assert_eq!(
        f.text(cx),
        "whilst\nwhilst",
        "the only candidate is inserted"
    );
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

// ---- F_CTRL_CODE_DIALOG ----

impl Fixture {
    fn ctrl_code_open(&self, cx: &mut VisualTestContext) -> bool {
        self.view
            .read_with(cx, |v, _| v.support.ctrl_code.is_some())
    }
}

#[gpui::test]
fn the_control_code_dialog_opens_on_the_first_row_and_escape_cancels(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_CTRL_CODE_DIALOG);
    assert!(f.ctrl_code_open(cx));
    let selected = f.view.read_with(cx, |v, cx| {
        v.support.ctrl_code.as_ref().unwrap().read(cx).selected()
    });
    assert_eq!(selected, 0);
    cx.simulate_keystrokes("escape");
    assert!(!f.ctrl_code_open(cx));
    assert_eq!(f.text(cx), "abc");
}

#[gpui::test]
fn enter_types_the_chosen_code(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_CTRL_CODE_DIALOG);
    // NUL, SOH, STX, ETX
    cx.simulate_keystrokes("down down down enter");
    assert!(!f.ctrl_code_open(cx));
    assert_eq!(f.text(cx), "abc\u{3}");
}

#[gpui::test]
fn a_key_chooses_its_code_at_once(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_CTRL_CODE_DIALOG);
    cx.simulate_keystrokes("[");
    assert!(!f.ctrl_code_open(cx));
    assert_eq!(f.text(cx), "abc\u{1b}");
    f.run(cx, F_CTRL_CODE_DIALOG);
    cx.simulate_keystrokes("a");
    assert_eq!(f.text(cx), "abc\u{1b}\u{1}");
}

#[gpui::test]
fn the_keys_do_not_reach_the_text_while_the_control_code_dialog_is_open(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_CTRL_CODE_DIALOG);
    cx.simulate_input("1");
    assert_eq!(f.text(cx), "abc");
    assert!(f.ctrl_code_open(cx));
}
