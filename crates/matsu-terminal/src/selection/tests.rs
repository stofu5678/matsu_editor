use alacritty_terminal::event::VoidListener;
use alacritty_terminal::term::Config;
use alacritty_terminal::vte::ansi::Processor;

use super::*;
use crate::terminal::TerminalSize;

/// A terminal state of `columns` x `rows`, fed by hand.
fn term(columns: u16, rows: u16, output: &str) -> (Term<VoidListener>, Processor) {
    let size = TerminalSize::new(columns, rows);
    let config = Config {
        scrolling_history: 100,
        semantic_escape_chars: WORD_DELIMITERS.to_owned(),
        ..Config::default()
    };
    let mut term = Term::new(config, &size, VoidListener);
    let mut processor = Processor::new();
    processor.advance(&mut term, output.as_bytes());
    (term, processor)
}

fn select(term: &mut Term<VoidListener>, from: (i32, usize), to: (i32, usize)) {
    start(term, SelectionMode::Cells, from.0, from.1, CellSide::Left);
    update(term, to.0, to.1, CellSide::Right);
}

#[test]
fn nothing_is_selected_at_first_and_a_click_selects_nothing() {
    let (mut term, _) = term(20, 3, "hello");
    assert!(!is_selected(&term));
    assert_eq!(text(&term), None);
    start(&mut term, SelectionMode::Cells, 0, 1, CellSide::Left);
    assert!(!is_selected(&term), "no drag, no cells");
    assert_eq!(span(&term), None);
    // the pointer moved inside the same half of the cell: still nothing
    update(&mut term, 0, 1, CellSide::Left);
    assert!(!is_selected(&term));
}

#[test]
fn a_drag_selects_the_cells_it_passed_the_middle_of() {
    let (mut term, _) = term(20, 3, "hello world");
    select(&mut term, (0, 0), (0, 4));
    assert_eq!(text(&term).as_deref(), Some("hello"));
    assert_eq!(
        span(&term),
        Some(SelectionSpan {
            start_row: 0,
            start_column: 0,
            end_row: 0,
            end_column: 4
        })
    );
    // from the right half of a cell to the left half of another: those two cells are not selected
    start(&mut term, SelectionMode::Cells, 0, 0, CellSide::Right);
    update(&mut term, 0, 5, CellSide::Left);
    assert_eq!(text(&term).as_deref(), Some("ello"));
    // backwards
    start(&mut term, SelectionMode::Cells, 0, 10, CellSide::Right);
    update(&mut term, 0, 6, CellSide::Left);
    assert_eq!(text(&term).as_deref(), Some("world"));
}

#[test]
fn a_selection_across_lines_has_line_ends_and_cuts_trailing_blanks() {
    let (mut term, _) = term(20, 4, "one   \r\ntwo\r\n\r\nfour");
    select(&mut term, (0, 0), (3, 3));
    assert_eq!(text(&term).as_deref(), Some("one\ntwo\n\nfour"));
    // from the middle of one line to the middle of another
    select(&mut term, (0, 1), (1, 1));
    assert_eq!(text(&term).as_deref(), Some("ne\ntw"));
    let span = span(&term).unwrap();
    assert!(
        span.contains(0, 5) && span.contains(0, 19),
        "the rest of the first line"
    );
    assert!(span.contains(1, 0) && !span.contains(1, 2));
    assert!(!span.contains(2, 0));
}

#[test]
fn spaces_inside_a_line_stay() {
    let (mut term, _) = term(20, 2, "a   b");
    select(&mut term, (0, 0), (0, 10));
    assert_eq!(text(&term).as_deref(), Some("a   b"));
}

#[test]
fn wide_characters_are_selected_whole_and_copied_once() {
    let (mut term, _) = term(10, 2, "日本語abc");
    // the selection ends on the right half of 本 (column 3): 本 is selected whole
    select(&mut term, (0, 0), (0, 3));
    assert_eq!(text(&term).as_deref(), Some("日本"));
    // it starts on the right half of 日 (column 1)
    select(&mut term, (0, 1), (0, 5));
    assert_eq!(text(&term).as_deref(), Some("日本語"));
    // all of it
    select(&mut term, (0, 0), (0, 8));
    assert_eq!(text(&term).as_deref(), Some("日本語abc"));
}

#[test]
fn a_line_that_wraps_is_joined_without_a_line_end() {
    // 10 columns: the 15 characters wrap onto a second row
    let (mut term, _) = term(10, 3, "abcdefghijklmno");
    select(&mut term, (0, 0), (1, 9));
    assert_eq!(text(&term).as_deref(), Some("abcdefghijklmno"));
}

