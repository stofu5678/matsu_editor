//! Tests of the dialog 位置と大きさの設定 by itself, in GPUI's headless test windows. How the page
//! of the common settings uses it is tested in `common_settings/tests_small.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{AppContext as _, TestAppContext};

use super::{
    Field, MODE_DEFAULT, MODE_SAVE, MODE_SET, SIZE_MAXIMIZED, SIZE_MINIMIZED, SIZE_RESTORED,
    WinSize, current, open,
};

pub(super) const START: WinSize = WinSize {
    size_mode: MODE_SAVE,
    size_type: SIZE_RESTORED,
    width: 640,
    height: 480,
    pos_mode: MODE_DEFAULT,
    x: 10,
    y: 20,
};

type Got = Rc<RefCell<Vec<WinSize>>>;

pub(super) fn open_dialog(cx: &mut TestAppContext, start: WinSize) -> Got {
    cx.update(gpui_component::init);
    let got: Got = Rc::default();
    let sink = got.clone();
    cx.update(|app| {
        open(
            app,
            start,
            Box::new(move |_, now| sink.borrow_mut().push(now)),
        )
    });
    cx.run_until_parked();
    got
}

pub(super) fn with_dialog<R>(
    cx: &mut TestAppContext,
    f: impl FnOnce(
        &mut super::WinSizeDialog,
        &mut gpui::Window,
        &mut gpui::Context<super::WinSizeDialog>,
    ) -> R,
) -> R {
    let (dialog, handle) = cx.update(|app| current(app)).expect("the dialog is open");
    let result = handle
        .update(cx, |_, window, app| {
            dialog.update(app, |dialog, dcx| f(dialog, window, dcx))
        })
        .unwrap();
    cx.run_until_parked();
    result
}

#[gpui::test]
fn the_dialog_starts_with_the_values_it_was_given(cx: &mut TestAppContext) {
    let _got = open_dialog(cx, START);
    assert_eq!(with_dialog(cx, |d, _, dcx| d.values(dcx)), START);
}

#[gpui::test]
fn closing_gives_the_values_once_and_closes_the_window(cx: &mut TestAppContext) {
    let got = open_dialog(cx, START);
    with_dialog(cx, |d, window, dcx| {
        d.set_size_mode(MODE_SET, dcx);
        d.set_size_type(SIZE_MINIMIZED, dcx);
        d.set_text(Field::Width, "1024", window, dcx);
        d.set_pos_mode(MODE_SAVE, dcx);
        d.close(window, dcx);
    });
    assert!(cx.update(|app| current(app)).is_none());
    let got = got.borrow();
    assert_eq!(got.len(), 1);
    assert_eq!(
        got[0],
        WinSize {
            size_mode: MODE_SET,
            size_type: SIZE_MINIMIZED,
            width: 1024,
            pos_mode: MODE_SAVE,
            ..START
        }
    );
}

#[gpui::test]
fn the_kinds_of_window_are_the_numbers_of_wm_size(cx: &mut TestAppContext) {
    // `GetData`: 普通 is `SIZE_RESTORED` (0), 最大化 `SIZE_MAXIMIZED` (2), (最小化)
    // `SIZE_MINIMIZED` (1).
    assert_eq!([SIZE_RESTORED, SIZE_MINIMIZED, SIZE_MAXIMIZED], [0, 1, 2]);
    let got = open_dialog(cx, START);
    with_dialog(cx, |d, window, dcx| {
        d.set_size_type(SIZE_MAXIMIZED, dcx);
        d.close(window, dcx);
    });
    assert_eq!(got.borrow()[0].size_type, 2);
}

#[gpui::test]
fn a_second_open_does_not_make_a_second_window(cx: &mut TestAppContext) {
    let first = open_dialog(cx, START);
    let second: Got = Rc::default();
    let sink = second.clone();
    let opened = cx.update(|app| {
        open(
            app,
            WinSize { width: 1, ..START },
            Box::new(move |_, now| sink.borrow_mut().push(now)),
        )
    });
    assert!(opened.is_none());
    with_dialog(cx, |d, window, dcx| d.close(window, dcx));
    assert_eq!(first.borrow().len(), 1);
    assert!(second.borrow().is_empty(), "the second request is dropped");
}

#[gpui::test]
fn enter_and_escape_close_the_dialog_like_the_button(cx: &mut TestAppContext) {
    // `IDOK` and `IDCANCEL` both run `GetData`.
    for key in ["enter", "escape"] {
        let got = open_dialog(cx, START);
        let (_, handle) = cx.update(|app| current(app)).unwrap();
        let vcx = gpui::VisualTestContext::from_window(handle, cx).into_mut();
        vcx.simulate_keystrokes(key);
        cx.run_until_parked();
        assert!(cx.update(|app| current(app)).is_none(), "{key}");
        assert_eq!(got.borrow().as_slice(), [START], "{key}");
    }
}

