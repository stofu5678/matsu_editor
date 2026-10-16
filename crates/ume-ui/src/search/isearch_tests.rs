//! Tests of the incremental search (`isearch.rs`), in GPUI's headless test window. The text is
//! typed through the input handler and the keys go through the key map, as in the real window.
//!
//! The rules are those of `sakura_core/view/CEditView_Cmdisrch.cpp`.

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use ume_commands::sakura::FUNCTIONS;
use ume_core::buffer::{Document, Pos};
use ume_core::model::DocumentModel;

use super::isearch::{Translation, translate};
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

    fn caret_to(&self, cx: &mut VisualTestContext, pos: Pos) {
        self.view.update(cx, |v, _| v.cursor.select(pos, pos));
    }

    fn run(&self, cx: &mut VisualTestContext, name: &str) {
        let code = FUNCTIONS.iter().find(|f| f.name == name).unwrap().code;
        self.view
            .update_in(cx, |v, window, cx| v.run_function(code, &[], window, cx));
    }

    /// The status bar text of the mode, if it is on.
    fn message(&self, cx: &mut VisualTestContext) -> Option<String> {
        self.view
            .read_with(cx, |v, _| v.isearch().map(|a| a.message().to_owned()))
    }

    fn key(&self, cx: &mut VisualTestContext) -> Option<String> {
        self.view
            .read_with(cx, |v, _| v.isearch().map(|a| a.key().to_owned()))
    }

    fn active(&self, cx: &mut VisualTestContext) -> bool {
        self.view.read_with(cx, |v, _| v.isearch().is_some())
    }

    fn pattern(&self, cx: &mut VisualTestContext) -> String {
        self.view
            .read_with(cx, |v, _| v.search_state().pattern().to_owned())
    }
}

fn range(a: (usize, usize), b: (usize, usize)) -> Option<(Pos, Pos)> {
    Some((Pos::new(a.0, a.1), Pos::new(b.0, b.1)))
}

#[test]
fn the_translation_table_is_sakuras() {
    assert_eq!(translate("F_ISEARCH_NEXT"), Translation::Enter);
    assert_eq!(translate("F_ISEARCH_REGEXP_PREV"), Translation::Enter);
    assert_eq!(translate("F_WCHAR"), Translation::AddChar(None));
    assert_eq!(translate("F_IME_CHAR"), Translation::AddChar(None));
    assert_eq!(translate("F_INSTEXT_W"), Translation::AddStr);
    assert_eq!(translate("F_INDENT_TAB"), Translation::AddChar(Some('\t')));
    assert_eq!(
        translate("F_UNINDENT_TAB"),
        Translation::AddChar(Some('\t'))
    );
    assert_eq!(translate("F_INDENT_SPACE"), Translation::AddChar(Some(' ')));
    assert_eq!(
        translate("F_UNINDENT_SPACE"),
        Translation::AddChar(Some(' '))
    );
    assert_eq!(translate("F_DELETE_BACK"), Translation::DelBack);
    assert_eq!(translate("F_LEFT"), Translation::Exit);
    assert_eq!(translate("F_SEARCH_NEXT"), Translation::Exit);
}

#[gpui::test]
fn forward_search_extends_the_match_as_characters_come(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo bar foo baz\r\nfoo");
    f.run(cx, "F_ISEARCH_NEXT");
    assert!(f.active(cx));
    assert_eq!(f.message(cx).as_deref(), Some("I-Search: "));
    cx.simulate_input("b");
    cx.simulate_input("a");
    assert_eq!(f.selection(cx), range((0, 4), (0, 6)));
    // the caret is at the start of the match, as in a normal search
    assert_eq!(f.caret(cx), Pos::new(0, 4));
    cx.simulate_input("r");
    assert_eq!(f.selection(cx), range((0, 4), (0, 7)));
    assert_eq!(f.message(cx).as_deref(), Some("I-Search: bar"));
    // the text is not touched
    assert_eq!(f.text(cx), "foo bar foo baz\r\nfoo");
}

#[gpui::test]
fn the_search_ignores_case_and_starts_at_the_caret(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "Foo foo\r\nFOO");
    f.caret_to(cx, Pos::new(0, 1));
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_input("FOO");
    // from the caret on: the "foo" of the first line, not the "Foo" before the caret
    assert_eq!(f.selection(cx), range((0, 4), (0, 7)));
}

