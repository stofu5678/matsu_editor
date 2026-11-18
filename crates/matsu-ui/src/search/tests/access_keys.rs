//! Alt+letter in the find, replace and go-to-line panels (`IsDialogMessage`, `IDD_FIND`, `IDD_REPLACE`, `IDD_JUMP`):
//! the letters of the labels act while the focus is in a box, where the letter is not typed.

use gpui::{TestAppContext, VisualTestContext};
use matsu_core::buffer::Pos;
use matsu_core::search::ReplaceTarget;

use super::{Fixture, PanelMode, setup};

impl Fixture {
    fn options(&self, cx: &mut VisualTestContext) -> [bool; 6] {
        self.view.read_with(cx, |v, _| {
            let o = v.search.options();
            [
                o.whole_word,
                o.match_case,
                o.regex,
                o.notify_not_found,
                o.wrap,
                o.auto_close,
            ]
        })
    }

    fn pattern_now(&self, cx: &mut VisualTestContext) -> String {
        self.view
            .read_with(cx, |v, _| v.search.pattern().to_owned())
    }

    fn marked(&self, cx: &mut VisualTestContext) -> Vec<usize> {
        self.document
            .read_with(cx, |d, _| d.model().bookmarks().iter().collect())
    }
}

#[gpui::test]
fn the_letters_of_the_check_boxes_switch_them_from_the_box_and_type_nothing(
    cx: &mut TestAppContext,
) {
    let (f, cx) = setup(cx, "abc");
    cx.simulate_keystrokes("ctrl-f");
    assert_eq!(f.pattern_now(cx), "abc");
    // 単語単位 / 大小 / 正規表現 / 見つからないとき / 先頭から / 自動で閉じる: the start
    assert_eq!(f.options(cx), [false, false, false, true, false, true]);
    for letter in ["w", "c", "e"] {
        cx.simulate_keystrokes(&format!("alt-{letter}"));
    }
    // 正規表現 greys 単語単位 out, but the box was switched before it
    assert_eq!(f.options(cx), [true, true, true, true, false, true]);
    // a switched-off box that is greyed out does not react
    cx.simulate_keystrokes("alt-w");
    assert_eq!(f.options(cx), [true, true, true, true, false, true]);
    for letter in ["m", "z", "l"] {
        cx.simulate_keystrokes(&format!("alt-{letter}"));
    }
    assert_eq!(f.options(cx), [true, true, true, false, true, false]);
    // case does not matter, and nothing went into the box
    cx.simulate_keystrokes("alt-shift-c");
    assert!(!f.options(cx)[1]);
    assert_eq!(f.pattern_now(cx), "abc");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Find));
}

#[gpui::test]
fn alt_d_and_alt_u_search_and_alt_b_marks_the_lines_and_alt_x_closes(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "bar\nfoo\nbar");
    cx.simulate_keystrokes("ctrl-f");
    cx.simulate_keystrokes("secondary-a");
    cx.simulate_input("foo");
    // the dialog stays open after a search
    cx.simulate_keystrokes("alt-l");
    cx.simulate_keystrokes("alt-d");
    assert_eq!(f.selection(cx), Some((Pos::new(1, 0), Pos::new(1, 3))));
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Find));
    cx.simulate_keystrokes("end");
    // 上検索 from the selection finds nothing before it; the dialog is still the one that is open
    cx.simulate_keystrokes("alt-u");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Find));
    assert_eq!(f.pattern_now(cx), "foo");
    // 該当行マーク
    cx.simulate_keystrokes("alt-b");
    assert_eq!(f.marked(cx), [1]);
    assert_eq!(f.text(cx), "bar\nfoo\nbar");
    cx.simulate_keystrokes("alt-x");
    assert_eq!(f.panel_mode(cx), None);
}

#[gpui::test]
fn the_letters_of_the_replace_panel_choose_its_radio_buttons_and_check_box(
    cx: &mut TestAppContext,
) {
    let (f, cx) = setup(cx, "aa");
    cx.simulate_keystrokes("ctrl-r");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Replace));
    let replace = |f: &Fixture, cx: &mut VisualTestContext| {
        f.view.read_with(cx, |v, _| {
            (
                v.search.replace.target,
                v.search.replace.in_selection,
                v.search.replace.consecutive_all,
            )
        })
    };
    assert_eq!(replace(&f, cx), (ReplaceTarget::Match, false, false));
    for (letter, target) in [
        ("1", ReplaceTarget::InsertBefore),
        ("2", ReplaceTarget::InsertAfter),
        ("3", ReplaceTarget::DeleteLine),
        ("0", ReplaceTarget::Match),
    ] {
        cx.simulate_keystrokes(&format!("alt-{letter}"));
        assert_eq!(replace(&f, cx).0, target, "alt-{letter}");
    }
    cx.simulate_keystrokes("alt-s");
    assert!(replace(&f, cx).1);
    cx.simulate_keystrokes("alt-o");
    assert!(!replace(&f, cx).1);
    // 「すべて置換」は置換の繰返し is greyed out without 正規表現
    cx.simulate_keystrokes("alt-i");
    assert!(!replace(&f, cx).2);
    cx.simulate_keystrokes("alt-e alt-i");
    assert!(replace(&f, cx).2);
    cx.simulate_keystrokes("alt-i");
    assert!(!replace(&f, cx).2);
    // nothing was typed
    assert_eq!(f.pattern_now(cx), "aa");
}

