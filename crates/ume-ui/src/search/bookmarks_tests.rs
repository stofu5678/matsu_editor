//! Tests of the bookmark functions in GPUI's headless test window: the keys reach the commands,
//! the jumps follow the search options, and the status bar and the box say what Sakura says.
//! The rules of setting and following bookmarks are tested in `ume_commands` and `ume_core`.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Bookmark.cpp`; the keys are in
//! `sakura_core/func/CKeyBind.cpp` (F2, Shift+F2, Ctrl+F2, Ctrl+Shift+F2).

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use ume_commands::sakura::FUNCTIONS;
use ume_core::buffer::{Document, Pos};
use ume_core::model::DocumentModel;

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
    fn marked(&self, cx: &mut VisualTestContext) -> Vec<usize> {
        self.document
            .read_with(cx, |d, _| d.model().bookmarks().iter().collect())
    }

    fn caret(&self, cx: &mut VisualTestContext) -> Pos {
        self.view.read_with(cx, |v, _| v.cursor.caret())
    }

    fn notice(&self, cx: &mut VisualTestContext) -> Option<String> {
        self.view.read_with(cx, |v, _| v.notice.clone())
    }

    fn wrap(&self, cx: &mut VisualTestContext, wrap: bool) {
        self.view.update(cx, |v, _| {
            let mut o = v.search.options();
            o.wrap = wrap;
            v.search.set_options(o);
        });
    }

    fn run(&self, cx: &mut VisualTestContext, name: &str) {
        let code = FUNCTIONS.iter().find(|f| f.name == name).unwrap().code;
        self.view
            .update_in(cx, |v, window, cx| v.run_function(code, &[], window, cx));
    }
}

/// Bookmarks lines 1 and 3 of "a\nb\nc\nd\ne" and puts the caret back on line 0.
fn two_bookmarks(f: &Fixture, cx: &mut VisualTestContext) {
    cx.simulate_keystrokes("down ctrl-f2 down down ctrl-f2 ctrl-home");
    assert_eq!(f.marked(cx), [1, 3]);
    assert_eq!(f.caret(cx), Pos::new(0, 0));
}

#[gpui::test]
fn ctrl_f2_toggles_the_bookmark_of_the_caret_line(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "日本語\nb");
    cx.simulate_keystrokes("ctrl-f2");
    assert_eq!(f.marked(cx), [0]);
    cx.simulate_keystrokes("ctrl-f2");
    assert!(f.marked(cx).is_empty());
    // bookmarks are not an edit
    assert!(!f.document.read_with(cx, |d, _| d.model().is_modified()));
}

#[gpui::test]
fn f2_and_shift_f2_move_to_the_next_and_previous_bookmark(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb\nc\nd\ne");
    two_bookmarks(&f, cx);
    cx.simulate_keystrokes("f2");
    assert_eq!(f.caret(cx), Pos::new(1, 0));
    cx.simulate_keystrokes("f2");
    assert_eq!(f.caret(cx), Pos::new(3, 0));
    assert_eq!(f.notice(cx), None);
    cx.simulate_keystrokes("shift-f2");
    assert_eq!(f.caret(cx), Pos::new(1, 0));
    // no selection is made by a plain move
    assert!(!f.view.read_with(cx, |v, _| v.cursor.has_selection()));
}

#[gpui::test]
fn without_the_wrap_option_the_end_of_the_file_is_the_end(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb\nc\nd\ne");
    two_bookmarks(&f, cx);
    cx.simulate_keystrokes("f2 f2 f2");
    assert_eq!(f.caret(cx), Pos::new(3, 0));
    assert_eq!(f.notice(cx).as_deref(), Some("▽見つかりませんでした"));
    let (message, detail) = cx.pending_prompt().expect("the not-found box");
    assert_eq!(message, "UME Editor");
    assert_eq!(detail, "前方(↓) にブックマークが見つかりません。");
}

#[gpui::test]
fn the_box_can_be_turned_off(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb");
    f.view.update(cx, |v, _| {
        let mut o = v.search.options();
        o.notify_not_found = false;
        v.search.set_options(o);
    });
    cx.simulate_keystrokes("shift-f2");
    assert!(!cx.has_pending_prompt());
    assert_eq!(f.notice(cx).as_deref(), Some("△見つかりませんでした"));
    assert_eq!(f.caret(cx), Pos::new(0, 0));
}

