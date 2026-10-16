use super::*;

fn lines(text: &str) -> Vec<String> {
    // the lines with their line endings, as the DIFF compares them
    text.split_inclusive('\n').map(str::to_owned).collect()
}

fn hunks(old: &str, new: &str) -> Vec<(HunkKind, Range<usize>, Range<usize>)> {
    hunks_with(old, new, DiffOptions::default())
}

fn hunks_with(
    old: &str,
    new: &str,
    options: DiffOptions,
) -> Vec<(HunkKind, Range<usize>, Range<usize>)> {
    diff_lines(&lines(old), &lines(new), options)
        .into_iter()
        .map(|h| (h.kind, h.old, h.new))
        .collect()
}

fn marks(m: &DiffMarks) -> Vec<(usize, DiffMark)> {
    m.iter().collect()
}

/// The marks of a document `own` compared with `other`.
fn marked(own: &str, other: &str, own_is_old: bool) -> DiffMarks {
    let (own_lines, other_lines) = (lines(own), lines(other));
    let hunks = if own_is_old {
        diff_lines(&own_lines, &other_lines, DiffOptions::default())
    } else {
        diff_lines(&other_lines, &own_lines, DiffOptions::default())
    };
    let mut m = DiffMarks::new();
    m.replace_with_hunks(&hunks, own_is_old, own_lines.len());
    m
}

#[test]
fn equal_texts_have_no_hunk() {
    assert!(hunks("a\nb\n", "a\nb\n").is_empty());
    assert!(hunks("", "").is_empty());
    assert!(hunks("日本語\r\n", "日本語\r\n").is_empty());
}

#[test]
fn the_three_kinds_of_hunks() {
    // `diff` says 2a3 / 2c2 / 2d1 for these
    assert_eq!(hunks("a\nb\n", "a\nb\nc\n"), [(HunkKind::Add, 2..2, 2..3)]);
    assert_eq!(
        hunks("a\nb\nc\n", "a\nx\nc\n"),
        [(HunkKind::Change, 1..2, 1..2)]
    );
    assert_eq!(
        hunks("a\nb\nc\n", "a\nc\n"),
        [(HunkKind::Delete, 1..2, 1..1)]
    );
    // several hunks, a change that is longer on one side
    assert_eq!(
        hunks("a\nb\nc\nd\ne\n", "a\nX\nY\nc\ne\nf\n"),
        [
            (HunkKind::Change, 1..2, 1..3),
            (HunkKind::Delete, 3..4, 4..4),
            (HunkKind::Add, 5..5, 5..6),
        ]
    );
}

#[test]
fn a_line_ending_is_part_of_the_line() {
    // Sakura writes the lines with their endings: LF against CRLF differs
    assert_eq!(
        hunks("a\nb\n", "a\r\nb\n"),
        [(HunkKind::Change, 0..1, 0..1)]
    );
    // a last line without a line ending differs from one with it
    assert_eq!(hunks("a\nb", "a\nb\n"), [(HunkKind::Change, 1..2, 1..2)]);
    // Japanese text
    assert_eq!(
        hunks("日本語\r\nです\r\n", "日本語\r\nでした\r\n"),
        [(HunkKind::Change, 1..2, 1..2)]
    );
}

#[test]
fn the_options_of_the_dialog() {
    let ignore_case = DiffOptions {
        ignore_case: true,
        ..DiffOptions::default()
    };
    assert!(hunks_with("Hello\nABC\n", "hello\nabc\n", ignore_case).is_empty());
    assert!(!hunks("Hello\n", "hello\n").is_empty());
    // full width letters fold as well (`to_lowercase`)
    assert!(hunks_with("ＡＢＣ\n", "ａｂｃ\n", ignore_case).is_empty());

    let all_space = DiffOptions {
        ignore_all_space: true,
        ..DiffOptions::default()
    };
    assert!(hunks_with("a b\tc\n", "abc\n", all_space).is_empty());
    assert!(hunks_with("a\u{3000}b\n", "ab\n", all_space).is_empty());
    assert!(!hunks("a b\n", "ab\n").is_empty());

    let space_change = DiffOptions {
        ignore_space_change: true,
        ..DiffOptions::default()
    };
    // a run of white space is one, white space at the end is nothing, but none is not some
    assert!(hunks_with("a   b \n", "a b\n", space_change).is_empty());
    assert!(hunks_with("a\t\tb\r\n", "a b\n", space_change).is_empty());
    assert!(!hunks_with("a b\n", "ab\n", space_change).is_empty());
    assert!(!hunks_with(" a\n", "a\n", space_change).is_empty());
    assert!(hunks_with("  a\n", " a\n", space_change).is_empty());

    // the tab option only changes the output of `diff`
    let tabs = DiffOptions {
        expand_tabs: true,
        ..DiffOptions::default()
    };
    assert_eq!(
        hunks_with("a\tb\n", "a b\n", tabs),
        [(HunkKind::Change, 0..1, 0..1)]
    );
}

