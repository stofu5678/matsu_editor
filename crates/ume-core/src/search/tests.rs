//! Tests of the search. Expectations that come from Sakura name their source.

use std::sync::Arc;
use std::sync::Mutex;

use super::chars::Units;
use super::*;
use crate::buffer::{Document, Pos};

fn doc(text: &str) -> Document {
    Document::from_utf8(text.as_bytes().to_vec()).unwrap()
}

fn text_of(doc: &Document) -> String {
    doc.text_between(Pos::new(0, 0), doc.end_pos().unwrap())
        .unwrap()
}

fn plain(pattern: &str) -> PlainMatcher {
    PlainMatcher::new(pattern, PlainOptions::default())
}

fn cased(pattern: &str) -> PlainMatcher {
    PlainMatcher::new(
        pattern,
        PlainOptions {
            match_case: true,
            whole_word: false,
        },
    )
}

fn word(pattern: &str, match_case: bool) -> PlainMatcher {
    PlainMatcher::new(
        pattern,
        PlainOptions {
            match_case,
            whole_word: true,
        },
    )
}

fn next(doc: &Document, m: &dyn Matcher, line: usize, col: usize, wrap: bool) -> Option<Found> {
    find(
        doc,
        m,
        Pos::new(line, col),
        Direction::Forward,
        wrap,
        &mut Control::new(),
    )
    .unwrap()
}

fn prev(doc: &Document, m: &dyn Matcher, line: usize, col: usize, wrap: bool) -> Option<Found> {
    find(
        doc,
        m,
        Pos::new(line, col),
        Direction::Backward,
        wrap,
        &mut Control::new(),
    )
    .unwrap()
}

/// `replace_and_find_next` with both results unwrapped.
fn step(
    doc: &mut Document,
    m: &dyn Matcher,
    found: &std::ops::Range<Pos>,
    text: &str,
    target: ReplaceTarget,
    wrap: bool,
    ctl: &mut Control<'_>,
) -> Result<(AppliedEdit, Option<Found>), SearchError> {
    let s = replace_and_find_next(doc, m, found, text, target, wrap, ctl)?;
    Ok((s.edit, s.next?))
}

fn all(doc: &Document, m: &dyn Matcher) -> Vec<(Pos, Pos)> {
    find_all(doc, m, None, Control::new())
        .map(|f| f.unwrap())
        .map(|f| (f.start, f.end))
        .collect()
}

fn span(a: (usize, usize), b: (usize, usize)) -> (Pos, Pos) {
    (Pos::new(a.0, a.1), Pos::new(b.0, b.1))
}

fn at(f: &Option<Found>) -> Option<((usize, usize), (usize, usize))> {
    f.as_ref()
        .map(|f| ((f.start.line, f.start.col), (f.end.line, f.end.col)))
}

// ---- case folding ----

#[test]
fn fold_covers_what_towlower_covers() {
    // skr_towlower is towlower on the BMP (sakura_core/util/string_ex.cpp).
    assert_eq!(fold_char('A'), 'a');
    assert_eq!(fold_char('À'), 'à'); // Latin-1
    assert_eq!(fold_char('Ž'), 'ž'); // Latin Extended-A
    assert_eq!(fold_char('Σ'), 'σ'); // Greek
    assert_eq!(fold_char('Я'), 'я'); // Cyrillic
    assert_eq!(fold_char('Ａ'), 'ａ'); // full-width Latin folds with its own width
    assert_eq!(fold_char('あ'), 'あ');
    assert_eq!(fold_char('ß'), 'ß');
}

#[test]
fn fold_does_not_join_widths_or_touch_outside_the_bmp() {
    assert_ne!(fold_char('Ａ'), fold_char('a'));
    // "合成文字,サロゲートの「大文字小文字同一視」未対応" (CSearchAgent.cpp)
    assert_eq!(fold_char('\u{10400}'), '\u{10400}'); // Deseret capital long I
    // One unit to one unit only: the lower case of İ is two chars.
    assert_eq!(fold_char('İ'), 'İ');
}

#[test]
fn plain_search_folds_by_default() {
    let d = doc("Hello, World\n");
    assert_eq!(
        at(&next(&d, &plain("world"), 0, 0, false)),
        Some(((0, 7), (0, 12)))
    );
    assert_eq!(
        at(&next(&d, &plain("WORLD"), 0, 0, false)),
        Some(((0, 7), (0, 12)))
    );
    assert_eq!(next(&d, &cased("world"), 0, 0, false), None);
    assert_eq!(
        at(&next(&d, &cased("World"), 0, 0, false)),
        Some(((0, 7), (0, 12)))
    );
}

#[test]
fn full_width_letters_fold_but_do_not_equal_half_width() {
    let d = doc("ＡＢＣ abc\n");
    // Ａ (3 bytes) folds to ａ, so "ａｂｃ" finds ＡＢＣ.
    assert_eq!(
        at(&next(&d, &plain("ａｂｃ"), 0, 0, false)),
        Some(((0, 0), (0, 9)))
    );
    assert_eq!(next(&d, &cased("ａｂｃ"), 0, 0, false), None);
    assert_eq!(
        at(&next(&d, &cased("ＡＢＣ"), 0, 0, false)),
        Some(((0, 0), (0, 9)))
    );
    // Half-width "abc" does not find the full-width letters.
    assert_eq!(
        at(&next(&d, &plain("abc"), 0, 0, false)),
        Some(((0, 10), (0, 13)))
    );
}

#[test]
fn fold_that_changes_the_byte_length_reports_the_original_range() {
    // Ⱥ is 2 bytes, its lower case ⱥ (U+2C65) is 3 bytes.
    let d = doc("xȺx\n");
    assert_eq!(
        at(&next(&d, &plain("ⱥ"), 0, 0, false)),
        Some(((0, 1), (0, 3)))
    );
}

