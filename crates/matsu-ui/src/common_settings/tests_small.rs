//! Tests of the small options of the common settings that this file's pages made live: the dialog
//! 位置と大きさの設定, the click that activates the window, the flat toolbar buttons, the Grep
//! options and the wait before the question about stopping a macro.

use gpui::{AnyWindowHandle, TestAppContext};

use super::model::Kind;
use super::tests::{Ini, check, ok, run, setup, type_in, with_dialog};
use crate::saved_state::window::{SaveMode, state};
use crate::win_size_dialog::{
    self, Field, MODE_SAVE, MODE_SET, SIZE_MAXIMIZED, SIZE_MINIMIZED, WinSize,
};

/// Presses the button `id` of the page the dialog shows.
fn press(cx: &mut TestAppContext, id: &'static str) {
    with_dialog(cx, |d, window, dcx| {
        let control = d
            .page_controls()
            .into_iter()
            .find(|c| c.id == id)
            .expect("the button");
        assert!(d.is_usable(&control), "{id} is usable");
        let Kind::Button { run } = control.kind else {
            panic!("{id} is not a button");
        };
        run(d, window, dcx);
    });
}

/// Does something to the dialog 位置と大きさの設定 and its window.
fn with_win_size<R>(
    cx: &mut TestAppContext,
    f: impl FnOnce(
        &mut win_size_dialog::WinSizeDialog,
        &mut gpui::Window,
        &mut gpui::Context<win_size_dialog::WinSizeDialog>,
    ) -> R,
) -> R {
    let (dialog, handle) = cx
        .update(|app| win_size_dialog::current(app))
        .expect("the dialog is open");
    let result = handle
        .update(cx, |_, window, app| {
            dialog.update(app, |dialog, dcx| f(dialog, window, dcx))
        })
        .unwrap();
    cx.run_until_parked();
    result
}

fn win_size_is_open(cx: &mut TestAppContext) -> bool {
    cx.update(|app| win_size_dialog::current(app)).is_some()
}

fn open_window_page(cx: &mut TestAppContext, text: &str) -> (Ini, AnyWindowHandle) {
    let (ini, handle) = setup(cx, text);
    run(cx, handle, "F_OPTION_WINDOW");
    (ini, handle)
}

const KEYS: [&str; 7] = [
    "bSaveWindowSize",
    "nWinSizeType",
    "nWinSizeCX",
    "nWinSizeCY",
    "nSaveWindowPos",
    "nWinPosX",
    "nWinPosY",
];

#[gpui::test]
fn the_button_opens_the_dialog_with_the_values_of_the_file(cx: &mut TestAppContext) {
    let (_ini, _handle) = open_window_page(
        cx,
        "[Common]\r\nbSaveWindowSize=2\r\nnWinSizeType=1\r\nnWinSizeCX=800\r\nnWinSizeCY=600\r\n\
         nSaveWindowPos=1\r\nnWinPosX=-30\r\nnWinPosY=40\r\n",
    );
    press(cx, "win_size");
    assert!(win_size_is_open(cx));
    let now = with_win_size(cx, |d, _, dcx| d.values(dcx));
    assert_eq!(
        now,
        WinSize {
            size_mode: MODE_SET,
            size_type: SIZE_MINIMIZED,
            width: 800,
            height: 600,
            pos_mode: MODE_SAVE,
            x: -30,
            y: 40,
        }
    );
}

