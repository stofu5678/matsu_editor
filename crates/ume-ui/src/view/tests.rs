//! Tests of the editor view in GPUI's headless test window: keys go through the key map and the
//! command registry, text goes through the input handler, and the layout follows each change.
//!
//! The shortcuts are written with `ctrl-`: Control is Sakura's Ctrl on every OS, macOS included
//! (ADR-0008; Cmd there only has the shortcuts every Mac app has).
//!
//! The test platform has no fonts, so nothing here depends on how the text looks; the sizes of the
//! text area are set by the tests (`set_metrics`), and the pure arithmetic of painting and
//! hit-testing is tested in `geometry`, `hit` and `segments`.

use gpui::{
    AppContext as _, Entity, EntityInputHandler, TestAppContext, VisualTestContext, point, px,
};
use ume_core::buffer::{Document, Pos};
use ume_core::model::DocumentModel;

use super::{EditorView, Redo, Undo};
use crate::document::SharedDocument;
use crate::geometry::Metrics;

struct Fixture {
    document: Entity<SharedDocument>,
    view: Entity<EditorView>,
}

fn setup<'a>(cx: &'a mut TestAppContext, text: &str) -> (Fixture, &'a mut VisualTestContext) {
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
    fn lines(&self, cx: &mut VisualTestContext) -> Vec<String> {
        self.document.read_with(cx, |shared, _| {
            let doc = shared.doc();
            (0..doc.line_count())
                .map(|i| doc.line(i).unwrap().unwrap().text)
                .collect()
        })
    }

    fn caret(&self, cx: &mut VisualTestContext) -> Pos {
        self.view.read_with(cx, |v, _| v.cursor.caret())
    }

    fn rows(&self, cx: &mut VisualTestContext) -> usize {
        self.view.read_with(cx, |v, _| v.layout.row_count())
    }
}

/// The window position of the cell at `row` and `col`, from what the last paint of the test
/// window measured (the test platform has no fonts, so the sizes are the fallbacks).
fn point_of(
    f: &Fixture,
    cx: &mut VisualTestContext,
    row: usize,
    col: usize,
) -> gpui::Point<gpui::Pixels> {
    f.view.read_with(cx, |v, _| {
        let paint = v.paint.expect("the window was painted");
        let m = v.metrics;
        point(
            px(paint.origin_x + (col as f32 + 0.1) * m.cell_width),
            px(paint.origin_y + (row as f32 + 0.5) * m.line_height - v.scroll_y),
        )
    })
}

#[gpui::test]
fn typed_characters_reach_the_document(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    cx.simulate_input("abc");
    assert_eq!(f.lines(cx), ["abc"]);
    assert_eq!(f.caret(cx), Pos::new(0, 3));
}

#[gpui::test]
fn enter_and_tab_are_typed_as_characters(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    cx.simulate_keystrokes("a enter tab b");
    assert_eq!(f.lines(cx), ["a", "\tb"]);
    // the Enter key inserted the line ending of the document, CRLF for a new one
    f.document.read_with(cx, |shared, _| {
        assert_eq!(
            shared.doc().line(0).unwrap().unwrap().eol,
            Some(ume_core::buffer::Eol::CrLf)
        );
    });
}

#[gpui::test]
fn keys_move_the_caret_and_delete(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc\r\ndef");
    cx.simulate_keystrokes("right right");
    assert_eq!(f.caret(cx), Pos::new(0, 2));
    cx.simulate_keystrokes("down");
    assert_eq!(f.caret(cx), Pos::new(1, 2));
    cx.simulate_keystrokes("home");
    assert_eq!(f.caret(cx), Pos::new(1, 0));
    cx.simulate_keystrokes("delete");
    assert_eq!(f.lines(cx), ["abc", "ef"]);
    cx.simulate_keystrokes("backspace");
    // Backspace at the start of a line joins it to the line before
    assert_eq!(f.lines(cx), ["abcef"]);
    assert_eq!(f.caret(cx), Pos::new(0, 3));
    cx.simulate_keystrokes("backspace");
    assert_eq!(f.lines(cx), ["abef"]);
    cx.simulate_keystrokes("ctrl-end");
    assert_eq!(f.caret(cx), Pos::new(0, 4));
    cx.simulate_keystrokes("ctrl-home");
    assert_eq!(f.caret(cx), Pos::new(0, 0));
}

