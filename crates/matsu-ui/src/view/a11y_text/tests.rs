//! The text runs, the caret and the selection that a screen reader gets, from the rows of the
//! last paint of a test window. The test platform connects no screen reader, so no tree is built
//! there; these tests read the snapshot the paint leaves and the runs made from it, which is
//! what the hook of the node writes.

use crate::settings::UserSettings;
use gpui::{Entity, TestAppContext, VisualTestContext};

use super::{At, Text, word_starts};
use crate::view::EditorView;
use crate::view::tests::{Fixture, setup};

fn text_of(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> Text {
    cx.run_until_parked();
    view.read_with(cx, |v, _| {
        v.a11y_text
            .borrow()
            .as_ref()
            .expect("the window was painted")
            .text()
    })
}

fn values(text: &Text) -> Vec<&str> {
    text.runs.iter().map(|r| r.value.as_str()).collect()
}

fn apply(f: &Fixture, cx: &mut VisualTestContext, ini: &str) {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(ini.as_bytes());
    let settings = UserSettings::from_profile(matsu_config::Profile::from_bytes(&bytes), None);
    f.view
        .update(cx, |v, cx| v.apply_user_settings(&settings, cx));
}

fn at(run: usize, index: usize) -> At {
    At { run, index }
}

#[test]
fn words_start_where_the_kind_of_character_changes() {
    // Trailing spaces belong to the word before; each mark of punctuation is a word.
    assert_eq!(word_starts("ab cd", None, false, false), [3]);
    assert_eq!(word_starts("ab, cd", None, false, false), [2, 4]);
    // leading spaces are a word of their own
    assert_eq!(word_starts("  ab", None, false, false), [2]);
    assert_eq!(
        word_starts("ひらがなカタカナ漢字abc", None, false, false),
        [4, 8, 10]
    );
    // the same rules as Ctrl+Right: symbols make a word together, the prolonged sound mark goes
    // with the kana around it
    assert_eq!(word_starts("a+-b", None, false, false), [1, 3]);
    assert_eq!(
        word_starts("あーい", None, false, false),
        Vec::<usize>::new()
    );
    assert_eq!(word_starts("", None, false, false), Vec::<usize>::new());
    // after a soft wrap: a word that starts at the wrap, and one that goes on
    assert_eq!(word_starts("kl", Some(' '), false, false), [0]);
    assert_eq!(
        word_starts("kl", Some('k'), false, false),
        Vec::<usize>::new()
    );
    assert_eq!(word_starts("kl mn", Some('k'), false, false), [3]);
}

#[gpui::test]
fn a_word_that_starts_at_a_soft_wrap_starts_a_word_in_its_run(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abcdefghi kl\nabcdefghijkl");
    apply(
        &f,
        cx,
        "[Types(0)]\r\nnInts=0,10,0,4,-1,-1,0,1,1,0,0,0\r\nnTextWrapMethod=1\r\n",
    );
    let text = text_of(&f.view, cx);
    assert_eq!(values(&text), ["abcdefghi ", "kl\n", "abcdefghij", "kl"]);
    // "kl" follows a space: a new word, and the line break after it is a start (where the key stops). The second "kl" goes on from "abcdefghij".
    assert_eq!(text.runs[1].word_starts, [0, 2]);
    assert!(text.runs[3].word_starts.is_empty());
}

#[gpui::test]
fn a_row_of_more_than_255_characters_has_no_word_starts(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &"a ".repeat(200));
    let text = text_of(&f.view, cx);
    assert_eq!(text.runs[0].char_lengths.len(), 400);
    assert!(text.runs[0].word_starts.is_empty());
}

#[gpui::test]
fn the_rows_are_runs_and_every_kind_of_line_break_is_one_newline(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab\r\ncd\nef\rgh");
    let text = text_of(&f.view, cx);
    assert_eq!(values(&text), ["ab\n", "cd\n", "ef\n", "gh"]);
    for run in &text.runs {
        assert_eq!(run.char_lengths.len(), run.value.chars().count());
        assert_eq!(run.positions.len(), run.char_lengths.len());
        assert_eq!(run.widths.len(), run.char_lengths.len());
    }
}

#[gpui::test]
fn an_empty_last_line_is_an_empty_run(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab\n");
    let text = text_of(&f.view, cx);
    assert_eq!(values(&text), ["ab\n", ""]);
}

