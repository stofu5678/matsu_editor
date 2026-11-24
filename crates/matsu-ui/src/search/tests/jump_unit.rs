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
        .read_with(cx, |v, cx| v.panel.clone().unwrap().read(cx).unit(cx))
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

/// The key macro that the last recording left, as text.
fn recorded(f: &Fixture, cx: &mut VisualTestContext) -> String {
    f.view
        .read_with(cx, |v, _| v.recorded_key_macro_text())
        .expect("a macro was recorded")
}

#[gpui::test]
fn a_panel_jump_is_recorded_as_jump_with_the_unit_and_replays_to_the_same_place(
    cx: &mut TestAppContext,
) {
    let (f, cx) = wrapped(cx);
    f.run(cx, "F_RECKEYMACRO");
    // 改行単位 (the start): option 1
    go(cx, None, "2");
    // 折り返し単位: option 0
    go(cx, Some("alt-r"), "2");
    f.run(cx, "F_RECKEYMACRO");
    let text = recorded(&f, cx);
    assert!(text.contains("Jump(2, 1)"), "{text}");
    assert!(text.contains("Jump(2, 0)"), "{text}");
    assert!(
        text.find("Jump(2, 1)") < text.find("Jump(2, 0)"),
        "in the order of the jumps: {text}"
    );
    // replay from the top: the two jumps end on the wrapped second row of line 1
    f.view
        .update(cx, |v, cx| v.run_command("go_file_top", &[], cx));
    assert_eq!(f.caret(cx), Pos::new(0, 0));
    f.run(cx, "F_EXECKEYMACRO");
    cx.run_until_parked();
    assert_eq!(f.caret(cx), Pos::new(0, 15));
}

#[gpui::test]
fn a_jump_with_a_number_that_is_not_valid_records_nothing(cx: &mut TestAppContext) {
    let (f, cx) = wrapped(cx);
    f.run(cx, "F_RECKEYMACRO");
    cx.simulate_keystrokes("ctrl-j alt-n enter");
    cx.simulate_keystrokes("alt-x");
    f.run(cx, "F_RECKEYMACRO");
    let text = recorded(&f, cx);
    assert!(!text.contains("Jump("), "{text}");
}

#[gpui::test]
fn a_macro_jump_sets_the_unit_the_panel_shows_next(cx: &mut TestAppContext) {
    let (f, cx) = wrapped(cx);
    let jump = |cx: &mut VisualTestContext, args: &[matsu_commands::command::Arg]| {
        let code = matsu_commands::sakura::FUNCTIONS
            .iter()
            .find(|f| f.name == "F_JUMP")
            .unwrap()
            .code;
        f.view.update_in(cx, |v, window, cx| {
            v.run_macro_function_for_test(code, args, window, cx)
        });
    };
    use matsu_commands::command::Arg;
    // option 0: 折り返し単位
    jump(cx, &[Arg::Int(2), Arg::Int(0)]);
    assert_eq!(f.caret(cx), Pos::new(0, 15));
    cx.simulate_keystrokes("ctrl-j");
    assert_eq!(panel_unit(&f, cx), JumpUnit::Rows);
    cx.simulate_keystrokes("alt-x");
    // option 1: 改行単位
    jump(cx, &[Arg::Int(2), Arg::Int(1)]);
    assert_eq!(f.caret(cx), Pos::new(1, 0));
    cx.simulate_keystrokes("ctrl-j");
    assert_eq!(panel_unit(&f, cx), JumpUnit::Lines);
    cx.simulate_keystrokes("alt-x");
    // option 0 again, then the option left out is 1
    jump(cx, &[Arg::Int(3), Arg::Int(0)]);
    jump(cx, &[Arg::Int(3)]);
    cx.simulate_keystrokes("ctrl-j");
    assert_eq!(panel_unit(&f, cx), JumpUnit::Lines);
    assert_eq!(f.caret(cx), Pos::new(2, 0));
}

fn f_jump() -> u16 {
    matsu_commands::sakura::FUNCTIONS
        .iter()
        .find(|f| f.name == "F_JUMP")
        .unwrap()
        .code
}

#[gpui::test]
fn a_text_option_of_a_macro_jump_is_read_like_the_jump_reads_it(cx: &mut TestAppContext) {
    use matsu_commands::command::Arg;
    let (f, cx) = wrapped(cx);
    let jump = |cx: &mut VisualTestContext, option: &str| {
        let args = [Arg::Int(2), Arg::Text(option.into())];
        f.view.update_in(cx, |v, window, cx| {
            v.run_macro_function_for_test(f_jump(), &args, window, cx)
        });
    };
    // "0x" is 0 (rows) and jumps by rows; "1x" is 1 (lines) and jumps by lines
    jump(cx, "0x");
    assert_eq!(f.caret(cx), Pos::new(0, 15));
    cx.simulate_keystrokes("ctrl-j");
    assert_eq!(panel_unit(&f, cx), JumpUnit::Rows);
    cx.simulate_keystrokes("alt-x");
    jump(cx, "1x");
    assert_eq!(f.caret(cx), Pos::new(1, 0));
    cx.simulate_keystrokes("ctrl-j");
    assert_eq!(panel_unit(&f, cx), JumpUnit::Lines);
}