#[gpui::test]
fn shift_arrows_select_and_typing_replaces_the_selection(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abcd");
    cx.simulate_keystrokes("right shift-right shift-right");
    f.view.read_with(cx, |v, _| {
        assert_eq!(v.cursor.selection(), Some((Pos::new(0, 1), Pos::new(0, 3))));
    });
    cx.simulate_input("X");
    assert_eq!(f.lines(cx), ["aXd"]);
    cx.simulate_keystrokes("ctrl-a");
    f.view.read_with(cx, |v, _| {
        assert_eq!(v.cursor.selection(), Some((Pos::new(0, 0), Pos::new(0, 3))));
    });
}

#[gpui::test]
fn undo_and_redo_by_key_and_by_menu_action(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    cx.simulate_input("ab");
    cx.simulate_keystrokes("ctrl-z");
    assert_eq!(f.lines(cx), ["a"]);
    cx.simulate_keystrokes("ctrl-y");
    assert_eq!(f.lines(cx), ["ab"]);
    cx.dispatch_action(Undo);
    cx.dispatch_action(Undo);
    assert!(f.lines(cx).is_empty(), "an empty document has no lines");
    cx.dispatch_action(Redo);
    assert_eq!(f.lines(cx), ["a"]);
}

#[gpui::test]
fn a_function_without_a_command_says_so(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    // Ctrl+F is Sakura's search dialog, which has no command yet
    cx.simulate_keystrokes("ctrl-f");
    let notice = f.view.read_with(cx, |v, _| v.notice.clone());
    assert!(notice.unwrap().contains("F_SEARCH_DIALOG"));
    // and the text is untouched
    assert_eq!(f.lines(cx), ["abc"]);
}

#[gpui::test]
fn the_layout_follows_every_change(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "one");
    assert_eq!(f.rows(cx), 1);
    cx.simulate_keystrokes("enter enter");
    assert_eq!(f.rows(cx), 3); // "", "", "one"
    cx.simulate_keystrokes("ctrl-z"); // each Enter is an undo step of its own
    assert_eq!(f.rows(cx), 2);
    cx.simulate_keystrokes("ctrl-a delete");
    assert_eq!(f.rows(cx), 1);
}

#[gpui::test]
fn the_caret_stays_in_view_while_moving_through_a_long_document(cx: &mut TestAppContext) {
    let text = (0..2000)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (f, cx) = setup(cx, &text);
    f.view.update(cx, |v, _| {
        v.set_metrics(Metrics {
            cell_width: 8.0,
            line_height: 20.0,
            view_width: 400.0,
            view_height: 100.0,
        })
    });
    let in_view = |f: &Fixture, cx: &mut VisualTestContext| {
        f.view.read_with(cx, |v, cx| {
            let row = v.caret_row(cx).row as f32;
            let top = v.scroll_y() / v.metrics().line_height;
            let rows = v.metrics().visible_rows() as f32;
            assert!(
                row >= top && row < top + rows,
                "row {row} top {top} rows {rows}"
            );
        });
    };
    cx.simulate_keystrokes("ctrl-end");
    assert_eq!(f.caret(cx).line, 1999);
    in_view(&f, cx);
    cx.simulate_keystrokes("ctrl-home");
    assert_eq!(f.view.read_with(cx, |v, _| v.scroll_y()), 0.0);
    for _ in 0..30 {
        cx.simulate_keystrokes("down");
    }
    in_view(&f, cx);
    assert_eq!(f.caret(cx).line, 30);
    for _ in 0..12 {
        cx.simulate_keystrokes("up");
    }
    in_view(&f, cx);
}

#[gpui::test]
fn ctrl_h_scrolls_the_caret_row_to_the_middle(cx: &mut TestAppContext) {
    let text = (0..2000)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (f, cx) = setup(cx, &text);
    f.view.update(cx, |v, cx| {
        v.set_metrics(Metrics {
            cell_width: 8.0,
            line_height: 20.0,
            view_width: 400.0,
            view_height: 100.0,
        });
        v.run_with(cx, |c| c.move_caret(Pos::new(1000, 0), false))
            .unwrap();
    });
    // F_CURLINECENTER is a UI command: there is no core command for its key, Ctrl+H.
    cx.simulate_keystrokes("ctrl-h");
    f.view.read_with(cx, |v, cx| {
        let rows = v.metrics().visible_rows();
        let top = (v.scroll_y() / v.metrics().line_height).round() as usize;
        assert_eq!(v.caret_row(cx).row, 1000);
        assert_eq!(top, 1000 - rows / 2);
        assert_eq!(v.status(cx).notice, None);
    });
}