// ---- plain search in a document ----

#[test]
fn finds_japanese_text() {
    let d = doc("あいうえお\nかきくけこ\n");
    assert_eq!(
        at(&next(&d, &plain("うえ"), 0, 0, false)),
        Some(((0, 6), (0, 12)))
    );
    assert_eq!(
        at(&next(&d, &plain("くけ"), 0, 0, false)),
        Some(((1, 6), (1, 12)))
    );
    // from inside the text: the match at the caret counts
    assert_eq!(
        at(&next(&d, &plain("うえ"), 0, 6, false)),
        Some(((0, 6), (0, 12)))
    );
    assert_eq!(next(&d, &plain("うえ"), 0, 7, false), None);
}

#[test]
fn matches_at_line_starts_and_ends() {
    let d = doc("abc\nxabc\nabcx\nabc");
    assert_eq!(
        all(&d, &plain("abc")),
        vec![
            span((0, 0), (0, 3)),
            span((1, 1), (1, 4)),
            span((2, 0), (2, 3)),
            span((3, 0), (3, 3)),
        ]
    );
}

#[test]
fn crlf_documents_are_searched_without_the_line_ending() {
    let d = doc("ab\r\nab\r\n\r\nab");
    assert_eq!(
        all(&d, &plain("ab")),
        vec![
            span((0, 0), (0, 2)),
            span((1, 0), (1, 2)),
            span((3, 0), (3, 2))
        ]
    );
    // A match never contains the line ending, and the CR is not a column.
    assert_eq!(next(&d, &plain("b\r"), 0, 0, false), None);
}

#[test]
fn matches_do_not_span_lines() {
    let d = doc("ab\ncd\n");
    assert_eq!(next(&d, &plain("bc"), 0, 0, false), None);
}

#[test]
fn empty_pattern_never_matches() {
    // CSearchAgent::SearchString returns null for an empty pattern.
    let d = doc("abc\n");
    assert_eq!(next(&d, &plain(""), 0, 0, false), None);
    assert_eq!(prev(&d, &plain(""), 0, 3, false), None);
    assert_eq!(next(&d, &word("  ", true), 0, 0, false), None);
}

#[test]
fn position_past_the_end_finds_nothing_forward() {
    let d = doc("abc\n");
    assert_eq!(next(&d, &plain("abc"), 5, 0, false), None);
    assert_eq!(next(&d, &plain("abc"), 0, 99, false), None);
    assert_eq!(next(&Document::empty(), &plain("a"), 0, 0, true), None);
}

#[test]
fn position_inside_a_char_rounds_forward() {
    let d = doc("あい\n");
    assert_eq!(
        at(&next(&d, &plain("い"), 0, 1, false)),
        Some(((0, 3), (0, 6)))
    );
}

// ---- next, previous, wrap-around ----

#[test]
fn wrap_around_forward() {
    let d = doc("foo\nbar\nfoo\nbaz\n");
    let m = plain("foo");
    let f = next(&d, &m, 2, 1, true).unwrap();
    assert_eq!((f.start, f.wrapped), (Pos::new(0, 0), true));
    assert_eq!(next(&d, &m, 2, 1, false), None);
    let f = next(&d, &m, 0, 1, true).unwrap();
    assert_eq!((f.start, f.wrapped), (Pos::new(2, 0), false));
}

#[test]
fn wrap_around_backward() {
    let d = doc("foo\nbar\nfoo\nbaz\n");
    let m = plain("foo");
    let f = prev(&d, &m, 0, 0, true).unwrap();
    assert_eq!((f.start, f.wrapped), (Pos::new(2, 0), true));
    assert_eq!(prev(&d, &m, 0, 0, false), None);
    let f = prev(&d, &m, 2, 0, true).unwrap();
    assert_eq!((f.start, f.wrapped), (Pos::new(0, 0), false));
}

#[test]
fn wrap_finds_a_match_before_the_caret_in_the_same_line() {
    let d = doc("foo bar\n");
    let f = next(&d, &plain("foo"), 0, 4, true).unwrap();
    assert_eq!((f.start, f.wrapped), (Pos::new(0, 0), true));
    let f = prev(&d, &plain("bar"), 0, 4, true).unwrap();
    assert_eq!((f.start, f.wrapped), (Pos::new(0, 4), true));
}

#[test]
fn nothing_found_with_wrap_is_none() {
    let d = doc("foo\nbar\n");
    assert_eq!(next(&d, &plain("zzz"), 1, 1, true), None);
    assert_eq!(prev(&d, &plain("zzz"), 1, 1, true), None);
}

#[test]
fn backward_from_the_end_of_the_document() {
    // Sakura searches back from [EOF] by moving to the last line with the column at its end
    // (Command_SEARCH_PREV, "pcLayoutはNULLとなるのは、[EOF]から前検索した場合").
    let d = doc("foo\nbar foo\n");
    assert_eq!(
        at(&prev(&d, &plain("foo"), 2, 0, false)),
        Some(((1, 4), (1, 7)))
    );
    assert_eq!(
        at(&prev(&d, &plain("foo"), 99, 99, false)),
        Some(((1, 4), (1, 7)))
    );
}

#[test]
fn backward_needs_the_match_to_begin_before_the_caret() {
    let d = doc("foofoo\n");
    assert_eq!(
        at(&prev(&d, &plain("foo"), 0, 3, false)),
        Some(((0, 0), (0, 3)))
    );
    assert_eq!(
        at(&prev(&d, &plain("foo"), 0, 4, false)),
        Some(((0, 3), (0, 6)))
    );
    assert_eq!(prev(&d, &plain("foo"), 0, 0, false), None);
}

