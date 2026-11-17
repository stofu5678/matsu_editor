//! The keys of the print setting dialog: the drop-down lists (the arrows, Home, End, PageUp, PageDown,
//! Enter and Escape, as the list of a combo box has them) and Alt+letter (`IsDialogMessage`,
//! `IDD_PRINTSETTING`: the letters of the labels act while the focus is in a box, where the letter is
//! not typed).

use gpui::TestAppContext;
use matsu_print::{Orientation, PAPER_INFO};

use super::{Number, dialog_of, open, press_ok, saved, setup, shown, type_number};
use crate::print_setting::dialog::List;

#[gpui::test]
fn the_keys_move_in_the_list_of_the_settings_and_enter_chooses(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    // ページ設定(1) opens the list, with the setting that is shown under the keys
    cx.simulate_keystrokes("alt-1");
    assert_eq!(
        dialog.read_with(cx, |d, _| d.open_list_and_cursor()),
        Some((List::Setting, 0))
    );
    cx.simulate_keystrokes("down down");
    assert_eq!(
        dialog.read_with(cx, |d, _| d.open_list_and_cursor()),
        Some((List::Setting, 2))
    );
    // the setting is not chosen before Enter, and Enter does not press OK
    assert_eq!(dialog.read_with(cx, |d, _| d.current()), 0);
    cx.simulate_keystrokes("enter");
    assert_eq!(dialog.read_with(cx, |d, _| d.current()), 2);
    assert_eq!(dialog.read_with(cx, |d, _| d.open_list_and_cursor()), None);
    assert!(dialog_of(&view, cx).is_some(), "the dialog is still open");
    // Home, End, Up and the pages stay inside the list
    cx.simulate_keystrokes("alt-1");
    assert_eq!(
        dialog.read_with(cx, |d, _| d.open_list_and_cursor()),
        Some((List::Setting, 2))
    );
    cx.simulate_keystrokes("end");
    assert_eq!(
        dialog.read_with(cx, |d, _| d.open_list_and_cursor()),
        Some((List::Setting, 7))
    );
    cx.simulate_keystrokes("down");
    assert_eq!(
        dialog.read_with(cx, |d, _| d.open_list_and_cursor()),
        Some((List::Setting, 7)),
        "not past the last"
    );
    cx.simulate_keystrokes("pageup");
    assert_eq!(
        dialog.read_with(cx, |d, _| d.open_list_and_cursor()),
        Some((List::Setting, 0))
    );
    cx.simulate_keystrokes("up");
    assert_eq!(
        dialog.read_with(cx, |d, _| d.open_list_and_cursor()),
        Some((List::Setting, 0)),
        "not before the first"
    );
    cx.simulate_keystrokes("pagedown home");
    assert_eq!(
        dialog.read_with(cx, |d, _| d.open_list_and_cursor()),
        Some((List::Setting, 0))
    );
    // Escape closes the list and keeps the setting, and only then the dialog
    cx.simulate_keystrokes("down escape");
    assert_eq!(dialog.read_with(cx, |d, _| d.open_list_and_cursor()), None);
    assert_eq!(dialog.read_with(cx, |d, _| d.current()), 2);
    assert!(dialog_of(&view, cx).is_some());
    cx.simulate_keystrokes("escape");
    assert!(dialog_of(&view, cx).is_none());
}

#[gpui::test]
fn the_keys_choose_the_paper(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    let at = dialog.read_with(cx, |d, _| {
        PAPER_INFO
            .iter()
            .position(|p| p.id == d.shown().print_paper_size)
            .unwrap()
    });
    // 用紙サイズ(Z): the list opens on the paper of the setting
    cx.simulate_keystrokes("alt-z");
    assert_eq!(
        dialog.read_with(cx, |d, _| d.open_list_and_cursor()),
        Some((List::Paper, at))
    );
    cx.simulate_keystrokes("down enter");
    assert_eq!(
        dialog.read_with(cx, |d, _| d.shown().print_paper_size),
        PAPER_INFO[at + 1].id
    );
    assert_eq!(dialog.read_with(cx, |d, _| d.open_list_and_cursor()), None);
}

