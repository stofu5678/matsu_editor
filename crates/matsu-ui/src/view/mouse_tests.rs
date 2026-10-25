//! Tests of the mouse in the editor view, in GPUI's headless test window (see `tests.rs`): the
//! clicks and drags of Sakura (`CEditView_Mouse.cpp`) with the keys held. The selections
//! themselves are tested without a window in `matsu_commands::context::mouse` and `mouse_drop`;
//! here the events are turned into them.

use std::sync::Mutex;

use gpui::{
    AppContext as _, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, TestAppContext,
    VisualTestContext, point, px,
};
use matsu_commands::edit_options::{self, EditOptions};
use matsu_commands::keymap::{Key, MouseAction, UserRow};
use matsu_core::buffer::{Document, Pos};
use matsu_core::model::DocumentModel;

use super::EditorView;
use super::mouse::next_click_phase;
use super::tests::{Fixture, point_of, setup};
use crate::document::SharedDocument;
use crate::settings::UserSettings;

/// The tests that read or change the options of the mouse run one at a time: the options are
/// shared by the whole process.
static OPTIONS: Mutex<()> = Mutex::new(());

/// Puts the options back when a test ends, also when it fails.
struct Restore<'a>(#[allow(dead_code)] std::sync::MutexGuard<'a, ()>);

impl Drop for Restore<'_> {
    fn drop(&mut self) {
        edit_options::set(EditOptions::default());
    }
}

fn options() -> Restore<'static> {
    let guard = OPTIONS.lock().unwrap_or_else(|e| e.into_inner());
    edit_options::set(EditOptions::default());
    Restore(guard)
}

fn down(cx: &mut VisualTestContext, at: gpui::Point<gpui::Pixels>, count: usize, m: Modifiers) {
    cx.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position: at,
        modifiers: m,
        click_count: count,
        first_mouse: false,
    });
}

fn up(cx: &mut VisualTestContext, at: gpui::Point<gpui::Pixels>, m: Modifiers) {
    cx.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position: at,
        modifiers: m,
        click_count: 1,
    });
}

fn drag_to(cx: &mut VisualTestContext, at: gpui::Point<gpui::Pixels>, m: Modifiers) {
    cx.simulate_mouse_move(at, MouseButton::Left, m);
}

/// A click series: a press and a release for each count.
fn click_series(cx: &mut VisualTestContext, at: gpui::Point<gpui::Pixels>, count: usize) {
    for n in 1..=count {
        down(cx, at, n, Modifiers::default());
        up(cx, at, Modifiers::default());
    }
}

/// A point in the line number column at the row.
fn gutter_of(f: &Fixture, cx: &mut VisualTestContext, row: usize) -> gpui::Point<gpui::Pixels> {
    let p = point_of(f, cx, row, 0);
    let origin = f.view.read_with(cx, |v, _| v.paint.unwrap().origin_x);
    point(px(origin - 2.0), p.y)
}

fn down_at(
    f: &Fixture,
    cx: &mut VisualTestContext,
    row: usize,
    col: usize,
    n: usize,
    m: Modifiers,
) {
    let p = point_of(f, cx, row, col);
    down(cx, p, n, m);
}

fn up_at(f: &Fixture, cx: &mut VisualTestContext, row: usize, col: usize, m: Modifiers) {
    let p = point_of(f, cx, row, col);
    up(cx, p, m);
}

fn drag_at(f: &Fixture, cx: &mut VisualTestContext, row: usize, col: usize, m: Modifiers) {
    let p = point_of(f, cx, row, col);
    drag_to(cx, p, m);
}

fn down_g(f: &Fixture, cx: &mut VisualTestContext, row: usize, n: usize, m: Modifiers) {
    let p = gutter_of(f, cx, row);
    down(cx, p, n, m);
}

fn up_g(f: &Fixture, cx: &mut VisualTestContext, row: usize, m: Modifiers) {
    let p = gutter_of(f, cx, row);
    up(cx, p, m);
}

