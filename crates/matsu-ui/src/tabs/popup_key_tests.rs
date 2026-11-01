//! The letters in the popup menus that MATSU opens (`menu_bar::popup_keys`), in a whole window: the
//! keystroke interceptor of the window runs them before the key bindings.
//!
//! Sakura: `CMenuDrawer::OnMenuChar` (`sakura_core/uiparts/CMenuDrawer.cpp`) for the letters of a Win32
//! menu: one item with the key runs, several are selected one after the other.

use gpui::{
    AnyWindowHandle, KeyDownEvent, Keystroke, Modifiers, MouseButton, TestAppContext,
    VisualTestContext, point, px,
};
use matsu_config::Profile;
use matsu_core::buffer::Pos;

use super::tests::{active_view, code_of, run, type_text, visual, window, workspace};
use crate::settings::UserSettings;

/// Custom menu 1: すべて選択 (A), a separator, 全体の最後 (A, the same key), 行頭 (B), and a gray
/// item with the key C (Redo with nothing to redo).
fn menu_ini() -> String {
    let select_all = code_of("F_SELECTALL");
    let file_end = code_of("F_GOFILEEND");
    let line_top = code_of("F_GOLINETOP");
    let redo = code_of("F_REDO");
    format!(
        "[CustMenu]\nnCMIN[01]=5\n\
         nCMIF[01][00]={select_all}\nnCMIK[01][00]=A\n\
         nCMIF[01][01]=0\n\
         nCMIF[01][02]={file_end}\nnCMIK[01][02]=A\n\
         nCMIF[01][03]={line_top}\nnCMIK[01][03]=B\n\
         nCMIF[01][04]={redo}\nnCMIK[01][04]=C\n"
    )
}

/// A window with the text `abc` (the caret at its end after typing, at the top after `top`).
fn opened(cx: &mut TestAppContext) -> AnyWindowHandle {
    let user = UserSettings::from_profile(Profile::from_bytes(menu_ini().as_bytes()), None);
    cx.update(|app| app.set_global(user));
    let handle = window(cx, false);
    type_text(cx, handle, "abc");
    run(cx, handle, "F_GOFILETOP");
    handle
}

fn open_menu(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    run(cx, handle, "F_CUSTMENU_1");
    assert!(is_open(cx, handle), "the menu is open");
}

fn is_open(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    active_view(cx, handle).read_with(cx, |v, _| v.custom_menu.is_open())
}