#[gpui::test]
fn the_repeated_command_goes_to_the_next_match_and_wraps_after_a_miss(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo bar foo\r\nfoo");
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_input("foo");
    assert_eq!(f.selection(cx), range((0, 0), (0, 3)));
    f.run(cx, "F_ISEARCH_NEXT");
    assert_eq!(f.selection(cx), range((0, 8), (0, 11)));
    f.run(cx, "F_ISEARCH_NEXT");
    assert_eq!(f.selection(cx), range((1, 0), (1, 3)));
    // nothing after the last match: it says so and keeps the selection
    f.run(cx, "F_ISEARCH_NEXT");
    assert_eq!(f.selection(cx), range((1, 0), (1, 3)));
    assert_eq!(
        f.message(cx).as_deref(),
        Some("I-Search: foo (見つかりません)")
    );
    // the command once more begins again at the top of the document
    f.run(cx, "F_ISEARCH_NEXT");
    assert_eq!(f.selection(cx), range((0, 0), (0, 3)));
    assert_eq!(f.message(cx).as_deref(), Some("I-Search: foo"));
}

#[gpui::test]
fn backward_search_looks_up_from_the_caret(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo bar foo\r\nfoo baz");
    f.caret_to(cx, Pos::new(1, 7));
    f.run(cx, "F_ISEARCH_PREV");
    assert_eq!(f.message(cx).as_deref(), Some("I-Search Backward: "));
    cx.simulate_input("foo");
    assert_eq!(f.selection(cx), range((1, 0), (1, 3)));
    assert_eq!(f.caret(cx), Pos::new(1, 0));
    assert_eq!(f.message(cx).as_deref(), Some("I-Search Backward: foo"));
    f.run(cx, "F_ISEARCH_PREV");
    assert_eq!(f.selection(cx), range((0, 8), (0, 11)));
    f.run(cx, "F_ISEARCH_PREV");
    assert_eq!(f.selection(cx), range((0, 0), (0, 3)));
    // past the first match: no match, then it begins again at the end
    f.run(cx, "F_ISEARCH_PREV");
    assert_eq!(f.selection(cx), range((0, 0), (0, 3)));
    assert!(f.message(cx).unwrap().ends_with("(見つかりません)"));
    f.run(cx, "F_ISEARCH_PREV");
    assert_eq!(f.selection(cx), range((1, 0), (1, 3)));
}

#[gpui::test]
fn a_longer_backward_pattern_keeps_looking_from_the_current_match(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "goo gob");
    f.caret_to(cx, Pos::new(0, 7));
    f.run(cx, "F_ISEARCH_PREV");
    cx.simulate_input("go");
    assert_eq!(f.selection(cx), range((0, 4), (0, 6)));
    // "gob" is no "goo": the search goes on from the end of the selection and finds "goo"
    cx.simulate_input("o");
    assert_eq!(f.selection(cx), range((0, 0), (0, 3)));
}

#[gpui::test]
fn the_direction_can_change_with_the_other_command(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a1 a2 a3");
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_input("a");
    assert_eq!(f.selection(cx), range((0, 0), (0, 1)));
    f.run(cx, "F_ISEARCH_NEXT");
    f.run(cx, "F_ISEARCH_NEXT");
    assert_eq!(f.selection(cx), range((0, 6), (0, 7)));
    // the backward command in the same mode goes back
    f.run(cx, "F_ISEARCH_PREV");
    assert_eq!(f.selection(cx), range((0, 3), (0, 4)));
    assert_eq!(f.message(cx).as_deref(), Some("I-Search Backward: a"));
}

#[gpui::test]
fn japanese_text_is_found_by_chars(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "あいう えお あいえ\r\n日本語の検索");
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_input("あい");
    assert_eq!(f.selection(cx), range((0, 0), (0, 6)));
    f.run(cx, "F_ISEARCH_NEXT");
    assert_eq!(f.selection(cx), range((0, 17), (0, 23)));
    cx.simulate_input("え");
    assert_eq!(f.selection(cx), range((0, 17), (0, 26)));
    assert_eq!(f.message(cx).as_deref(), Some("I-Search: あいえ"));
    // the key is "あいえ" now: a repeat finds nothing more
    f.run(cx, "F_ISEARCH_NEXT");
    assert!(f.message(cx).unwrap().ends_with("(見つかりません)"));
}