fn drag_g(f: &Fixture, cx: &mut VisualTestContext, row: usize, m: Modifiers) {
    let p = gutter_of(f, cx, row);
    drag_to(cx, p, m);
}

fn selection(f: &Fixture, cx: &mut VisualTestContext) -> Option<(Pos, Pos)> {
    f.view.read_with(cx, |v, _| v.cursor.selection())
}

fn alt() -> Modifiers {
    Modifiers {
        alt: true,
        ..Default::default()
    }
}

fn ctrl() -> Modifiers {
    Modifiers {
        control: true,
        ..Default::default()
    }
}

#[test]
fn the_series_of_clicks_goes_to_four_and_starts_over() {
    assert_eq!(next_click_phase(1, 1), 1);
    assert_eq!(next_click_phase(1, 2), 2);
    assert_eq!(next_click_phase(2, 3), 3);
    assert_eq!(next_click_phase(3, 4), 4);
    // `OnLBUTTONDBLCLK` clears the check after the quadruple click: the next press is a plain one
    assert_eq!(next_click_phase(4, 5), 1);
    assert_eq!(next_click_phase(1, 6), 2);
    // a lone count of 3 (the press before it got lost) is still a click of the series
    assert_eq!(next_click_phase(1, 3), 2);
}

// ---- double, triple and quadruple clicks ----

#[gpui::test]
fn a_double_click_selects_a_word_and_the_drag_goes_by_words(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "foo bar baz");
    let p = point_of(&f, cx, 0, 5);
    click_series(cx, p, 1);
    down(cx, p, 2, Modifiers::default());
    assert_eq!(selection(&f, cx), Some((Pos::new(0, 4), Pos::new(0, 7))));
    // dragging to the middle of "baz" takes the whole word (`m_bBeginWordSelect`)
    drag_at(&f, cx, 0, 9, Modifiers::default());
    assert_eq!(selection(&f, cx), Some((Pos::new(0, 4), Pos::new(0, 11))));
    // and back into "foo"
    drag_at(&f, cx, 0, 1, Modifiers::default());
    assert_eq!(selection(&f, cx), Some((Pos::new(0, 0), Pos::new(0, 7))));
    up(cx, p, Modifiers::default());
}

#[gpui::test]
fn a_triple_click_selects_the_line_and_the_drag_goes_by_lines(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "abc def\r\nghi\r\njkl\r\nmno");
    let p = point_of(&f, cx, 1, 1);
    click_series(cx, p, 2);
    down(cx, p, 3, Modifiers::default());
    // the line with its ending (`F_SELECTLINE`)
    assert_eq!(selection(&f, cx), Some((Pos::new(1, 0), Pos::new(2, 0))));
    // the "2.5 click": the drag takes whole lines, down
    drag_at(&f, cx, 2, 1, Modifiers::default());
    assert_eq!(selection(&f, cx), Some((Pos::new(1, 0), Pos::new(3, 0))));
    // and up
    drag_at(&f, cx, 0, 2, Modifiers::default());
    assert_eq!(selection(&f, cx), Some((Pos::new(0, 0), Pos::new(2, 0))));
    up(cx, p, Modifiers::default());
    // the next press is a plain click: the caret moves and the selection is gone
    down_at(&f, cx, 3, 2, 1, Modifiers::default());
    up_at(&f, cx, 3, 2, Modifiers::default());
    assert_eq!(selection(&f, cx), None);
    assert_eq!(f.caret(cx), Pos::new(3, 2));
}

#[gpui::test]
fn a_quadruple_click_selects_all_and_the_series_starts_over(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "abc\ndef\nghi");
    let p = point_of(&f, cx, 1, 1);
    click_series(cx, p, 3);
    down(cx, p, 4, Modifiers::default());
    up(cx, p, Modifiers::default());
    assert_eq!(selection(&f, cx), Some((Pos::new(0, 0), Pos::new(2, 3))));
    // the fifth press is a plain one, the sixth a double click
    down(cx, p, 5, Modifiers::default());
    up(cx, p, Modifiers::default());
    assert_eq!(selection(&f, cx), None);
    assert_eq!(f.caret(cx), Pos::new(1, 1));
    down(cx, p, 6, Modifiers::default());
    assert_eq!(selection(&f, cx), Some((Pos::new(1, 0), Pos::new(1, 3))));
}

