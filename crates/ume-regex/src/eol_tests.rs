//! A regular expression is matched against each line together with its own line ending, as
//! Sakura does (`CDocLine::GetDocLineStrWithEOL` in `CSearchAgent::SearchWord`), but never
//! across lines. The cases that the real Sakura answered are in
//! `crates/ume/tests/sakura_oracle/cases/regex-*.mac`.

use std::ops::Range;
use std::sync::Arc;

use ume_core::buffer::{Document, Pos};
use ume_core::search::{Control, Direction, ReplaceTarget, find, find_all};

use crate::LIMITS_LOCK;
use crate::{RegexMatcher, RegexReplacer, RegexSearchOptions, ReplaceAllMode};

fn doc(text: &str) -> Document {
    Document::from_utf8(text.as_bytes().to_vec()).unwrap()
}

fn text_of(doc: &Document) -> String {
    doc.text_between(Pos::new(0, 0), doc.end_pos().unwrap())
        .unwrap()
}

fn replacer(pattern: &str, with: &str) -> RegexReplacer {
    let m = RegexMatcher::new(pattern, RegexSearchOptions { match_case: true }).unwrap();
    RegexReplacer::new(Arc::new(m), with, ReplaceTarget::Match).unwrap()
}

fn run(
    pattern: &str,
    with: &str,
    text: &str,
    mode: ReplaceAllMode,
    range: Option<Range<Pos>>,
) -> (String, usize) {
    let _l = LIMITS_LOCK.read().unwrap_or_else(|e| e.into_inner());
    let mut d = doc(text);
    let r = replacer(pattern, with)
        .replace_all(&mut d, mode, range, Control::new())
        .unwrap();
    (text_of(&d), r.edits.len())
}

fn all(pattern: &str, with: &str, text: &str) -> (String, usize) {
    run(pattern, with, text, ReplaceAllMode::Batch, None)
}

fn search(pattern: &str, text: &str, from: Pos, direction: Direction) -> Option<(Pos, Pos)> {
    let _l = LIMITS_LOCK.read().unwrap_or_else(|e| e.into_inner());
    let d = doc(text);
    let m = RegexMatcher::new(pattern, RegexSearchOptions { match_case: true }).unwrap();
    find(&d, &m, from, direction, false, &mut Control::new())
        .unwrap()
        .map(|f| (f.start, f.end))
}

fn forward(pattern: &str, text: &str, from: Pos) -> Option<(Pos, Pos)> {
    search(pattern, text, from, Direction::Forward)
}

fn backward(pattern: &str, text: &str, from: Pos) -> Option<(Pos, Pos)> {
    search(pattern, text, from, Direction::Backward)
}

fn p(line: usize, col: usize) -> Pos {
    Pos::new(line, col)
}

#[test]
fn a_pattern_finds_the_line_break_of_each_kind_of_document() {
    for (text, pat) in [
        ("ab\ncd\n", r"\n"),
        ("ab\r\ncd\r\n", r"\r\n"),
        ("ab\rcd\r", r"\r"),
        ("ab\r\ncd\r\n", r"[\r\n]+"),
    ] {
        // the match starts at the end of the text and ends at the start of the next line
        assert_eq!(
            forward(pat, text, p(0, 0)),
            Some((p(0, 2), p(1, 0))),
            "{text:?}"
        );
    }
}

#[test]
fn a_match_that_starts_inside_the_line_ending_starts_at_its_beginning() {
    // Sakura: 改行文字内でマッチした場合、改行文字の始めからマッチしたことにする
    assert_eq!(
        forward(r"\n", "ab\r\ncd", p(0, 0)),
        Some((p(0, 2), p(1, 0)))
    );
}

#[test]
fn replacing_only_the_lf_of_a_crlf_keeps_the_cr() {
    assert_eq!(all(r"\n", "X", "ab\r\ncd\r\n"), ("ab\rXcd\rX".into(), 2));
    // and only the CR keeps the LF (Command_REPLACE: 「\rを置換するとその後ろの\nが消えてしまう」)
    assert_eq!(all(r"\r", "X", "ab\r\ncd\r\n"), ("abX\ncdX\n".into(), 2));
}