#[gpui::test]
fn the_letters_of_the_labels_move_the_focus_between_the_boxes_and_the_buttons_replace(
    cx: &mut TestAppContext,
) {
    let (f, cx) = setup(cx, "aa");
    cx.simulate_keystrokes("ctrl-r");
    cx.simulate_keystrokes("secondary-a");
    cx.simulate_input("a");
    // 置換後(P)
    cx.simulate_keystrokes("alt-p");
    cx.simulate_input("bb");
    let replacement = f.view.read_with(cx, |v, _| v.search.replace.text.clone());
    assert_eq!(replacement, "bb");
    assert_eq!(f.pattern_now(cx), "a");
    // 置換前(N): the box is selected, so the typing replaces it
    cx.simulate_keystrokes("alt-n");
    cx.simulate_input("aa");
    assert_eq!(f.pattern_now(cx), "aa");
    let replacement = f.view.read_with(cx, |v, _| v.search.replace.text.clone());
    assert_eq!(replacement, "bb");
    // 置換(R) (the second press only finds nothing more, if the first one replaced already)
    cx.simulate_keystrokes("alt-r");
    cx.simulate_keystrokes("alt-r");
    assert_eq!(f.text(cx), "bb");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Replace));
}

#[gpui::test]
fn alt_a_replaces_all_from_the_box(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "aa a");
    cx.simulate_keystrokes("ctrl-r");
    cx.simulate_keystrokes("secondary-a");
    cx.simulate_input("a");
    cx.simulate_keystrokes("alt-p secondary-a");
    cx.simulate_input("b");
    cx.simulate_keystrokes("alt-a");
    assert_eq!(f.text(cx), "bb b");
    let (_, detail) = cx.pending_prompt().expect("the count box");
    assert_eq!(detail, "3箇所を置換しました。");
}

#[gpui::test]
fn the_jump_panel_has_the_letters_of_its_label_and_close(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb\nc\nd");
    cx.simulate_keystrokes("ctrl-j");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Jump));
    cx.simulate_input("9");
    // 行番号(N) takes the focus to the box and selects its text, and types nothing
    cx.simulate_keystrokes("alt-n");
    cx.simulate_input("3");
    cx.simulate_keystrokes("enter");
    assert_eq!(f.caret(cx), Pos::new(2, 0));
    assert_eq!(f.panel_mode(cx), None);
    cx.simulate_keystrokes("ctrl-j");
    cx.simulate_keystrokes("alt-x");
    assert_eq!(f.panel_mode(cx), None);
}

#[gpui::test]
fn the_help_is_alt_h_f1_and_a_click_and_each_panel_opens_its_own_page(cx: &mut TestAppContext) {
    use crate::support::help::testing::{clear, page_of_function, take};
    let (f, cx) = setup(cx, "abc");
    // `CDlgFind`: `MyWinHelp( ..., F_SEARCH_DIALOG )`; the panel stays open and its text unchanged
    cx.simulate_keystrokes("ctrl-f");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Find));
    clear();
    cx.simulate_keystrokes("alt-h");
    assert_eq!(take(), [page_of_function("F_SEARCH_DIALOG")]);
    cx.simulate_keystrokes("f1");
    assert_eq!(take(), [page_of_function("F_SEARCH_DIALOG")]);
    assert_eq!(f.pattern_now(cx), "abc");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Find));
    // the button itself
    let help = cx.debug_bounds("search-help").expect("the help button");
    cx.simulate_click(help.center(), gpui::Modifiers::none());
    assert_eq!(take(), [page_of_function("F_SEARCH_DIALOG")]);
    // `CDlgReplace`: `F_REPLACE_DIALOG`
    cx.simulate_keystrokes("alt-x ctrl-r");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Replace));
    cx.simulate_keystrokes("alt-h");
    assert_eq!(take(), [page_of_function("F_REPLACE_DIALOG")]);
    cx.simulate_keystrokes("f1");
    assert_eq!(take(), [page_of_function("F_REPLACE_DIALOG")]);
    // `CDlgJump`: `F_JUMP_DIALOG`
    cx.simulate_keystrokes("alt-x ctrl-j");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Jump));
    cx.simulate_keystrokes("alt-h");
    assert_eq!(take(), [page_of_function("F_JUMP_DIALOG")]);
    cx.simulate_keystrokes("f1");
    assert_eq!(take(), [page_of_function("F_JUMP_DIALOG")]);
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Jump));
}
