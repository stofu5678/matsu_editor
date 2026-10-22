//! Tests of the regular expression option of find and replace in GPUI's headless test window.
//! The engine and the replacement syntax are tested in `matsu_regex`; here the panel's state, the
//! commands and the boxes are.
//!
//! Sakura: `CSearchAgent::SearchWord` and `CViewCommander_Search.cpp` for the searches,
//! `CheckRegexpSyntax` (`sakura_core/extmodule/CBregexp.cpp`) for the box of a bad pattern.

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_commands::sakura::FUNCTIONS;
use matsu_core::buffer::{Document, Pos};
use matsu_core::model::DocumentModel;
use matsu_core::search::ReplaceTarget;

use super::SearchOptions;
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

    fn selection(&self, cx: &mut VisualTestContext) -> Option<(Pos, Pos)> {
        self.view.read_with(cx, |v, _| v.cursor.selection())
    }

    fn caret(&self, cx: &mut VisualTestContext) -> Pos {
        self.view.read_with(cx, |v, _| v.cursor.caret())
    }

    /// Turns the regular expression option on with `pattern`.
    fn regex(&self, cx: &mut VisualTestContext, pattern: &str) {
        self.view.update(cx, |v, _| {
            let mut o = v.search.options();
            o.regex = true;
            v.search.set_options(o);
            v.search.set_pattern(pattern);
        });
    }

    fn options(&self, cx: &mut VisualTestContext, change: impl FnOnce(&mut SearchOptions)) {
        self.view.update(cx, |v, _| {
            let mut o = v.search.options();
            change(&mut o);
            v.search.set_options(o);
        });
    }

    fn replacement(&self, cx: &mut VisualTestContext, text: &str) {
        self.view
            .update(cx, |v, _| v.search.replace.text = text.to_owned());
    }

    fn run(&self, cx: &mut VisualTestContext, name: &str) {
        let code = FUNCTIONS.iter().find(|f| f.name == name).unwrap().code;
        self.view
            .update_in(cx, |v, window, cx| v.run_function(code, &[], window, cx));
    }

    fn marked(&self, cx: &mut VisualTestContext) -> Vec<usize> {
        self.document
            .read_with(cx, |d, _| d.model().bookmarks().iter().collect())
    }

    fn caret_to(&self, cx: &mut VisualTestContext, pos: Pos) {
        self.view.update(cx, |v, _| v.cursor.select(pos, pos));
    }
}

#[gpui::test]
fn a_regular_expression_finds_what_it_describes(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo123 bar45");
    f.regex(cx, r"\d+");
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 3), Pos::new(0, 6))));
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 10), Pos::new(0, 12))));
    f.run(cx, "F_SEARCH_PREV");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 3), Pos::new(0, 6))));
}

#[gpui::test]
fn the_same_pattern_is_plain_text_without_the_option(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a1c a.c");
    f.regex(cx, "a.c");
    f.options(cx, |o| o.regex = false);
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 4), Pos::new(0, 7))));
}

#[gpui::test]
fn japanese_text_matches_with_the_unicode_classes(cx: &mut TestAppContext) {
    // `\d` matches the full-width digits (help/sakura/res/HLP000089.html)
    let (f, cx) = setup(cx, "あいう 漢字１２３");
    f.regex(cx, r"\d+");
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 16), Pos::new(0, 25))));
    f.regex(cx, "(?:漢|あ)字?");
    f.caret_to(cx, Pos::new(0, 0));
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 0), Pos::new(0, 3))));
}

#[gpui::test]
fn anchors_see_a_line_without_its_line_ending(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab\r\ncd\r\nef");
    f.regex(cx, "^c");
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.selection(cx), Some((Pos::new(1, 0), Pos::new(1, 1))));
    f.regex(cx, "b$");
    f.caret_to(cx, Pos::new(0, 0));
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 1), Pos::new(0, 2))));
}

#[gpui::test]
fn match_case_goes_with_the_regular_expression(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "Abc abc");
    f.regex(cx, "a.c");
    // off: the pattern is compiled with the `i` modifier
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 0), Pos::new(0, 3))));
    f.options(cx, |o| o.match_case = true);
    f.caret_to(cx, Pos::new(0, 0));
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 4), Pos::new(0, 7))));
}

