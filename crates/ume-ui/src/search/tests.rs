//! Tests of find and replace in GPUI's headless test window: the keys and function codes reach
//! the UI commands, the panel stands for the dialogs, and replace all is one undo step. The
//! rules of matching themselves are tested in `ume_core::search`; the arithmetic of the
//! highlight is tested in [`super::highlight`].
//!
//! Expectations about Sakura's behavior cite `sakura_core/cmd/CViewCommander_Search.cpp`.

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use ume_commands::sakura::FUNCTIONS;
use ume_core::buffer::{Document, Pos};
use ume_core::model::DocumentModel;
use ume_core::search::ReplaceTarget;

use super::PanelMode;
use crate::document::SharedDocument;
use crate::view::EditorView;

struct Fixture {
    document: Entity<SharedDocument>,
    view: Entity<EditorView>,
}

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

    fn selection(&self, cx: &mut VisualTestContext) -> Option<(Pos, Pos)> {
        self.view.read_with(cx, |v, _| v.cursor.selection())
    }

    fn notice(&self, cx: &mut VisualTestContext) -> Option<String> {
        self.view.read_with(cx, |v, _| v.notice.clone())
    }

    fn pattern(&self, cx: &mut VisualTestContext, pattern: &str) {
        self.view.update(cx, |v, _| v.search.set_pattern(pattern));
    }

    /// Runs a Sakura function by name, the way a menu item does.
    fn run(&self, cx: &mut VisualTestContext, name: &str) {
        let code = FUNCTIONS.iter().find(|f| f.name == name).unwrap().code;
        self.view
            .update_in(cx, |v, window, cx| v.run_function(code, &[], window, cx));
    }

    fn panel_mode(&self, cx: &mut VisualTestContext) -> Option<PanelMode> {
        self.view.read_with(cx, |v, cx| v.search_panel_mode(cx))
    }
}

#[gpui::test]
fn f3_selects_the_next_match_with_the_caret_at_its_start(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo bar foo\nfoo");
    f.pattern(cx, "foo");
    cx.simulate_keystrokes("f3");
    // Sakura puts the caret at the start of the match it selects
    assert_eq!(f.selection(cx), Some((Pos::new(0, 0), Pos::new(0, 3))));
    assert_eq!(f.caret(cx), Pos::new(0, 0));
    cx.simulate_keystrokes("f3");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 8), Pos::new(0, 11))));
    cx.simulate_keystrokes("f3");
    assert_eq!(f.selection(cx), Some((Pos::new(1, 0), Pos::new(1, 3))));
}

#[gpui::test]
fn shift_f3_selects_the_previous_match(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo bar foo");
    f.pattern(cx, "foo");
    cx.simulate_keystrokes("end shift-f3");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 8), Pos::new(0, 11))));
    cx.simulate_keystrokes("shift-f3");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 0), Pos::new(0, 3))));
}

#[gpui::test]
fn not_found_says_so_in_the_status_bar_and_a_box(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo bar");
    f.pattern(cx, "foo");
    cx.simulate_keystrokes("f3 f3");
    assert_eq!(f.notice(cx).as_deref(), Some("▽見つかりませんでした"));
    let (message, detail) = cx.pending_prompt().expect("the not-found box");
    assert_eq!(message, "UME Editor");
    assert_eq!(detail, "前方(↓) に文字列 'foo' が１つも見つかりません。");
    // the selection stays where it was
    assert_eq!(f.selection(cx), Some((Pos::new(0, 0), Pos::new(0, 3))));
}

#[gpui::test]
fn without_the_message_option_nothing_is_shown_but_the_status(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo");
    f.view.update(cx, |v, _| {
        let mut o = v.search.options();
        o.notify_not_found = false;
        v.search.set_options(o);
        v.search.set_pattern("zzz");
    });
    cx.simulate_keystrokes("f3");
    assert!(!cx.has_pending_prompt());
    assert_eq!(f.notice(cx).as_deref(), Some("▽見つかりませんでした"));
}

#[gpui::test]
fn wrap_around_goes_on_from_the_start_and_says_so(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo bar\nbaz foo");
    f.view.update(cx, |v, _| {
        let mut o = v.search.options();
        o.wrap = true;
        v.search.set_options(o);
        v.search.set_pattern("foo");
    });
    cx.simulate_keystrokes("f3 f3");
    assert_eq!(f.selection(cx), Some((Pos::new(1, 4), Pos::new(1, 7))));
    assert_eq!(f.notice(cx), None);
    cx.simulate_keystrokes("f3");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 0), Pos::new(0, 3))));
    assert_eq!(f.notice(cx).as_deref(), Some("▼先頭から再検索しました"));
    cx.simulate_keystrokes("shift-f3");
    assert_eq!(f.notice(cx).as_deref(), Some("▲末尾から再検索しました"));
    assert_eq!(f.selection(cx), Some((Pos::new(1, 4), Pos::new(1, 7))));
}

