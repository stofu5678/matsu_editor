//! Alt+letter in the DIFF dialog (`IsDialogMessage`, `IDD_DIFF`): the letters of the labels act while the
//! focus is in the box of the file, where the letter is not typed.

use gpui::TestAppContext;
use matsu_core::diff::DiffMark;

use super::{Target, remember_options, temp_dir, window};

#[gpui::test]
fn the_letters_of_the_check_boxes_and_the_choice_change_the_options_and_type_nothing(
    cx: &mut TestAppContext,
) {
    remember_options(cx, 0, false);
    let w = window(cx, "a\nb\n", None);
    assert!(w.run(cx, "F_DIFF_DIALOG"));
    let panel = w.view.read_with(cx, |v, _| v.diff.panel.clone()).unwrap();
    let flags = |cx: &mut TestAppContext| panel.read_with(cx, |p, _| p.flags());
    let search = |cx: &mut TestAppContext| panel.read_with(cx, |p, _| p.search_flags());
    assert_eq!(flags(cx), 0);
    // the two search options start as the program has them
    let (notify, wrap) = search(cx);
    // 大文字小文字 / 空白 / 空白変更 / 空行 / タブ
    for (letter, bit) in [("c", 1), ("w", 2), ("b", 4), ("l", 8), ("t", 0x10)] {
        w.cx(cx).simulate_keystrokes(&format!("alt-{letter}"));
        assert_eq!(flags(cx) & bit, bit, "Alt+{letter}");
    }
    assert_eq!(flags(cx), 0x1f);
    // the same letter again switches it off, whatever the case and Shift
    w.cx(cx).simulate_keystrokes("alt-shift-c");
    assert_eq!(flags(cx), 0x1e);
    // 旧ファイル(D) and 新ファイル(N)
    w.cx(cx).simulate_keystrokes("alt-d");
    assert_eq!(flags(cx) & 0x20, 0x20);
    w.cx(cx).simulate_keystrokes("alt-n");
    assert_eq!(flags(cx) & 0x20, 0);
    // 見つからないときにメッセージを表示(M) / 先頭（末尾）から再検索する(Z)
    w.cx(cx).simulate_keystrokes("alt-m");
    assert_eq!(search(cx), (!notify, wrap));
    w.cx(cx).simulate_keystrokes("alt-z");
    assert_eq!(search(cx), (!notify, !wrap));
    // a letter that no label has is not taken, and no letter reached the box of the file
    w.cx(cx).simulate_keystrokes("alt-q");
    assert!(w.view.read_with(cx, |v, _| v.diff_panel_open()));
    assert_eq!(panel.read_with(cx, |p, cx| p.target(cx)), None);
}

#[gpui::test]
fn alt_r_asks_for_the_file_and_alt_x_cancels(cx: &mut TestAppContext) {
    remember_options(cx, 0, false);
    let w = window(cx, "a\nb\n", None);
    assert!(w.run(cx, "F_DIFF_DIALOG"));
    crate::file_dialog::tests::asked();
    w.cx(cx).simulate_keystrokes("alt-r");
    cx.run_until_parked();
    assert_eq!(crate::file_dialog::tests::asked().len(), 1);
    assert!(w.view.read_with(cx, |v, _| v.diff_panel_open()));
    w.cx(cx).simulate_keystrokes("alt-x");
    assert!(!w.view.read_with(cx, |v, _| v.diff_panel_open()));
    assert!(w.marks(cx).is_empty());
}

#[gpui::test]
fn alt_o_is_ok(cx: &mut TestAppContext) {
    remember_options(cx, 0, false);
    let dir = temp_dir("access-key-ok");
    let path = dir.join("other.txt");
    std::fs::write(&path, "a\nB\n").unwrap();
    let w = window(cx, "a\nb\n", None);
    assert!(w.run(cx, "F_DIFF_DIALOG"));
    let panel = w.view.read_with(cx, |v, _| v.diff.panel.clone()).unwrap();
    w.handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| p.set_file(&path.to_string_lossy(), window, cx))
        })
        .unwrap();
    assert_eq!(
        panel.read_with(cx, |p, cx| p.target(cx)),
        Some(Target::File(path.clone()))
    );
    w.cx(cx).simulate_keystrokes("alt-o");
    cx.run_until_parked();
    assert!(!w.view.read_with(cx, |v, _| v.diff_panel_open()));
    assert_eq!(w.marks(cx), [(1, DiffMark::Change)]);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_compare_dialog_has_t_o_and_x(cx: &mut TestAppContext) {
    remember_options(cx, 0, false);
    let w = window(cx, "abc\n日本語\n", None);
    let other = window(cx, "abc\n日本人\n", None);
    assert!(w.run(cx, "F_COMPARE"));
    let panel = w.view.read_with(cx, |v, _| v.diff.compare.clone()).unwrap();
    // 左右に並べて表示(T)
    assert!(!panel.read_with(cx, |p, _| p.tile()));
    w.cx(cx).simulate_keystrokes("alt-t");
    assert!(panel.read_with(cx, |p, _| p.tile()));
    w.cx(cx).simulate_keystrokes("alt-shift-t");
    assert!(!panel.read_with(cx, |p, _| p.tile()));
    // キャンセル(X)
    w.cx(cx).simulate_keystrokes("alt-x");
    assert!(!w.view.read_with(cx, |v, _| v.compare_panel_open()));
    // OK (`&OK`)
    assert!(w.run(cx, "F_COMPARE"));
    let panel = w.view.read_with(cx, |v, _| v.diff.compare.clone()).unwrap();
    let target = other.doc(cx);
    panel.update(cx, |p, cx| p.select_window(target, cx));
    w.cx(cx).simulate_keystrokes("alt-o");
    cx.run_until_parked();
    assert!(!w.view.read_with(cx, |v, _| v.compare_panel_open()));
    assert_eq!(
        other.take_box(cx).as_deref(),
        Some("異なる箇所が見つかりました。")
    );
}
