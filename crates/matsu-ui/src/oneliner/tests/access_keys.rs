//! Alt+letter in the one-liner dialog (`IsDialogMessage`): the letters of 実行(E), 登録...(T), キャンセル(X) and of the
//! places 置き換える(R), 次の行に入れる(N), アウトプットに出す(O), from the box of the line, where the letter is not typed.

use gpui::TestAppContext;
use matsu_config::settings::oneliner::OutputTo;

use super::ai::{open, press};
use super::{UPPER, Win, fill, open_dialog, wait_until};

fn output(w: &Win, cx: &mut TestAppContext) -> OutputTo {
    w.dialog(cx).unwrap().read_with(cx, |d, _| d.output)
}

fn line(w: &Win, cx: &mut TestAppContext) -> String {
    w.dialog(cx)
        .unwrap()
        .read_with(cx, |d, app| d.choice(app).line)
}

#[gpui::test]
fn alt_e_in_the_box_of_the_line_runs_it_and_types_no_letter(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "", "abc\n");
    open_dialog(&w, cx);
    fill(&w, cx, UPPER);
    // the focus is in the box of the line
    press(&w, cx, "alt-e");
    wait_until(cx, "the end of the run", |cx| {
        w.dialog(cx).is_none() && !w.running(cx) && w.text(cx) == "ABC\n"
    });
    // what was run is the line as it was: no `e` went into it
    assert_eq!(
        cx.update(|app| crate::oneliner::table(app)).recent[0],
        UPPER
    );
}

#[gpui::test]
fn alt_with_the_letter_of_a_place_chooses_it_from_the_box_of_the_line(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "", "abc\n");
    open_dialog(&w, cx);
    fill(&w, cx, "sort");
    assert_eq!(output(&w, cx), OutputTo::Replace);
    press(&w, cx, "alt-o");
    assert_eq!(output(&w, cx), OutputTo::OutputWindow);
    press(&w, cx, "alt-n");
    assert_eq!(output(&w, cx), OutputTo::NextLine);
    press(&w, cx, "alt-r");
    assert_eq!(output(&w, cx), OutputTo::Replace);
    // the letters are not in the line, and the dialog is still open
    assert_eq!(line(&w, cx), "sort");
    assert!(w.dialog(cx).is_some());
}

#[gpui::test]
fn the_letter_is_matched_without_case_and_with_shift(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "", "abc\n");
    open_dialog(&w, cx);
    fill(&w, cx, "sort");
    press(&w, cx, "alt-shift-o");
    assert_eq!(output(&w, cx), OutputTo::OutputWindow);
    assert_eq!(line(&w, cx), "sort");
}

#[gpui::test]
fn alt_x_cancels_and_alt_t_shows_the_box_of_the_name(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "", "abc\n");
    open_dialog(&w, cx);
    fill(&w, cx, "sort");
    press(&w, cx, "alt-t");
    assert!(w.dialog(cx).unwrap().read_with(cx, |d, _| d.registering));
    // 実行 and 登録 are not there while the box of the name is: Alt+E does nothing, and the letter is not typed
    press(&w, cx, "alt-e");
    assert!(w.dialog(cx).is_some());
    assert_eq!(line(&w, cx), "sort");
    assert!(!w.running(cx));
    // キャンセル is not drawn then either, but Escape leaves the box of the name
    press(&w, cx, "escape");
    assert!(!w.dialog(cx).unwrap().read_with(cx, |d, _| d.registering));
    press(&w, cx, "alt-x");
    assert!(w.dialog(cx).is_none());
    assert!(!w.running(cx));
    assert_eq!(w.text(cx), "abc\n");
}

#[gpui::test]
fn a_place_that_is_not_offered_is_not_chosen(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "", "abc\n");
    w.view_mode(cx);
    let dialog = open_dialog(&w, cx);
    fill(&w, cx, "sort");
    press(&w, cx, "alt-r");
    press(&w, cx, "alt-n");
    assert_eq!(output(&w, cx), OutputTo::OutputWindow);
    assert!(w.dialog(cx).is_some());
    // a letter that no control has is not the dialog's
    let used = w
        .handle
        .update(cx, |_, window, app| {
            dialog.update(app, |d, cx| d.use_access_key('q', window, cx))
        })
        .unwrap();
    assert!(!used);
}