#[gpui::test]
fn the_wheel_scrolls_without_moving_the_caret(cx: &mut TestAppContext) {
    let text = (0..500)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let (f, cx) = setup(cx, &text);
    f.view.update(cx, |v, _| {
        v.set_metrics(Metrics::default());
        v.scroll_by(0.0, -100.0); // the wheel down: the text moves up
        assert_eq!(v.scroll_y(), 100.0);
        v.scroll_by(0.0, 1e6);
        assert_eq!(v.scroll_y(), 0.0);
        v.scroll_by(0.0, -1e9);
        assert!(v.scroll_y() > 0.0 && v.scroll_y() < 1e9);
    });
    assert_eq!(f.caret(cx), Pos::new(0, 0));
}

#[gpui::test]
fn a_click_moves_the_caret_and_a_drag_selects(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abcdef\nghijkl");
    let at = |cx: &mut VisualTestContext, row: usize, col: usize| point_of(&f, cx, row, col);
    let p = at(cx, 1, 3);
    cx.simulate_mouse_down(p, gpui::MouseButton::Left, Default::default());
    assert_eq!(f.caret(cx), Pos::new(1, 3));
    let p = at(cx, 0, 5);
    cx.simulate_mouse_move(p, gpui::MouseButton::Left, Default::default());
    f.view.read_with(cx, |v, _| {
        assert_eq!(v.cursor.selection(), Some((Pos::new(0, 5), Pos::new(1, 3))));
    });
    cx.simulate_mouse_up(p, gpui::MouseButton::Left, Default::default());
    // a click with Shift extends the selection from the anchor
    let p = at(cx, 0, 0);
    cx.simulate_mouse_down(p, gpui::MouseButton::Left, gpui::Modifiers::shift());
    f.view.read_with(cx, |v, _| {
        assert_eq!(v.cursor.caret(), Pos::new(0, 0));
        assert!(v.cursor.has_selection());
    });
}

#[gpui::test]
fn a_double_click_selects_a_word(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo bar");
    let at = point_of(&f, cx, 0, 5);
    cx.simulate_event(gpui::MouseDownEvent {
        button: gpui::MouseButton::Left,
        position: at,
        modifiers: Default::default(),
        click_count: 2,
        first_mouse: false,
    });
    f.view.read_with(cx, |v, _| {
        assert_eq!(v.cursor.selection(), Some((Pos::new(0, 4), Pos::new(0, 7))));
    });
}

#[gpui::test]
fn an_ime_composition_is_one_undo_step(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab");
    cx.simulate_keystrokes("right");
    f.view.update_in(cx, |v, window, cx| {
        v.replace_and_mark_text_in_range(None, "に", Some(1..1), window, cx);
        v.replace_and_mark_text_in_range(None, "にほん", Some(3..3), window, cx);
        assert_eq!(v.marked_text_range(window, cx), Some(1..4));
        v.replace_text_in_range(None, "日本", window, cx);
        assert_eq!(v.marked_text_range(window, cx), None);
    });
    assert_eq!(f.lines(cx), ["a日本b"]);
    assert_eq!(f.caret(cx), Pos::new(0, 1 + "日本".len()));
    cx.simulate_keystrokes("ctrl-z");
    assert_eq!(f.lines(cx), ["ab"]);
    assert_eq!(f.caret(cx), Pos::new(0, 1));
}

#[gpui::test]
fn a_composition_over_lines_is_marked_where_it_starts(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abcdef\ngh");
    f.view.update_in(cx, |v, window, cx| {
        v.run_with(cx, |c| c.select_range(Pos::new(0, 5), Pos::new(1, 1)))
            .unwrap();
        v.replace_and_mark_text_in_range(None, "に", None, window, cx);
        assert_eq!(v.marked_text_range(window, cx), Some(5..6));
        v.replace_text_in_range(None, "日", window, cx);
    });
    assert_eq!(f.lines(cx), ["abcde日h"]);
}