#[test]
fn blank_lines_can_be_ignored() {
    let blank = DiffOptions {
        ignore_blank_lines: true,
        ..DiffOptions::default()
    };
    // lines that are only added or deleted blank lines
    assert!(hunks_with("a\nb\n", "a\n\n\nb\n", blank).is_empty());
    assert!(hunks_with("a\n\nb\n", "a\nb\n", blank).is_empty());
    assert!(!hunks("a\nb\n", "a\n\nb\n").is_empty());
    // a hunk with something else in it counts as a whole
    assert_eq!(
        hunks_with("a\nb\n", "a\n\nX\nb\n", blank),
        [(HunkKind::Add, 1..1, 1..3)]
    );
    // a blank line that is a CRLF
    assert!(hunks_with("a\r\nb\r\n", "a\r\n\r\nb\r\n", blank).is_empty());
    // a line of spaces is not blank unless white space is ignored too
    assert!(!hunks_with("a\nb\n", "a\n  \nb\n", blank).is_empty());
    let blank_and_space = DiffOptions {
        ignore_blank_lines: true,
        ignore_all_space: true,
        ..DiffOptions::default()
    };
    assert!(hunks_with("a\nb\n", "a\n  \nb\n", blank_and_space).is_empty());
}

#[test]
fn the_flags_of_the_macro() {
    let all = 0x7f;
    let o = DiffOptions::from_flags(all);
    assert!(
        o.ignore_case
            && o.ignore_all_space
            && o.ignore_space_change
            && o.ignore_blank_lines
            && o.expand_tabs
            && o.own_is_old
            && o.notify_no_diff
    );
    assert_eq!(o.flags(), all);
    assert_eq!(DiffOptions::from_flags(0), DiffOptions::default());
    assert_eq!(DiffOptions::from_flags(0x0005).flags(), 0x0005);
    // bits that mean nothing are dropped
    assert_eq!(DiffOptions::from_flags(0x1000).flags(), 0);
}

#[test]
fn the_marks_of_a_new_file() {
    // the document is the new file (the default): `diff other own`
    // added line
    let m = marked("a\nb\nc\n", "a\nc\n", false);
    assert_eq!(marks(&m), [(1, DiffMark::Append)]);
    assert!(m.in_use());
    // changed lines
    let m = marked("a\nX\nY\nd\n", "a\nb\nc\nd\n", false);
    assert_eq!(marks(&m), [(1, DiffMark::Change), (2, DiffMark::Change)]);
    // a line of the other file is missing: the mark is on the line after the gap
    let m = marked("a\nc\n", "a\nb\nc\n", false);
    assert_eq!(marks(&m), [(1, DiffMark::Delete)]);
    // a gap at the top
    let m = marked("b\n", "a\nb\n", false);
    assert_eq!(marks(&m), [(0, DiffMark::Delete)]);
    // a gap at the end is on the last line, as 削除(EOF)
    let m = marked("a\n", "a\nb\nc\n", false);
    assert_eq!(marks(&m), [(0, DiffMark::DeleteEx)]);
}

#[test]
fn the_marks_of_an_old_file() {
    // the document is the old file (0x20): `diff own other`
    // lines only in the document are "added" to it
    let m = marked("a\nb\nc\n", "a\nc\n", true);
    assert_eq!(marks(&m), [(1, DiffMark::Append)]);
    // lines only in the other file are gaps in the document
    let m = marked("a\nc\n", "a\nb\nc\n", true);
    assert_eq!(marks(&m), [(1, DiffMark::Delete)]);
    let m = marked("a\n", "a\nb\nc\n", true);
    assert_eq!(marks(&m), [(0, DiffMark::DeleteEx)]);
    let m = marked("a\nb\nc\n", "a\nX\nc\n", true);
    assert_eq!(marks(&m), [(1, DiffMark::Change)]);
}

#[test]
fn an_empty_document_has_no_line_to_mark() {
    let m = marked("", "a\nb\n", false);
    assert!(m.is_empty());
    assert!(m.in_use());
    // nothing differs: no DIFF is in use (Sakura then says so)
    let m = marked("a\n", "a\n", false);
    assert!(!m.in_use());
}

