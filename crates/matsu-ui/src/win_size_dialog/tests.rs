//! Tests of the dialog 位置と大きさの設定 by itself, in GPUI's headless test windows. How the page
//! of the common settings uses it is tested in `common_settings/tests_small.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::TestAppContext;

use super::{
    Field, MODE_DEFAULT, MODE_SAVE, MODE_SET, SIZE_MAXIMIZED, SIZE_MINIMIZED, SIZE_RESTORED,
    WinSize, current, open,
};

const START: WinSize = WinSize {
    size_mode: MODE_SAVE,
    size_type: SIZE_RESTORED,
    width: 640,
    height: 480,
    pos_mode: MODE_DEFAULT,
    x: 10,
    y: 20,
};

type Got = Rc<RefCell<Vec<WinSize>>>;

fn open_dialog(cx: &mut TestAppContext, start: WinSize) -> Got {
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

fn with_dialog<R>(
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

fn keys(cx: &mut TestAppContext, keys: &str) {
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