pub(super) fn keys(cx: &mut TestAppContext, keys: &str) {
    let (_, handle) = cx.update(|app| current(app)).unwrap();
    let vcx = gpui::VisualTestContext::from_window(handle, cx).into_mut();
    vcx.simulate_keystrokes(keys);
    cx.run_until_parked();
}

#[gpui::test]
fn the_letters_of_the_radio_buttons_choose_them(cx: &mut TestAppContext) {
    // `IDD_WINPOSSIZE`: 起動時の位置 D, I, S and 大きさ E, N, T
    let _got = open_dialog(cx, START);
    keys(cx, "alt-s alt-t");
    let now = with_dialog(cx, |d, _, dcx| d.values(dcx));
    assert_eq!((now.pos_mode, now.size_mode), (MODE_SET, MODE_SET));
    keys(cx, "alt-i alt-shift-e");
    let now = with_dialog(cx, |d, _, dcx| d.values(dcx));
    assert_eq!((now.pos_mode, now.size_mode), (MODE_SAVE, MODE_DEFAULT));
    keys(cx, "alt-d alt-n");
    let now = with_dialog(cx, |d, _, dcx| d.values(dcx));
    assert_eq!((now.pos_mode, now.size_mode), (MODE_DEFAULT, MODE_SAVE));
}

#[gpui::test]
fn the_label_of_a_box_moves_the_focus_to_it_only_while_the_box_can_be_used(
    cx: &mut TestAppContext,
) {
    let _got = open_dialog(cx, START);
    // 直接指定 is not chosen: X, Y, W and G find no box that can be used, and the letter is not typed
    keys(cx, "alt-x alt-y alt-w alt-g");
    assert_eq!(with_dialog(cx, |d, _, dcx| d.values(dcx)), START);
    // with 直接指定, Alt+X focuses the box of X and the next keys type into it
    keys(cx, "alt-s alt-x");
    let (_, handle) = cx.update(|app| current(app)).unwrap();
    let vcx = gpui::VisualTestContext::from_window(handle, cx).into_mut();
    vcx.simulate_input("7");
    cx.run_until_parked();
    assert_eq!(with_dialog(cx, |d, _, dcx| d.values(dcx)).x, 7);
    // the size boxes are the same after 直接指定 of 大きさ
    keys(cx, "alt-t alt-w");
    let vcx = gpui::VisualTestContext::from_window(handle, cx).into_mut();
    vcx.simulate_input("9");
    cx.run_until_parked();
    assert_eq!(with_dialog(cx, |d, _, dcx| d.values(dcx)).width, 9);
}

#[gpui::test]
fn alt_c_closes_the_dialog_like_the_button(cx: &mut TestAppContext) {
    let got = open_dialog(cx, START);
    keys(cx, "alt-c");
    assert!(cx.update(|app| current(app)).is_none());
    assert_eq!(got.borrow().as_slice(), [START]);
}

#[test]
fn the_radio_buttons_show_and_speak_their_labels_like_the_other_settings() {
    assert_eq!(
        super::radio_texts("指定しない(&D)"),
        ("指定しない(D)".to_owned(), "指定しない".to_owned())
    );
    assert_eq!(
        super::radio_texts("直接指定(&S)"),
        ("直接指定(S)".to_owned(), "直接指定".to_owned())
    );
}

#[gpui::test]
fn the_help_is_alt_h_and_f1_and_opens_the_page_of_the_window_settings(cx: &mut TestAppContext) {
    use crate::support::help::testing::{clear, take, web_page};
    // `CDlgWinSize`: `MyWinHelp( .., HLP000286 )`, which is a topic and not a function; the
    // dialog stays and keeps what it holds
    let got = open_dialog(cx, START);
    clear();
    keys(cx, "alt-h");
    assert_eq!(take(), [web_page("common-settings/page-window.md")]);
    keys(cx, "f1");
    assert_eq!(take(), [web_page("common-settings/page-window.md")]);
    assert!(cx.update(|app| current(app)).is_some());
    assert!(got.borrow().is_empty());
    assert_eq!(with_dialog(cx, |d, _, dcx| d.values(dcx)), START);
}