#[gpui::test]
fn text_committed_by_the_ime_is_added_to_the_pattern(cx: &mut TestAppContext) {
    use gpui::EntityInputHandler as _;
    let (f, cx) = setup(cx, "日本語の検索\r\n検索");
    f.caret_to(cx, Pos::new(0, 9));
    f.run(cx, "F_ISEARCH_NEXT");
    // a composition is not shown, and its commit arrives as one string
    f.view.update_in(cx, |v, window, cx| {
        v.replace_and_mark_text_in_range(None, "けんさく", None, window, cx);
        v.replace_text_in_range(None, "検索", window, cx);
    });
    assert_eq!(f.key(cx).as_deref(), Some("検索"));
    assert_eq!(f.selection(cx), range((0, 12), (0, 18)));
    assert_eq!(f.text(cx), "日本語の検索\r\n検索");
}

#[gpui::test]
fn backspace_goes_back_through_the_history(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ga gb gbb");
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_input("g");
    assert_eq!(f.selection(cx), range((0, 0), (0, 1)));
    cx.simulate_input("b");
    assert_eq!(f.selection(cx), range((0, 3), (0, 5)));
    cx.simulate_input("b");
    assert_eq!(f.selection(cx), range((0, 6), (0, 9)));
    cx.simulate_keystrokes("backspace");
    assert_eq!(f.selection(cx), range((0, 3), (0, 5)));
    assert_eq!(f.key(cx).as_deref(), Some("gb"));
    assert_eq!(f.message(cx).as_deref(), Some("I-Search: gb"));
    cx.simulate_keystrokes("backspace");
    assert_eq!(f.selection(cx), range((0, 0), (0, 1)));
    assert_eq!(f.key(cx).as_deref(), Some("g"));
    // back to where the mode began: no selection, the caret there, nothing shown
    cx.simulate_keystrokes("backspace");
    assert_eq!(f.selection(cx), None);
    assert_eq!(f.caret(cx), Pos::new(0, 0));
    assert_eq!(f.message(cx).as_deref(), Some("I-Search: "));
    // nothing more to undo; the mode stays
    cx.simulate_keystrokes("backspace");
    assert!(f.active(cx));
    assert_eq!(f.text(cx), "ga gb gbb");
}

#[gpui::test]
fn backspace_after_the_repeated_command_keeps_the_pattern(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab ab ab");
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_input("ab");
    f.run(cx, "F_ISEARCH_NEXT");
    assert_eq!(f.selection(cx), range((0, 3), (0, 5)));
    // that step only moved to the next match: Backspace undoes it and keeps "ab"
    cx.simulate_keystrokes("backspace");
    assert_eq!(f.selection(cx), range((0, 0), (0, 2)));
    assert_eq!(f.key(cx).as_deref(), Some("ab"));
    cx.simulate_keystrokes("backspace");
    assert_eq!(f.key(cx).as_deref(), Some("a"));
}

#[gpui::test]
fn no_match_is_said_in_the_status_bar_and_backspace_returns(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "alpha beta");
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_input("be");
    assert_eq!(f.selection(cx), range((0, 6), (0, 8)));
    cx.simulate_input("x");
    // the old match stays selected
    assert_eq!(f.selection(cx), range((0, 6), (0, 8)));
    assert_eq!(
        f.message(cx).as_deref(),
        Some("I-Search: bex (見つかりません)")
    );
    let notice = f.view.read_with(cx, |v, cx| v.status(cx).notice);
    assert_eq!(notice.as_deref(), Some("I-Search: bex (見つかりません)"));
    cx.simulate_keystrokes("backspace");
    assert_eq!(f.selection(cx), range((0, 6), (0, 8)));
    assert_eq!(f.message(cx).as_deref(), Some("I-Search: be"));
}

#[gpui::test]
fn the_regular_expression_search_uses_the_pattern_as_one(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "go goo.bar gooo");
    f.run(cx, "F_ISEARCH_REGEXP_NEXT");
    assert_eq!(f.message(cx).as_deref(), Some("[RegExp] I-Search: "));
    cx.simulate_input("o+");
    assert_eq!(f.selection(cx), range((0, 1), (0, 2)));
    assert_eq!(f.message(cx).as_deref(), Some("[RegExp] I-Search: o+"));
    f.run(cx, "F_ISEARCH_REGEXP_NEXT");
    assert_eq!(f.selection(cx), range((0, 4), (0, 6)));
    // case is ignored here too
    cx.simulate_keystrokes("backspace backspace backspace");
    cx.simulate_input("G.O");
    assert_eq!(f.selection(cx), range((0, 3), (0, 6)));
    let options = f.view.read_with(cx, |v, _| v.search_state().options());
    assert!(options.regex && !options.match_case);
}