#[gpui::test]
fn the_options_decide_what_matches(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "Foo food foo");
    f.view.update(cx, |v, _| {
        let mut o = v.search.options();
        o.whole_word = true;
        o.match_case = true;
        v.search.set_options(o);
        v.search.set_pattern("foo");
    });
    cx.simulate_keystrokes("f3");
    // only the last "foo" is a whole word of the same case
    assert_eq!(f.selection(cx), Some((Pos::new(0, 9), Pos::new(0, 12))));
}

#[gpui::test]
fn f3_without_a_pattern_opens_the_find_panel(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    cx.simulate_keystrokes("f3");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Find));
}

#[gpui::test]
fn the_find_panel_starts_with_the_word_at_the_caret_and_searches(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo bar\nbar");
    cx.simulate_keystrokes("right right right right right");
    cx.simulate_keystrokes("secondary-f");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Find));
    let pattern = f.view.read_with(cx, |v, _| v.search.pattern().to_owned());
    assert_eq!(pattern, "bar");
    // Enter in the input is 下検索; the dialog closes itself (the default) and the focus returns
    cx.simulate_keystrokes("enter");
    assert_eq!(f.selection(cx), Some((Pos::new(1, 0), Pos::new(1, 3))));
    assert_eq!(f.panel_mode(cx), None);
    // the search start position is where the caret was when the panel opened
    f.run(cx, "F_JUMP_SRCHSTARTPOS");
    assert_eq!(f.caret(cx), Pos::new(0, 5));
}

#[gpui::test]
fn text_typed_in_the_panel_is_the_pattern(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc xyz");
    cx.simulate_keystrokes("secondary-f");
    // the caret is on "abc" so the panel starts with it; replace it
    cx.simulate_keystrokes("secondary-a");
    cx.simulate_input("xyz");
    cx.simulate_keystrokes("enter");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 4), Pos::new(0, 7))));
    // the typing went to the panel, not to the document
    assert_eq!(f.text(cx), "abc xyz");
}

#[gpui::test]
fn escape_closes_the_panel(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    cx.simulate_keystrokes("secondary-f");
    assert!(f.panel_mode(cx).is_some());
    cx.simulate_keystrokes("escape");
    assert_eq!(f.panel_mode(cx), None);
    // the focus is back in the text
    cx.simulate_input("Z");
    assert_eq!(f.text(cx), "Zabc");
}

#[gpui::test]
fn an_empty_pattern_in_the_panel_asks_for_one(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    cx.simulate_keystrokes("secondary-f enter");
    let (_, detail) = cx.pending_prompt().expect("the box");
    assert_eq!(detail, "検索条件を指定してください。");
    assert!(f.panel_mode(cx).is_some());
}

#[gpui::test]
fn the_replace_panel_opens_on_ctrl_r(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    cx.simulate_keystrokes("secondary-r");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Replace));
}

fn set_replace(f: &Fixture, cx: &mut VisualTestContext, pattern: &str, text: &str) {
    f.view.update(cx, |v, _| {
        v.search.set_pattern(pattern);
        v.search.replace.text = text.to_owned();
    });
}

#[gpui::test]
fn replace_all_is_one_undo_step_and_reports_the_count(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab ab\r\nab\r\nxab");
    set_replace(&f, cx, "ab", "Z");
    f.run(cx, "F_REPLACE_ALL");
    assert_eq!(f.text(cx), "Z Z\r\nZ\r\nxZ");
    assert_eq!(f.notice(cx).as_deref(), Some("4箇所を置換しました。"));
    let (_, detail) = cx.pending_prompt().expect("the count box");
    assert_eq!(detail, "4箇所を置換しました。");
    // the caret is after the last replacement
    assert_eq!(f.caret(cx), Pos::new(2, 2));
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(f.text(cx), "ab ab\r\nab\r\nxab");
    cx.simulate_keystrokes("secondary-y");
    assert_eq!(f.text(cx), "Z Z\r\nZ\r\nxZ");
}

#[gpui::test]
fn replace_all_in_the_selection_keeps_the_outside_and_the_selection(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab\r\nab ab\r\nab");
    set_replace(&f, cx, "ab", "xyz");
    f.view.update(cx, |v, _| {
        v.search.replace.in_selection = true;
        v.cursor.select(Pos::new(1, 0), Pos::new(2, 0));
    });
    f.run(cx, "F_REPLACE_ALL");
    assert_eq!(f.text(cx), "ab\r\nxyz xyz\r\nab");
    // the selection grew with the text (`Command_REPLACE_ALL` restores it)
    assert_eq!(f.selection(cx), Some((Pos::new(1, 0), Pos::new(2, 0))));
}

#[gpui::test]
fn replace_all_with_the_other_targets(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a1 a2");
    set_replace(&f, cx, "a", "<");
    f.view.update(cx, |v, _| {
        v.search.replace.target = ReplaceTarget::InsertBefore
    });
    f.run(cx, "F_REPLACE_ALL");
    assert_eq!(f.text(cx), "<a1 <a2");
    cx.simulate_keystrokes("secondary-z");
    f.view.update(cx, |v, _| {
        v.search.replace.target = ReplaceTarget::DeleteLine
    });
    f.run(cx, "F_REPLACE_ALL");
    assert_eq!(f.text(cx), "");
}