#[gpui::test]
fn the_double_click_follows_the_key_assignment(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "abc def\nghi");
    // Sakura reads the row `ダブルクリック`: here it is changed to `F_SELECTLINE`
    f.view.update(cx, |v, _| {
        v.keymap.apply_user_rows(&[UserRow {
            key: Key::Mouse(MouseAction::DoubleClick),
            functions: [30403; 8],
        }]);
    });
    let p = point_of(&f, cx, 0, 5);
    down(cx, p, 1, Modifiers::default());
    up(cx, p, Modifiers::default());
    down(cx, p, 2, Modifiers::default());
    assert_eq!(selection(&f, cx), Some((Pos::new(0, 0), Pos::new(1, 0))));
}

// ---- the line number column ----

#[gpui::test]
fn a_click_in_the_line_number_column_selects_the_row(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "abc\r\ndef\r\nghi\r\njkl");
    let p = gutter_of(&f, cx, 1);
    down(cx, p, 1, Modifiers::default());
    assert_eq!(selection(&f, cx), Some((Pos::new(1, 0), Pos::new(2, 0))));
    // dragging down along the column adds rows
    drag_g(&f, cx, 2, Modifiers::default());
    assert_eq!(selection(&f, cx), Some((Pos::new(1, 0), Pos::new(3, 0))));
    // up: from the end of the row that was pressed
    drag_g(&f, cx, 0, Modifiers::default());
    assert_eq!(selection(&f, cx), Some((Pos::new(0, 0), Pos::new(2, 0))));
    up(cx, p, Modifiers::default());
    // the last row has no ending: it is selected to the end of the text
    down_g(&f, cx, 3, 1, Modifiers::default());
    up_g(&f, cx, 3, Modifiers::default());
    assert_eq!(selection(&f, cx), Some((Pos::new(3, 0), Pos::new(3, 3))));
    // a second press there is not a double click: it selects the row again
    down_g(&f, cx, 0, 1, Modifiers::default());
    down_g(&f, cx, 0, 2, Modifiers::default());
    assert_eq!(selection(&f, cx), Some((Pos::new(0, 0), Pos::new(1, 0))));
}

#[gpui::test]
fn shift_and_the_line_number_column_is_a_click_at_the_start_of_the_row(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "abc\ndef\nghi");
    let at = point_of(&f, cx, 0, 2);
    down(cx, at, 1, Modifiers::default());
    up(cx, at, Modifiers::default());
    down_g(&f, cx, 2, 1, Modifiers::shift());
    assert_eq!(selection(&f, cx), Some((Pos::new(0, 2), Pos::new(2, 0))));
}

#[gpui::test]
fn the_ruler_does_nothing(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "abc\ndef\nghi");
    let p = point_of(&f, cx, 2, 1);
    down(cx, p, 1, Modifiers::default());
    up(cx, p, Modifiers::default());
    let origin_y = f.view.read_with(cx, |v, _| v.paint.unwrap().origin_y);
    let ruler = point(p.x, px(origin_y - 2.0));
    down(cx, ruler, 1, Modifiers::default());
    up(cx, ruler, Modifiers::default());
    assert_eq!(f.caret(cx), Pos::new(2, 1));
}

// ---- Ctrl, Shift, Alt ----

#[gpui::test]
fn ctrl_click_selects_the_word_and_the_drag_goes_by_words(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "hello world again");
    down_at(&f, cx, 0, 7, 1, ctrl());
    assert_eq!(selection(&f, cx), Some((Pos::new(0, 6), Pos::new(0, 11))));
    drag_at(&f, cx, 0, 14, ctrl());
    assert_eq!(selection(&f, cx), Some((Pos::new(0, 6), Pos::new(0, 17))));
    up_at(&f, cx, 0, 14, ctrl());
}