#[gpui::test]
fn japanese_characters_count_three_bytes_and_two_cells(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "aあ\u{1F600}b");
    let text = text_of(&f.view, cx);
    let run = &text.runs[0];
    assert_eq!(run.value, "aあ\u{1F600}b");
    assert_eq!(run.char_lengths, [1, 3, 4, 1]);
    // widths in cells, all with the same factor: 1, 2, 2, 1
    let unit = run.widths[0];
    assert!(unit > 0.0);
    assert_eq!(run.widths, [unit, 2.0 * unit, 2.0 * unit, unit]);
    assert_eq!(run.positions, [0.0, unit, 3.0 * unit, 5.0 * unit]);
}

#[gpui::test]
fn a_tab_takes_the_columns_up_to_the_next_stop(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\tb");
    let text = text_of(&f.view, cx);
    let run = &text.runs[0];
    let unit = run.widths[0];
    // tab width 4: the tab at column 1 takes 3 columns, and b starts at column 4
    assert_eq!(run.widths, [unit, 3.0 * unit, unit]);
    assert_eq!(run.positions[2], 4.0 * unit);
}

#[gpui::test]
fn the_caret_follows_typing_in_japanese(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    cx.simulate_input("あい");
    let text = text_of(&f.view, cx);
    assert_eq!(values(&text), ["あい"]);
    // after the second character: index 2 (not the byte offset 6)
    assert_eq!(text.selection, Some((at(0, 2), at(0, 2))));
    cx.simulate_keystrokes("left");
    let text = text_of(&f.view, cx);
    assert_eq!(text.selection, Some((at(0, 1), at(0, 1))));
}

#[gpui::test]
fn the_selection_runs_from_the_anchor_to_the_caret(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc\r\ndef");
    cx.simulate_keystrokes("right shift-right shift-down");
    let text = text_of(&f.view, cx);
    // the anchor is where the first shift-right started, (0, 1); the caret is now at (1, 2)
    assert_eq!(text.selection, Some((at(0, 1), at(1, 2))));
    // backwards over the line break: the anchor stays, the caret is at the end of the first line
    cx.simulate_keystrokes("shift-left shift-left shift-left");
    let text = text_of(&f.view, cx);
    assert_eq!(text.selection, Some((at(0, 1), at(0, 3))));
}

#[gpui::test]
fn the_caret_at_the_end_of_a_line_is_before_the_line_break(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc\r\ndef");
    cx.simulate_keystrokes("end");
    let text = text_of(&f.view, cx);
    assert_eq!(text.selection, Some((at(0, 3), at(0, 3))));
}

#[gpui::test]
fn the_caret_after_the_last_line_break_is_in_the_empty_row(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "ab\n");
    cx.simulate_keystrokes("ctrl-end");
    let text = text_of(&f.view, cx);
    assert_eq!(text.selection, Some((at(1, 0), at(1, 0))));
}

#[gpui::test]
fn a_wrapped_line_is_a_run_for_each_row(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abcdefghijkl\nm");
    apply(
        &f,
        cx,
        "[Types(0)]\r\nnInts=0,10,0,4,-1,-1,0,1,1,0,0,0\r\nnTextWrapMethod=1\r\n",
    );
    let text = text_of(&f.view, cx);
    // a soft wrap has no line break in the text
    assert_eq!(values(&text), ["abcdefghij", "kl\n", "m"]);
    // the end of the first row is the start of the second (as `pos_to_row` has it)
    cx.simulate_keystrokes("right right right right right right right right right right");
    let text = text_of(&f.view, cx);
    assert_eq!(text.selection, Some((at(1, 0), at(1, 0))));
}

#[gpui::test]
fn only_the_rows_on_the_screen_are_runs_and_they_follow_the_scroll(cx: &mut TestAppContext) {
    let lines: Vec<String> = (0..2000).map(|i| format!("line {i}")).collect();
    let (f, cx) = setup(cx, &lines.join("\n"));
    let first = text_of(&f.view, cx);
    assert!(first.runs.len() < 200, "{} runs", first.runs.len());
    assert_eq!(first.runs[0].value, "line 0\n");
    // The caret is in the first row.
    assert_eq!(first.selection, Some((at(0, 0), at(0, 0))));

    let line_height = f.view.read_with(cx, |v, _| {
        v.a11y_text.borrow().as_ref().unwrap().line_height
    });
    f.view.update(cx, |v, cx| {
        v.scroll_y = 1000.0 * line_height;
        cx.notify();
    });
    let scrolled = text_of(&f.view, cx);
    assert_eq!(scrolled.runs[0].value, "line 1000\n");
    assert_eq!(scrolled.runs[0].row, 1000);
    assert!(scrolled.runs.len() < 200);
    // The caret is out of the screen: no selection (nothing to say about a place not shown).
    assert_eq!(scrolled.selection, None);
}