#[gpui::test]
fn whole_word_is_ignored_with_the_regular_expression(cx: &mut TestAppContext) {
    // Sakura greys the box out and `SearchWord` takes the regex branch first
    let (f, cx) = setup(cx, "foobar foo");
    f.options(cx, |o| o.whole_word = true);
    f.regex(cx, "foo");
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 0), Pos::new(0, 3))));
    // and it counts again when the option is off
    f.options(cx, |o| o.regex = false);
    f.caret_to(cx, Pos::new(0, 0));
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 7), Pos::new(0, 10))));
}

#[gpui::test]
fn an_empty_match_is_stepped_over_by_the_next_search(cx: &mut TestAppContext) {
    // `b0Match` in `Command_SEARCH_NEXT`
    let (f, cx) = setup(cx, "ab\r\ncd\r\nef");
    f.regex(cx, "^");
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.selection(cx), None);
    assert_eq!(f.caret(cx), Pos::new(0, 0));
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.caret(cx), Pos::new(1, 0));
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.caret(cx), Pos::new(2, 0));
    // looking backwards goes from the caret, so it finds the one before
    f.run(cx, "F_SEARCH_PREV");
    assert_eq!(f.caret(cx), Pos::new(1, 0));
}

#[gpui::test]
fn a_pattern_that_does_not_compile_says_so_and_does_not_search(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "(a)");
    f.regex(cx, "(a");
    f.run(cx, "F_SEARCH_NEXT");
    let (_, detail) = cx.pending_prompt().expect("the error box");
    assert!(detail.starts_with("正規表現の構文に誤りがあります。\n詳細："));
    assert_eq!(f.selection(cx), None);
    assert!(f.view.read_with(cx, |v, _| v.search.matcher().is_none()));
    // turning the option off makes it a plain search for "(a"
    f.options(cx, |o| o.regex = false);
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 0), Pos::new(0, 2))));
}

#[gpui::test]
fn replace_all_with_a_bad_pattern_changes_nothing(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.regex(cx, "[a");
    f.replacement(cx, "x");
    f.run(cx, "F_REPLACE_ALL");
    assert_eq!(f.text(cx), "abc");
    let (_, detail) = cx.pending_prompt().expect("the error box");
    assert!(detail.contains("正規表現の構文に誤りがあります。"));
}

#[gpui::test]
fn replace_all_uses_the_groups_and_is_one_undo_step(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "2026-10-16\r\n1999-01-02 x");
    f.regex(cx, r"(\d+)-(\d+)-(\d+)");
    f.replacement(cx, "$3/$2/$1");
    f.run(cx, "F_REPLACE_ALL");
    assert_eq!(f.text(cx), "16/10/2026\r\n02/01/1999 x");
    assert!(!cx.has_pending_prompt(), "the box is the panel's");
    cx.simulate_keystrokes("ctrl-z");
    assert_eq!(f.text(cx), "2026-10-16\r\n1999-01-02 x");
    cx.simulate_keystrokes("ctrl-y");
    assert_eq!(f.text(cx), "16/10/2026\r\n02/01/1999 x");
}

#[gpui::test]
fn the_replacement_has_the_whole_match_and_case_conversion(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab cd");
    f.regex(cx, r"\w+");
    f.replacement(cx, r"[\U$&\E]");
    f.run(cx, "F_REPLACE_ALL");
    assert_eq!(f.text(cx), "[AB] [CD]");
}

#[gpui::test]
fn replace_all_with_japanese_text_and_the_insert_targets(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "東京1 大阪22");
    f.regex(cx, r"(\p{Han}+)(\d+)");
    f.replacement(cx, "$2:$1");
    f.run(cx, "F_REPLACE_ALL");
    assert_eq!(f.text(cx), "1:東京 22:大阪");
    cx.simulate_keystrokes("ctrl-z");
    f.view.update(cx, |v, _| {
        v.search.replace.target = ReplaceTarget::InsertAfter
    });
    f.replacement(cx, "!");
    f.run(cx, "F_REPLACE_ALL");
    assert_eq!(f.text(cx), "東京1! 大阪22!");
}