#[gpui::test]
fn the_fonts_have_their_lists_with_the_keys_too(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    // 半角フォント(F) and 全角フォント(K)
    cx.simulate_keystrokes("alt-f");
    assert!(matches!(
        dialog.read_with(cx, |d, _| d.open_list_and_cursor()),
        Some((List::FontHan, _))
    ));
    // another list replaces the open one
    cx.simulate_keystrokes("alt-k");
    assert!(matches!(
        dialog.read_with(cx, |d, _| d.open_list_and_cursor()),
        Some((List::FontZen, _))
    ));
    cx.simulate_keystrokes("end enter");
    let face = dialog.read_with(cx, |d, _| d.shown().print_font_face_zen.clone());
    assert!(!face.is_empty());
    assert_eq!(dialog.read_with(cx, |d, _| d.open_list_and_cursor()), None);
}

#[gpui::test]
fn the_letters_of_the_check_boxes_and_the_orientation_change_them_and_type_nothing(
    cx: &mut TestAppContext,
) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    let before = shown(&dialog, Number::FontHeight, cx);
    // 行番号を印刷(N), 英文ワードラップ(W), カラー印刷(V)
    let flags = |cx: &mut gpui::VisualTestContext| {
        dialog.read_with(cx, |d, _| {
            let s = d.shown();
            (s.print_line_number, s.print_word_wrap, s.color_print)
        })
    };
    let start = flags(cx);
    cx.simulate_keystrokes("alt-n alt-w alt-v");
    assert_eq!(flags(cx), (!start.0, !start.1, !start.2));
    cx.simulate_keystrokes("alt-shift-n");
    assert_eq!(flags(cx).0, start.0);
    // 横(A) and 縦(I)
    cx.simulate_keystrokes("alt-a");
    assert_eq!(
        dialog.read_with(cx, |d, _| d.shown().print_paper_orientation),
        Orientation::Landscape
    );
    cx.simulate_keystrokes("alt-i");
    assert_eq!(
        dialog.read_with(cx, |d, _| d.shown().print_paper_orientation),
        Orientation::Portrait
    );
    // no letter went into the box that has the focus
    assert_eq!(shown(&dialog, Number::FontHeight, cx), before);
}

#[gpui::test]
fn the_label_of_a_number_moves_the_focus_to_its_box(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    // 行送り(S) selects the text of its box, so what is typed replaces it
    cx.simulate_keystrokes("alt-s");
    cx.simulate_input("55");
    assert_eq!(shown(&dialog, Number::LineSpacing, cx), "55");
    // 余白／上(T), 下(B), 左(L), 右(R)
    for (letter, number) in [
        ("t", Number::MarginTop),
        ("b", Number::MarginBottom),
        ("l", Number::MarginLeft),
        ("r", Number::MarginRight),
    ] {
        cx.simulate_keystrokes(&format!("alt-{letter}"));
        cx.simulate_input("3");
        assert_eq!(shown(&dialog, number, cx), "3", "Alt+{letter}");
    }
    // ヘッダー(U) and フッター(D): the first part
    cx.simulate_keystrokes("alt-u");
    cx.simulate_input("H");
    cx.simulate_keystrokes("alt-d");
    cx.simulate_input("F");
    press_ok(&dialog, cx);
    let one = saved(cx).get(0).clone();
    assert_eq!(one.header_form[0], "H");
    assert_eq!(one.footer_form[0], "F");
    assert_eq!(one.print_line_spacing, 55);
}

#[gpui::test]
fn alt_2_asks_for_the_name_and_alt_x_and_alt_o_close_the_dialog(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    // 設定名変更(2)...
    cx.simulate_keystrokes("alt-2");
    cx.run_until_parked();
    assert!(view.read_with(cx, |v, _| v.input_dialog.is_some()));
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    // OK (`&OK`)
    type_number(&dialog, Number::Dansuu, "2", cx);
    cx.simulate_keystrokes("alt-o");
    cx.run_until_parked();
    assert!(dialog_of(&view, cx).is_none());
    assert_eq!(saved(cx).get(0).print_dansuu, 2);
    // キャンセル(X)
    let _dialog = open(&view, cx);
    type_number(&_dialog, Number::Dansuu, "3", cx);
    cx.simulate_keystrokes("alt-x");
    assert!(dialog_of(&view, cx).is_none());
    assert_eq!(saved(cx).get(0).print_dansuu, 2);
}