#[gpui::test]
fn a_regular_expression_that_does_not_compile_finds_nothing_without_a_box(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a(b");
    f.run(cx, "F_ISEARCH_REGEXP_NEXT");
    cx.simulate_input("a");
    assert_eq!(f.selection(cx), range((0, 0), (0, 1)));
    cx.simulate_input("(");
    assert_eq!(f.selection(cx), range((0, 0), (0, 1)));
    assert!(f.message(cx).unwrap().ends_with("(見つかりません)"));
    // typed on, the pattern compiles again
    cx.simulate_input("b)");
    assert!(f.message(cx).unwrap().ends_with("(見つかりません)"));
    cx.simulate_keystrokes("backspace backspace backspace");
    cx.simulate_input("\\(");
    assert_eq!(f.selection(cx), range((0, 0), (0, 2)));
}

#[gpui::test]
fn the_plain_search_takes_regex_characters_as_they_are(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "aab a.b");
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_input("a.b");
    assert_eq!(f.selection(cx), range((0, 4), (0, 7)));
}

#[gpui::test]
fn switching_the_mode_keeps_the_pattern_and_the_start_position(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "x goo gooo");
    f.caret_to(cx, Pos::new(0, 1));
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_input("go");
    assert_eq!(f.selection(cx), range((0, 2), (0, 4)));
    f.run(cx, "F_ISEARCH_REGEXP_NEXT");
    // only the mode changed: the selection went, the pattern stayed
    assert_eq!(f.selection(cx), None);
    assert_eq!(f.key(cx).as_deref(), Some("go"));
    assert_eq!(f.message(cx).as_deref(), Some("[RegExp] I-Search: "));
    // the command again searches for the old pattern
    f.run(cx, "F_ISEARCH_REGEXP_NEXT");
    assert_eq!(f.selection(cx), range((0, 2), (0, 4)));
    // the position of the jump back is where the first mode began
    let start = f.view.read_with(cx, |v, _| v.search_state().start_pos());
    assert_eq!(start, Some(Pos::new(0, 1)));
}

#[gpui::test]
fn the_mode_begins_without_a_selection(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo bar");
    f.view
        .update(cx, |v, _| v.cursor.select(Pos::new(0, 0), Pos::new(0, 3)));
    f.run(cx, "F_ISEARCH_NEXT");
    assert_eq!(f.selection(cx), None);
}

#[gpui::test]
fn enter_ends_the_mode_and_the_pattern_goes_on_with_f3(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo bar foo\r\nfoo");
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_input("foo");
    assert_eq!(f.selection(cx), range((0, 0), (0, 3)));
    cx.simulate_keystrokes("enter");
    assert!(!f.active(cx));
    // Enter ends the mode and types nothing
    assert_eq!(f.text(cx), "foo bar foo\r\nfoo");
    assert_eq!(f.message(cx), None);
    assert_eq!(f.pattern(cx), "foo");
    assert!(f.view.read_with(cx, |v, _| v.search_state().mark()));
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.selection(cx), range((0, 8), (0, 11)));
}

#[gpui::test]
fn the_options_of_the_search_stay_for_the_next_search(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "Foo foo");
    f.run(cx, "F_ISEARCH_REGEXP_NEXT");
    cx.simulate_input("F.o");
    cx.simulate_keystrokes("enter");
    let options = f.view.read_with(cx, |v, _| v.search_state().options());
    assert!(options.regex && !options.match_case);
    // F3 goes on as a regular expression, ignoring case
    f.run(cx, "F_SEARCH_NEXT");
    assert_eq!(f.selection(cx), range((0, 4), (0, 7)));
}

#[gpui::test]
fn leaving_without_a_search_drops_the_pattern(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo");
    f.view.update(cx, |v, _| v.search.set_pattern("foo"));
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_keystrokes("enter");
    // `ISearchExit` clears the key when no search was made
    assert_eq!(f.pattern(cx), "");
}

#[gpui::test]
fn the_old_pattern_is_searched_by_the_command_twice(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "xx foo foo");
    f.view.update(cx, |v, _| v.search.set_pattern("foo"));
    f.run(cx, "F_ISEARCH_NEXT");
    // the first press of the command again searches for the key of the last search
    f.run(cx, "F_ISEARCH_NEXT");
    assert_eq!(f.selection(cx), range((0, 3), (0, 6)));
    assert!(f.active(cx));
}