#[gpui::test]
fn an_open_panel_follows_a_macro_jump_and_keeps_its_unit_in_the_next_jump(cx: &mut TestAppContext) {
    use matsu_commands::command::Arg;
    let (f, cx) = wrapped(cx);
    cx.simulate_keystrokes("ctrl-j");
    assert_eq!(panel_unit(&f, cx), JumpUnit::Lines);
    // the panel stays open while a macro (of another tab) says `Jump(2, 0)`
    f.view.update_in(cx, |v, window, cx| {
        v.run_macro_function_for_test(f_jump(), &[Arg::Int(2), Arg::Int(0)], window, cx)
    });
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Jump));
    assert_eq!(panel_unit(&f, cx), JumpUnit::Rows);
    // a jump from the panel keeps the macro's unit: row 3 is line 2
    cx.simulate_keystrokes("alt-n");
    cx.simulate_input("3");
    cx.simulate_keystrokes("enter");
    assert_eq!(f.caret(cx), Pos::new(1, 0));
    cx.simulate_keystrokes("ctrl-j");
    assert_eq!(panel_unit(&f, cx), JumpUnit::Rows);
}

#[gpui::test]
fn a_choice_clicked_in_an_open_panel_stays_while_another_panel_jumps(cx: &mut TestAppContext) {
    let (f, cx) = wrapped(cx);
    cx.simulate_keystrokes("ctrl-j");
    assert_eq!(panel_unit(&f, cx), JumpUnit::Lines);
    // panel A: the user clicks 折り返し単位 and has not pressed ジャンプ
    cx.simulate_keystrokes("alt-r");
    assert_eq!(panel_unit(&f, cx), JumpUnit::Rows);
    // panel B (another tab) commits 改行単位 with its ジャンプ: the same write as `jump` makes
    cx.update(|_, app| crate::saved_state::other::remember_jump_unit(app, JumpUnit::Lines));
    cx.run_until_parked();
    assert_eq!(
        panel_unit(&f, cx),
        JumpUnit::Rows,
        "the click is A's own until it is committed"
    );
    // A's ジャンプ commits what A shows: row 3 is line 2, and Rows is remembered
    cx.simulate_keystrokes("alt-n");
    cx.simulate_input("3");
    cx.simulate_keystrokes("enter");
    assert_eq!(f.caret(cx), Pos::new(1, 0));
    assert_eq!(
        cx.update(|_, app| crate::saved_state::other::jump_unit(app)),
        JumpUnit::Rows
    );
}

#[gpui::test]
fn a_jump_with_a_number_that_is_not_valid_keeps_the_radio_of_the_panel(cx: &mut TestAppContext) {
    let (f, cx) = wrapped(cx);
    cx.simulate_keystrokes("ctrl-j");
    // click 折り返し単位, then ジャンプ with no number (`GetData` writes the unit first)
    cx.simulate_keystrokes("alt-r alt-n enter");
    assert_eq!(
        f.panel_mode(cx),
        Some(PanelMode::Jump),
        "the panel stays open"
    );
    assert_eq!(panel_unit(&f, cx), JumpUnit::Rows);
    assert_eq!(
        cx.update(|_, app| crate::saved_state::other::jump_unit(app)),
        JumpUnit::Rows
    );
    // another panel or a macro commits 改行単位: the radio of this panel does not move
    cx.update(|_, app| crate::saved_state::other::remember_jump_unit(app, JumpUnit::Lines));
    cx.run_until_parked();
    assert_eq!(panel_unit(&f, cx), JumpUnit::Rows);
    // a valid ジャンプ lands in Rows (row 3 is line 2) and remembers Rows
    cx.simulate_keystrokes("alt-n");
    cx.simulate_input("3");
    cx.simulate_keystrokes("enter");
    assert_eq!(f.caret(cx), Pos::new(1, 0));
    assert_eq!(
        cx.update(|_, app| crate::saved_state::other::jump_unit(app)),
        JumpUnit::Rows
    );
}

#[gpui::test]
fn the_access_keys_choose_the_radio_buttons(cx: &mut TestAppContext) {
    let (f, cx) = wrapped(cx);
    cx.simulate_keystrokes("ctrl-j");
    assert_eq!(panel_unit(&f, cx), JumpUnit::Lines);
    // Alt+R chooses the rows and Alt+W the lines again: the access keys stay with the radio buttons
    cx.simulate_keystrokes("alt-r");
    assert_eq!(panel_unit(&f, cx), JumpUnit::Rows);
    cx.simulate_keystrokes("alt-w");
    assert_eq!(panel_unit(&f, cx), JumpUnit::Lines);
}

#[gpui::test]
fn the_unit_choices_are_two_small_radio_buttons_one_above_the_other(cx: &mut TestAppContext) {
    // `IDC_RADIO_LINENUM_LAYOUT` and `_CRLF` (`BS_AUTORADIOBUTTON`): a circle and the label, not
    // two wide buttons that fill the panel
    let (_f, cx) = wrapped(cx);
    cx.simulate_keystrokes("ctrl-j");
    let panel = cx.debug_bounds("search-panel").expect("the panel");
    let rows = cx.debug_bounds("jump-unit-rows").expect("the radio button");
    let lines = cx
        .debug_bounds("jump-unit-lines")
        .expect("the radio button");
    assert!(rows.bottom() <= lines.top(), "one above the other");
    assert!(rows.size.width < panel.size.width / 2.0, "{rows:?}");
}

#[gpui::test]
fn the_line_number_box_is_as_wide_as_sakuras_50_dialog_units(cx: &mut TestAppContext) {
    // `IDC_EDIT_LINENUM` is 50 dialog units wide, spin button included (`IDD_JUMP`): 75 px at 1.5
    // px a unit, of which the arrows take `spin::ARROW_WIDTH`
    let (_f, cx) = wrapped(cx);
    cx.simulate_keystrokes("ctrl-j");
    let field = cx.debug_bounds("jump-line-box").expect("the box");
    let whole = f32::from(field.size.width) + crate::spin::ARROW_WIDTH;
    assert!((whole - 75.0).abs() < 0.5, "{whole}");
}
