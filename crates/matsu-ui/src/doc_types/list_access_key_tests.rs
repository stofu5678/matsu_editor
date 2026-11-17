//! Alt+letter in the type list (`IsDialogMessage`, `IDD_TYPELIST`): the letters of the buttons press them.

use gpui::{TestAppContext, VisualTestContext};

use super::list_tests::{labels, open_list, select, selected, setup, types};
use super::tests::view_of;

/// `n` types, the ones after the 17 built-in ones the user's own.
fn own_types(n: usize) -> String {
    let mut text = format!("[Other]\r\nnTypesCount={n}\r\n");
    for i in 17..n {
        text.push_str(&format!(
            "[Types({i})]\r\nszTypeName=自分{i}\r\nszTypeExts=x{i}\r\n"
        ));
    }
    text
}

#[gpui::test]
fn the_letters_of_the_buttons_that_change_the_list_press_them(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, &own_types(20));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    // 追加(A)
    cx.simulate_keystrokes("alt-a");
    cx.run_until_parked();
    assert_eq!(labels(&list, cx).len(), 21);
    assert_eq!(selected(&list, cx), 20);
    // 複製(C)
    select(&list, cx, 18);
    cx.simulate_keystrokes("alt-c");
    cx.run_until_parked();
    assert_eq!(labels(&list, cx).len(), 22);
    // ↑(U) and ↓(O)
    select(&list, cx, 18);
    cx.simulate_keystrokes("alt-u");
    cx.run_until_parked();
    assert_eq!(selected(&list, cx), 17);
    cx.simulate_keystrokes("alt-o");
    cx.run_until_parked();
    assert_eq!(selected(&list, cx), 18);
    assert_eq!(types(&ini).get(18).unwrap().type_name, "自分18");
    // 削除(D) asks first
    cx.simulate_keystrokes("alt-shift-d");
    cx.run_until_parked();
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("はい(Y)");
    cx.run_until_parked();
    assert_eq!(labels(&list, cx).len(), 21);
}

#[gpui::test]
fn a_button_that_is_grey_for_the_selected_type_does_not_react(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &own_types(20));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    // 基本 cannot move or go
    select(&list, cx, 0);
    for letter in ["u", "o", "d"] {
        cx.simulate_keystrokes(&format!("alt-{letter}"));
    }
    cx.run_until_parked();
    assert!(!cx.has_pending_prompt());
    assert_eq!(labels(&list, cx).len(), 20);
    assert_eq!(selected(&list, cx), 0);
}

#[gpui::test]
fn the_letters_of_the_other_buttons_apply_open_the_settings_and_cancel(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &own_types(20));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    // キャンセル(X)
    let list = open_list(&view, cx);
    cx.simulate_keystrokes("alt-x");
    assert!(view.read_with(cx, |v, _| v.doc_type.list.is_none()));
    drop(list);
    // 一時適用(R): the selected type is applied to the document
    let list = open_list(&view, cx);
    select(&list, cx, 1);
    cx.simulate_keystrokes("alt-r");
    cx.run_until_parked();
    assert!(view.read_with(cx, |v, _| v.doc_type.list.is_none()));
    // 設定変更(S)...
    open_list(&view, cx);
    cx.simulate_keystrokes("alt-s");
    cx.run_until_parked();
    assert!(view.read_with(cx, |v, _| v.doc_type.list.is_none()));
}
