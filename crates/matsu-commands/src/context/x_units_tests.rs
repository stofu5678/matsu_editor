//! The commands count in `X` (ADR-0034): with a column of 64 `X` they must give the same text and
//! the same positions, in columns, as with a column of 1 (what the screen has today). Each test
//! runs one list of commands on both and compares what is left after every step.
//!
//! The text has tabs and full-width chars, which are the characters whose width is a number of
//! columns, and long and short lines, so that the goal column of a vertical move is on text on
//! some rows and in the empty space on others.

use matsu_core::buffer::Pos;
use matsu_core::cursor::Cursor;

use crate::clipboard::ClipKind;
use crate::command::Arg;
use crate::edit_options::{EditOptions, with_options};
use crate::layout::X;
use crate::testing::Editor;

const PER_COLUMN: [u32; 2] = [1, 64];

fn free() -> EditOptions {
    EditOptions {
        free_cursor: true,
        ..EditOptions::DEFAULT
    }
}

/// What a command leaves behind, with every horizontal position in columns.
#[derive(Debug, PartialEq)]
struct Outcome {
    marked: String,
    box_state: Option<((usize, usize), (usize, usize))>,
    free: Option<usize>,
    goal: Option<usize>,
}

fn outcome(e: &Editor) -> Outcome {
    let per = e.xs(1);
    Outcome {
        marked: e.marked(),
        box_state: e.box_state(),
        free: e.cursor().free_col().map(|x| x / per),
        goal: e.cursor().goal_col().map(|x| x / per),
    }
}

type Step<'a> = (&'a str, &'a [Arg]);

/// Runs `steps` on `make()` once per column size and returns what was left after each step with a
/// column of 1, after checking that a column of 64 left the same.
fn same_in_both(make: impl Fn(u32) -> Editor, steps: &[Step]) -> Vec<Outcome> {
    let runs: Vec<Vec<Outcome>> = PER_COLUMN
        .iter()
        .map(|&per| {
            let mut e = make(per);
            steps
                .iter()
                .map(|&(id, args)| {
                    e.run_with(id, args);
                    outcome(&e)
                })
                .collect()
        })
        .collect();
    for (i, (id, _)) in steps.iter().enumerate() {
        assert_eq!(runs[0][i], runs[1][i], "after step {i}, {id}");
    }
    runs.into_iter().next().unwrap_or_default()
}

fn editor(marked: &str, per_column: u32) -> Editor {
    Editor::new(marked).with_per_column(per_column)
}

const TEXT: &str = "|a\tb日c\nxy\n日本語テキスト\n\t\tq\nabc\nlonger line here";

fn repeat<'a>(id: &'a str, times: usize) -> Vec<Step<'a>> {
    vec![(id, &[][..]); times]
}

#[test]
fn up_and_down_keep_the_goal_column_through_tabs_and_full_width_chars() {
    let mut steps = repeat("cursor_right", 3);
    steps.extend(repeat("cursor_down", 5));
    steps.extend(repeat("cursor_up", 5));
    steps.extend(repeat("cursor_right", 2));
    steps.extend(repeat("cursor_down", 2));
    steps.push(("go_line_end", &[]));
    steps.extend(repeat("cursor_up", 3));
    let done = same_in_both(|per| editor(TEXT, per), &steps);
    // the goal is remembered over the short line "xy": it was column 4 (after the tab) and
    // is still there on the row below
    assert_eq!(done[3].goal, Some(5));
}

#[test]
fn a_box_selection_moves_through_tabs_and_full_width_chars() {
    let mut steps = vec![("begin_box", &[][..])];
    steps.extend(repeat("cursor_right_box", 3));
    steps.extend(repeat("cursor_down_box", 3));
    steps.extend(repeat("cursor_right_box", 6));
    steps.extend(repeat("cursor_down_box", 2));
    steps.extend(repeat("cursor_left_box", 4));
    steps.extend(repeat("cursor_up_box", 4));
    steps.push(("go_line_end_box", &[]));
    steps.push(("cursor_down_box", &[]));
    steps.push(("go_file_end_box", &[]));
    steps.push(("go_file_top_box", &[]));
    let done = same_in_both(|per| editor(TEXT, per), &steps);
    // the box got into the empty space right of the text at some step
    assert!(
        done.iter()
            .any(|o| o.box_state.is_some_and(|(_, caret)| caret.1 > 9))
    );
}

#[test]
fn a_box_is_copied_deleted_and_typed_into_the_same_way() {
    let mut steps = vec![("begin_box", &[][..])];
    steps.extend(repeat("cursor_right_box", 2));
    steps.extend(repeat("cursor_down_box", 3));
    steps.push(("copy", &[]));
    steps.push(("input_char", &[Arg::Char('Z')]));
    steps.push(("cursor_right_box", &[]));
    steps.push(("delete", &[]));
    steps.push(("undo", &[]));
    steps.push(("undo", &[]));
    same_in_both(|per| editor(TEXT, per), &steps);
}

#[test]
fn the_text_of_a_box_is_the_same() {
    let text = |per: u32| {
        let mut e = editor("a|\tb日c\nxy\n日本語テキスト\nabc", per);
        e.run("begin_box")
            .run("cursor_down_box")
            .run("cursor_down_box");
        for _ in 0..4 {
            e.run("cursor_right_box");
        }
        e.with_context(|c| c.box_text().unwrap())
    };
    assert_eq!(text(1), text(64));
    // the box runs from column 1 to column 8 (the caret moved right over four chars of the
    // third row): the tab, "b", "日" and "c" of the first row, "y", and "日本語テ"
    assert_eq!(text(64).as_deref(), Some("\tb日c\r\ny\r\n日本語テ\r\n"));
}