#[test]
fn the_line_ending_is_replaced_and_the_lines_are_joined() {
    for (text, want) in [
        ("a\nb\nc\n", "a|b|c|"),
        ("a\r\nb\r\nc\r\n", "a|b|c|"),
        ("a\rb\rc\r", "a|b|c|"),
        ("a\nb\nc", "a|b|c"),
    ] {
        assert_eq!(all(r"\r\n|\n|\r", "|", text).0, want);
        assert_eq!(all(r"[\r\n]+", "|", text).0, want);
        assert_eq!(
            run(r"[\r\n]+", "|", text, ReplaceAllMode::Repeat, None).0,
            want
        );
    }
    // Replaced by nothing, the lines join into one and the search goes on in the joined line.
    assert_eq!(all(r"\n", "", "a\nb\nc\n"), ("abc".into(), 3));
    assert_eq!(all(r"\s+", "", "a b\nc d\n"), ("abcd".into(), 4));
    assert_eq!(
        run(r"\s+", "", "a b\nc d\n", ReplaceAllMode::Repeat, None),
        ("abcd".into(), 4)
    );
}

#[test]
fn replacing_a_line_break_with_itself_ends() {
    assert_eq!(all(r"\n", "\n", "a\nb\n"), ("a\nb\n".into(), 2));
    assert_eq!(
        run(r"\n", "\n", "a\nb\n", ReplaceAllMode::Repeat, None),
        ("a\nb\n".into(), 2)
    );
    assert_eq!(all(r"\r\n", "\n", "a\r\nb\r\n"), ("a\nb\n".into(), 2));
}

#[test]
fn a_negated_class_and_the_whitespace_classes_take_the_line_break() {
    assert_eq!(all("[^a-z]+", "_", "ab\ncd\n").0, "ab_cd_");
    assert_eq!(all(r"\W", "_", "ab\ncd\n").0, "ab_cd_");
    assert_eq!(all(r"\D", "_", "a\nb").0, "___");
    // `.` never takes the line break (MakePatternAlternate)
    assert_eq!(all(".+", "_", "ab\ncd\n").0, "_\n_\n");
}

#[test]
fn a_match_never_goes_on_into_the_next_line() {
    assert_eq!(forward(r"b\nc", "ab\ncd", p(0, 0)), None);
    assert_eq!(forward(r"\n\n", "a\n\nb", p(0, 0)), None);
    assert_eq!(all(r"\n\w", "_", "a\nb\nc").0, "a\nb\nc");
}

#[test]
fn anchors_see_the_line_ending() {
    // `$` is before the line break, `^` at the start of every line
    assert_eq!(all("$", "!", "a\nb\n").0, "a!\nb!\n");
    assert_eq!(all("^", "!", "a\nb\n").0, "!a\n!b\n");
    assert_eq!(all("^", "!", "a\r\nb\r\n").0, "!a\r\n!b\r\n");
    assert_eq!(all("$", "!", "a\r\nb\r\n").0, "a!\r\nb!\r\n");
    // `\Z` is before a final line break, `\z` only at the very end of the line with its ending
    assert_eq!(all(r"b\Z", "!", "a\nb\n").0, "a\n!\n");
    assert_eq!(all(r"b\z", "!", "a\nb\n").0, "a\nb\n");
    assert_eq!(all(r"b\z", "!", "a\nb").0, "a\n!");
}

#[test]
fn a_search_does_not_start_at_the_end_of_a_line_without_a_line_ending() {
    // CBregexp::Match finds nothing for nStart >= len
    assert_eq!(forward("$", "ab", p(0, 0)), None);
    assert_eq!(all("$", "!", "a\nb").0, "a!\nb");
    assert_eq!(forward("$", "ab", p(0, 2)), None);
    assert_eq!(forward("$", "ab\n", p(0, 2)), Some((p(0, 2), p(0, 2))));
    assert_eq!(forward("x*", "ab", p(0, 2)), None);
}

#[test]
fn empty_matches_at_the_end_of_a_line_take_one_replacement_each() {
    // Measured with the real Sakura (regex-empty-match-star): `x*` replaced by `-`: the empty
    // match at the end of the text and the one after the line ending are two
    assert_eq!(all("x*", "-", "ab\ncd\n").0, "-a-b-\n--c-d-\n-");
    assert_eq!(all("x*", "-", "ab").0, "-a-b-");
    assert_eq!(run("x*", "-", "ab", ReplaceAllMode::Repeat, None).0, "-a-b");
}

