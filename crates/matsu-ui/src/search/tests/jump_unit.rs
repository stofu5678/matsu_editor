//! The unit of the line number in the go-to-line panel (`CDlgJump`: `IDC_RADIO_LINENUM_LAYOUT`,
//! `IDC_RADIO_LINENUM_CRLF`, `m_bLineNumIsCRLF_ForJump`; `Command_JUMP` in
//! `CViewCommander_Bookmark.cpp`). The same number lands on different lines in the two units
//! when a line wraps.

use gpui::{TestAppContext, VisualTestContext};
use matsu_core::buffer::Pos;

use super::{Fixture, PanelMode, setup};
use crate::saved_state::other::JumpUnit;

/// Rows at 10 columns (a full-width letter takes 2): line 1 is 2 rows (rows 1 and 2), line 2 is
/// row 3 and line 3 is row 4.
const TEXT: &str = "あいうえおかきくけ\nさしすせそ\nたちつてと";

fn wrapped(cx: &mut TestAppContext) -> (Fixture, &mut VisualTestContext) {
    let (f, cx) = setup(cx, TEXT);
    f.view.update(cx, |v, cx| v.set_wrap_columns(10, cx));
    (f, cx)
}

fn panel_unit(f: &Fixture, cx: &mut VisualTestContext) -> JumpUnit {
    f.view
        .read_with(cx, |v, cx| v.panel.clone().unwrap().read(cx).unit())
}

/// Opens the panel, chooses the unit with its access key (none for the one it has), types the
/// number and presses Enter.
fn go(cx: &mut VisualTestContext, unit_key: Option<&str>, number: &str) {
    cx.simulate_keystrokes("ctrl-j");
    if let Some(key) = unit_key {
        cx.simulate_keystrokes(key);
    }
    cx.simulate_keystrokes("alt-n");
    cx.simulate_input(number);
    cx.simulate_keystrokes("enter");
}

#[gpui::test]
fn the_same_number_lands_on_different_lines_in_the_two_units(cx: &mut TestAppContext) {
    let (f, cx) = wrapped(cx);
    // 改行単位 is the start: line 2 is the second line
    go(cx, None, "2");
    assert_eq!(f.caret(cx), Pos::new(1, 0));
    // 折り返し単位 (Alt+R): row 2 is the wrapped second row of the first line (col 15: bytes, 5 letters of 3)
    go(cx, Some("alt-r"), "2");
    assert_eq!(f.caret(cx), Pos::new(0, 15));
    // the panel kept the choice: row 3 is line 2
    go(cx, None, "3");
    assert_eq!(f.caret(cx), Pos::new(1, 0));
    // Alt+W: back to the lines, where 3 is the third line
    go(cx, Some("alt-w"), "3");
    assert_eq!(f.caret(cx), Pos::new(2, 0));
    // past the end is the last line in both units
    go(cx, None, "99");
    assert_eq!(f.caret(cx), Pos::new(2, 0));
    go(cx, Some("alt-r"), "99");
    assert_eq!(f.caret(cx), Pos::new(2, 0));
}

#[gpui::test]
fn the_choice_is_kept_for_the_next_panel_and_cancel_drops_it(cx: &mut TestAppContext) {
    let (f, cx) = wrapped(cx);
    cx.simulate_keystrokes("ctrl-j");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Jump));
    assert_eq!(panel_unit(&f, cx), JumpUnit::Lines);
    // chosen and cancelled: the next panel starts as before
    cx.simulate_keystrokes("alt-r");
    assert_eq!(panel_unit(&f, cx), JumpUnit::Rows);
    cx.simulate_keystrokes("alt-x ctrl-j");
    assert_eq!(panel_unit(&f, cx), JumpUnit::Lines);
    // ジャンプ keeps it, even when the number is not valid (`GetData` takes it first)
    cx.simulate_keystrokes("alt-r alt-n enter");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Jump));
    cx.simulate_keystrokes("alt-x ctrl-j");
    assert_eq!(panel_unit(&f, cx), JumpUnit::Rows);
}

#[gpui::test]
fn the_choice_is_made_with_a_click_too(cx: &mut TestAppContext) {
    let (f, cx) = wrapped(cx);
    cx.simulate_keystrokes("ctrl-j");
    let rows = cx.debug_bounds("jump-unit-rows").expect("the radio button");
    cx.simulate_click(rows.center(), gpui::Modifiers::none());
    assert_eq!(panel_unit(&f, cx), JumpUnit::Rows);
    let lines = cx
        .debug_bounds("jump-unit-lines")
        .expect("the radio button");
    cx.simulate_click(lines.center(), gpui::Modifiers::none());
    assert_eq!(panel_unit(&f, cx), JumpUnit::Lines);
}

#[gpui::test]
fn a_panel_that_is_reused_for_the_jump_takes_the_remembered_choice(cx: &mut TestAppContext) {
    let (f, cx) = wrapped(cx);
    // the find panel is open; the choice is made elsewhere (another window)
    cx.simulate_keystrokes("ctrl-f");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Find));
    cx.update(|_, app| crate::saved_state::other::remember_jump_unit(app, JumpUnit::Rows));
    f.run(cx, "F_JUMP_DIALOG");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Jump));
    assert_eq!(panel_unit(&f, cx), JumpUnit::Rows);
    // a choice that was not committed is dropped when the panel goes through another mode
    cx.simulate_keystrokes("alt-w");
    assert_eq!(panel_unit(&f, cx), JumpUnit::Lines);
    f.run(cx, "F_SEARCH_DIALOG");
    f.run(cx, "F_JUMP_DIALOG");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Jump));
    assert_eq!(panel_unit(&f, cx), JumpUnit::Rows);
}