#[test]
fn backward_scans_forward_without_overlap() {
    // CSearchAgent::SearchWord: backward search steps through the line with nIdxPos = hit + len,
    // so overlapping matches are not seen. "aaa" holds "aa" at 0 and 1, but only 0 is found.
    let d = doc("aaa\n");
    assert_eq!(
        at(&prev(&d, &plain("aa"), 0, 3, false)),
        Some(((0, 0), (0, 2)))
    );
    // and forward from 1 does find the one at 1
    assert_eq!(
        at(&next(&d, &plain("aa"), 0, 1, false)),
        Some(((0, 1), (0, 3)))
    );
}

#[test]
fn next_and_previous_are_symmetric() {
    let d = doc("ab ab\nxab\n\nab ab ab\n");
    let m = plain("ab");
    let mut forward = Vec::new();
    let mut pos = Pos::new(0, 0);
    while let Some(f) = find(&d, &m, pos, Direction::Forward, false, &mut Control::new()).unwrap() {
        forward.push((f.start, f.end));
        pos = f.end;
    }
    let mut backward = Vec::new();
    let mut pos = Pos::new(99, 0);
    while let Some(f) = find(&d, &m, pos, Direction::Backward, false, &mut Control::new()).unwrap()
    {
        backward.push((f.start, f.end));
        pos = f.start;
    }
    backward.reverse();
    assert_eq!(forward.len(), 6);
    assert_eq!(forward, backward);
    assert_eq!(forward, all(&d, &m));
}

#[test]
fn start_after_moves_past_an_empty_match() {
    // Command_SEARCH_NEXT: "現在、長さ０でマッチしている場合は１文字進める(無限マッチ対策)"
    let d = doc("あb\n\n");
    let p = |l, c| Pos::new(l, c);
    assert_eq!(start_after(&d, &(p(0, 0)..p(0, 3))).unwrap(), p(0, 3));
    assert_eq!(start_after(&d, &(p(0, 0)..p(0, 0))).unwrap(), p(0, 3));
    assert_eq!(start_after(&d, &(p(0, 3)..p(0, 3))).unwrap(), p(0, 4));
    assert_eq!(start_after(&d, &(p(0, 4)..p(0, 4))).unwrap(), p(1, 0));
}

// ---- whole words ----

#[test]
fn whole_word_needs_the_word_boundaries() {
    let d = doc("foo foobar Foo _foo\n");
    let hits = |m: &PlainMatcher| all(&d, m);
    assert_eq!(
        hits(&word("foo", false)),
        vec![span((0, 0), (0, 3)), span((0, 11), (0, 14))]
    );
    assert_eq!(hits(&word("foo", true)), vec![span((0, 0), (0, 3))]);
    // without the option, every occurrence counts
    assert_eq!(hits(&plain("foo")).len(), 4);
}

#[test]
fn whole_word_searches_for_the_words_of_the_pattern() {
    // CreateWordList splits the pattern into words; any one of them is a hit.
    let d = doc("a,b\n");
    let hits = all(&d, &word("x,y", true));
    assert_eq!(hits, vec![span((0, 1), (0, 2))]);
    let hits = all(&d, &word("a b", true));
    assert_eq!(hits, vec![span((0, 0), (0, 1)), span((0, 2), (0, 3))]);
}

#[test]
fn whole_word_in_japanese() {
    // Words are runs of one kind of char (CWordParse): kanji, hiragana, katakana, ...
    let d = doc("漢字かなカタカナ交じり\n");
    assert_eq!(
        at(&next(&d, &word("かな", true), 0, 0, false)),
        Some(((0, 6), (0, 12)))
    );
    assert_eq!(
        at(&next(&d, &word("カタカナ", true), 0, 0, false)),
        Some(((0, 12), (0, 24)))
    );
    assert_eq!(next(&d, &word("漢", true), 0, 0, false), None);
    assert_eq!(
        at(&next(&d, &word("漢字", true), 0, 0, false)),
        Some(((0, 0), (0, 6)))
    );
    // 交 (kanji) and じり (hiragana) are different words
    assert_eq!(
        at(&next(&d, &word("交", true), 0, 0, false)),
        Some(((0, 24), (0, 27)))
    );
}

#[test]
fn whole_word_prolonged_sound_mark_sticks_to_kana() {
    // WhatKindOfTwoChars: ー joins the hiragana/katakana next to it.
    let d = doc("コーヒー ラーメン\n");
    assert_eq!(
        at(&next(&d, &word("コーヒー", true), 0, 0, false)),
        Some(((0, 0), (0, 12)))
    );
    assert_eq!(
        at(&next(&d, &word("ラーメン", true), 0, 0, false)),
        Some(((0, 13), (0, 25)))
    );
}

#[test]
fn whole_word_folds_full_width_letters() {
    let d = doc("ＡＢＣ　ａｂｃ\n");
    assert_eq!(
        at(&next(&d, &word("ａｂｃ", false), 0, 0, false)),
        Some(((0, 0), (0, 9)))
    );
    assert_eq!(
        at(&next(&d, &word("ａｂｃ", true), 0, 0, false)),
        Some(((0, 12), (0, 21)))
    );
}

#[test]
fn whole_word_from_inside_a_word_finds_that_word() {
    // SearchStringWord asks for the word that holds the start column, which may begin before it.
    let d = doc("foobar baz\n");
    assert_eq!(
        at(&next(&d, &word("foobar", true), 0, 3, false)),
        Some(((0, 0), (0, 6)))
    );
    assert_eq!(
        at(&next(&d, &word("baz", true), 0, 3, false)),
        Some(((0, 7), (0, 10)))
    );
}

#[test]
fn whole_word_backward() {
    let d = doc("foo bar foo\nfoo\n");
    let m = word("foo", true);
    assert_eq!(at(&prev(&d, &m, 0, 11, false)), Some(((0, 8), (0, 11))));
    assert_eq!(at(&prev(&d, &m, 0, 8, false)), Some(((0, 0), (0, 3))));
    assert_eq!(prev(&d, &m, 0, 0, false), None);
    assert_eq!(at(&prev(&d, &m, 1, 0, false)), Some(((0, 8), (0, 11))));
    // from the middle of a word: the word counts, since it begins before the caret
    assert_eq!(at(&prev(&d, &m, 0, 10, false)), Some(((0, 8), (0, 11))));
}

