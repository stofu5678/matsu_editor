//! Tests of the window of mode b in GPUI's headless test window. The run after 「送る」 is tested with the flow
//! (`crate::oneliner::tests::ai_ask`).

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_core::ai_send::{LARGE_TEXT_BYTES, PreviewLimits, preview_range};
use matsu_core::buffer::{Document, Pos};
use matsu_core::model::DocumentModel;

use super::{Chosen, Destination, Request, initial_destination};
use crate::document::SharedDocument;
use crate::view::EditorView;

type Slot = Rc<RefCell<Option<Option<Chosen>>>>;

fn setup(cx: &mut TestAppContext) -> (Entity<EditorView>, &mut VisualTestContext) {
    cx.update(gpui_component::init);
    cx.update(EditorView::bind_keys);
    let doc = Document::from_utf8(b"abc".to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    cx.add_window_view(move |window, cx| {
        let view = EditorView::new(document, cx);
        window.focus(&view.focus_handle, cx);
        view
    })
}

fn destination(index: usize, name: &str) -> Destination {
    Destination {
        index,
        name: name.to_owned(),
        command_line: format!("tool-{index} -p"),
    }
}

fn request_for(text: &str, history: &[&str]) -> Request {
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let end = doc.end_pos().unwrap();
    Request {
        material: preview_range(&doc, Pos::new(0, 0), end, &PreviewLimits::default()).unwrap(),
        destinations: vec![destination(0, "英訳"), destination(3, "要約")],
        history: history.iter().map(|s| (*s).to_owned()).collect(),
        initial: 0,
    }
}

fn open(view: &Entity<EditorView>, cx: &mut VisualTestContext, request: Request) -> Slot {
    let slot: Slot = Rc::default();
    let answer = view.update_in(cx, |v, window, cx| {
        v.ask_ai_instruction(request, window, cx)
    });
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
    view.read_with(cx, |v, _| v.ai_ask.is_some())
}

fn dialog_text(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> String {
    view.read_with(cx, |v, cx| v.ai_ask.as_ref().unwrap().read(cx).text(cx))
}

fn selected(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> usize {
    view.read_with(cx, |v, cx| {
        v.ai_ask.as_ref().unwrap().read(cx).selected_index()
    })
}

fn chosen(index: usize, instruction: &str) -> Option<Chosen> {
    Some(Chosen {
        index,
        instruction: instruction.to_owned(),
    })
}

#[gpui::test]
fn nothing_is_answered_until_a_button_is_pressed(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request_for("こんにちは", &[]));
    assert!(is_open(&view, cx));
    cx.simulate_input("英語に");
    assert_eq!(*slot.borrow(), None);
}

#[gpui::test]
fn the_field_has_the_focus_and_takes_several_lines(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let _slot = open(&view, cx, request_for("こんにちは", &[]));
    cx.simulate_input("一行目");
    cx.simulate_keystrokes("enter");
    cx.simulate_input("二行目");
    assert_eq!(dialog_text(&view, cx), "一行目\n二行目");
    assert!(is_open(&view, cx), "Enter in the field is a line break");
}

#[gpui::test]
fn the_chosen_destination_is_the_first_unless_the_last_used_is_offered(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let _slot = open(&view, cx, request_for("x", &[]));
    assert_eq!(selected(&view, cx), 0);

    let destinations = vec![
        destination(0, "a"),
        destination(3, "b"),
        destination(5, "c"),
    ];
    assert_eq!(initial_destination(&destinations, None), 0);
    assert_eq!(initial_destination(&destinations, Some(3)), 1);
    assert_eq!(initial_destination(&destinations, Some(5)), 2);
    // the one used last is no longer offered (it was removed, or the document cannot take its result)
    assert_eq!(initial_destination(&destinations, Some(4)), 0);
}

#[gpui::test]
fn the_window_opens_on_the_initial_destination_and_shows_its_command_line(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let mut request = request_for("x", &[]);
    request.initial = 1;
    let _slot = open(&view, cx, request);
    assert_eq!(selected(&view, cx), 3);
    let shown = view.read_with(cx, |v, cx| v.ai_ask.as_ref().unwrap().read(cx).shown(cx));
    assert_eq!(shown.name, "要約");
    assert_eq!(shown.command_line, "tool-3 -p");
}

#[gpui::test]
fn the_destination_is_chosen_with_the_arrows_when_the_list_has_the_focus(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request_for("x", &[]));
    cx.simulate_input("go");
    // field -> the list of destinations
    cx.simulate_keystrokes("tab down");
    assert_eq!(selected(&view, cx), 3);
    cx.simulate_keystrokes("up");
    assert_eq!(selected(&view, cx), 0);
    cx.simulate_keystrokes("down tab enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(chosen(3, "go")));
}

#[gpui::test]
fn sending_gives_the_trimmed_instruction_in_lf_and_the_destination(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request_for("こんにちは", &[]));
    cx.simulate_input("  英語に翻訳して\n口調はそのまま  ");
    // field -> list -> 送る
    cx.simulate_keystrokes("tab tab enter");
    cx.run_until_parked();
    assert_eq!(
        *slot.borrow(),
        Some(chosen(0, "英語に翻訳して\n口調はそのまま"))
    );
    assert!(!is_open(&view, cx));
}

#[gpui::test]
fn send_cannot_be_pressed_while_the_instruction_is_empty(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request_for("こんにちは", &[]));
    cx.simulate_keystrokes("tab tab enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), None, "an empty field");
    assert!(is_open(&view, cx));
    // only white space is empty too
    cx.simulate_keystrokes("shift-tab shift-tab");
    cx.simulate_input("   ");
    cx.simulate_keystrokes("enter");
    cx.simulate_input("\t ");
    cx.simulate_keystrokes("tab tab space");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), None, "white space only");
    assert!(is_open(&view, cx));
    // something to say: it can be sent
    cx.simulate_keystrokes("shift-tab shift-tab");
    cx.simulate_input("x");
    cx.simulate_keystrokes("tab tab space");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(chosen(0, "x")));
}