#[gpui::test]
fn shift_click_extends_a_box_from_the_caret(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "abcd\nefgh\nijkl");
    down_at(&f, cx, 0, 1, 1, alt());
    drag_at(&f, cx, 1, 3, alt());
    up_at(&f, cx, 1, 3, alt());
    assert!(f.view.read_with(cx, |v, _| v.cursor.is_box()));
    // Sakura drops the box and selects from the caret
    down_at(&f, cx, 2, 2, 1, Modifiers::shift());
    f.view.read_with(cx, |v, _| {
        assert!(!v.cursor.is_box());
        assert_eq!(v.cursor.selection(), Some((Pos::new(1, 3), Pos::new(2, 2))));
    });
}

#[gpui::test]
fn alt_drag_selects_a_box_and_the_release_of_an_empty_one_ends_it(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "abcd\nefgh\nij");
    let start = point_of(&f, cx, 0, 1);
    down(cx, start, 1, alt());
    drag_at(&f, cx, 1, 3, alt());
    f.view.read_with(cx, |v, _| {
        assert!(v.cursor.is_box());
        assert_eq!(v.cursor.box_anchor_col(), Some(1));
        assert_eq!(v.cursor.box_caret_col(), Some(3));
    });
    // the corner can go right of a short row, into the empty space
    drag_at(&f, cx, 2, 5, alt());
    f.view.read_with(cx, |v, _| {
        assert_eq!(v.cursor.box_caret_col(), Some(5));
        assert_eq!(v.cursor.caret(), Pos::new(2, 2));
    });
    // back on the pressed cell: nothing is covered, and the release ends the box
    drag_to(cx, start, alt());
    up(cx, start, alt());
    f.view.read_with(cx, |v, _| {
        assert!(!v.cursor.is_box());
        assert_eq!(v.cursor.caret(), Pos::new(0, 1));
    });
    // a box that stays keeps its mode after the release
    down(cx, start, 1, alt());
    drag_at(&f, cx, 1, 3, alt());
    up_at(&f, cx, 1, 3, alt());
    f.view.read_with(cx, |v, cx| {
        assert!(v.cursor.is_box());
        let text = v.document.read(cx).doc().line(1).unwrap().unwrap().text;
        assert_eq!(text, "efgh");
    });
}

#[gpui::test]
fn a_box_over_tabs_and_wide_characters_follows_display_columns(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "あいう\r\na\tbc\r\nxyz");
    // Alt press between あ and い (column 2), drag to column 4 of the next row
    down_at(&f, cx, 0, 2, 1, alt());
    drag_at(&f, cx, 1, 4, alt());
    up_at(&f, cx, 1, 4, alt());
    f.view.read_with(cx, |v, _| {
        assert_eq!(v.cursor.box_anchor_col(), Some(2));
        assert_eq!(v.cursor.box_caret_col(), Some(4));
        assert_eq!(v.cursor.anchor(), Some(Pos::new(0, 3)));
        // the tab of the second row reaches column 4: the corner is on it or after it
        assert_eq!(v.cursor.caret().line, 1);
    });
    // the text of the box is what the columns cover: い on the first row, the tab on the second
    let text = f.view.update(cx, |v, cx| {
        v.run_with(cx, |c| c.box_text())
            .unwrap()
            .unwrap_or_default()
    });
    assert!(text.starts_with("い\r\n"), "{text:?}");
}

// ---- URLs ----