#[gpui::test]
fn a_click_during_a_composition_ends_its_undo_step(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab\ncd");
    f.view.update_in(cx, |v, window, cx| {
        v.replace_and_mark_text_in_range(None, "に", Some(1..1), window, cx);
    });
    // The click comes before the IME ends the composition.
    let p = point_of(&f, cx, 1, 2);
    cx.simulate_mouse_down(p, gpui::MouseButton::Left, Default::default());
    cx.simulate_mouse_up(p, gpui::MouseButton::Left, Default::default());
    f.view.update_in(cx, |v, window, cx| {
        assert_eq!(v.marked_text_range(window, cx), None);
    });
    cx.simulate_keystrokes("x");
    assert_eq!(f.lines(cx), ["にab", "cdx"]);
    // The typed character is its own undo step, apart from the composition.
    cx.simulate_keystrokes("ctrl-z");
    assert_eq!(f.lines(cx), ["にab", "cd"]);
    cx.simulate_keystrokes("ctrl-z");
    assert_eq!(f.lines(cx), ["ab", "cd"]);
}

#[gpui::test]
fn the_ime_sees_the_caret_line_in_utf16(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "x\n日本語😀a");
    cx.simulate_keystrokes("down end");
    f.view.update_in(cx, |v, window, cx| {
        let sel = v.selected_text_range(true, window, cx).unwrap();
        // 日本語 = 3 units, 😀 = 2 units, a = 1
        assert_eq!(sel.range, 6..6);
        let mut actual = None;
        assert_eq!(
            v.text_for_range(3..5, &mut actual, window, cx).as_deref(),
            Some("😀")
        );
        assert_eq!(actual, Some(3..5));
    });
}

#[gpui::test]
fn two_views_of_one_document_stay_in_step(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb");
    let other = f.document.clone();
    let second = cx.update(|_, cx| cx.new(|cx| EditorView::new(other, cx)));
    assert_eq!(second.read_with(cx, |v, _| v.layout.row_count()), 2);
    cx.simulate_keystrokes("ctrl-end enter enter");
    assert_eq!(f.rows(cx), 4);
    cx.run_until_parked();
    // the second view heard the event and laid the document out again
    assert_eq!(second.read_with(cx, |v, _| v.layout.row_count()), 4);
    // and the first view did not lay out twice: its layout is exactly right
    f.view.read_with(cx, |v, cx| {
        let doc = v.document.read(cx).doc();
        let fresh = ume_layout::DocumentLayout::laid_out(doc, Default::default()).unwrap();
        assert_eq!(v.layout.row_count(), fresh.row_count());
    });
}

#[gpui::test]
fn the_title_and_the_status_follow_the_document(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab\ncd");
    f.view.read_with(cx, |v, cx| {
        assert_eq!(v.title(cx), "(無題) - UME Editor");
        assert_eq!(v.status(cx).line, 1);
    });
    cx.simulate_keystrokes("down right x");
    f.view.read_with(cx, |v, cx| {
        assert_eq!(v.title(cx), "(無題) * - UME Editor");
        let status = v.status(cx);
        assert_eq!((status.line, status.column, status.lines), (2, 3, 2));
        assert!(status.modified);
    });
}

#[gpui::test]
fn the_insert_key_switches_to_overwrite_mode_and_back(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abcd");
    let overwrite = |f: &Fixture, cx: &mut VisualTestContext| {
        f.document.read_with(cx, |shared, _| shared.overwrite())
    };
    assert!(!overwrite(&f, cx));
    cx.simulate_keystrokes("insert");
    assert!(overwrite(&f, cx));
    assert_eq!(f.lines(cx), ["abcd"]);
    cx.simulate_keystrokes("x y");
    assert_eq!(f.lines(cx), ["xycd"]);
    assert_eq!(f.caret(cx), Pos::new(0, 2));
    cx.simulate_keystrokes("insert z");
    assert!(!overwrite(&f, cx));
    assert_eq!(f.lines(cx), ["xyzcd"]);
}

#[gpui::test]
fn the_status_bar_shows_the_mode(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab");
    f.view
        .read_with(cx, |v, cx| assert!(!v.status(cx).overwrite));
    cx.simulate_keystrokes("insert");
    f.view.read_with(cx, |v, cx| {
        assert!(v.status(cx).overwrite);
        assert!(crate::status::status_text(&v.status(cx)).contains("上書"));
    });
}

#[gpui::test]
fn the_mode_belongs_to_the_document(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab");
    let second = f.document.clone();
    let view2 = cx.update(|_, cx| cx.new(|cx| EditorView::new(second, cx)));
    cx.simulate_keystrokes("insert");
    view2.read_with(cx, |v, cx| assert!(v.status(cx).overwrite));
}