#[test]
fn set_range_follows_sakura() {
    let mut m = DiffMarks::new();
    // a start before the first line counts from the first; an end past the last gives that line
    // 削除(EOF), and a range that reaches it replaces the mark
    m.set_range(DiffMark::Change, -1, 1, 5);
    assert_eq!(marks(&m), [(0, DiffMark::Change), (1, DiffMark::Change)]);
    let mut m = DiffMarks::new();
    m.set_range(DiffMark::Append, 3, 7, 5);
    assert_eq!(marks(&m), [(3, DiffMark::Append), (4, DiffMark::Append)]);
    let mut m = DiffMarks::new();
    m.set_range(DiffMark::Delete, 5, 5, 5);
    assert_eq!(marks(&m), [(4, DiffMark::DeleteEx)]);
}

#[test]
fn the_search_goes_over_the_marked_lines() {
    let mut m = DiffMarks::new();
    m.set_range(DiffMark::Change, 2, 3, 10);
    m.set_range(DiffMark::Append, 7, 7, 10);
    assert_eq!((m.first(), m.last()), (Some(2), Some(7)));
    assert_eq!(m.next_after(0), Some(2));
    assert_eq!(m.next_after(2), Some(3));
    assert_eq!(m.next_after(3), Some(7));
    assert_eq!(m.next_after(7), None);
    assert_eq!(m.prev_before(7), Some(3));
    assert_eq!(m.prev_before(2), None);
    assert_eq!(m.next_after(usize::MAX), None);
}

#[test]
fn the_marks_follow_their_lines() {
    let mut m = DiffMarks::new();
    m.set_range(DiffMark::Change, 2, 2, 10);
    m.set_range(DiffMark::Append, 5, 5, 10);
    // two lines come in before them
    m.apply(LineDelta {
        start: 1,
        removed: 0,
        inserted: 2,
    });
    assert_eq!(marks(&m), [(4, DiffMark::Change), (7, DiffMark::Append)]);
    // the marked line goes with its text
    m.apply(LineDelta {
        start: 4,
        removed: 1,
        inserted: 0,
    });
    assert_eq!(marks(&m), [(6, DiffMark::Append)]);
    m.truncate(6);
    assert!(m.is_empty());
    m.clear();
    assert!(!m.in_use());
}

type Place = (usize, usize);

fn doc(text: &str) -> Document {
    Document::from_utf8(text.as_bytes().to_vec()).unwrap()
}

fn difference(a: &str, from_a: Place, b: &str, from_b: Place) -> Option<(Place, Place)> {
    let (pa, pb) = (Pos::new(from_a.0, from_a.1), Pos::new(from_b.0, from_b.1));
    first_difference(&doc(a), pa, &doc(b), pb)
        .unwrap()
        .map(|(x, y)| ((x.line, x.col), (y.line, y.col)))
}

#[test]
fn the_first_difference_of_two_documents() {
    // the same: nothing
    assert_eq!(difference("a\nb\n", (0, 0), "a\nb\n", (0, 0)), None);
    assert_eq!(difference("", (0, 0), "", (0, 0)), None);
    // in the middle of a line (Japanese: columns are bytes)
    assert_eq!(
        difference("日本語\nabc\n", (0, 0), "日本人\nabc\n", (0, 0)),
        Some(((0, 6), (0, 6)))
    );
    // on a later line
    assert_eq!(
        difference("a\nb\nc\n", (0, 0), "a\nb\nX\n", (0, 0)),
        Some(((2, 0), (2, 0)))
    );
    // the end of one line against more text of the other
    assert_eq!(
        difference("ab\n", (0, 0), "abc\n", (0, 0)),
        Some(((0, 2), (0, 2)))
    );
}

#[test]
fn the_line_endings_count() {
    // CRLF against LF: the difference is at the end of the text of the line
    assert_eq!(
        difference("a\r\nb\n", (0, 0), "a\nb\n", (0, 0)),
        Some(((0, 1), (0, 1)))
    );
    // a last line without a line ending against one with it
    assert_eq!(
        difference("a\nb", (0, 0), "a\nb\n", (0, 0)),
        Some(((1, 1), (1, 1)))
    );
}

#[test]
fn one_document_ends_first() {
    // the other has more lines: the row after the last line of the shorter one
    assert_eq!(
        difference("a\n", (0, 0), "a\nb\n", (0, 0)),
        Some(((1, 0), (1, 0)))
    );
    assert_eq!(
        difference("a\nb\n", (0, 0), "a\n", (0, 0)),
        Some(((1, 0), (1, 0)))
    );
}

#[test]
fn the_comparison_starts_at_the_carets() {
    // the first lines differ, but the carets are past them
    assert_eq!(difference("x\nab\n", (1, 0), "y\nab\n", (1, 0)), None);
    // different places in the two documents
    assert_eq!(
        difference("a\nbc\n", (1, 1), "xx\nc\nq\n", (1, 0)),
        Some(((2, 0), (2, 0)))
    );
    // a place in the middle of a Japanese char is moved to its start
    assert_eq!(difference("日本\n", (0, 1), "日本\n", (0, 0)), None);
}