#[test]
fn indenting_and_pasting_a_box_past_the_end_of_a_line_fill_with_the_same_spaces() {
    // the free cursor stands 8 columns right of "ab", and a box is pasted there: both rows are
    // filled with spaces up to column 10
    let mut steps = repeat("cursor_right", 8);
    steps.push(("paste", &[]));
    let make = |per| editor("ab|\ncd\nefgh\nij", per).with_clipboard("XY\r\nZ\r\n", ClipKind::Box);
    let done = with_options(free(), || same_in_both(make, &steps));
    let marked = &done[done.len() - 1].marked;
    assert!(marked.starts_with("ab        |XY"), "{marked:?}");
    assert!(marked.contains("\ncd        Z"), "{marked:?}");

    let steps = vec![("begin_box", &[][..]), ("paste", &[])];
    let make =
        |per| editor("\tab|\n日\nxxxxxxx", per).with_clipboard("12\r\n34\r\n56", ClipKind::Box);
    same_in_both(make, &steps);
}

#[test]
fn the_tab_indent_of_a_box_and_a_soft_tab_are_the_same() {
    let make = |per| {
        editor("|abc\n\txy\n日本語\nz", per)
            .with_tab_width(4)
            .with_ins_space(true)
    };
    let mut steps = vec![("begin_box", &[][..])];
    steps.extend(repeat("cursor_right_box", 1));
    steps.extend(repeat("cursor_down_box", 3));
    steps.push(("indent_tab", &[]));
    steps.push(("indent_tab", &[]));
    steps.push(("indent_space", &[]));
    same_in_both(make, &steps);
    // a soft tab at the caret and over several rows
    let steps = vec![
        ("cursor_right", &[][..]),
        ("indent_tab", &[]),
        ("select_all", &[]),
        ("indent_tab", &[]),
    ];
    same_in_both(make, &steps);
}

#[test]
fn the_free_cursor_goes_through_the_gap_in_the_same_columns() {
    with_options(free(), || {
        let mut steps = repeat("go_line_end", 1);
        steps.extend(repeat("cursor_right", 6));
        steps.extend(repeat("cursor_down", 4));
        steps.extend(repeat("cursor_left", 3));
        steps.extend(repeat("cursor_up", 2));
        steps.push(("input_char", &[Arg::Char('x')]));
        steps.extend(repeat("cursor_down", 2));
        steps.push(("delete_back", &[]));
        steps.extend(repeat("cursor_right", 3));
        steps.push(("delete", &[]));
        same_in_both(|per| editor(TEXT, per), &steps);
    });
}

#[test]
fn typing_in_the_free_gap_adds_the_spaces_that_reach_the_position() {
    with_options(free(), || {
        for per in PER_COLUMN {
            // 6 columns right of the end of "ab" and 10 `X` more
            let per_column = per as usize;
            let mut cursor = Cursor::new(Pos::new(0, 2));
            cursor.set_free_col(Some(2 * per_column + 6 * per_column));
            let mut e = Editor::from_parts("ab\ncd", cursor).with_per_column(per);
            e.run_with("input_char", &[Arg::Char('x')]);
            assert_eq!(e.marked(), "ab      x|\ncd", "per_column {per}");
        }
        // 64 per column: a gap that is not a whole number of spaces gets one more, and the
        // typed char starts past the position that was asked for
        let mut cursor = Cursor::new(Pos::new(0, 2));
        cursor.set_free_col(Some(2 * 64 + 3 * 64 + 10));
        let mut e = Editor::from_parts("ab\ncd", cursor).with_per_column(64);
        e.run_with("input_char", &[Arg::Char('x')]);
        assert_eq!(e.marked(), "ab    x|\ncd");
    });
}

#[test]
fn spaces_to_fill_counts_the_spaces_that_reach_the_gap() {
    let count = |per: u32, raw: u32| {
        let mut e = editor("a|", per);
        e.with_context(|c| c.spaces_to_fill(X::from_raw(raw)))
    };
    // one `X` per column: one space for each
    for gap in [0, 1, 2, 7] {
        assert_eq!(count(1, gap), gap as usize);
    }
    // 64 per column: whole spaces are counted, and the rest takes another one (Sakura's loop
    // `for (i = 0; i < gap; i += width of a space)` in `InsertData_CEditView`)
    assert_eq!(count(64, 0), 0);
    assert_eq!(count(64, 1), 1);
    assert_eq!(count(64, 64), 1);
    assert_eq!(count(64, 65), 2);
    assert_eq!(count(64, 64 * 5), 5);
    assert_eq!(count(64, 64 * 5 - 1), 5);
}

#[test]
fn a_soft_tab_counts_the_spaces_up_to_the_next_stop_in_x() {
    let spaces = |per: u32, raw: u32| {
        let mut e = editor("a|", per).with_tab_width(4);
        e.with_context(|c| c.soft_tab_at(X::from_raw(raw)).len())
    };
    // one `X` per column: up to the next multiple of 4
    for (col, want) in [(0, 4), (1, 3), (3, 1), (4, 4), (6, 2)] {
        assert_eq!(spaces(1, col), want, "column {col}");
    }
    // the same with 64 per column, also a little past a column. Sakura's `SSoftTabData::Len`
    // is `(tab + w - (x + w) % tab) / dx` with `w = dx - 1`.
    for (col, want) in [(0, 4), (1, 3), (3, 1), (4, 4), (6, 2)] {
        assert_eq!(spaces(64, col * 64), want, "column {col}");
    }
    // a little past a column counts as the next whole column: 65 is taken as 2 columns (2 spaces
    // to the stop at 4), and 193 as 4 columns, which is a stop (a whole tab of 4 spaces)
    assert_eq!(spaces(64, 64 + 1), 2);
    assert_eq!(spaces(64, 3 * 64 + 1), 4);
}