#[test]
fn whole_word_from_the_end_of_the_document_sees_the_last_line() {
    // Sakura gives up on the last line here (the caret column is past its end); we do not.
    let d = doc("foo\nbar\n");
    let f = prev(&d, &word("bar", true), 99, 0, false);
    assert_eq!(at(&f), Some(((1, 0), (1, 3))));
}

#[test]
fn word_boundaries_match_sakura_tests() {
    // src/test/cpp/tests1/test-cwordparse.cpp
    // SearchNextWordPosition.StopsAtIncompatibleKinds: "sakura!" -> 6
    assert_eq!(Units::new("sakura!").next_word(0), Some(6));
    // SearchNextWordPosition.RespectsBooleanArgument: "sakura editor" -> 7 (spaces are skipped)
    assert_eq!(Units::new("sakura editor").next_word(0), Some(7));
    // SearchPrevWordPosition.RespectsBooleanArgument: " sakura !" from 8 -> 1
    assert_eq!(Units::new(" sakura !").prev_word(8), Some(1));
    // SearchPrevWordPosition.ReturnsFalseWhenIndexIsZero
    assert_eq!(Units::new("").prev_word(0), None);
    // WhereCurrentWord_2.ReturnsCMemoryIfSpecified: "sakura editor" at 10 -> 7..13
    assert_eq!(Units::new("sakura editor").current_word(10), Some(7..13));
    // WhereCurrentWord_2.ReturnsFalseIfIndexIsAtNewLineOrEOS
    assert_eq!(Units::new("sakura").current_word(6), None);
}

#[test]
fn word_boundaries_with_variation_selectors() {
    // CNativeW::GetSizeOfChar counts a char and its variation selector as one; the pair is a
    // wide char (test-cwordparse.cpp, WhatKindOfChar.IVS).
    let line = "a葛\u{E0100}b";
    let u = Units::new(line);
    assert_eq!(u.len(), 3);
    assert_eq!(u.byte(2), line.len() - 1);
    assert_eq!(u.current_word(1), Some(1..2));
}

// ---- matcher trait ----

/// A matcher that finds an empty match at every position, and a captured group.
#[derive(Debug)]
struct EmptyEverywhere;

impl Matcher for EmptyEverywhere {
    fn find_at(&self, line: &str, start: usize) -> Result<Option<Match>, MatcherError> {
        Ok((start <= line.len()).then(|| Match {
            start,
            end: start,
            captures: vec![Some(start..start), None],
        }))
    }
}

#[test]
fn empty_matches_advance_one_char() {
    let d = doc("aあ\n");
    let hits = all(&d, &EmptyEverywhere);
    assert_eq!(
        hits,
        vec![
            span((0, 0), (0, 0)),
            span((0, 1), (0, 1)),
            span((0, 4), (0, 4))
        ]
    );
    let first = find_all(&d, &EmptyEverywhere, None, Control::new())
        .next()
        .unwrap()
        .unwrap();
    assert_eq!(first.captures, vec![Some(0..0), None]);
    // backward: the last empty match before the caret
    assert_eq!(
        at(&prev(&d, &EmptyEverywhere, 0, 4, false)),
        Some(((0, 1), (0, 1)))
    );
    assert_eq!(at(&prev(&d, &EmptyEverywhere, 0, 0, false)), None);
    // an empty line holds one empty match
    let d = doc("\n");
    assert_eq!(all(&d, &EmptyEverywhere), vec![span((0, 0), (0, 0))]);
}

#[test]
fn matcher_errors_stop_the_search() {
    #[derive(Debug)]
    struct Failing;
    impl Matcher for Failing {
        fn find_at(&self, _: &str, _: usize) -> Result<Option<Match>, MatcherError> {
            Err(MatcherError::new("stack limit"))
        }
    }
    let d = doc("a\n");
    let r = find(
        &d,
        &Failing,
        Pos::new(0, 0),
        Direction::Forward,
        false,
        &mut Control::new(),
    );
    assert!(matches!(r, Err(SearchError::Matcher(ref e)) if e.message() == "stack limit"));
    let mut it = find_all(&d, &Failing, None, Control::new());
    assert!(matches!(it.next(), Some(Err(SearchError::Matcher(_)))));
    assert!(it.next().is_none());
}

// ---- find_all ----

#[test]
fn find_all_does_not_overlap() {
    let d = doc("aaaa\naaa\n");
    assert_eq!(
        all(&d, &plain("aa")),
        vec![
            span((0, 0), (0, 2)),
            span((0, 2), (0, 4)),
            span((1, 0), (1, 2))
        ]
    );
}

#[test]
fn find_all_in_a_range() {
    let d = doc("ab ab\nab ab\nab ab\n");
    let m = plain("ab");
    let run = |range| {
        find_all(&d, &m, Some(range), Control::new())
            .map(|f| f.unwrap().start)
            .collect::<Vec<_>>()
    };
    let p = Pos::new;
    // starts in the middle, ends in the middle: a match must lie inside
    assert_eq!(run(p(0, 3)..p(1, 4)), vec![p(0, 3), p(1, 0)]);
    assert_eq!(run(p(0, 3)..p(1, 5)), vec![p(0, 3), p(1, 0), p(1, 3)]);
    // Command_REPLACE_ALL: a selection that ends at column 0 leaves that line out
    assert_eq!(run(p(0, 0)..p(1, 0)), vec![p(0, 0), p(0, 3)]);
    assert_eq!(run(p(1, 0)..p(0, 0)), Vec::<Pos>::new());
}

// ---- cancellation and progress ----