fn with_url_color<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
) -> (Fixture, &'a mut VisualTestContext) {
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let shared = document.clone();
    let settings = UserSettings::none().view_settings(Some("a.txt"));
    assert!(settings.coloring.shows_url(), "the basic type colors URLs");
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let mut view = EditorView::new(shared, cx);
        view.apply_settings(settings, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    (Fixture { document, view }, vcx)
}

#[gpui::test]
fn a_click_on_a_url_selects_it_and_a_double_click_opens_it(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = with_url_color(cx, "see https://example.com/a?b=1 now");
    let p = point_of(&f, cx, 0, 10);
    down(cx, p, 1, Modifiers::default());
    up(cx, p, Modifiers::default());
    assert_eq!(selection(&f, cx), Some((Pos::new(0, 4), Pos::new(0, 29))));
    down(cx, p, 2, Modifiers::default());
    up(cx, p, Modifiers::default());
    assert_eq!(
        cx.opened_url().as_deref(),
        Some("https://example.com/a?b=1")
    );
    // a click beside it selects nothing
    down_at(&f, cx, 0, 1, 1, Modifiers::default());
    up_at(&f, cx, 0, 1, Modifiers::default());
    assert_eq!(selection(&f, cx), None);
}

#[gpui::test]
fn the_setting_turns_the_selecting_of_urls_off(cx: &mut TestAppContext) {
    let _o = options();
    edit_options::set(EditOptions {
        select_clicked_url: false,
        ..EditOptions::default()
    });
    let (f, cx) = with_url_color(cx, "see https://example.com/a now");
    let p = point_of(&f, cx, 0, 10);
    down(cx, p, 1, Modifiers::default());
    up(cx, p, Modifiers::default());
    assert_eq!(selection(&f, cx), None);
    assert_eq!(f.caret(cx), Pos::new(0, 10));
}

#[gpui::test]
fn a_mail_address_opens_as_mailto(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = with_url_color(cx, "write to someone@example.com today");
    let p = point_of(&f, cx, 0, 12);
    down(cx, p, 1, Modifiers::default());
    up(cx, p, Modifiers::default());
    down(cx, p, 2, Modifiers::default());
    up(cx, p, Modifiers::default());
    assert_eq!(
        cx.opened_url().as_deref(),
        Some("mailto:someone@example.com")
    );
}

// ---- drag and drop ----

fn select(f: &Fixture, cx: &mut VisualTestContext, from: Pos, to: Pos) {
    f.view.update(cx, |v, cx| {
        v.run_with(cx, |c| c.select_range(from, to)).unwrap();
    });
}

#[gpui::test]
fn dragging_the_selection_moves_it_as_one_undo_step(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "abcdef ghi");
    select(&f, cx, Pos::new(0, 0), Pos::new(0, 3));
    let inside = point_of(&f, cx, 0, 1);
    down(cx, inside, 1, Modifiers::default());
    let target = point_of(&f, cx, 0, 8);
    drag_to(cx, target, Modifiers::default());
    // the caret shows where the text would go, the selection stays
    f.view.read_with(cx, |v, _| {
        assert_eq!(v.drop_caret, Some(Pos::new(0, 8)));
        assert_eq!(v.cursor.selection(), Some((Pos::new(0, 0), Pos::new(0, 3))));
    });
    up(cx, target, Modifiers::default());
    assert_eq!(f.lines(cx), ["def gabchi"]);
    assert_eq!(selection(&f, cx), Some((Pos::new(0, 5), Pos::new(0, 8))));
    f.view.read_with(cx, |v, _| assert_eq!(v.drop_caret, None));
    cx.simulate_keystrokes("ctrl-z");
    assert_eq!(f.lines(cx), ["abcdef ghi"]);
}

#[gpui::test]
fn dragging_with_ctrl_copies(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "あいう\r\nえお");
    select(&f, cx, Pos::new(0, 0), Pos::new(0, "あい".len()));
    down_at(&f, cx, 0, 1, 1, Modifiers::default());
    let target = point_of(&f, cx, 1, 2);
    drag_to(cx, target, ctrl());
    up(cx, target, ctrl());
    assert_eq!(f.lines(cx), ["あいう", "えあいお"]);
    assert_eq!(
        selection(&f, cx),
        Some((Pos::new(1, "え".len()), Pos::new(1, "えあい".len())))
    );
}

