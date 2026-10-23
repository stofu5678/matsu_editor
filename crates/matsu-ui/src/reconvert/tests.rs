//! Tests of the text and the range handed to the IME, and of the flow with a stand-in IME.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_core::buffer::{Document, Pos};
use matsu_core::model::DocumentModel;

use super::*;
use crate::document::SharedDocument;

/// `plan` over a text with lines split at `\n`.
fn plan_of(text: &str, caret: Pos, selection: Option<(Pos, Pos)>) -> Option<Plan> {
    let lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    plan(caret, selection, lines.len(), &|y| lines.get(y).cloned())
}

fn text_of(plan: &Plan) -> String {
    String::from_utf16(&plan.request.text).unwrap()
}

#[test]
fn the_caret_alone_hands_over_the_line_with_an_empty_range() {
    let p = plan_of("日本語を入力", Pos::new(0, 9), None).unwrap();
    assert_eq!(text_of(&p), "日本語を入力");
    // the range is in UTF-16 units: three chars of three bytes
    assert_eq!(
        (p.line, p.index, p.request.start, p.request.len),
        (0, 0, 3, 0)
    );
}

#[test]
fn a_selection_in_one_line_is_the_range() {
    let p = plan_of(
        "ab日本語cd",
        Pos::new(0, 8),
        Some((Pos::new(0, 2), Pos::new(0, 8))),
    )
    .unwrap();
    assert_eq!((p.request.start, p.request.len), (2, 2));
}

#[test]
fn units_are_utf16() {
    // the pair is two units, so the range after it starts at 3
    let p = plan_of("a😀b", Pos::new(0, 5), None).unwrap();
    assert_eq!(p.request.start, 3);
    let p = plan_of(
        "a😀b",
        Pos::new(0, 1),
        Some((Pos::new(0, 1), Pos::new(0, 5))),
    )
    .unwrap();
    assert_eq!((p.request.start, p.request.len), (1, 2));
}

#[test]
fn an_empty_line_and_a_missing_line_hand_over_nothing() {
    assert_eq!(plan_of("", Pos::new(0, 0), None), None);
    assert_eq!(plan_of("a\n\nb", Pos::new(1, 0), None), None);
    assert_eq!(plan_of("a", Pos::new(5, 0), None), None);
}

#[test]
fn a_caret_past_the_end_of_the_line_hands_over_the_end() {
    let p = plan_of("abc", Pos::new(0, 9), None).unwrap();
    assert_eq!((p.request.start, p.request.len), (3, 0));
}

#[test]
fn a_selection_of_several_lines_is_cut_to_the_line_of_the_caret() {
    let text = "one\ntwo\nthree\nfour";
    let sel = Some((Pos::new(0, 1), Pos::new(3, 2)));
    // caret at the end of the selection (line 3, the last): from its start to the end
    let p = plan_of(text, Pos::new(3, 2), sel).unwrap();
    assert_eq!((p.line, p.request.start, p.request.len), (3, 0, 2));
    // caret at the start (line 0, the first): from the start of the range to the end of the line
    let p = plan_of(text, Pos::new(0, 1), sel).unwrap();
    assert_eq!((p.line, p.request.start, p.request.len), (0, 1, 2));
    // caret in a line in the middle: the whole line
    let p = plan_of(text, Pos::new(1, 1), sel).unwrap();
    assert_eq!((p.line, p.request.start, p.request.len), (1, 0, 3));
}

#[test]
fn a_caret_at_the_end_of_the_upper_line_takes_the_next_line() {
    // Shift+Up from the end of a line: the selection starts in the caret's line, at its end
    let p = plan_of(
        "abc\ndef",
        Pos::new(0, 3),
        Some((Pos::new(0, 3), Pos::new(1, 2))),
    )
    .unwrap();
    assert_eq!((p.line, text_of(&p)), (1, "def".to_owned()));
    assert_eq!((p.request.start, p.request.len), (0, 2));
}

#[test]
fn a_caret_at_the_start_of_the_lower_line_takes_the_previous_line() {
    let p = plan_of(
        "abc\ndef",
        Pos::new(1, 0),
        Some((Pos::new(0, 1), Pos::new(1, 0))),
    )
    .unwrap();
    assert_eq!((p.line, text_of(&p)), (0, "abc".to_owned()));
    assert_eq!((p.request.start, p.request.len), (1, 2));
    // nothing before the first line
    assert_eq!(
        plan_of(
            "abc",
            Pos::new(0, 0),
            Some((Pos::new(0, 0), Pos::new(0, 0)))
        )
        .map(|p| p.line),
        Some(0)
    );
}

#[test]
fn only_200_units_around_the_range_are_handed_over() {
    let line = "あ".repeat(500);
    let at = |chars: usize| Pos::new(0, chars * 3);
    let p = plan_of(&line, at(250), None).unwrap();
    // 200 before the range, and up to 200 after it plus the char that crosses the limit
    assert_eq!(p.index, 50);
    assert_eq!(p.request.start, 200);
    assert_eq!(p.request.text.len(), 200 + 201);
    // near the start: from the start of the line
    let p = plan_of(&line, at(100), None).unwrap();
    assert_eq!((p.index, p.request.start), (0, 100));
    // near the end: to the end of the line
    let p = plan_of(&line, at(450), None).unwrap();
    assert_eq!(p.index + p.request.text.len(), 500);
}

#[test]
fn the_cut_does_not_split_a_pair() {
    // a pair at the place where the text is cut at the start
    let line = format!("{}{}", "😀".repeat(150), "x");
    let p = plan_of(&line, Pos::new(0, line.len()), None).unwrap();
    assert_eq!(p.index % 2, 0, "starts on a pair");
    assert!(String::from_utf16(&p.request.text).is_ok());
}