fn numbered(lines: usize) -> Document {
    let mut s = String::with_capacity(lines * 12);
    for i in 0..lines {
        s.push_str("line ");
        s.push_str(&i.to_string());
        s.push('\n');
    }
    doc(&s)
}

#[test]
fn a_cancelled_token_stops_the_search() {
    let d = numbered(100);
    let token = CancelToken::new();
    token.cancel();
    let r = find(
        &d,
        &plain("zzz"),
        Pos::new(0, 0),
        Direction::Forward,
        true,
        &mut Control::new().with_cancel(token),
    );
    assert!(matches!(r, Err(SearchError::Cancelled)));
}

#[test]
fn cancelling_between_lines() {
    let d = numbered(1000);
    let token = CancelToken::new();
    let reports = Arc::new(Mutex::new(Vec::new()));
    let (t, r) = (token.clone(), Arc::clone(&reports));
    let mut ctl = Control::new()
        .with_cancel(token)
        .with_interval(100)
        .with_progress(move |p| {
            r.lock().unwrap().push(p);
            if p.lines_done >= 300 {
                t.cancel();
            }
        });
    let r = find(
        &d,
        &plain("zzz"),
        Pos::new(0, 0),
        Direction::Forward,
        false,
        &mut ctl,
    );
    assert!(matches!(r, Err(SearchError::Cancelled)));
    let reports = reports.lock().unwrap();
    assert_eq!(
        reports.iter().map(|p| p.lines_done).collect::<Vec<_>>(),
        [100, 200, 300]
    );
    assert!(reports.iter().all(|p| p.lines_total == 1000));
}

#[test]
fn progress_ends_with_the_whole_document() {
    let d = numbered(250);
    let reports = Arc::new(Mutex::new(Vec::new()));
    let r = Arc::clone(&reports);
    let mut ctl = Control::new()
        .with_interval(100)
        .with_progress(move |p| r.lock().unwrap().push(p));
    let found = find(
        &d,
        &plain("zzz"),
        Pos::new(0, 0),
        Direction::Backward,
        true,
        &mut ctl,
    )
    .unwrap();
    assert!(found.is_none());
    let reports = reports.lock().unwrap();
    // with wrap-around the line of the caret is read twice: once from the caret, once up to it
    assert_eq!(
        reports.last().copied(),
        Some(Progress {
            lines_done: 251,
            lines_total: 251
        })
    );
    assert!(
        reports
            .windows(2)
            .all(|w| w[0].lines_done <= w[1].lines_done)
    );
}

#[test]
fn find_all_can_be_cancelled() {
    let d = numbered(100);
    let token = CancelToken::new();
    let m = plain("line");
    let mut it = find_all(&d, &m, None, Control::new().with_cancel(token.clone()));
    assert!(it.next().unwrap().is_ok());
    token.cancel();
    assert!(matches!(it.next(), Some(Err(SearchError::Cancelled))));
    assert!(it.next().is_none());
}

#[test]
fn a_search_runs_on_another_thread_on_a_clone() {
    let mut d = numbered(5000);
    let clone = d.clone();
    let m: Arc<dyn Matcher> = Arc::new(plain("line 4999"));
    let job = SearchJob::spawn(move |ctl| {
        find(&clone, &*m, Pos::new(0, 0), Direction::Forward, false, ctl)
    });
    // editing meanwhile does not disturb the search
    d.insert(Pos::new(0, 0), "line 4999\n").unwrap();
    let found = job.join().unwrap().unwrap();
    assert_eq!(found.start, Pos::new(4999, 0));
}

#[test]
fn a_job_can_be_cancelled() {
    let d = numbered(2000);
    let m: Arc<dyn Matcher> = Arc::new(plain("zzz"));
    let job = SearchJob::spawn(move |ctl| {
        // Wait until the caller has cancelled, so that the outcome does not depend on timing.
        while !ctl_cancelled(ctl) {
            std::thread::yield_now();
        }
        find(&d, &*m, Pos::new(0, 0), Direction::Forward, true, ctl)
    });
    job.cancel();
    assert!(matches!(job.join(), Err(SearchError::Cancelled)));
}

fn ctl_cancelled(ctl: &mut Control<'_>) -> bool {
    // `Control::step` is private; a one-line search on a scratch document reads the flag.
    let d = doc("x\n");
    matches!(
        find(
            &d,
            &plain("x"),
            Pos::new(0, 0),
            Direction::Forward,
            false,
            ctl
        ),
        Err(SearchError::Cancelled)
    )
}

#[test]
fn a_job_reports_progress() {
    let d = numbered(30_000);
    let m: Arc<dyn Matcher> = Arc::new(plain("zzz"));
    let job =
        SearchJob::spawn(move |ctl| find(&d, &*m, Pos::new(0, 0), Direction::Forward, false, ctl));
    while !job.is_finished() {
        std::thread::yield_now();
    }
    let p = job.progress();
    assert_eq!(
        p,
        Progress {
            lines_done: 30_000,
            lines_total: 30_000
        }
    );
    assert!(job.join().unwrap().is_none());
}

// ---- replace ----

#[test]
fn replace_all_counts_and_does_not_search_the_new_text() {
    let mut d = doc("a b a\na\n");
    let r = replace_all(
        &mut d,
        &plain("a"),
        "aa",
        ReplaceTarget::Match,
        None,
        Control::new(),
    )
    .unwrap();
    assert_eq!(text_of(&d), "aa b aa\naa\n");
    assert_eq!(r.edits.len(), 3);
    // the caret goes to the end of the last replacement
    assert_eq!(r.caret, Some(Pos::new(1, 2)));
}

#[test]
fn replace_all_with_nothing_found_changes_nothing() {
    let mut d = doc("abc\n");
    let r = replace_all(
        &mut d,
        &plain("x"),
        "y",
        ReplaceTarget::Match,
        None,
        Control::new(),
    )
    .unwrap();
    assert_eq!(r, ReplaceAllResult::default());
    assert_eq!(text_of(&d), "abc\n");
}