#[gpui::test]
fn escape_and_the_dont_send_button_send_nothing(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request_for("こんにちは", &[]));
    cx.simulate_input("英訳");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(None));
    assert!(!is_open(&view, cx));

    let slot = open(&view, cx, request_for("こんにちは", &[]));
    cx.simulate_input("英訳");
    // field -> list -> 送る -> 送らない
    cx.simulate_keystrokes("tab tab tab enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(None));
}

#[gpui::test]
fn a_nothing_sent_leaves_the_history_alone(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let before = cx.update(|_, app| super::history::items(app));
    let _slot = open(&view, cx, request_for("x", &[]));
    cx.simulate_input("not sent");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert_eq!(cx.update(|_, app| super::history::items(app)), before);
}

#[gpui::test]
fn closing_the_window_without_an_answer_is_no(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request_for("こんにちは", &[]));
    view.update(cx, |v, cx| {
        v.ai_ask = None;
        cx.notify();
    });
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(None));
}

#[gpui::test]
fn a_second_window_is_answered_no_at_once(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let first = open(&view, cx, request_for("一つ目", &[]));
    let second = open(&view, cx, request_for("二つ目", &[]));
    assert_eq!(*second.borrow(), Some(None));
    assert_eq!(*first.borrow(), None);
    assert!(is_open(&view, cx));
}

#[gpui::test]
fn text_over_100_kb_starts_on_dont_send(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let line = "x".repeat(1000) + "\n";
    let big = line.repeat(LARGE_TEXT_BYTES / 1000 + 5);
    let request = request_for(&big, &[]);
    assert!(request.material.large);
    let slot = open(&view, cx, request);
    // Enter at once must not send: the focus is on 送らない
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(None));

    // the field is one Shift+Tab step... three (送らない -> 送る -> list -> field)
    let slot = open(&view, cx, request_for(&big, &[]));
    cx.simulate_keystrokes("shift-tab shift-tab shift-tab");
    cx.simulate_input("go");
    cx.simulate_keystrokes("tab tab enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(chosen(0, "go")));
}