#[test]
fn backward_search_finds_the_last_line_break() {
    let text = "ab\ncd\nef\n";
    // from [EOF]: the line break of the last line
    assert_eq!(backward(r"\n", text, p(3, 0)), Some((p(2, 2), p(3, 0))));
    // a match that starts at the caret is not before it
    assert_eq!(backward(r"\n", text, p(1, 2)), Some((p(0, 2), p(1, 0))));
    assert_eq!(backward(r"\n", text, p(1, 1)), Some((p(0, 2), p(1, 0))));
    assert_eq!(backward(r"\n", text, p(0, 2)), None);
    assert_eq!(
        backward(r"\r\n", "a\r\nb\r\n", p(2, 0)),
        Some((p(1, 1), p(2, 0)))
    );
}

#[test]
fn replacing_in_a_selection_stops_at_its_end() {
    // the selection ends in the middle of line 1: the line break of line 0 is in, that of line 1 is out
    let r = run(
        r"\n",
        "|",
        "ab\ncd\nef\n",
        ReplaceAllMode::Batch,
        Some(p(0, 0)..p(1, 1)),
    );
    assert_eq!(r.0, "ab|cd\nef\n");
    // a selection that ends at column 0 leaves the line break that ends there out
    for mode in [ReplaceAllMode::Batch, ReplaceAllMode::Repeat] {
        let r = run(r"\n", "|", "ab\ncd\nef\n", mode, Some(p(0, 0)..p(2, 0)));
        assert_eq!(r.0, "ab|cd\nef\n", "{mode:?}");
    }
}

#[test]
fn find_all_reports_the_line_break_of_every_line() {
    let _l = LIMITS_LOCK.read().unwrap_or_else(|e| e.into_inner());
    let d = doc("a\n\nb");
    let m = RegexMatcher::new(r"\n", RegexSearchOptions::default()).unwrap();
    let found: Vec<_> = find_all(&d, &m, None, Control::new())
        .map(|f| f.unwrap().range())
        .collect();
    assert_eq!(found, vec![p(0, 1)..p(1, 0), p(1, 0)..p(2, 0)]);
}

#[test]
fn groups_and_the_whole_match_hold_the_line_break() {
    assert_eq!(all(r"(b)(\n)", "[$1$2]", "ab\nc").0, "a[b\n]c");
    assert_eq!(all(r"\n", "<$&>", "a\nb").0, "a<\n>b");
    assert_eq!(all(r"\n", "<$&>", "a\r\nb").0, "a\r<\n>b");
}

#[test]
fn matches_of_the_two_halves_of_a_crlf_are_replaced_together() {
    assert_eq!(all(r"[\r\n]", "X", "ab\r\ncd").0, "abXXcd");
    // the count is the number of matches, as Sakura's `nReplaceNum`
    let _l = LIMITS_LOCK.read().unwrap_or_else(|e| e.into_inner());
    let mut d = doc("ab\r\ncd");
    let r = replacer(r"[\r\n]", "X")
        .replace_all(&mut d, ReplaceAllMode::Batch, None, Control::new())
        .unwrap();
    assert_eq!((r.count, r.edits.len()), (2, 1));
    drop(_l);
    assert_eq!(all(r"[\r\n]", "X", "ab\r\ncd\r\n").0, "abXXcdXX");
    // The repeated replacement puts the rest of the line ending back with the text and goes on
    // after it (`Command_REPLACE`), so the LF that follows a replaced CR is not searched again.
    assert_eq!(
        run(r"[\r\n]", "X", "ab\r\ncd", ReplaceAllMode::Repeat, None).0,
        "abX\ncd"
    );
}

#[test]
fn an_empty_match_in_a_selection_that_ends_inside_a_line_ends() {
    // the replaced text is cut at the end of the selection, and is not searched again
    let r = run(
        "x*",
        "-",
        "abcd",
        ReplaceAllMode::Batch,
        Some(p(0, 0)..p(0, 2)),
    );
    assert_eq!(r.0, "-a-b-cd");
}