#[gpui::test]
fn a_press_in_the_selection_without_a_drag_is_a_click(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "abcdef");
    select(&f, cx, Pos::new(0, 1), Pos::new(0, 5));
    let p = point_of(&f, cx, 0, 3);
    down(cx, p, 1, Modifiers::default());
    // nothing has changed until the button is released
    assert_eq!(selection(&f, cx), Some((Pos::new(0, 1), Pos::new(0, 5))));
    up(cx, p, Modifiers::default());
    assert_eq!(selection(&f, cx), None);
    assert_eq!(f.caret(cx), Pos::new(0, 3));
}

#[gpui::test]
fn text_let_go_outside_the_view_is_not_moved(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "abcdef ghi");
    select(&f, cx, Pos::new(0, 0), Pos::new(0, 3));
    down_at(&f, cx, 0, 1, 1, Modifiers::default());
    drag_at(&f, cx, 0, 8, Modifiers::default());
    f.view.read_with(cx, |v, _| assert!(v.drop_caret.is_some()));
    let outside = point(px(-50.0), px(-50.0));
    up(cx, outside, Modifiers::default());
    assert_eq!(f.lines(cx), ["abcdef ghi"]);
    assert_eq!(selection(&f, cx), Some((Pos::new(0, 0), Pos::new(0, 3))));
    f.view.read_with(cx, |v, _| {
        assert_eq!(v.drop_caret, None);
        assert_eq!(v.drag, super::Drag::None);
    });
}

#[gpui::test]
fn dropping_on_the_selection_changes_nothing(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "abcdef");
    select(&f, cx, Pos::new(0, 1), Pos::new(0, 5));
    down_at(&f, cx, 0, 2, 1, Modifiers::default());
    let target = point_of(&f, cx, 0, 4);
    drag_to(cx, target, Modifiers::default());
    up(cx, target, Modifiers::default());
    assert_eq!(f.lines(cx), ["abcdef"]);
    assert_eq!(selection(&f, cx), Some((Pos::new(0, 1), Pos::new(0, 5))));
}

#[gpui::test]
fn without_drag_and_drop_a_press_in_the_selection_selects_again(cx: &mut TestAppContext) {
    let _o = options();
    edit_options::set(EditOptions {
        drag_drop: false,
        ..EditOptions::default()
    });
    let (f, cx) = setup(cx, "abcdef ghi");
    select(&f, cx, Pos::new(0, 0), Pos::new(0, 3));
    down_at(&f, cx, 0, 1, 1, Modifiers::default());
    drag_at(&f, cx, 0, 8, Modifiers::default());
    up_at(&f, cx, 0, 8, Modifiers::default());
    assert_eq!(f.lines(cx), ["abcdef ghi"]);
    assert_eq!(selection(&f, cx), Some((Pos::new(0, 1), Pos::new(0, 8))));
    // the drop source alone off: the same
    edit_options::set(EditOptions {
        drop_source: false,
        ..EditOptions::default()
    });
    select(&f, cx, Pos::new(0, 0), Pos::new(0, 3));
    down_at(&f, cx, 0, 1, 1, Modifiers::default());
    drag_at(&f, cx, 0, 8, Modifiers::default());
    up_at(&f, cx, 0, 8, Modifiers::default());
    assert_eq!(f.lines(cx), ["abcdef ghi"]);
}

#[gpui::test]
fn dragging_a_box_moves_it(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "abcd\r\nefgh\r\nijkl\r\n");
    down_at(&f, cx, 0, 0, 1, alt());
    drag_at(&f, cx, 1, 2, alt());
    up_at(&f, cx, 1, 2, alt());
    // press inside the box, drop two rows down
    down_at(&f, cx, 0, 1, 1, Modifiers::default());
    let target = point_of(&f, cx, 2, 2);
    drag_to(cx, target, Modifiers::default());
    up(cx, target, Modifiers::default());
    assert_eq!(f.lines(cx), ["cd", "gh", "ijabkl", "  ef"]);
}