#[test]
fn a_double_click_selects_a_word_and_a_triple_click_the_line() {
    let (mut term, _) = term(30, 3, "echo foo/bar-baz qux");
    start(&mut term, SelectionMode::Words, 0, 6, CellSide::Left);
    assert_eq!(text(&term).as_deref(), Some("foo"), "/ ends a word");
    start(&mut term, SelectionMode::Words, 0, 14, CellSide::Left);
    assert_eq!(text(&term).as_deref(), Some("baz"), "- ends a word");
    start(&mut term, SelectionMode::Words, 0, 0, CellSide::Left);
    assert_eq!(text(&term).as_deref(), Some("echo"));
    // the word grows by words as the pointer moves
    update(&mut term, 0, 6, CellSide::Right);
    assert_eq!(text(&term).as_deref(), Some("echo foo"));
    // a line: its text, no line end after it, wherever the pointer is
    start(&mut term, SelectionMode::Lines, 0, 7, CellSide::Left);
    assert_eq!(text(&term).as_deref(), Some("echo foo/bar-baz qux"));
    assert!(is_selected(&term));
}

#[test]
fn a_triple_click_on_a_wrapped_line_selects_all_of_it() {
    let (mut term, _) = term(10, 3, "abcdefghijklmno");
    start(&mut term, SelectionMode::Lines, 1, 2, CellSide::Left);
    assert_eq!(text(&term).as_deref(), Some("abcdefghijklmno"));
}

#[test]
fn clearing_removes_the_selection() {
    let (mut term, _) = term(20, 3, "hello");
    select(&mut term, (0, 0), (0, 4));
    assert!(is_selected(&term));
    clear(&mut term);
    assert!(!is_selected(&term));
    assert_eq!(text(&term), None);
    // moving the end of no selection does nothing
    update(&mut term, 0, 3, CellSide::Right);
    assert!(!is_selected(&term));
}

#[test]
fn positions_outside_the_screen_are_pulled_in() {
    let (mut term, _) = term(10, 2, "hello");
    start(&mut term, SelectionMode::Cells, 0, 0, CellSide::Left);
    // far to the right of the last column
    update(&mut term, 0, 500, CellSide::Right);
    assert_eq!(text(&term).as_deref(), Some("hello"));
}

#[test]
fn output_that_scrolls_the_lines_moves_the_selection_with_them() {
    let (mut term, mut processor) = term(10, 3, "one\r\ntwo\r\nthree");
    select(&mut term, (1, 0), (1, 2));
    assert_eq!(text(&term).as_deref(), Some("two"));
    // a new line scrolls the screen up by one: "two" is on row 0 now
    processor.advance(&mut term, b"\r\nfour");
    assert_eq!(text(&term).as_deref(), Some("two"));
    assert_eq!(span(&term).map(|s| s.start_row), Some(0));
    // two more lines: "two" is in the scrollback, above the view; the selection is still there
    processor.advance(&mut term, b"\r\nfive\r\nsix");
    assert_eq!(text(&term).as_deref(), Some("two"));
    assert_eq!(span(&term).map(|s| s.start_row), Some(-2));
}

#[test]
fn erasing_the_selected_lines_clears_the_selection() {
    let (mut term, mut processor) = term(10, 3, "one\r\ntwo");
    select(&mut term, (0, 0), (0, 2));
    assert!(is_selected(&term));
    // clear the screen (ED 2)
    processor.advance(&mut term, b"\x1b[2J");
    assert!(!is_selected(&term));
}

#[test]
fn the_alternate_screen_has_no_selection() {
    let (mut term, mut processor) = term(10, 3, "one");
    select(&mut term, (0, 0), (0, 2));
    processor.advance(&mut term, b"\x1b[?1049h");
    assert!(!is_selected(&term));
}

#[test]
fn a_selection_in_the_scrollback_is_on_the_view_while_it_is_scrolled_back() {
    let (mut term, _) = term(10, 3, "a\r\nb\r\nc\r\nd\r\ne");
    // "a" and "b" are in the scrollback now
    term.scroll_display(alacritty_terminal::grid::Scroll::Delta(2));
    // row 0 of the view is "a"
    select(&mut term, (0, 0), (1, 0));
    assert_eq!(text(&term).as_deref(), Some("a\nb"));
    assert_eq!(span(&term).map(|s| (s.start_row, s.end_row)), Some((0, 1)));
    // back at the bottom the same selection is above the view
    term.scroll_display(alacritty_terminal::grid::Scroll::Bottom);
    assert_eq!(
        span(&term).map(|s| (s.start_row, s.end_row)),
        Some((-2, -1))
    );
    assert_eq!(text(&term).as_deref(), Some("a\nb"));
}

#[test]
fn the_span_knows_which_cells_it_holds() {
    let span = SelectionSpan {
        start_row: 1,
        start_column: 3,
        end_row: 3,
        end_column: 2,
    };
    assert!(!span.contains(0, 5));
    assert!(!span.contains(1, 2));
    assert!(span.contains(1, 3));
    assert!(span.contains(2, 0) && span.contains(2, 99));
    assert!(span.contains(3, 2));
    assert!(!span.contains(3, 3));
    assert!(!span.contains(4, 0));
}

#[test]
fn combining_characters_stay_with_their_base() {
    let (mut term, _) = term(10, 2, "e\u{301}x");
    select(&mut term, (0, 0), (0, 1));
    assert_eq!(text(&term).as_deref(), Some("e\u{301}x"));
}
