//! Tests of the AI confirmation window in GPUI's headless test window.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_core::ai_send::LARGE_TEXT_BYTES;
use matsu_core::buffer::{Document, Pos};
use matsu_core::model::DocumentModel;

use super::{Answer, Request, count_text, counts_text, size_text};
use crate::document::SharedDocument;
use crate::view::EditorView;

type Slot = Rc<RefCell<Option<Answer>>>;

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

fn request_for(text: &str) -> Request {
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let end = doc.end_pos().unwrap();
    Request::from_document("Claude に聞く", "claude -p", &doc, Pos::new(0, 0), end).unwrap()
}

fn open(view: &Entity<EditorView>, cx: &mut VisualTestContext, request: Request) -> Slot {
    let slot: Slot = Rc::default();
    let answer = view.update_in(cx, |v, window, cx| v.ask_ai_confirm(request, window, cx));
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

fn is_open(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> bool {
    view.read_with(cx, |v, _| v.ai_confirm.is_some())
}

const SEND: Answer = Answer {
    send: true,
    dont_ask_again: false,
};
const SEND_AND_DONT_ASK: Answer = Answer {
    send: true,
    dont_ask_again: true,
};
const NO: Answer = Answer {
    send: false,
    dont_ask_again: false,
};

#[gpui::test]
fn nothing_is_answered_until_a_button_is_pressed(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request_for("こんにちは"));
    assert!(is_open(&view, cx));
    assert_eq!(*slot.borrow(), None);
}

#[gpui::test]
fn enter_sends_by_default(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request_for("こんにちは"));
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(SEND));
    assert!(!is_open(&view, cx));
}

#[gpui::test]
fn escape_does_not_send(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request_for("こんにちは"));
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(NO));
    assert!(!is_open(&view, cx));
}

#[gpui::test]
fn the_dont_send_button_does_not_send(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request_for("こんにちは"));
    // 送る -> 送らない
    cx.simulate_keystrokes("tab enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(NO));
}

#[gpui::test]
fn the_check_box_is_in_the_answer_when_sending(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request_for("こんにちは"));
    // 送る -> (Shift+Tab) check box, Space toggles it, Tab back to 送る.
    cx.simulate_keystrokes("shift-tab space tab enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(SEND_AND_DONT_ASK));

    // Toggled twice: off again.
    let slot = open(&view, cx, request_for("こんにちは"));
    cx.simulate_keystrokes("shift-tab space space tab enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(SEND));
}

#[gpui::test]
fn the_check_box_is_ignored_when_not_sending(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request_for("こんにちは"));
    cx.simulate_keystrokes("shift-tab space escape");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(NO), "not sending changes no setting");
}

#[gpui::test]
fn enter_on_the_check_box_does_nothing(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request_for("こんにちは"));
    cx.simulate_keystrokes("shift-tab enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), None);
    assert!(is_open(&view, cx));
}

#[gpui::test]
fn large_text_starts_on_dont_send(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let line = "x".repeat(1000) + "\n";
    let request = request_for(&line.repeat(LARGE_TEXT_BYTES / 1000 + 5));
    assert!(request.preview.large);
    let slot = open(&view, cx, request);
    // Enter at once must not send.
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(NO));

    // 送る is one Shift+Tab away.
    let request = request_for(&line.repeat(LARGE_TEXT_BYTES / 1000 + 5));
    let slot = open(&view, cx, request);
    cx.simulate_keystrokes("shift-tab enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(SEND));
}

#[test]
fn small_text_is_not_large() {
    assert!(!request_for("短い").preview.large);
}

#[gpui::test]
fn a_second_window_is_answered_no_at_once(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let first = open(&view, cx, request_for("一つ目"));
    let second = open(&view, cx, request_for("二つ目"));
    assert_eq!(*second.borrow(), Some(NO));
    assert_eq!(*first.borrow(), None);
    assert!(is_open(&view, cx));
}

#[gpui::test]
fn closing_the_window_without_an_answer_is_no(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request_for("こんにちは"));
    view.update(cx, |v, cx| {
        v.ai_confirm = None;
        cx.notify();
    });
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(NO));
}

#[gpui::test]
fn the_window_holds_the_destination_the_command_and_the_ends_of_the_text(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let text: String = (1..=40).map(|i| format!("行{i}\n")).collect();
    let _slot = open(&view, cx, request_for(&text));
    view.read_with(cx, |v, cx| {
        let request = v.ai_confirm.as_ref().unwrap().read(cx).request().clone();
        assert_eq!(request.name, "Claude に聞く");
        assert_eq!(request.command_line, "claude -p");
        let p = &request.preview;
        assert_eq!(p.head.len(), 10);
        assert_eq!(p.tail.len(), 5);
        assert_eq!(p.tail[4].text, "行40");
        assert_eq!(p.lines, 40);
    });
}

#[test]
fn the_counts_are_written_with_a_lower_bound_mark() {
    let mut p = request_for("あいう\nえお").preview;
    assert_eq!(counts_text(&p), "6 文字 / 2 行 / 1 KB");
    p.chars_exact = false;
    assert!(counts_text(&p).contains("以上"));
}

#[test]
fn counts_and_sizes_read_well() {
    assert_eq!(count_text(0), "0");
    assert_eq!(count_text(999), "999");
    assert_eq!(count_text(1000), "1,000");
    assert_eq!(count_text(1234567), "1,234,567");
    assert_eq!(size_text(1), "1 KB");
    assert_eq!(size_text(100 * 1024), "100 KB");
    assert_eq!(size_text(3 * 1024 * 1024 / 2), "1.5 MB");
}

#[test]
fn a_secret_in_the_text_is_warned_about_and_masked() {
    let request = request_for("普通\nKEY=sk-proj-abc123DEF456ghi789JKL012mno345\n");
    let p = &request.preview;
    assert_eq!(p.secret_lines_total, 1);
    assert_eq!(p.secrets[0].number, 2);
    assert!(!p.secrets[0].text.contains("abc123"));
    assert!(p.head.iter().all(|l| !l.text.contains("abc123")));
}
