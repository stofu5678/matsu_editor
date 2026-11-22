//! A held left or right key moves `nRepeatedMoveCaretNum` columns a time (2 by default):
//! `Command_LEFT` and `Command_RIGHT` in `sakura_core/cmd/CViewCommander_Cursor.cpp` run their
//! move `nRepeat` times when `bRepeat` is set (`CViewCommander::HandleCommand`, lines 313-314,
//! 359-360 and 385-386 for the plain, the `_SEL` and the `_BOX` forms). The tests do not touch
//! the file system, paths or line endings of the host: the texts are in the source and use `\n`.

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_core::buffer::{Document, Pos};
use matsu_core::model::DocumentModel;

use super::EditorView;
use crate::common_settings::ScrollOptions;
use crate::document::SharedDocument;

fn setup<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
) -> (Entity<EditorView>, &'a mut VisualTestContext) {
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(document, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    vcx.update(|window, _| window.activate_window());
    vcx.run_until_parked();
    (view, vcx)
}

fn place(view: &Entity<EditorView>, cx: &mut VisualTestContext, line: usize, col: usize) {
    view.update(cx, |v, cx| {
        v.run_with(cx, |c| c.move_caret(Pos::new(line, col), false))
            .unwrap();
    });
}

fn key(view: &Entity<EditorView>, cx: &mut VisualTestContext, keystroke: &str, held: bool) {
    let event = gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse(keystroke).unwrap(),
        is_held: held,
        prefer_character_input: false,
    };
    let view = view.clone();
    cx.update(|window, cx| view.update(cx, |v, cx| v.on_key_down(&event, window, cx)));
}