#[gpui::test]
fn replace_all_with_matches_that_are_empty(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab\r\ncd");
    f.regex(cx, "^");
    f.replacement(cx, "> ");
    f.run(cx, "F_REPLACE_ALL");
    assert_eq!(f.text(cx), "> ab\r\n> cd");
    cx.simulate_keystrokes("ctrl-z");
    assert_eq!(f.text(cx), "ab\r\ncd");
    f.regex(cx, "$");
    f.replacement(cx, ";");
    f.run(cx, "F_REPLACE_ALL");
    // the last line has no line ending: no search finds a match at its end (`regex-dollar-nofinal`)
    assert_eq!(f.text(cx), "ab;\r\ncd");
}

#[gpui::test]
fn replace_all_in_the_selection_only(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a1\r\na2\r\na3");
    f.regex(cx, r"a(\d)");
    f.replacement(cx, "<$1>");
    f.view.update(cx, |v, _| {
        v.search.replace.in_selection = true;
        v.cursor.select(Pos::new(1, 0), Pos::new(2, 0));
    });
    f.run(cx, "F_REPLACE_ALL");
    assert_eq!(f.text(cx), "a1\r\n<2>\r\na3");
}

#[gpui::test]
fn replace_all_in_repeat_mode_looks_behind_at_the_changed_text(cx: &mut TestAppContext) {
    // 一括置換 sees the line as it was: only the first "a" follows an "x"
    let (f, cx) = setup(cx, "xaa");
    f.regex(cx, "(?<=x)a");
    f.replacement(cx, "x");
    f.run(cx, "F_REPLACE_ALL");
    assert_eq!(f.text(cx), "xxa");
    cx.simulate_keystrokes("ctrl-z");
    // 「すべて置換」は置換の繰返し: the second "a" now follows the new "x"
    f.view
        .update(cx, |v, _| v.search.replace.consecutive_all = true);
    f.run(cx, "F_REPLACE_ALL");
    assert_eq!(f.text(cx), "xxx");
}

#[gpui::test]
fn replace_one_uses_the_groups_and_selects_the_next(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "k=1 k=2");
    f.regex(cx, r"k=(\d)");
    f.replacement(cx, "v$1$1");
    f.run(cx, "F_REPLACE");
    assert_eq!(f.text(cx), "v11 k=2");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 4), Pos::new(0, 7))));
    f.run(cx, "F_REPLACE");
    assert_eq!(f.text(cx), "v11 v22");
    cx.simulate_keystrokes("ctrl-z");
    assert_eq!(f.text(cx), "v11 k=2");
}

#[gpui::test]
fn a_replacement_that_ends_with_a_backslash_is_refused(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.regex(cx, "b");
    f.replacement(cx, "x\\");
    f.run(cx, "F_REPLACE");
    assert_eq!(f.text(cx), "abc");
    let (_, detail) = cx.pending_prompt().expect("the error box");
    assert!(detail.contains("正規表現の構文に誤りがあります。"));
}

#[gpui::test]
fn the_bookmark_pattern_command_uses_the_regular_expression(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc\r\n123\r\nx45\r\n678");
    f.regex(cx, r"^\d+$");
    f.run(cx, "F_BOOKMARK_PATTERN");
    assert_eq!(f.marked(cx), vec![1, 3]);
}

#[gpui::test]
fn the_search_mark_command_drops_the_regular_expression(cx: &mut TestAppContext) {
    // `m_sCurSearchOption.bRegularExp = false; bWordOnly = false;` in `Command_SEARCH_CLEARMARK`
    let (f, cx) = setup(cx, "a.c abc");
    f.regex(cx, "x");
    f.view
        .update(cx, |v, _| v.cursor.select(Pos::new(0, 0), Pos::new(0, 3)));
    f.run(cx, "F_SEARCH_CLEARMARK");
    let options = f.view.read_with(cx, |v, _| v.search.options());
    assert!(!options.regex);
}

#[gpui::test]
fn an_empty_match_selected_by_replace_is_stepped_over_too(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab\r\ncd\r\nef");
    f.regex(cx, "^");
    f.replacement(cx, ">");
    f.run(cx, "F_REPLACE");
    assert_eq!(f.text(cx), ">ab\r\ncd\r\nef");
    assert_eq!(f.caret(cx), Pos::new(1, 0));
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.caret(cx), Pos::new(2, 0));
}