#[gpui::test]
fn replace_all_that_matches_nothing_changes_nothing(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    set_replace(&f, cx, "zz", "y");
    f.run(cx, "F_REPLACE_ALL");
    assert_eq!(f.text(cx), "abc");
    assert_eq!(f.notice(cx).as_deref(), Some("0箇所を置換しました。"));
    // and there is nothing to undo
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(f.text(cx), "abc");
}

#[gpui::test]
fn replace_one_replaces_the_match_and_selects_the_next(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab ab ab");
    set_replace(&f, cx, "ab", "X");
    f.run(cx, "F_REPLACE");
    // the first call finds the first match at the caret and replaces it
    assert_eq!(f.text(cx), "X ab ab");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 2), Pos::new(0, 4))));
    f.run(cx, "F_REPLACE");
    assert_eq!(f.text(cx), "X X ab");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 4), Pos::new(0, 6))));
    f.run(cx, "F_REPLACE");
    assert_eq!(f.text(cx), "X X X");
    // no more matches: the caret is after the last replacement and the box says it is done
    assert_eq!(f.selection(cx), None);
    assert_eq!(f.caret(cx), Pos::new(0, 5));
    let (_, detail) = cx.pending_prompt().expect("the box");
    assert_eq!(detail, "最後まで置換しました。");
}

#[gpui::test]
fn each_replace_is_an_undo_step_of_its_own(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab ab");
    set_replace(&f, cx, "ab", "X");
    f.run(cx, "F_REPLACE");
    f.run(cx, "F_REPLACE");
    assert_eq!(f.text(cx), "X X");
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(f.text(cx), "X ab");
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(f.text(cx), "ab ab");
}

#[gpui::test]
fn replace_without_a_match_says_not_found(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    set_replace(&f, cx, "zz", "y");
    f.run(cx, "F_REPLACE");
    assert_eq!(f.text(cx), "abc");
    assert_eq!(f.notice(cx).as_deref(), Some("▽見つかりませんでした"));
}

#[gpui::test]
fn the_replace_panel_replaces_all_from_its_inputs(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "aa");
    cx.simulate_keystrokes("secondary-r");
    cx.simulate_keystrokes("secondary-a");
    cx.simulate_input("a");
    // the replacement input takes the focus next
    let panel = f.view.read_with(cx, |v, _| v.panel.clone().unwrap());
    panel.update_in(cx, |p, window, cx| p.focus_replacement(window, cx));
    cx.simulate_input("bb");
    let replacement = f.view.read_with(cx, |v, _| v.search.replace.text.clone());
    assert_eq!(replacement, "bb");
    f.view
        .read_with(cx, |v, _| assert_eq!(v.search.pattern(), "a"));
    f.run(cx, "F_REPLACE_ALL");
    assert_eq!(f.text(cx), "bbbb");
}

#[gpui::test]
fn the_search_clearmark_command_toggles_the_marks(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo bar foo");
    f.pattern(cx, "foo");
    cx.simulate_keystrokes("f3");
    assert!(f.view.read_with(cx, |v, _| v.search.mark()));
    // no selection: the marks go (Ctrl+F3)
    cx.simulate_keystrokes("right secondary-f3");
    assert!(!f.view.read_with(cx, |v, _| v.search.mark()));
    // with a selection: it becomes the pattern and is marked
    cx.simulate_keystrokes("end shift-left shift-left shift-left secondary-f3");
    f.view.read_with(cx, |v, _| {
        assert_eq!(v.search.pattern(), "foo");
        assert!(v.search.mark());
    });
    f.view
        .update(cx, |v, _| v.cursor.select(Pos::new(0, 4), Pos::new(0, 7)));
    f.run(cx, "F_SEARCH_CLEARMARK");
    f.view
        .read_with(cx, |v, _| assert_eq!(v.search.pattern(), "bar"));
}

#[gpui::test]
fn jump_to_the_search_start_without_one_says_so(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, "F_JUMP_SRCHSTARTPOS");
    assert!(f.notice(cx).is_some());
    assert_eq!(f.caret(cx), Pos::new(0, 0));
}

#[gpui::test]
fn the_jump_panel_goes_to_a_line(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb\nc\nd");
    cx.simulate_keystrokes("secondary-j");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Jump));
    cx.simulate_input("3");
    cx.simulate_keystrokes("enter");
    assert_eq!(f.caret(cx), Pos::new(2, 0));
    assert_eq!(f.panel_mode(cx), None);
    // a number past the end goes to the last line (`Command_JUMP`)
    f.run(cx, "F_JUMP_DIALOG");
    cx.simulate_input("99");
    cx.simulate_keystrokes("enter");
    assert_eq!(f.caret(cx), Pos::new(3, 0));
}