#[gpui::test]
fn the_values_of_the_dialog_are_written_when_ok_is_pressed(cx: &mut TestAppContext) {
    let (ini, _handle) = open_window_page(cx, "[Other]\r\nx=1\r\n");
    press(cx, "win_size");
    with_win_size(cx, |d, window, dcx| {
        d.set_size_mode(MODE_SET, dcx);
        d.set_size_type(SIZE_MAXIMIZED, dcx);
        d.set_text(Field::Width, "900", window, dcx);
        d.set_text(Field::Height, "700", window, dcx);
        d.set_pos_mode(MODE_SET, dcx);
        d.set_text(Field::X, "-12", window, dcx);
        d.set_text(Field::Y, "34", window, dcx);
        d.close(window, dcx);
    });
    assert!(!win_size_is_open(cx), "閉じる closes the dialog");
    // Nothing is written until OK of the common settings.
    assert_eq!(ini.get("nWinSizeCX"), None);
    ok(cx);
    for (key, value) in KEYS
        .into_iter()
        .zip(["2", "2", "900", "700", "2", "-12", "34"])
    {
        assert_eq!(ini.get(key).as_deref(), Some(value), "{key}");
    }
    // The windows that open from now on take them.
    let now = cx.update(|app| state(app));
    assert_eq!(
        (now.size_mode, now.pos_mode),
        (SaveMode::Set, SaveMode::Set)
    );
    assert_eq!((now.width, now.height, now.x, now.y), (900, 700, -12, 34));
    assert_eq!(now.size_type, SIZE_MAXIMIZED);
    assert_eq!(ini.profile().get("Other", "x"), Some("1"));
}

#[gpui::test]
fn cancel_of_the_sheet_throws_the_dialog_away(cx: &mut TestAppContext) {
    let (ini, _handle) = open_window_page(cx, "[Common]\r\nnWinSizeCX=640\r\nnWinSizeCY=480\r\n");
    let bytes = ini.bytes();
    press(cx, "win_size");
    with_win_size(cx, |d, window, dcx| {
        d.set_size_mode(MODE_SET, dcx);
        d.set_text(Field::Width, "999", window, dcx);
        d.close(window, dcx);
    });
    super::tests::cancel(cx);
    assert_eq!(ini.bytes(), bytes);
    assert_eq!(cx.update(|app| state(app)).width, 640);
}

#[gpui::test]
fn closing_without_a_change_keeps_a_size_that_was_never_stored(cx: &mut TestAppContext) {
    // `CW_USEDEFAULT` is far outside the range of the spin button; 閉じる must not clamp it.
    let (_ini, _handle) = open_window_page(cx, "");
    press(cx, "win_size");
    let before = with_win_size(cx, |d, _, dcx| d.values(dcx));
    assert_eq!(before.width, i32::MIN);
    assert_eq!(before.x, i32::MIN);
    assert_eq!(before.size_mode, MODE_SAVE);
    assert_eq!(before.pos_mode, 0);
    with_win_size(cx, |d, window, dcx| d.close(window, dcx));
    press(cx, "win_size");
    let after = with_win_size(cx, |d, _, dcx| d.values(dcx));
    assert_eq!(after, before);
}

#[gpui::test]
fn the_numbers_are_held_to_the_range_of_the_spin_buttons(cx: &mut TestAppContext) {
    let (_ini, _handle) = open_window_page(cx, "");
    press(cx, "win_size");
    let now = with_win_size(cx, |d, window, dcx| {
        d.set_text(Field::Width, "99999", window, dcx);
        d.set_text(Field::Height, "-5", window, dcx);
        d.set_text(Field::X, "-99999", window, dcx);
        d.set_text(Field::Y, "abc", window, dcx);
        d.values(dcx)
    });
    assert_eq!((now.width, now.height, now.x, now.y), (30000, 0, -30000, 0));
}

#[gpui::test]
fn the_close_button_of_the_window_reads_the_boxes_too(cx: &mut TestAppContext) {
    let (_ini, _handle) = open_window_page(cx, "");
    press(cx, "win_size");
    with_win_size(cx, |d, _, dcx| d.set_size_type(SIZE_MAXIMIZED, dcx));
    // `IDCANCEL` reads the controls as well (`CDlgWinSize::OnBnClicked`).
    let (_, handle) = cx.update(|app| win_size_dialog::current(app)).unwrap();
    let closed = gpui::VisualTestContext::from_window(handle, cx)
        .into_mut()
        .simulate_close();
    assert!(closed);
    cx.run_until_parked();
    // The sheet took the kind of window.
    assert_eq!(
        with_dialog(cx, |d, _, _| d.values().int("win_size_type")),
        SIZE_MAXIMIZED
    );
}

