//! Tests of the text input dialog in GPUI's headless test window.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use ume_core::buffer::Document;
use ume_core::model::DocumentModel;

use super::{InputRequest, limit_edit};
use crate::document::SharedDocument;
use crate::view::EditorView;

type Answer = Rc<RefCell<Option<Option<String>>>>;

fn setup(cx: &mut TestAppContext) -> (Entity<EditorView>, &mut VisualTestContext) {
    cx.update(gpui_component::init);
    let doc = Document::from_utf8(b"abc".to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    cx.add_window_view(move |window, cx| {
        let view = EditorView::new(document, cx);
        window.focus(&view.focus_handle, cx);
        view
    })
}

fn request(default: &str, max_len: usize) -> InputRequest {
    InputRequest {
        title: "題名".into(),
        message: "メッセージ".into(),
        default: default.into(),
        max_len,
    }
}

/// Opens the dialog and returns the slot that gets the answer.
fn open(view: &Entity<EditorView>, cx: &mut VisualTestContext, request: InputRequest) -> Answer {
    let slot: Answer = Rc::default();
    let answer = view.update_in(cx, |v, window, cx| v.open_input_dialog(request, window, cx));
    let out = slot.clone();
    cx.update(|_, app| {
        app.spawn(async move |_| {
            *out.borrow_mut() = Some(answer.await);
        })
        .detach();
    });
    cx.run_until_parked();
    slot
}

fn field_text(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> String {
    view.read_with(cx, |v, cx| {
        v.input_dialog
            .as_ref()
            .map(|d| d.read(cx).text(cx))
            .unwrap_or_default()
    })
}

fn is_open(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> bool {
    view.read_with(cx, |v, _| v.input_dialog.is_some())
}

#[gpui::test]
fn the_field_starts_with_the_default_and_enter_answers_the_text(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request("既定", 260));
    assert!(is_open(&view, cx));
    assert_eq!(field_text(&view, cx), "既定");
    // The field has the focus: typing goes into it, not into the document. The default is
    // selected (`WM_INITDIALOG` returns TRUE); End keeps it.
    cx.simulate_keystrokes("end");
    cx.simulate_input("の値");
    assert_eq!(field_text(&view, cx), "既定の値");
    assert_eq!(*slot.borrow(), None, "nothing is answered yet");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(Some("既定の値".to_owned())));
    assert!(!is_open(&view, cx));
    let text = view.read_with(cx, |v, cx| {
        v.document.read(cx).doc().line(0).unwrap().unwrap().text
    });
    assert_eq!(text, "abc");
}

#[gpui::test]
fn escape_cancels(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request("x", 260));
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(None));
    assert!(!is_open(&view, cx));
}

#[gpui::test]
fn the_cancel_button_cancels_and_ok_answers_by_keys(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    // Tab leaves the field for OK, then Cancel; Enter presses the focused button.
    let slot = open(&view, cx, request("値", 260));
    cx.simulate_keystrokes("tab enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(Some("値".to_owned())));
    let slot = open(&view, cx, request("値", 260));
    cx.simulate_keystrokes("tab tab enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(None));
    // Shift+Tab goes back.
    let slot = open(&view, cx, request("値", 260));
    cx.simulate_keystrokes("shift-tab enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(None));
}

#[gpui::test]
fn the_default_is_selected_so_typing_replaces_it(cx: &mut TestAppContext) {
    // `CDlgInput1::OnInitDialog` returns true: the dialog manager focuses the edit and selects its
    // text. (Headless, Enter would replace a selection with nothing before the handler runs, so
    // the keys here are typed text only.)
    let (view, cx) = setup(cx);
    let _slot = open(&view, cx, request("既定", 260));
    cx.simulate_input("新");
    assert_eq!(field_text(&view, cx), "新");
}

#[gpui::test]
fn the_text_stops_at_the_maximum_length(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request("", 5));
    cx.simulate_input("あいうえおかき");
    assert_eq!(field_text(&view, cx), "あいうえお");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(Some("あいうえお".to_owned())));
    // The default is cut too, and a character outside the BMP counts two units.
    let slot = open(&view, cx, request("a𠮷bc", 3));
    assert_eq!(field_text(&view, cx), "a𠮷");
    // (End: see `the_default_is_selected_so_typing_replaces_it`)
    cx.simulate_keystrokes("end enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(Some("a𠮷".to_owned())));
}

#[gpui::test]
fn a_second_dialog_is_cancelled_while_one_is_open(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let first = open(&view, cx, request("1", 260));
    let second = open(&view, cx, request("2", 260));
    assert_eq!(*second.borrow(), Some(None));
    assert_eq!(*first.borrow(), None);
    assert_eq!(field_text(&view, cx), "1");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert_eq!(*first.borrow(), Some(None));
}

#[test]
fn an_edit_that_does_not_fit_keeps_the_text_around_it() {
    // It fits: nothing to do.
    assert_eq!(limit_edit("abc", "abXc", 4), None);
    // Typed at the end: the new characters that do not fit are cut.
    assert_eq!(
        limit_edit("abc", "abcdef", 5)
            .map(|(text, _)| text)
            .as_deref(),
        Some("abcde")
    );
    // Typed at the start of a full field: the old text stays, the new character is refused.
    assert_eq!(
        limit_edit("abcde", "Xabcde", 5)
            .map(|(text, _)| text)
            .as_deref(),
        Some("abcde")
    );
    // Typed in the middle: the end stays.
    assert_eq!(
        limit_edit("abcde", "abXcde", 5)
            .map(|(text, _)| text)
            .as_deref(),
        Some("abcde")
    );
    assert_eq!(
        limit_edit("abcd", "abXYcd", 5)
            .map(|(text, _)| text)
            .as_deref(),
        Some("abXcd")
    );
    // A pasted text over a selection: what fits of it is taken.
    assert_eq!(
        limit_edit("abcde", "aXYZWde", 5)
            .map(|(text, _)| text)
            .as_deref(),
        Some("aXYde")
    );
    // Japanese, and a character outside the BMP (two units) is not split.
    assert_eq!(
        limit_edit("あいう", "あXいう", 3)
            .map(|(text, _)| text)
            .as_deref(),
        Some("あいう")
    );
    assert_eq!(
        limit_edit("a", "a𠮷𠮷", 3).map(|(text, _)| text).as_deref(),
        Some("a𠮷")
    );
    assert_eq!(
        limit_edit("", "あいうえお", 3)
            .map(|(text, _)| text)
            .as_deref(),
        Some("あいう")
    );
}

#[gpui::test]
fn text_typed_in_front_of_a_full_field_is_refused(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request("abcde", 5));
    cx.simulate_keystrokes("home");
    cx.simulate_input("X");
    assert_eq!(field_text(&view, cx), "abcde");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(Some("abcde".to_owned())));
}

#[gpui::test]
fn a_refused_insert_leaves_the_caret(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let _slot = open(&view, cx, request("", 5));
    cx.simulate_input("abcde");
    cx.simulate_keystrokes("home right right");
    cx.simulate_input("X");
    assert_eq!(field_text(&view, cx), "abcde");
    let caret = view.read_with(cx, |v, cx| {
        let dialog = v.input_dialog.as_ref().unwrap().read(cx);
        dialog.input.read(cx).cursor()
    });
    assert_eq!(caret, 2);
}