#[gpui::test]
fn backward_not_found_has_its_own_text(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb");
    cx.simulate_keystrokes("shift-f2");
    let (_, detail) = cx.pending_prompt().expect("the not-found box");
    assert_eq!(detail, "後方(↑) にブックマークが見つかりません。");
    assert_eq!(f.caret(cx), Pos::new(0, 0));
}

#[gpui::test]
fn with_the_wrap_option_the_search_goes_on_from_the_other_end(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb\nc\nd\ne");
    two_bookmarks(&f, cx);
    f.wrap(cx, true);
    cx.simulate_keystrokes("f2 f2");
    assert_eq!(f.caret(cx), Pos::new(3, 0));
    cx.simulate_keystrokes("f2");
    assert_eq!(f.caret(cx), Pos::new(1, 0));
    assert_eq!(f.notice(cx).as_deref(), Some("▼先頭から再検索しました"));
    assert!(!cx.has_pending_prompt());
    cx.simulate_keystrokes("shift-f2");
    assert_eq!(f.caret(cx), Pos::new(3, 0));
    assert_eq!(f.notice(cx).as_deref(), Some("▲末尾から再検索しました"));
}

#[gpui::test]
fn the_only_bookmark_is_found_again_when_wrapping(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb\nc");
    f.wrap(cx, true);
    cx.simulate_keystrokes("down ctrl-f2 f2");
    // Sakura finds the line of the caret and says it started again
    assert_eq!(f.caret(cx), Pos::new(1, 0));
    assert_eq!(f.notice(cx).as_deref(), Some("▼先頭から再検索しました"));
}

#[gpui::test]
fn a_jump_with_the_selection_locked_extends_the_selection(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab\ncd\nef");
    cx.simulate_keystrokes("down down ctrl-f2 ctrl-home");
    f.run(cx, "F_BEGIN_BOX");
    cx.simulate_keystrokes("f2");
    assert_eq!(f.caret(cx), Pos::new(2, 0));
    assert!(f.view.read_with(cx, |v, _| v.cursor.is_box()));
}

#[gpui::test]
fn ctrl_shift_f2_removes_every_bookmark(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb\nc\nd\ne");
    two_bookmarks(&f, cx);
    cx.simulate_keystrokes("ctrl-shift-f2");
    assert!(f.marked(cx).is_empty());
}

#[gpui::test]
fn bookmarks_follow_edits_of_the_document(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb\nc");
    cx.simulate_keystrokes("down ctrl-f2 ctrl-home enter");
    assert_eq!(f.marked(cx), [2]);
    cx.simulate_keystrokes("ctrl-z");
    assert_eq!(f.marked(cx), [1]);
}

#[gpui::test]
fn bookmark_pattern_marks_the_lines_with_a_match(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo\nbar\nFOO bar\n日本語 foo\nbaz");
    f.view.update(cx, |v, _| v.search.set_pattern("foo"));
    f.run(cx, "F_BOOKMARK_PATTERN");
    // the search ignores the case by default
    assert_eq!(f.marked(cx), [0, 2, 3]);
    // the lines that were marked stay marked
    cx.simulate_keystrokes("down down down down ctrl-f2");
    f.run(cx, "F_BOOKMARK_PATTERN");
    assert_eq!(f.marked(cx), [0, 2, 3, 4]);
}

#[gpui::test]
fn bookmark_pattern_follows_the_search_options(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo\nfoot\nFoo");
    f.view.update(cx, |v, _| {
        let mut o = v.search.options();
        o.whole_word = true;
        o.match_case = true;
        v.search.set_options(o);
        v.search.set_pattern("foo");
    });
    f.run(cx, "F_BOOKMARK_PATTERN");
    assert_eq!(f.marked(cx), [0]);
}

#[gpui::test]
fn bookmark_pattern_without_a_pattern_does_nothing(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo");
    f.run(cx, "F_BOOKMARK_PATTERN");
    assert!(f.marked(cx).is_empty());
    assert!(!cx.has_pending_prompt());
}