#[gpui::test]
fn small_text_starts_in_the_field(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request_for("短い", &[]));
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), None);
    assert_eq!(dialog_text(&view, cx), "\n");
}

#[gpui::test]
fn the_window_shows_the_final_standard_input(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let _slot = open(&view, cx, request_for("こんにちは\r\nさようなら\r\n", &[]));
    let shown = |view: &Entity<EditorView>, cx: &mut VisualTestContext| {
        view.read_with(cx, |v, cx| v.ai_ask.as_ref().unwrap().read(cx).shown(cx))
    };
    // no instruction yet: the text alone
    let before = shown(&view, cx);
    assert_eq!(before.preview.lines, 2);
    cx.simulate_input("英訳\n丁寧に");
    let after = shown(&view, cx);
    let lines: Vec<_> = after.preview.head.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(
        lines,
        ["英訳", "丁寧に", "", "---", "こんにちは", "さようなら"]
    );
    assert_eq!(after.preview.lines, 6);
}

#[gpui::test]
fn the_history_is_walked_with_the_arrows_of_an_empty_field(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request_for("x", &["新しい", "古い"]));
    cx.simulate_keystrokes("up");
    assert_eq!(dialog_text(&view, cx), "新しい");
    cx.simulate_keystrokes("up");
    assert_eq!(dialog_text(&view, cx), "古い");
    // at the oldest: it stays
    cx.simulate_keystrokes("up");
    assert_eq!(dialog_text(&view, cx), "古い");
    cx.simulate_keystrokes("down");
    assert_eq!(dialog_text(&view, cx), "新しい");
    // Down at the newest is the empty field it was
    cx.simulate_keystrokes("down");
    assert_eq!(dialog_text(&view, cx), "");
    cx.simulate_keystrokes("up up tab tab enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(chosen(0, "古い")));
}

#[gpui::test]
fn the_arrows_of_a_field_with_text_of_its_own_move_the_caret(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let _slot = open(&view, cx, request_for("x", &["古い"]));
    cx.simulate_input("自分の\n指示");
    cx.simulate_keystrokes("up");
    assert_eq!(dialog_text(&view, cx), "自分の\n指示", "not replaced");
}

#[gpui::test]
fn a_row_of_the_history_goes_into_the_field(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let slot = open(&view, cx, request_for("x", &["一つ目", "二つ目"]));
    view.update_in(cx, |v, window, cx| {
        v.ai_ask
            .as_ref()
            .unwrap()
            .clone()
            .update(cx, |dialog, cx| dialog.use_history(1, window, cx));
    });
    cx.run_until_parked();
    assert_eq!(dialog_text(&view, cx), "二つ目");
    cx.simulate_keystrokes("tab tab enter");
    cx.run_until_parked();
    assert_eq!(*slot.borrow(), Some(chosen(0, "二つ目")));
}

#[gpui::test]
fn a_sent_instruction_and_its_destination_are_remembered_for_the_next_window(
    cx: &mut TestAppContext,
) {
    let (view, cx) = setup(cx);
    let _slot = open(&view, cx, request_for("x", &[]));
    cx.simulate_input("覚える");
    cx.simulate_keystrokes("tab down tab enter");
    cx.run_until_parked();
    assert_eq!(
        cx.update(|_, app| super::history::items(app)),
        ["覚える".to_owned()]
    );
    assert_eq!(
        cx.update(|_, app| super::history::last_destination(app)),
        Some(3)
    );
}

#[test]
fn a_row_of_the_history_is_its_first_line() {
    assert_eq!(super::row_text("一行"), "一行");
    assert_eq!(super::row_text("一行目\n二行目"), "一行目…");
    assert!(super::row_text(&"あ".repeat(200)).ends_with('…'));
    assert_eq!(super::row_text(&"あ".repeat(200)).chars().count(), 81);
}