#[test]
fn the_range_the_ime_settled_on_becomes_the_selection_positions() {
    assert_eq!(byte_at("ab日本語", 0), 0);
    assert_eq!(byte_at("ab日本語", 2), 2);
    assert_eq!(byte_at("ab日本語", 4), 8);
    assert_eq!(byte_at("ab日本語", 5), 11);
    assert_eq!(byte_at("a😀b", 1), 1);
    assert_eq!(byte_at("a😀b", 2), 1, "inside a pair: the start of it");
    assert_eq!(byte_at("a😀b", 3), 5);
    assert_eq!(byte_at("ab", 9), 2);
}

// -- the flow, with a stand-in for the IME --

/// Widens the range to the given one and records what it was asked.
struct Fake {
    widen_to: Option<(usize, usize)>,
    fail: Option<Error>,
    calls: RefCell<Vec<&'static str>>,
}

impl Ime for Fake {
    fn query(&self, reconvert: &mut Reconvert) -> Result<(), Error> {
        self.calls.borrow_mut().push("query");
        if let Some(error) = self.fail.clone() {
            return Err(error);
        }
        if let Some((start, len)) = self.widen_to {
            reconvert.set_range(start, len);
        }
        Ok(())
    }

    fn set(&self, _: &Reconvert) -> Result<(), Error> {
        self.calls.borrow_mut().push("set");
        Ok(())
    }
}

fn fake(widen_to: Option<(usize, usize)>, fail: Option<Error>) -> Rc<Fake> {
    Rc::new(Fake {
        widen_to,
        fail,
        calls: RefCell::default(),
    })
}

fn setup<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
) -> (Entity<EditorView>, &'a mut VisualTestContext) {
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    cx.add_window_view(move |window, cx| {
        let view = EditorView::new(document, cx);
        window.focus(&view.focus_handle, cx);
        view
    })
}

fn start(view: &Entity<EditorView>, cx: &mut VisualTestContext, ime: Rc<Fake>) {
    view.update_in(cx, |v, window, cx| v.start_reconvert(ime, window, cx));
    cx.run_until_parked();
}

#[gpui::test]
fn the_range_the_ime_widens_to_is_selected_with_the_caret_at_its_start(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "これは日本語です");
    view.update(cx, |v, _| v.cursor.move_to(Pos::new(0, 15), false));
    // the IME widens the empty range at the caret to the word "日本語" (units 3..6)
    let ime = fake(Some((3, 3)), None);
    start(&view, cx, ime.clone());
    view.read_with(cx, |v, _| {
        assert_eq!(
            v.cursor.selection(),
            Some((Pos::new(0, 9), Pos::new(0, 18)))
        );
        assert_eq!(
            v.cursor.caret(),
            Pos::new(0, 9),
            "the caret is at the start"
        );
    });
    assert_eq!(*ime.calls.borrow(), ["query", "set"]);
}

#[gpui::test]
fn a_range_that_is_not_widened_stays_as_the_selection(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abcdef");
    view.update(cx, |v, _| v.cursor.select(Pos::new(0, 1), Pos::new(0, 4)));
    start(&view, cx, fake(None, None));
    view.read_with(cx, |v, _| {
        assert_eq!(v.cursor.selection(), Some((Pos::new(0, 1), Pos::new(0, 4))));
        assert_eq!(v.cursor.caret(), Pos::new(0, 1));
    });
}

#[gpui::test]
fn nothing_is_asked_where_sakura_asks_for_nothing(cx: &mut TestAppContext) {
    // an empty line
    let (view, cx) = setup(cx, "");
    let ime = fake(None, None);
    start(&view, cx, ime.clone());
    assert!(ime.calls.borrow().is_empty());
    // a box selection
    let (view, cx) = setup(cx, "abc\ndef");
    view.update(cx, |v, _| {
        v.cursor.begin_box(0, false);
        v.cursor.move_box_to(Pos::new(1, 2), 2);
    });
    let ime = fake(None, None);
    start(&view, cx, ime.clone());
    assert!(ime.calls.borrow().is_empty());
    // view mode
    let (view, cx) = setup(cx, "abc");
    view.update(cx, |v, cx| {
        v.document().update(cx, |d, cx| d.set_view_mode(true, cx));
    });
    let ime = fake(None, None);
    start(&view, cx, ime.clone());
    assert!(ime.calls.borrow().is_empty());
}

#[gpui::test]
fn an_ime_that_cannot_reconvert_is_told_on_the_status_bar(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let ime = fake(None, Some(Error::Unsupported("使えない")));
    start(&view, cx, ime.clone());
    assert_eq!(
        *ime.calls.borrow(),
        ["query"],
        "the conversion is not started"
    );
    view.read_with(cx, |v, _| {
        assert_eq!(v.notice.as_deref(), Some("再変換できない: 使えない"));
    });
}

#[gpui::test]
fn the_function_is_a_command_and_does_nothing_without_a_window_handle(cx: &mut TestAppContext) {
    // `F_RECONVERT` is registered; the headless window has no OS handle, so no IME is asked
    let (view, cx) = setup(cx, "abc");
    let code = matsu_commands::sakura::FUNCTIONS
        .iter()
        .find(|f| f.name == "F_RECONVERT")
        .unwrap()
        .code;
    let ok = view.update_in(cx, |v, window, cx| v.run_function(code, &[], window, cx));
    assert!(ok);
}