#[gpui::test]
fn ok_twice_with_no_change_leaves_the_file_as_it_is(cx: &mut TestAppContext) {
    let (ini, handle) = open_window_page(
        cx,
        "[Common]\r\nbSaveWindowSize=2\r\nnWinSizeType=0\r\nnWinSizeCX=800\r\nnWinSizeCY=600\r\n\
         nSaveWindowPos=2\r\nnWinPosX=5\r\nnWinPosY=6\r\n",
    );
    // The first OK may add the keys of the pages that the file lacks.
    ok(cx);
    let bytes = ini.bytes();
    run(cx, handle, "F_OPTION_WINDOW");
    press(cx, "win_size");
    with_win_size(cx, |d, window, dcx| d.close(window, dcx));
    ok(cx);
    assert_eq!(ini.bytes(), bytes);
    // And the seven lines are the ones that the file had.
    let text = String::from_utf8_lossy(&ini.bytes()).into_owned();
    for line in ["nWinSizeCX=800", "nWinPosX=5", "bSaveWindowSize=2"] {
        assert!(text.contains(line), "{line}");
    }
}

#[gpui::test]
fn a_window_that_moved_while_the_sheet_was_open_keeps_its_place(cx: &mut TestAppContext) {
    use gpui::{Bounds, WindowBounds, point, px, size};
    let (ini, _handle) = open_window_page(
        cx,
        "[Common]\r\nbSaveWindowSize=1\r\nnWinSizeCX=640\r\nnWinSizeCY=480\r\n\
         nSaveWindowPos=1\r\nnWinPosX=10\r\nnWinPosY=20\r\n",
    );
    // The sheet started with 640 x 480 at (10, 20); a window is moved and resized meanwhile.
    cx.update(|app| {
        crate::saved_state::window::bounds_changed(
            app,
            WindowBounds::Windowed(Bounds::new(
                point(px(30.), px(40.)),
                size(px(800.), px(600.)),
            )),
        )
    });
    check(cx, "disp_status_bar", false);
    ok(cx);
    assert_eq!(ini.get("nWinSizeCX").as_deref(), Some("800"));
    assert_eq!(ini.get("nWinSizeCY").as_deref(), Some("600"));
    assert_eq!(ini.get("nWinPosX").as_deref(), Some("30"));
    assert_eq!(ini.get("nWinPosY").as_deref(), Some("40"));
}

#[gpui::test]
fn a_change_in_the_dialog_wins_over_a_window_that_moved(cx: &mut TestAppContext) {
    use gpui::{Bounds, WindowBounds, point, px, size};
    let (ini, _handle) = open_window_page(cx, "[Common]\r\nnWinSizeCX=640\r\nnWinSizeCY=480\r\n");
    press(cx, "win_size");
    with_win_size(cx, |d, window, dcx| {
        d.set_size_mode(MODE_SET, dcx);
        d.set_text(Field::Width, "1000", window, dcx);
        d.close(window, dcx);
    });
    cx.update(|app| {
        crate::saved_state::window::bounds_changed(
            app,
            WindowBounds::Windowed(Bounds::new(point(px(0.), px(0.)), size(px(800.), px(600.)))),
        )
    });
    ok(cx);
    assert_eq!(ini.get("nWinSizeCX").as_deref(), Some("1000"));
    assert_eq!(ini.get("bSaveWindowSize").as_deref(), Some("2"));
}

// ---- 全般 ----

#[gpui::test]
fn the_activating_click_option_is_live_and_kept(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION_GENERAL");
    assert!(!cx.update(|app| super::no_caret_move_by_activation(app)));
    check(cx, "no_move_activate_by_mouse", true);
    ok(cx);
    assert_eq!(ini.get("NoCaretMoveByActivation").as_deref(), Some("1"));
    assert!(cx.update(|app| super::no_caret_move_by_activation(app)));
    // The next start reads it.
    run(cx, handle, "F_OPTION_GENERAL");
    assert!(with_dialog(cx, |d, _, _| d
        .values()
        .bool("no_move_activate_by_mouse")));
    check(cx, "no_move_activate_by_mouse", false);
    ok(cx);
    assert_eq!(ini.get("NoCaretMoveByActivation").as_deref(), Some("0"));
    assert!(!cx.update(|app| super::no_caret_move_by_activation(app)));
}