#[test]
fn replace_all_with_line_breaks_in_the_text() {
    let mut d = doc("a-a\nxa\n");
    let r = replace_all(
        &mut d,
        &plain("a"),
        "1\n2",
        ReplaceTarget::Match,
        None,
        Control::new(),
    )
    .unwrap();
    assert_eq!(text_of(&d), "1\n2-1\n2\nx1\n2\n");
    assert_eq!(r.edits.len(), 3);
    assert_eq!(r.caret, Some(Pos::new(4, 1)));
    // the positions of the edits are in the document as it is after the ones before
    assert_eq!(r.edits[1].start, Pos::new(1, 2));
    assert_eq!(r.edits[1].end, Pos::new(2, 1));
}

#[test]
fn replace_all_can_be_undone_from_the_edits() {
    let original = "foo bar foo\nfoo\n\nbaz foo";
    let mut d = doc(original);
    let r = replace_all(
        &mut d,
        &plain("foo"),
        "[x\ny]",
        ReplaceTarget::Match,
        None,
        Control::new(),
    )
    .unwrap();
    assert_ne!(text_of(&d), original);
    for e in r.edits.iter().rev() {
        d.replace(e.start, e.end, &e.removed).unwrap();
    }
    assert_eq!(text_of(&d), original);
}

#[test]
fn replace_all_in_a_selection() {
    let mut d = doc("a a a\na a a\na a a\n");
    let p = Pos::new;
    let r = replace_all(
        &mut d,
        &plain("a"),
        "bb",
        ReplaceTarget::Match,
        Some(p(0, 2)..p(1, 3)),
        Control::new(),
    )
    .unwrap();
    assert_eq!(text_of(&d), "a bb bb\nbb bb a\na a a\n");
    assert_eq!(r.edits.len(), 4);
}

#[test]
fn replace_all_is_not_applied_when_cancelled() {
    let mut d = numbered(100);
    let before = text_of(&d);
    let token = CancelToken::new();
    token.cancel();
    let r = replace_all(
        &mut d,
        &plain("line"),
        "x",
        ReplaceTarget::Match,
        None,
        Control::new().with_cancel(token),
    );
    assert!(matches!(r, Err(SearchError::Cancelled)));
    assert_eq!(text_of(&d), before);
}

#[test]
fn replace_targets() {
    let run = |target: ReplaceTarget, text: &str| {
        let mut d = doc("xax\nbb\nxax a\nend a");
        let r = replace_all(&mut d, &plain("a"), text, target, None, Control::new()).unwrap();
        (text_of(&d), r.edits.len(), r.caret)
    };
    // 選択始点に挿入 / 選択終点に挿入
    assert_eq!(
        run(ReplaceTarget::InsertBefore, "<").0,
        "x<ax\nbb\nx<ax <a\nend <a"
    );
    assert_eq!(
        run(ReplaceTarget::InsertAfter, ">").0,
        "xa>x\nbb\nxa>x a>\nend a>"
    );
    // 行削除: a line goes once even if it holds two matches
    let (t, n, caret) = run(ReplaceTarget::DeleteLine, "ignored");
    assert_eq!(t, "bb\n");
    assert_eq!(n, 3);
    assert_eq!(caret, Some(Pos::new(1, 0)));
}

#[test]
fn delete_line_of_the_last_line_without_a_line_ending() {
    let mut d = doc("keep\nlast a");
    replace_all(
        &mut d,
        &plain("a"),
        "",
        ReplaceTarget::DeleteLine,
        None,
        Control::new(),
    )
    .unwrap();
    // Sakura selects up to the end of the last line (the EOF hangs on the last data line)
    assert_eq!(text_of(&d), "keep\n");
}

#[test]
fn insert_before_resumes_after_the_kept_match() {
    // Command_REPLACE: for 選択始点に挿入 the caret moves on by the length of the match
    let mut d = doc("aa\n");
    let found = Pos::new(0, 0)..Pos::new(0, 1);
    let e = replace_one(&mut d, &found, "<<", ReplaceTarget::InsertBefore).unwrap();
    assert_eq!(text_of(&d), "<<aa\n");
    assert_eq!(e.end, Pos::new(0, 2));
    assert_eq!(e.resume, Pos::new(0, 3));
}

#[test]
fn replace_and_find_next_goes_on_after_the_new_text() {
    let mut d = doc("a a a\n");
    let m = plain("a");
    let mut ctl = Control::new();
    let first = next(&d, &m, 0, 0, false).unwrap();
    let (edit, found) = step(
        &mut d,
        &m,
        &first.range(),
        "aaa",
        ReplaceTarget::Match,
        false,
        &mut ctl,
    )
    .unwrap();
    assert_eq!(text_of(&d), "aaa a a\n");
    assert_eq!(edit.resume, Pos::new(0, 3));
    // the "a"s of the new text are not found: the next hit is the second original "a"
    assert_eq!(found.unwrap().start, Pos::new(0, 4));
    let last = Pos::new(0, 6)..Pos::new(0, 7);
    let (_, found) = step(
        &mut d,
        &m,
        &last,
        "b",
        ReplaceTarget::Match,
        false,
        &mut ctl,
    )
    .unwrap();
    assert!(found.is_none());
    let (_, found) = step(
        &mut d,
        &m,
        &(Pos::new(0, 4)..Pos::new(0, 5)),
        "b",
        ReplaceTarget::Match,
        true,
        &mut ctl,
    )
    .unwrap();
    // wrap-around finds the first "a" of the new text
    let found = found.unwrap();
    assert_eq!((found.start, found.wrapped), (Pos::new(0, 0), true));
}

// ---- size ----

