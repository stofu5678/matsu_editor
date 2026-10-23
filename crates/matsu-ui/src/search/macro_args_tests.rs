//! The arguments a macro gives to the search and replace commands (`super::macro_args`).
//!
//! Sakura: `CMacro::HandleCommand` in `sakura_core/macro/CMacro.cpp`, the cases `F_SEARCH_NEXT`,
//! `F_SEARCH_PREV`, `F_REPLACE` and `F_REPLACE_ALL`.

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;
use matsu_core::buffer::{Document, Pos};
use matsu_core::model::DocumentModel;
use matsu_core::search::ReplaceTarget;

use super::history;
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

fn text(s: &str) -> Arg {
    Arg::Text(s.to_owned())
}

impl Fixture {
    fn text(&self, cx: &mut VisualTestContext) -> String {
        self.document.read_with(cx, |shared, _| {
            let doc = shared.doc();
            doc.text_between(Pos::new(0, 0), Pos::new(doc.line_count(), 0))
                .unwrap()
        })
    }

    /// Runs a Sakura function with the arguments of a macro; the result is what a key macro
    /// sees (false stops it).
    fn call(&self, cx: &mut VisualTestContext, name: &str, args: &[Arg]) -> bool {
        let code = FUNCTIONS.iter().find(|f| f.name == name).unwrap().code;
        let ok = self
            .view
            .update_in(cx, |v, window, cx| v.run_function(code, args, window, cx));
        cx.run_until_parked();
        ok
    }

    fn selection(&self, cx: &mut VisualTestContext) -> Option<(Pos, Pos)> {
        self.view.read_with(cx, |v, _| v.cursor.selection())
    }
}

#[gpui::test]
fn the_string_and_the_options_of_the_call_become_the_search(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo Foo");
    // 2: match case, 1: whole word, 0x20: search again from the top, 0x08: tell when not found
    assert!(f.call(cx, "F_SEARCH_NEXT", &[text("Foo"), Arg::Int(2 | 0x20)]));
    assert_eq!(f.selection(cx), Some((Pos::new(0, 4), Pos::new(0, 7))));
    f.view.read_with(cx, |v, _| {
        let o = v.search.options();
        assert_eq!(v.search.pattern(), "Foo");
        assert!(o.match_case && !o.whole_word && !o.regex && o.wrap);
        // the flags that the call does not give are off, also the ones that are on by default
        assert!(!o.notify_not_found && !o.auto_close);
    });
    // the next call without a string keeps the string and the options it had
    assert!(f.call(cx, "F_SEARCH_PREV", &[text(""), Arg::Int(2)]));
    f.view
        .read_with(cx, |v, _| assert_eq!(v.search.pattern(), "Foo"));
}

#[gpui::test]
fn a_search_that_finds_nothing_says_nothing_unless_asked(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo");
    assert!(f.call(cx, "F_SEARCH_NEXT", &[text("zzz"), Arg::Int(0)]));
    assert!(!cx.has_pending_prompt());
    // 0x08: the box of `AlertNotFound`
    assert!(f.call(cx, "F_SEARCH_NEXT", &[text("zzz"), Arg::Int(0x08)]));
    assert!(cx.has_pending_prompt());
}

#[gpui::test]
fn a_regular_expression_that_does_not_compile_shows_a_box_and_changes_nothing(
    cx: &mut TestAppContext,
) {
    let (f, cx) = setup(cx, "a(b");
    assert!(f.call(cx, "F_SEARCH_NEXT", &[text("a"), Arg::Int(0)]));
    // `CheckRegexpSyntax` shows the box and the macro goes on without running the command
    assert!(f.call(cx, "F_SEARCH_NEXT", &[text("("), Arg::Int(4)]));
    let (message, _) = cx.pending_prompt().expect("the box");
    assert_eq!(message, "正規表現エラー");
    f.view.read_with(cx, |v, _| {
        assert_eq!(v.search.pattern(), "a");
        assert!(!v.search.options().regex);
    });
}

#[gpui::test]
fn replace_takes_the_replacement_and_the_target_from_the_flags(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab ab");
    // 0x200: insert after the match; 0x80 is for the selection of "replace all" only
    assert!(f.call(
        cx,
        "F_REPLACE_ALL",
        &[text("ab"), text("-"), Arg::Int(0x200 | 0x80)]
    ));
    assert_eq!(f.text(cx), "ab- ab-");
    f.view.read_with(cx, |v, _| {
        assert_eq!(v.search.replace.text, "-");
        assert_eq!(v.search.replace.target, ReplaceTarget::InsertAfter);
        assert!(v.search.replace.in_selection);
    });
    // `F_REPLACE` never takes the selection as its area
    assert!(f.call(
        cx,
        "F_REPLACE",
        &[text("ab"), text("+"), Arg::Int(0x100 | 0x80)]
    ));
    f.view.read_with(cx, |v, _| {
        assert_eq!(v.search.replace.target, ReplaceTarget::InsertBefore);
        assert!(!v.search.replace.in_selection);
    });
}

#[gpui::test]
fn replace_without_a_string_or_a_replacement_stops_the_macro(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab");
    // `STR_ERR_DLGMACRO09`, `STR_ERR_DLGMACRO10`
    assert!(!f.call(cx, "F_REPLACE_ALL", &[text(""), text("x"), Arg::Int(0)]));
    assert!(!f.call(cx, "F_REPLACE_ALL", &[text("a")]));
    assert!(!f.call(cx, "F_REPLACE", &[text("a")]));
    assert_eq!(f.text(cx), "ab");
    // the clipboard replacement (0x40) is not there yet: it stops instead of replacing by text
    assert!(!f.call(cx, "F_REPLACE_ALL", &[text("a"), text("x"), Arg::Int(0x40)]));
    assert_eq!(f.text(cx), "ab");
}

#[gpui::test]
fn the_strings_go_to_the_history_unless_the_call_says_not_to(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab");
    assert!(f.call(cx, "F_SEARCH_NEXT", &[text("a"), Arg::Int(0)]));
    assert!(f.call(cx, "F_REPLACE", &[text("b"), text("c"), Arg::Int(0)]));
    // 0x800: not in the history
    assert!(f.call(cx, "F_SEARCH_NEXT", &[text("hidden"), Arg::Int(0x800)]));
    assert!(f.call(
        cx,
        "F_REPLACE",
        &[text("hidden2"), text("hidden3"), Arg::Int(0x800)]
    ));
    cx.update(|_, app| {
        assert_eq!(history::search_keys(app), ["b", "a"]);
        assert_eq!(history::replace_keys(app), ["c"]);
    });
}

#[gpui::test]
fn the_settings_are_put_back_when_the_call_asks_for_it(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo bar bar");
    assert!(f.call(cx, "F_SEARCH_NEXT", &[text("foo"), Arg::Int(2)]));
    // 0x1000: back to "foo" with its options after the call, and the history is left alone
    assert!(f.call(
        cx,
        "F_REPLACE_ALL",
        &[text("bar"), text("baz"), Arg::Int(0x1000)]
    ));
    assert_eq!(f.text(cx), "foo baz baz");
    f.view.read_with(cx, |v, _| {
        assert_eq!(v.search.pattern(), "foo");
        assert!(v.search.options().match_case);
        assert_eq!(v.search.replace.text, "");
    });
    cx.update(|_, app| assert_eq!(history::search_keys(app), ["foo"]));
}