#[gpui::test]
fn a_selection_that_goes_off_the_screen_ends_at_the_edge(cx: &mut TestAppContext) {
    let lines: Vec<String> = (0..2000).map(|i| format!("line {i}")).collect();
    let (f, cx) = setup(cx, &lines.join("\n"));
    // select from the start to line 1500 (off the screen), then bring the caret into view
    f.view.update(cx, |v, cx| {
        v.cursor.select(
            matsu_core::buffer::Pos::new(0, 2),
            matsu_core::buffer::Pos::new(1500, 3),
        );
        cx.notify();
    });
    let text = text_of(&f.view, cx);
    // the caret (the active end) is off the screen
    assert_eq!(text.selection, None);
    f.view.update(cx, |v, cx| {
        v.cursor.select(
            matsu_core::buffer::Pos::new(1500, 3),
            matsu_core::buffer::Pos::new(1, 2),
        );
        cx.notify();
    });
    let text = text_of(&f.view, cx);
    let last = text.runs.len() - 1;
    let (anchor, focus) = text.selection.unwrap();
    assert_eq!(focus, at(1, 2));
    // the fixed end is below the screen: the end of the last run
    assert_eq!(anchor.run, last);
    assert_eq!(anchor.index, text.runs[last].char_lengths.len());
}

#[gpui::test]
fn a_box_selection_is_the_caret_only(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc\ndef");
    f.view.update(cx, |v, cx| {
        v.cursor.begin_box(0, false);
        v.cursor.move_box_to(matsu_core::buffer::Pos::new(1, 2), 2);
        cx.notify();
    });
    let text = text_of(&f.view, cx);
    let (anchor, focus) = text.selection.unwrap();
    assert_eq!(anchor, focus);
}

#[gpui::test]
fn a_row_of_255_characters_and_a_line_break_has_no_word_starts(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &format!("{}a\nb", "a ".repeat(127)));
    let text = text_of(&f.view, cx);
    assert_eq!(text.runs[0].char_lengths.len(), 256);
    assert!(text.runs[0].word_starts.is_empty());
}

#[test]
fn the_line_break_is_a_word_start_where_ctrl_right_stops() {
    assert_eq!(word_starts("abc", None, true, false), [3]);
    assert_eq!(word_starts("ab cd", None, true, false), [3, 5]);
    assert_eq!(word_starts("", None, true, false), Vec::<usize>::new());
    assert_eq!(word_starts("abc", None, false, false), Vec::<usize>::new());
}

#[gpui::test]
fn the_word_before_a_line_break_ends_at_it_for_crlf_and_lf(cx: &mut TestAppContext) {
    for text in ["abc\r\ndef", "abc\ndef"] {
        let (f, cx) = setup(cx, text);
        let text = text_of(&f.view, cx);
        assert_eq!(values(&text), ["abc\n", "def"]);
        // Ctrl+Right from "abc" stops before the line break (index 3), as the key does.
        assert_eq!(text.runs[0].word_starts, [3]);
        assert!(text.runs[1].word_starts.is_empty());
    }
}

#[gpui::test]
fn no_row_below_the_screen_is_a_run(cx: &mut TestAppContext) {
    let lines: Vec<String> = (0..2000).map(|i| format!("line {i}")).collect();
    let (f, cx) = setup(cx, &lines.join("\n"));
    let text = text_of(&f.view, cx);
    let (line_height, view_height) = f.view.read_with(cx, |v, _| {
        (
            v.a11y_text.borrow().as_ref().unwrap().line_height,
            v.metrics.view_height,
        )
    });
    // At scroll 0 the top of the last run is above the bottom of the text area.
    let last = (text.runs.len() - 1) as f32;
    assert!(last * line_height < view_height, "{last} rows");
    assert!((last + 1.0) * line_height >= view_height);
}

#[test]
fn with_the_stop_at_both_ends_setting_a_word_end_is_a_start_too() {
    // "ab cd": Ctrl+Right stops at 2 (the end of "ab") and then at 3 with the setting on.
    assert_eq!(word_starts("ab cd", None, false, true), [2, 3]);
    assert_eq!(word_starts("ab cd", None, false, false), [3]);
    assert_eq!(word_starts("ab \n", None, true, true), [2, 3]);
}