#[test]
fn a_missing_word_in_a_large_document() {
    let d = numbered(30_000);
    let started = std::time::Instant::now();
    assert!(next(&d, &plain("missing"), 0, 0, true).is_none());
    assert!(next(&d, &word("missing", false), 0, 0, true).is_none());
    // Debug builds are slow; this only guards against something quadratic.
    assert!(started.elapsed() < std::time::Duration::from_secs(60));
}

/// `cargo test -p ume-core --release -- --ignored --nocapture search_100mb`
#[test]
#[ignore = "builds a 100 MB document and prints timings"]
fn search_100mb() {
    let line = "The quick brown fox jumps over the lazy dog, again and again. 日本語のテキストもあります。\n";
    let target = 100 * 1024 * 1024;
    let mut s = String::with_capacity(target + line.len());
    while s.len() < target {
        s.push_str(line);
    }
    let built = std::time::Instant::now();
    let d = doc(&s);
    println!("build: {:?} ({} lines)", built.elapsed(), d.line_count());
    let time = |name: &str, m: &dyn Matcher, wrap: bool| {
        let t = std::time::Instant::now();
        let r = next(&d, m, 0, 0, wrap);
        println!("{name}: {:?} found={}", t.elapsed(), r.is_some());
    };
    time(
        "plain, case-sensitive, missing",
        &cased("missingword"),
        true,
    );
    time("plain, ignore case, missing", &plain("missingword"), true);
    time(
        "plain, ignore case, non-ascii, missing",
        &plain("存在しない"),
        true,
    );
    time("whole word, missing", &word("missingword", false), true);
    let t = std::time::Instant::now();
    let n = find_all(&d, &plain("lazy"), None, Control::new()).count();
    println!("find_all lazy: {:?} n={n}", t.elapsed());
}

// ---- found by review ----

#[test]
fn find_all_in_a_range_leaves_out_a_word_that_begins_before_it() {
    let d = doc("foobar baz\n");
    let p = Pos::new;
    let m = word("foobar", true);
    let run = |range| find_all(&d, &m, Some(range), Control::new()).count();
    assert_eq!(run(p(0, 3)..p(0, 10)), 0);
    assert_eq!(run(p(0, 0)..p(0, 10)), 1);
}

#[test]
fn the_matcher_gets_a_char_boundary() {
    #[derive(Debug)]
    struct Slicing;
    impl Matcher for Slicing {
        fn find_at(&self, line: &str, start: usize) -> Result<Option<Match>, MatcherError> {
            let rest = &line[start..]; // panics off a boundary
            Ok(rest
                .find('い')
                .map(|i| Match::new(start + i, start + i + 3)))
        }
    }
    let d = doc("あい\n");
    assert_eq!(at(&next(&d, &Slicing, 0, 1, false)), Some(((0, 3), (0, 6))));
}

#[test]
fn replace_and_find_next_steps_over_an_empty_match() {
    let mut d = doc("ab\n");
    let mut ctl = Control::new();
    let found = Pos::new(0, 0)..Pos::new(0, 0);
    let (edit, next) = step(
        &mut d,
        &EmptyEverywhere,
        &found,
        "-",
        ReplaceTarget::Match,
        false,
        &mut ctl,
    )
    .unwrap();
    assert_eq!(text_of(&d), "-ab\n");
    assert_eq!(edit.resume, Pos::new(0, 1));
    // not the same place again: one char on
    assert_eq!(next.unwrap().start, Pos::new(0, 2));
}

#[test]
fn replace_all_with_empty_matches_inserts_between_chars() {
    let mut d = doc("ab\n");
    let r = replace_all(
        &mut d,
        &EmptyEverywhere,
        "-",
        ReplaceTarget::Match,
        None,
        Control::new(),
    )
    .unwrap();
    assert_eq!(text_of(&d), "-a-b-\n");
    assert_eq!(r.edits.len(), 3);
}

#[test]
fn a_cr_inserted_before_an_lf_joins_it_and_does_not_undo_by_position() {
    // The limit that `AppliedEdit::removed` documents.
    let mut d = doc("a\nb");
    let found = Pos::new(0, 0)..Pos::new(0, 1);
    let e = replace_one(&mut d, &found, "x\r", ReplaceTarget::Match).unwrap();
    assert_eq!(text_of(&d), "x\r\nb");
    d.replace(e.start, e.end, &e.removed).unwrap();
    assert_ne!(text_of(&d), "a\nb");
}

#[test]
fn replace_and_find_next_keeps_the_edit_when_the_search_is_cancelled() {
    let mut d = doc("a a\n");
    let token = CancelToken::new();
    token.cancel();
    let mut ctl = Control::new().with_cancel(token);
    let found = Pos::new(0, 0)..Pos::new(0, 1);
    let s = replace_and_find_next(
        &mut d,
        &plain("a"),
        &found,
        "b",
        ReplaceTarget::Match,
        false,
        &mut ctl,
    )
    .unwrap();
    assert_eq!(text_of(&d), "b a\n");
    assert_eq!(s.edit.removed, "a");
    assert!(matches!(s.next, Err(SearchError::Cancelled)));
}

#[test]
fn deleting_the_line_of_an_empty_match_does_not_skip_the_next_line() {
    #[derive(Debug)]
    struct LineStart;
    impl Matcher for LineStart {
        fn find_at(&self, _: &str, start: usize) -> Result<Option<Match>, MatcherError> {
            Ok((start == 0).then(|| Match::new(0, 0)))
        }
    }
    let mut d = doc("a\nb\nc\n");
    let mut ctl = Control::new();
    let found = Pos::new(0, 0)..Pos::new(0, 0);
    let (_, next) = step(
        &mut d,
        &LineStart,
        &found,
        "",
        ReplaceTarget::DeleteLine,
        false,
        &mut ctl,
    )
    .unwrap();
    assert_eq!(text_of(&d), "b\nc\n");
    assert_eq!(next.unwrap().start, Pos::new(0, 0));
}