fn caret(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Pos {
    active_view(cx, handle).read_with(cx, |v, _| v.cursor.caret())
}

fn selected(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    active_view(cx, handle).read_with(cx, |v, _| v.cursor.selection().is_some())
}

fn keys(cx: &mut TestAppContext, handle: AnyWindowHandle, keys: &str) {
    let vcx = visual(cx, handle);
    vcx.simulate_keystrokes(keys);
    vcx.run_until_parked();
}

#[gpui::test]
fn a_letter_runs_the_item_with_that_access_key(cx: &mut TestAppContext) {
    let handle = opened(cx);
    open_menu(cx, handle);
    // 全体の最後 and すべて選択 share A; B is 行頭 only and runs at once.
    keys(cx, handle, "b");
    assert!(!is_open(cx, handle), "the menu closes");
    assert_eq!(caret(cx, handle), Pos::new(0, 0));
    run(cx, handle, "F_GOFILEEND");
    open_menu(cx, handle);
    // Upper case works too.
    keys(cx, handle, "shift-b");
    assert!(!is_open(cx, handle));
    assert_eq!(caret(cx, handle), Pos::new(0, 0));
    // The letter did not go into the text.
    let text = workspace(cx, handle).read_with(cx, |ws, cx| {
        ws.editor()
            .read(cx)
            .document()
            .read(cx)
            .doc()
            .line(0)
            .unwrap()
            .unwrap()
            .text
    });
    assert_eq!(text, "abc");
}

#[gpui::test]
fn the_same_letter_goes_through_the_items_that_share_it(cx: &mut TestAppContext) {
    let handle = opened(cx);
    open_menu(cx, handle);
    // The first A only selects: nothing runs and the menu stays.
    keys(cx, handle, "a");
    assert!(is_open(cx, handle));
    assert!(!selected(cx, handle));
    keys(cx, handle, "enter");
    assert!(!is_open(cx, handle));
    assert!(selected(cx, handle), "the first item with A is すべて選択");

    // A, A: the second item.
    run(cx, handle, "F_GOFILETOP");
    open_menu(cx, handle);
    keys(cx, handle, "a a enter");
    assert_eq!(caret(cx, handle), Pos::new(0, 3), "全体の最後");
    assert!(!selected(cx, handle));

    // A, A, A: round the end to the first one again.
    run(cx, handle, "F_GOFILETOP");
    open_menu(cx, handle);
    keys(cx, handle, "a a a enter");
    assert!(selected(cx, handle));
}

#[gpui::test]
fn the_letters_go_on_from_where_the_arrow_keys_left_the_selection(cx: &mut TestAppContext) {
    let handle = opened(cx);
    open_menu(cx, handle);
    // Down, Down, Down: the third row (全体の最後 is the second item, 行頭 the third).
    keys(cx, handle, "down down down");
    // The next A after 行頭 goes round to the first one.
    keys(cx, handle, "a enter");
    assert!(selected(cx, handle));
}

#[gpui::test]
fn a_letter_that_no_item_has_does_nothing(cx: &mut TestAppContext) {
    let handle = opened(cx);
    open_menu(cx, handle);
    keys(cx, handle, "z");
    assert!(is_open(cx, handle));
    assert_eq!(caret(cx, handle), Pos::new(0, 0));
    // A gray item is not run by its letter (Windows does not either).
    keys(cx, handle, "c");
    assert!(is_open(cx, handle));
}

#[gpui::test]
fn an_ime_in_progress_does_not_run_an_item(cx: &mut TestAppContext) {
    let handle = opened(cx);
    open_menu(cx, handle);
    // `key_char` is None for a key that the IME is still composing.
    let vcx: &mut VisualTestContext = visual(cx, handle);
    vcx.simulate_event(KeyDownEvent {
        keystroke: Keystroke {
            modifiers: Modifiers::default(),
            key: "b".into(),
            key_char: None,
        },
        is_held: false,
        prefer_character_input: false,
    });
    vcx.run_until_parked();
    assert!(is_open(cx, handle), "the letter was left to the IME");
    assert_eq!(caret(cx, handle), Pos::new(0, 0));
}

#[gpui::test]
fn after_the_mouse_pointed_at_an_item_the_letters_start_again(cx: &mut TestAppContext) {
    let handle = opened(cx);
    open_menu(cx, handle);
    keys(cx, handle, "down down");
    // The mouse may select what it points at, which the keys cannot see: A selects the first item
    // with it, whatever was selected.
    visual(cx, handle).simulate_mouse_move(point(px(300.), px(300.)), None, Modifiers::default());
    keys(cx, handle, "a enter");
    assert!(!is_open(cx, handle));
    assert!(selected(cx, handle), "the first A is すべて選択");
}

#[gpui::test]
fn the_letters_work_in_the_right_click_menu(cx: &mut TestAppContext) {
    // The default right-click menu: 元に戻す(U) is the only U.
    let user = UserSettings::from_profile(Profile::from_bytes(b""), None);
    cx.update(|app| app.set_global(user));
    let handle = window(cx, false);
    type_text(cx, handle, "abc");
    let vcx = visual(cx, handle);
    let at = point(px(200.), px(200.));
    vcx.simulate_mouse_down(at, MouseButton::Right, Modifiers::default());
    vcx.simulate_mouse_up(at, MouseButton::Right, Modifiers::default());
    vcx.run_until_parked();
    assert!(is_open(cx, handle));
    keys(cx, handle, "u");
    assert!(!is_open(cx, handle));
    let text = workspace(cx, handle).read_with(cx, |ws, cx| {
        ws.editor()
            .read(cx)
            .document()
            .read(cx)
            .doc()
            .line(0)
            .unwrap()
            .unwrap()
            .text
    });
    // Typing is undone a character at a time.
    assert_eq!(text, "ab", "Undo ran");
}