#[gpui::test]
fn another_command_ends_the_mode_and_then_runs(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo bar");
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_input("bar");
    assert_eq!(f.selection(cx), range((0, 4), (0, 7)));
    cx.simulate_keystrokes("left");
    assert!(!f.active(cx));
    assert_eq!(f.pattern(cx), "bar");
    // the move ran: the selection is gone and the caret moved
    assert_eq!(f.selection(cx), None);
    assert_eq!(f.text(cx), "foo bar");
}

#[gpui::test]
fn a_function_that_is_not_there_yet_ends_the_mode_too(cx: &mut TestAppContext) {
    // Esc is `F_CANCEL_MODE`
    let (f, cx) = setup(cx, "foo bar");
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_input("foo");
    cx.simulate_keystrokes("escape");
    assert!(!f.active(cx));
    assert_eq!(f.pattern(cx), "foo");
}

#[gpui::test]
fn tab_and_space_are_chars_of_the_pattern(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab a b\ta\tb");
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_input("a b");
    assert_eq!(f.selection(cx), range((0, 3), (0, 6)));
    cx.simulate_keystrokes("backspace backspace backspace");
    cx.simulate_keystrokes("a tab");
    assert_eq!(f.key(cx).as_deref(), Some("a\t"));
    assert_eq!(f.selection(cx), range((0, 7), (0, 9)));
    assert_eq!(f.text(cx), "ab a b\ta\tb");
}

#[gpui::test]
fn the_history_is_bounded(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &"a".repeat(400));
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_input(&"a".repeat(300));
    let (count, len) = f.view.read_with(cx, |v, _| {
        let a = v.isearch().unwrap();
        (a.count(), a.key().len())
    });
    assert_eq!(len, 300);
    assert!(count < 256, "{count}");
    // Backspace still works down to the oldest step that is kept
    cx.simulate_keystrokes("backspace");
    assert_eq!(f.key(cx).map(|k| k.len()), Some(299));
}

#[gpui::test]
fn the_ui_commands_are_registered_under_sakuras_functions(cx: &mut TestAppContext) {
    let _ = cx;
    let registry = crate::ui_command::UiRegistry::global();
    for (id, name) in [
        ("isearch_next", "F_ISEARCH_NEXT"),
        ("isearch_prev", "F_ISEARCH_PREV"),
        ("isearch_regexp_next", "F_ISEARCH_REGEXP_NEXT"),
        ("isearch_regexp_prev", "F_ISEARCH_REGEXP_PREV"),
    ] {
        let reg = registry.get(id).unwrap();
        assert_eq!(reg.sakura, name);
    }
}

#[gpui::test]
fn the_pattern_typed_in_the_mode_is_not_recorded_as_an_edit(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "xabx");
    cx.simulate_keystrokes("ctrl-shift-m");
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_input("ab");
    cx.simulate_keystrokes("enter");
    cx.simulate_keystrokes("ctrl-shift-m");
    // the replay must not type "ab" or a line break into the text
    cx.simulate_keystrokes("ctrl-shift-l");
    assert_eq!(f.text(cx), "xabx");
}

#[gpui::test]
fn the_function_dispatcher_gives_the_text_to_the_mode(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "one two");
    f.run(cx, "F_ISEARCH_NEXT");
    let code = FUNCTIONS
        .iter()
        .find(|f| f.name == "F_INSTEXT_W")
        .unwrap()
        .code;
    let ran = f.view.update_in(cx, |v, window, cx| {
        v.run_function(
            code,
            &[ume_commands::command::Arg::Text("two".to_owned())],
            window,
            cx,
        )
    });
    assert!(ran);
    assert_eq!(f.key(cx).as_deref(), Some("two"));
    assert_eq!(f.selection(cx), range((0, 4), (0, 7)));
}

#[gpui::test]
fn an_empty_match_at_the_start_is_found_again_after_wrapping(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, "F_ISEARCH_REGEXP_NEXT");
    cx.simulate_input("^");
    assert_eq!(f.caret(cx), Pos::new(0, 0));
    // no other match: it says so, and the next one begins again at the top
    f.run(cx, "F_ISEARCH_REGEXP_NEXT");
    assert!(f.message(cx).unwrap().ends_with("(見つかりません)"));
    f.run(cx, "F_ISEARCH_REGEXP_NEXT");
    assert_eq!(f.message(cx).as_deref(), Some("[RegExp] I-Search: ^"));
    assert_eq!(f.caret(cx), Pos::new(0, 0));
}