#[test]
fn offsets_given_to_matcher_overrides_are_char_boundaries() {
    #[derive(Debug)]
    struct Slicing;
    impl Matcher for Slicing {
        fn find_at(&self, line: &str, start: usize) -> Result<Option<Match>, MatcherError> {
            let _ = &line[start..];
            Ok(None)
        }
        fn find_last_before(
            &self,
            line: &str,
            limit: Option<usize>,
        ) -> Result<Option<Match>, MatcherError> {
            let _ = &line[..limit.unwrap_or(0)];
            Ok(None)
        }
        fn find_all_from(&self, line: &str, start: usize) -> Result<Vec<Match>, MatcherError> {
            let _ = &line[start..];
            Ok(Vec::new())
        }
    }
    let d = doc("あい\n");
    let p = Pos::new;
    assert_eq!(prev(&d, &Slicing, 0, 1, false), None);
    assert_eq!(prev(&d, &Slicing, 0, 99, false), None);
    let range = p(0, 1)..p(0, 5);
    assert_eq!(
        find_all(&d, &Slicing, Some(range), Control::new()).count(),
        0
    );
}

// ---- replace all, one after the other ----

#[test]
fn whole_word_replace_all_searches_the_text_as_it_is_after_each_replacement() {
    // Words of "a,": "a" and ",". Replacing "a" with "x" makes "x,a"; replacing "," makes "xxa",
    // and "xxa" is not a word of the pattern, so the last "a" stays (Command_REPLACE_ALL searches
    // again from the caret after every replacement).
    let mut d = doc("a,a\n");
    let r = replace_all(
        &mut d,
        &word("a,", true),
        "x",
        ReplaceTarget::Match,
        None,
        Control::new(),
    )
    .unwrap();
    assert_eq!(text_of(&d), "xxa\n");
    assert_eq!(r.edits.len(), 2);
}

/// A matcher that wraps another one but does not say it is context free, so that
/// `replace_all` takes the one-after-the-other path.
#[derive(Debug)]
struct Sequential(PlainMatcher);

impl Matcher for Sequential {
    fn find_at(&self, line: &str, start: usize) -> Result<Option<Match>, MatcherError> {
        self.0.find_at(line, start)
    }
}

#[test]
fn both_replace_all_paths_agree_for_plain_text() {
    let text = "ab ab\nxabx\n\nab\r\nabab ab\nlast ab";
    let p = Pos::new;
    let ranges = [
        None,
        Some(p(0, 3)..p(4, 2)),
        Some(p(1, 0)..p(4, 0)),
        Some(p(0, 0)..p(0, 4)),
    ];
    let targets = [
        ReplaceTarget::Match,
        ReplaceTarget::InsertBefore,
        ReplaceTarget::InsertAfter,
        ReplaceTarget::DeleteLine,
    ];
    for pattern in ["ab", "b", "x", "zz"] {
        for replacement in ["", "Q", "<\n>", "ababab"] {
            for target in targets {
                for range in &ranges {
                    let fast = plain(pattern);
                    let slow = Sequential(plain(pattern));
                    let (mut a, mut b) = (doc(text), doc(text));
                    let ra = replace_all(
                        &mut a,
                        &fast,
                        replacement,
                        target,
                        range.clone(),
                        Control::new(),
                    )
                    .unwrap();
                    let rb = replace_all(
                        &mut b,
                        &slow,
                        replacement,
                        target,
                        range.clone(),
                        Control::new(),
                    )
                    .unwrap();
                    let case = format!("{pattern:?} {replacement:?} {target:?} {range:?}");
                    assert_eq!(text_of(&a), text_of(&b), "{case}");
                    assert_eq!(ra, rb, "{case}");
                }
            }
        }
    }
}

#[test]
fn replace_all_one_after_the_other_steps_over_empty_matches() {
    let mut d = doc("ab\n");
    let m = Sequential(plain("")); // never matches
    let r = replace_all(&mut d, &m, "-", ReplaceTarget::Match, None, Control::new()).unwrap();
    assert!(r.edits.is_empty());
    #[derive(Debug)]
    struct Empty;
    impl Matcher for Empty {
        fn find_at(&self, line: &str, start: usize) -> Result<Option<Match>, MatcherError> {
            Ok((start <= line.len()).then(|| Match::new(start, start)))
        }
    }
    let r = replace_all(
        &mut d,
        &Empty,
        "-",
        ReplaceTarget::Match,
        None,
        Control::new(),
    )
    .unwrap();
    assert_eq!(text_of(&d), "-a-b-\n");
    assert_eq!(r.edits.len(), 3);
}

#[test]
fn replace_all_one_after_the_other_is_not_applied_when_cancelled() {
    let mut d = numbered(100);
    let before = text_of(&d);
    let token = CancelToken::new();
    let t = token.clone();
    // cancel after some lines have been replaced
    let ctl = Control::new()
        .with_cancel(token)
        .with_interval(10)
        .with_progress(move |p| {
            if p.lines_done >= 30 {
                t.cancel();
            }
        });
    let r = replace_all(
        &mut d,
        &word("line", true),
        "x",
        ReplaceTarget::Match,
        None,
        ctl,
    );
    assert!(matches!(r, Err(SearchError::Cancelled)));
    assert_eq!(text_of(&d), before);
}

#[test]
fn replace_all_one_after_the_other_in_a_selection() {
    let mut d = doc("a a a\na a a\na a a\n");
    let p = Pos::new;
    let r = replace_all(
        &mut d,
        &word("a", true),
        "bb",
        ReplaceTarget::Match,
        Some(p(0, 2)..p(1, 3)),
        Control::new(),
    )
    .unwrap();
    assert_eq!(text_of(&d), "a bb bb\nbb bb a\na a a\n");
    assert_eq!(r.edits.len(), 4);
}