// ---- ツールバー ----

#[gpui::test]
fn the_flat_buttons_are_live_and_kept_in_the_toolbar_section(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    assert!(cx.update(|app| crate::bars::toolbar::is_flat(app)));
    run(cx, handle, "F_OPTION_TOOLBAR");
    assert!(with_dialog(cx, |d, _, _| d.values().bool("toolbar_flat")));
    check(cx, "toolbar_flat", false);
    ok(cx);
    assert_eq!(
        ini.profile().get("Toolbar", "bToolBarIsFlat"),
        Some("0"),
        "the key is in [Toolbar] (ShareData_IO_Toolbar)"
    );
    assert_eq!(ini.get("bToolBarIsFlat"), None);
    assert!(!cx.update(|app| crate::bars::toolbar::is_flat(app)));
}

#[gpui::test]
fn a_file_with_flat_buttons_off_starts_with_them_raised(cx: &mut TestAppContext) {
    let (_ini, _handle) = setup(cx, "[Toolbar]\r\nbToolBarIsFlat=0\r\n");
    assert!(!cx.update(|app| crate::bars::toolbar::is_flat(app)));
}

// ---- 検索 (Grep) ----

#[gpui::test]
fn the_grep_options_are_live_and_kept(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    let defaults = cx.update(|app| crate::search::prefs::prefs(app));
    assert!(!defaults.grep_real_time);
    assert!(defaults.grep_enter_jumps && defaults.grep_double_click_jumps);
    run(cx, handle, "F_OPTION_GREP");
    check(cx, "grep_real_time", true);
    check(cx, "gtjw_return", false);
    check(cx, "gtjw_ldblclk", false);
    ok(cx);
    assert_eq!(ini.get("bGrepRealTime").as_deref(), Some("1"));
    assert_eq!(ini.get("bGTJW_RETURN").as_deref(), Some("0"));
    assert_eq!(ini.get("bGTJW_LDBLCLK").as_deref(), Some("0"));
    let now = cx.update(|app| crate::search::prefs::prefs(app));
    assert!(now.grep_real_time);
    assert!(!now.grep_enter_jumps && !now.grep_double_click_jumps);
}

#[gpui::test]
fn the_two_options_that_stay_gray_are_never_written(cx: &mut TestAppContext) {
    let (ini, handle) = setup(
        cx,
        "[Common]\r\nm_bInheritKeyOtherView=0\r\nszRegexpLib=x.dll\r\n",
    );
    run(cx, handle, "F_OPTION_GREP");
    check(cx, "inherit_key_other_view", true);
    type_in(cx, "regexp_lib", "other.dll");
    ok(cx);
    assert_eq!(ini.get("m_bInheritKeyOtherView").as_deref(), Some("0"));
    assert_eq!(ini.get("szRegexpLib").as_deref(), Some("x.dll"));
}

// ---- マクロ ----

#[gpui::test]
fn the_wait_before_the_cancel_question_is_live_and_kept(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    assert_eq!(cx.update(|app| crate::macros::cancel_wait(app)), 10);
    run(cx, handle, "F_OPTION_MACRO");
    assert_eq!(
        with_dialog(cx, |d, _, _| d.values().int("macro_cancel_timer")),
        10
    );
    type_in(cx, "macro_cancel_timer", "3");
    ok(cx);
    assert_eq!(
        ini.profile().get("Macro", "nMacroCancelTimer"),
        Some("3"),
        "the key is in [Macro]"
    );
    assert_eq!(cx.update(|app| crate::macros::cancel_wait(app)), 3);
}
