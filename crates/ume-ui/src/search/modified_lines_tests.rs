//! Tests of the modified line functions in GPUI's headless test window: the jumps follow the
//! search options and the status bar and the box say what Sakura says. Which lines are modified
//! is tested in `ume_core`, and where the jumps stop in `ume_commands`.
//!
//! Sakura: `Command_MODIFYLINE_NEXT` and `Command_MODIFYLINE_PREV` in
//! `sakura_core/cmd/CViewCommander_Cursor.cpp`; the functions have no key in the default key
//! bindings (`sakura_core/func/CKeyBind.cpp`), only the menu.

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
    fn caret(&self, cx: &mut VisualTestContext) -> Pos {
        self.view.read_with(cx, |v, _| v.cursor.caret())
    }

    fn notice(&self, cx: &mut VisualTestContext) -> Option<String> {
        self.view.read_with(cx, |v, _| v.notice.clone())
    }

    fn modified(&self, cx: &mut VisualTestContext) -> Vec<usize> {
        self.document.read_with(cx, |d, _| {
            (0..d.doc().row_count())
                .filter(|&l| d.model().is_line_modified(l))
                .collect()
        })
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

/// Types a char in lines 1 and 3 of "a\nb\nc\nd\ne" and puts the caret back on line 0.
fn two_edits(f: &Fixture, cx: &mut VisualTestContext) {
    cx.simulate_keystrokes("down x down down x ctrl-home");
    assert_eq!(f.modified(cx), [1, 3]);
    assert_eq!(f.caret(cx), Pos::new(0, 0));
}

#[gpui::test]
fn typing_modifies_the_line_and_the_menu_functions_jump(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb\nc\nd\ne");
    two_edits(&f, cx);
    f.run(cx, "F_MODIFYLINE_NEXT");
    assert_eq!(f.caret(cx), Pos::new(1, 0));
    f.run(cx, "F_MODIFYLINE_NEXT");
    assert_eq!(f.caret(cx), Pos::new(2, 0));
    f.run(cx, "F_MODIFYLINE_NEXT");
    assert_eq!(f.caret(cx), Pos::new(3, 0));
    assert_eq!(f.notice(cx), None);
    f.run(cx, "F_MODIFYLINE_PREV");
    assert_eq!(f.caret(cx), Pos::new(2, 0));
    // a plain move makes no selection
    assert!(!f.view.read_with(cx, |v, _| v.cursor.has_selection()));
}

#[gpui::test]
fn the_sel_forms_extend_the_selection(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab\ncd\nef");
    cx.simulate_keystrokes("down x ctrl-home right");
    f.run(cx, "F_MODIFYLINE_NEXT_SEL");
    let selection = f.view.read_with(cx, |v, _| v.cursor.selection());
    assert_eq!(selection, Some((Pos::new(0, 1), Pos::new(1, 0))));
    f.run(cx, "F_MODIFYLINE_NEXT_SEL");
    let selection = f.view.read_with(cx, |v, _| v.cursor.selection());
    assert_eq!(selection, Some((Pos::new(0, 1), Pos::new(2, 0))));
    f.run(cx, "F_MODIFYLINE_PREV_SEL");
    let selection = f.view.read_with(cx, |v, _| v.cursor.selection());
    assert_eq!(selection, Some((Pos::new(0, 1), Pos::new(1, 0))));
}

#[gpui::test]
fn without_the_wrap_option_the_end_of_the_file_is_the_end(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb\nc\nd\ne");
    two_edits(&f, cx);
    for _ in 0..4 {
        f.run(cx, "F_MODIFYLINE_NEXT");
    }
    assert_eq!(f.caret(cx), Pos::new(4, 0));
    assert_eq!(f.notice(cx), None);
    f.run(cx, "F_MODIFYLINE_NEXT");
    assert_eq!(f.caret(cx), Pos::new(4, 0));
    assert_eq!(f.notice(cx).as_deref(), Some("▽見つかりませんでした"));
    let (message, detail) = cx.pending_prompt().expect("the not-found box");
    assert_eq!(message, "UME Editor");
    assert_eq!(detail, "前方(↓) に変更行が見つかりません。");
}

#[gpui::test]
fn backward_not_found_has_its_own_text(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb");
    f.run(cx, "F_MODIFYLINE_PREV");
    let (_, detail) = cx.pending_prompt().expect("the not-found box");
    assert_eq!(detail, "後方(↑) に変更行が見つかりません。");
    assert_eq!(f.notice(cx).as_deref(), Some("△見つかりませんでした"));
}

#[gpui::test]
fn the_box_can_be_turned_off(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb");
    f.view.update(cx, |v, _| {
        let mut o = v.search.options();
        o.notify_not_found = false;
        v.search.set_options(o);
    });
    f.run(cx, "F_MODIFYLINE_NEXT");
    assert!(!cx.has_pending_prompt());
    assert_eq!(f.notice(cx).as_deref(), Some("▽見つかりませんでした"));
}

#[gpui::test]
fn with_the_wrap_option_the_search_goes_on_from_the_other_end(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb\nc\nd\ne");
    two_edits(&f, cx);
    f.wrap(cx, true);
    cx.simulate_keystrokes("ctrl-end");
    f.run(cx, "F_MODIFYLINE_NEXT");
    assert_eq!(f.caret(cx), Pos::new(1, 0));
    assert_eq!(f.notice(cx).as_deref(), Some("▼先頭から再検索しました"));
    assert!(!cx.has_pending_prompt());
    cx.simulate_keystrokes("ctrl-home");
    f.run(cx, "F_MODIFYLINE_PREV");
    assert_eq!(f.caret(cx), Pos::new(4, 0));
    assert_eq!(f.notice(cx).as_deref(), Some("▲末尾から再検索しました"));
}

#[gpui::test]
fn a_saved_document_has_no_modified_line(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb\nc");
    cx.simulate_keystrokes("down x ctrl-home");
    assert_eq!(f.modified(cx), [1]);
    f.document.update(cx, |d, _| {
        d.parts_mut().0.mark_saved(&Default::default());
    });
    assert!(f.modified(cx).is_empty());
    f.run(cx, "F_MODIFYLINE_NEXT");
    assert_eq!(f.caret(cx), Pos::new(0, 0));
    assert!(cx.pending_prompt().is_some());
}

#[gpui::test]
fn undo_takes_the_mark_back(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb\nc");
    cx.simulate_keystrokes("down x");
    assert_eq!(f.modified(cx), [1]);
    cx.simulate_keystrokes("ctrl-z");
    assert!(f.modified(cx).is_empty());
    cx.simulate_keystrokes("ctrl-y");
    assert_eq!(f.modified(cx), [1]);
}