fn caret(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> (usize, usize) {
    view.read_with(cx, |v, _| {
        let at = v.cursor.caret();
        (at.line, at.col)
    })
}

fn selection(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> Option<(Pos, Pos)> {
    view.read_with(cx, |v, _| v.cursor.selection())
}

fn columns(cx: &mut VisualTestContext, repeated_columns: usize) {
    cx.update(|_, cx| {
        cx.set_global(ScrollOptions {
            repeated_columns,
            ..Default::default()
        })
    });
}

#[gpui::test]
fn the_first_press_moves_one_column_and_the_repeats_two(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abcdefghij\n");
    place(&view, cx, 0, 5);
    key(&view, cx, "right", false);
    assert_eq!(caret(&view, cx), (0, 6));
    key(&view, cx, "right", true);
    assert_eq!(caret(&view, cx), (0, 8));
    key(&view, cx, "left", true);
    assert_eq!(caret(&view, cx), (0, 6));
    key(&view, cx, "left", false);
    assert_eq!(caret(&view, cx), (0, 5));
}

#[gpui::test]
fn the_setting_is_the_number_of_columns(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abcdefghij\n");
    place(&view, cx, 0, 5);
    columns(cx, 1);
    key(&view, cx, "right", true);
    assert_eq!(caret(&view, cx), (0, 6));
    columns(cx, 3);
    key(&view, cx, "right", true);
    assert_eq!(caret(&view, cx), (0, 9));
    key(&view, cx, "left", true);
    assert_eq!(caret(&view, cx), (0, 6));
}

#[gpui::test]
fn a_repeat_goes_over_the_end_and_the_start_of_a_line_one_column_at_a_time(
    cx: &mut TestAppContext,
) {
    // `Command_RIGHT` at the end of a line goes to the start of the next, and the repeat goes on
    // from there; `Command_LEFT` at the start goes to the end of the previous line.
    let (view, cx) = setup(cx, "ab\ncd\n");
    place(&view, cx, 0, 2);
    key(&view, cx, "right", true);
    assert_eq!(caret(&view, cx), (1, 1));
    place(&view, cx, 1, 0);
    key(&view, cx, "left", true);
    assert_eq!(caret(&view, cx), (0, 1));
}

#[gpui::test]
fn a_repeat_stops_at_the_start_and_the_end_of_the_document(cx: &mut TestAppContext) {
    // `Command_LEFT`: "これ以上左に動けぬ"; the extra repeats are not run.
    let (view, cx) = setup(cx, "ab");
    place(&view, cx, 0, 1);
    key(&view, cx, "left", true);
    assert_eq!(caret(&view, cx), (0, 0));
    key(&view, cx, "left", true);
    assert_eq!(caret(&view, cx), (0, 0));
    place(&view, cx, 0, 1);
    key(&view, cx, "right", true);
    assert_eq!(caret(&view, cx), (0, 2));
    key(&view, cx, "right", true);
    assert_eq!(caret(&view, cx), (0, 2));
}

#[gpui::test]
fn a_repeat_counts_characters_not_columns_or_bytes(cx: &mut TestAppContext) {
    // A wide character and a character of two UTF-16 units are one move each, as the
    // `CMemoryIterator` of Sakura steps over one character.
    let (view, cx) = setup(cx, "日本😀ab\n");
    place(&view, cx, 0, 0);
    key(&view, cx, "right", true);
    assert_eq!(caret(&view, cx), (0, "日本".len()));
    key(&view, cx, "right", true);
    assert_eq!(caret(&view, cx), (0, "日本😀a".len()));
    key(&view, cx, "left", true);
    assert_eq!(caret(&view, cx), (0, "日本".len()));
    key(&view, cx, "left", true);
    assert_eq!(caret(&view, cx), (0, 0));
}

#[gpui::test]
fn a_crlf_is_one_move(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "ab\r\ncd\r\n");
    place(&view, cx, 0, 1);
    key(&view, cx, "right", true);
    assert_eq!(caret(&view, cx), (1, 0));
    key(&view, cx, "left", true);
    assert_eq!(caret(&view, cx), (0, 1));
}

#[gpui::test]
fn the_selecting_forms_move_the_same_number_of_columns(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abcdefghij\n");
    place(&view, cx, 0, 5);
    key(&view, cx, "shift-right", true);
    assert_eq!(selection(&view, cx), Some((Pos::new(0, 5), Pos::new(0, 7))));
    key(&view, cx, "shift-left", true);
    assert_eq!(selection(&view, cx), None);
    assert_eq!(caret(&view, cx), (0, 5));
    columns(cx, 3);
    key(&view, cx, "shift-left", true);
    assert_eq!(selection(&view, cx), Some((Pos::new(0, 2), Pos::new(0, 5))));
}

#[gpui::test]
fn the_box_forms_move_the_same_number_of_columns(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abcdefghij\nabcdefghij\n");
    place(&view, cx, 0, 5);
    key(&view, cx, "alt-right", true);
    assert!(view.read_with(cx, |v, _| v.cursor.is_box()));
    assert_eq!(caret(&view, cx), (0, 7));
    key(&view, cx, "alt-left", true);
    assert_eq!(caret(&view, cx), (0, 5));
}

#[gpui::test]
fn a_repeat_that_drops_a_selection_spends_one_of_its_moves_on_it(cx: &mut TestAppContext) {
    // `Command_LEFT`: "選択のキャンセルで左移動を 1消費". The selection goes to its start, and the
    // other repeat moves from there. (Sakura's loop moves that one from the caret it had before the
    // selection was dropped; MATSU moves from where the caret is, see the PR.)
    let (view, cx) = setup(cx, "abcdefghij\n");
    view.update(cx, |v, _| v.cursor.select(Pos::new(0, 4), Pos::new(0, 7)));
    key(&view, cx, "left", true);
    assert_eq!(selection(&view, cx), None);
    assert_eq!(caret(&view, cx), (0, 3));
    view.update(cx, |v, _| v.cursor.select(Pos::new(0, 4), Pos::new(0, 7)));
    key(&view, cx, "right", true);
    assert_eq!(selection(&view, cx), None);
    assert_eq!(caret(&view, cx), (0, 8));
}

#[gpui::test]
fn other_held_keys_are_not_repeated(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abcdefghij\n");
    place(&view, cx, 0, 5);
    // Ctrl+Right is `F_WORDRIGHT`; it runs once.
    key(&view, cx, "ctrl-right", true);
    assert_eq!(caret(&view, cx), (0, 10));
}