// ---- the side buttons ----

#[gpui::test]
fn the_side_buttons_run_their_functions(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "abc\ndef");
    // no function is assigned to the side buttons by default: nothing happens
    let p = point_of(&f, cx, 1, 1);
    for button in [
        gpui::NavigationDirection::Back,
        gpui::NavigationDirection::Forward,
    ] {
        let button = MouseButton::Navigate(button);
        cx.simulate_mouse_down(p, button, Modifiers::default());
        cx.simulate_mouse_up(p, button, Modifiers::default());
    }
    assert_eq!(f.caret(cx), Pos::new(0, 0));
    // F_GOFILEEND on the back button, and F_GOFILETOP on the forward button with Ctrl
    f.view.update(cx, |v, _| {
        let mut back = [0; 8];
        back[0] = 30351;
        let mut forward = [0; 8];
        forward[2] = 30350;
        v.keymap.apply_user_rows(&[
            UserRow {
                key: Key::Mouse(MouseAction::LeftSideClick),
                functions: back,
            },
            UserRow {
                key: Key::Mouse(MouseAction::RightSideClick),
                functions: forward,
            },
        ]);
    });
    let back = MouseButton::Navigate(gpui::NavigationDirection::Back);
    cx.simulate_mouse_down(p, back, Modifiers::default());
    cx.simulate_mouse_up(p, back, Modifiers::default());
    assert_eq!(f.caret(cx), Pos::new(1, 3));
    let forward = MouseButton::Navigate(gpui::NavigationDirection::Forward);
    cx.simulate_mouse_down(p, forward, ctrl());
    cx.simulate_mouse_up(p, forward, ctrl());
    assert_eq!(f.caret(cx), Pos::new(0, 0));
}

// ---- the free cursor ----

#[gpui::test]
fn in_the_free_cursor_mode_a_click_right_of_the_text_puts_the_caret_there(cx: &mut TestAppContext) {
    // `CCaret::MoveCursorProperly`: the column of the click, when the mode is on.
    // Declared first, so that it is let go last: the options are back before the next test.
    let _serial = crate::common_settings::options_lock();
    let _o = options();
    edit_options::set(EditOptions {
        free_cursor: true,
        ..EditOptions::default()
    });
    let (f, cx) = setup(cx, "ab\ncdef");
    down_at(&f, cx, 0, 6, 1, Modifiers::default());
    up_at(&f, cx, 0, 6, Modifiers::default());
    f.view.read_with(cx, |v, app| {
        assert_eq!(v.cursor.caret(), Pos::new(0, 2));
        assert_eq!(v.cursor.free_col(), Some(6));
        assert_eq!(v.caret_column(app), 6);
    });
    // typing there fills the gap with spaces
    cx.simulate_input("x");
    assert_eq!(f.lines(cx), ["ab    x", "cdef"]);
    // a click on the text leaves the gap
    down_at(&f, cx, 1, 1, 1, Modifiers::default());
    up_at(&f, cx, 1, 1, Modifiers::default());
    f.view.read_with(cx, |v, _| {
        assert_eq!(v.cursor.caret(), Pos::new(1, 1));
        assert_eq!(v.cursor.free_col(), None);
    });
}

#[gpui::test]
fn without_the_free_cursor_mode_a_click_right_of_the_text_goes_to_its_end(cx: &mut TestAppContext) {
    let _o = options();
    let (f, cx) = setup(cx, "ab\ncdef");
    down_at(&f, cx, 0, 6, 1, Modifiers::default());
    up_at(&f, cx, 0, 6, Modifiers::default());
    f.view.read_with(cx, |v, _| {
        assert_eq!(v.cursor.caret(), Pos::new(0, 2));
        assert_eq!(v.cursor.free_col(), None);
    });
}