#[gpui::test]
fn a_committed_ime_text_overwrites_in_overwrite_mode(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abcde");
    cx.simulate_keystrokes("right insert");
    f.view.update_in(cx, |v, window, cx| {
        v.replace_and_mark_text_in_range(None, "にほん", Some(9..9), window, cx);
        v.replace_text_in_range(None, "日本", window, cx);
    });
    // two chars committed replace two chars: b and c
    assert_eq!(f.lines(cx), ["a日本de"]);
    // one undo step brings back the text
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(f.lines(cx), ["abcde"]);
}

#[gpui::test]
fn the_ime_inserts_in_insert_mode(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abcde");
    cx.simulate_keystrokes("right");
    f.view.update_in(cx, |v, window, cx| {
        v.replace_and_mark_text_in_range(None, "にほん", Some(9..9), window, cx);
        v.replace_text_in_range(None, "日本", window, cx);
    });
    assert_eq!(f.lines(cx), ["a日本bcde"]);
}

#[gpui::test]
fn a_composition_over_a_selection_overwrites_nothing_more(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abcde");
    cx.simulate_keystrokes("right shift-right shift-right insert");
    f.view.update_in(cx, |v, window, cx| {
        v.replace_and_mark_text_in_range(None, "に", Some(3..3), window, cx);
        v.replace_text_in_range(None, "日", window, cx);
    });
    assert_eq!(f.lines(cx), ["a日de"]);
}

#[gpui::test]
fn another_view_of_the_document_repaints_when_the_mode_changes(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab");
    let second = f.document.clone();
    let view2 = cx.update(|_, cx| cx.new(|cx| EditorView::new(second, cx)));
    let notified = std::rc::Rc::new(std::cell::Cell::new(0));
    let counter = notified.clone();
    cx.update(|_, cx| {
        cx.observe(&view2, move |_, _| counter.set(counter.get() + 1))
            .detach();
    });
    cx.simulate_keystrokes("insert");
    cx.run_until_parked();
    assert!(notified.get() > 0);
    view2.read_with(cx, |v, cx| assert!(v.status(cx).overwrite));
}

#[gpui::test]
fn an_unmarked_composition_overwrites_like_a_commit(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abcde");
    cx.simulate_keystrokes("right insert");
    f.view.update_in(cx, |v, window, cx| {
        v.replace_and_mark_text_in_range(None, "に", Some(3..3), window, cx);
        v.unmark_text(window, cx);
    });
    assert_eq!(f.lines(cx), ["aにcde"]);
    // the composition and its overwrite are one undo step
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(f.lines(cx), ["abcde"]);
}

#[gpui::test]
fn a_click_during_a_composition_overwrites_in_overwrite_mode(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abcde");
    cx.simulate_keystrokes("right insert");
    f.view.update_in(cx, |v, window, cx| {
        v.replace_and_mark_text_in_range(None, "に", Some(3..3), window, cx);
        v.interrupt_composition(cx);
    });
    assert_eq!(f.lines(cx), ["aにcde"]);
}

#[gpui::test]
fn an_unmarked_composition_over_a_selection_overwrites_nothing_more(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abcde");
    cx.simulate_keystrokes("right shift-right shift-right insert");
    f.view.update_in(cx, |v, window, cx| {
        v.replace_and_mark_text_in_range(None, "に", Some(3..3), window, cx);
        v.unmark_text(window, cx);
    });
    assert_eq!(f.lines(cx), ["aにde"]);
}

#[gpui::test]
fn an_unmarked_composition_inserts_in_insert_mode(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abcde");
    cx.simulate_keystrokes("right");
    f.view.update_in(cx, |v, window, cx| {
        v.replace_and_mark_text_in_range(None, "に", Some(3..3), window, cx);
        v.unmark_text(window, cx);
    });
    assert_eq!(f.lines(cx), ["aにbcde"]);
}

#[gpui::test]
fn an_edit_in_another_view_ends_an_overwrite_composition_without_deleting(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abcde");
    let second = f.document.clone();
    let view2 = cx.update(|_, cx| cx.new(|cx| EditorView::new(second, cx)));
    cx.simulate_keystrokes("right insert");
    f.view.update_in(cx, |v, window, cx| {
        v.replace_and_mark_text_in_range(None, "X", Some(1..1), window, cx);
    });
    view2.update(cx, |v, cx| {
        v.run_command("input_char", &[ume_commands::command::Arg::Char('Z')], cx)
    });
    cx.run_until_parked();
    // view 2 typed Z over the first char; the composition of view 1 is kept and nothing more is
    // deleted (b stays)
    assert_eq!(f.lines(cx), ["ZXbcde"]);
}