#[gpui::test]
fn the_spin_buttons_of_the_numbers_have_the_ranges_of_sakura(cx: &mut TestAppContext) {
    // `IDC_SPIN_WX`, `_WY`: -30000 to 30000 (a window may stand off the screen); `IDC_SPIN_SX`,
    // `_SY`: 0 to 30000 (`CDlgWinSize::SetData`)
    let _got = open_dialog(
        cx,
        WinSize {
            pos_mode: MODE_SET,
            size_mode: MODE_SET,
            x: -29999,
            width: 1,
            ..START
        },
    );
    let (_, handle) = cx.update(|app| current(app)).expect("the dialog is open");
    let cx = gpui::VisualTestContext::from_window(handle, cx).into_mut();
    cx.run_until_parked();
    let click = |cx: &mut gpui::VisualTestContext, selector: &'static str| {
        let bounds = cx
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("no {selector}"));
        cx.simulate_click(bounds.center(), gpui::Modifiers::none());
        cx.run_until_parked();
    };
    let values = |cx: &mut gpui::VisualTestContext| {
        let (dialog, _) = cx.update(|_, app| current(app)).expect("open");
        cx.read_entity(&dialog, |d, app| d.values(app))
    };
    // X: down below zero and to the end
    click(cx, "win-size-spin-X座標(&X)-down");
    click(cx, "win-size-spin-X座標(&X)-down");
    assert_eq!(values(cx).x, -30000);
    // 幅: up, and down to 0 and no further
    click(cx, "win-size-spin-幅(&W)-up");
    assert_eq!(values(cx).width, 2);
    click(cx, "win-size-spin-幅(&W)-down");
    click(cx, "win-size-spin-幅(&W)-down");
    click(cx, "win-size-spin-幅(&W)-down");
    assert_eq!(values(cx).width, 0);
}

pub(super) fn visual(cx: &mut TestAppContext) -> &mut gpui::VisualTestContext {
    let (_, handle) = cx.update(|app| current(app)).unwrap();
    gpui::VisualTestContext::from_window(handle, cx).into_mut()
}

/// Where the focus is: the position of the radio button in its group (`IDC_RADIO_WINPOS_*`, then
/// `IDC_RADIO_WINSIZE_*`), if it is on one.
fn radio_focus(cx: &mut TestAppContext) -> (Option<usize>, Option<usize>) {
    with_dialog(cx, |d, window, _| {
        (d.pos_ring.focused(window), d.size_ring.focused(window))
    })
}

#[gpui::test]
fn each_group_of_radio_buttons_is_one_tab_stop_at_the_checked_one(cx: &mut TestAppContext) {
    use crate::radio_group::testing::{assert_stops_at_checked, trail};
    // 起動時の位置 指定しない (0) and 大きさ 継承する (1): `WS_GROUP | WS_TABSTOP` on the first of
    // each group, and Windows' dialog manager tabs to the checked button (`IDD_WINPOSSIZE`)
    let _got = open_dialog(cx, START);
    let (pos, size) = with_dialog(cx, |d, _, _| (d.pos_ring.clone(), d.size_ring.clone()));
    let forward = trail(visual(cx), &[&pos, &size], "tab", 24);
    assert_stops_at_checked(&forward, &[0, 1]);
    let back = trail(visual(cx), &[&pos, &size], "shift-tab", 24);
    assert_stops_at_checked(&back, &[0, 1]);
}

#[gpui::test]
fn the_arrow_keys_move_among_the_radio_buttons_of_a_group_and_choose(cx: &mut TestAppContext) {
    let _got = open_dialog(cx, START);
    let (pos, size) = with_dialog(cx, |d, _, _| (d.pos_ring.clone(), d.size_ring.clone()));
    // Tab until the focus is on the checked button of 起動時の位置
    for _ in 0..24 {
        if radio_focus(cx).0.is_some() {
            break;
        }
        keys(cx, "tab");
    }
    assert_eq!(radio_focus(cx), (Some(0), None));
    for (key, mode, focus) in [
        ("down", MODE_SAVE, 1),
        ("right", MODE_SET, 2),
        // round the end to the first one
        ("down", MODE_DEFAULT, 0),
        // and from the first one to the last
        ("up", MODE_SET, 2),
        ("left", MODE_SAVE, 1),
    ] {
        keys(cx, key);
        let now = with_dialog(cx, |d, _, dcx| d.values(dcx));
        assert_eq!(now.pos_mode, mode, "{key}");
        assert_eq!(radio_focus(cx), (Some(focus), None), "{key}");
        assert_eq!(
            now.size_mode, START.size_mode,
            "{key}: the other group stays"
        );
    }
    // Tab leaves the group at once, and Shift+Tab comes back to the one that is checked now
    keys(cx, "tab");
    assert_eq!(radio_focus(cx).0, None);
    let back = crate::radio_group::testing::trail(visual(cx), &[&pos, &size], "shift-tab", 1);
    assert_eq!(back, [Some((0, 1))]);
}
