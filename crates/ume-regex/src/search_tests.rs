//! Tests of the document search with regular expressions.
//!
//! Where an expectation comes from bregonig, the comment names its documentation (`bregonig.html`
//! of https://github.com/k-takata/bregonig at `c65e85d`); for Sakura, the file under
//! `sakura_core/` or `help/sakura/res/`. Expectations that the documentation does not settle say
//! "not in bregonig.html"; those were measured on the real bregonig.dll 4.20 on 2026-10-16
//! (`tools/bregonig-compat`, replayed by `tests/bregonig_compat.rs`). bregonig's source code is
//! under the GPL or the Artistic License and is not used here (ADR-0006).

use std::sync::Arc;

use ume_core::buffer::{Document, Pos};
use ume_core::search::{Control, Direction, Matcher, ReplaceTarget, find, find_all};

use crate::LIMITS_LOCK;
use crate::{
    PatternError, RegexEngine, RegexMatcher, RegexReplacer, RegexSearchOptions, ReplaceAllMode,
    ReplacementError, ReplacementProblem, ReplacementTemplate, ends_with_escape,
    make_pattern_alternate,
};

/// Shared by every test that uses the engine; see `LIMITS_LOCK`.
fn read_lock() -> std::sync::RwLockReadGuard<'static, ()> {
    LIMITS_LOCK
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn doc(text: &str) -> Document {
    Document::from_utf8(text.as_bytes().to_vec()).unwrap()
}

fn text_of(doc: &Document) -> String {
    doc.text_between(Pos::new(0, 0), doc.end_pos().unwrap())
        .unwrap()
}

/// Case-sensitive.
fn matcher(pattern: &str) -> RegexMatcher {
    RegexMatcher::new(pattern, RegexSearchOptions { match_case: true })
        .unwrap_or_else(|e| panic!("/{pattern}/ should compile: {e}"))
}

/// Sakura's default: case is ignored.
fn matcher_ci(pattern: &str) -> RegexMatcher {
    RegexMatcher::new(pattern, RegexSearchOptions::default())
        .unwrap_or_else(|e| panic!("/{pattern}/ should compile: {e}"))
}

/// The expansion of `replacement` for the first match of `pattern` in `text`.
fn expand(pattern: &str, replacement: &str, text: &str) -> String {
    let _limits = read_lock();
    let m = matcher(pattern);
    let template = ReplacementTemplate::parse(replacement, m.names(), m.capture_count()).unwrap();
    let found = m
        .find_at(text, 0)
        .unwrap()
        .unwrap_or_else(|| panic!("/{pattern}/ should match {text:?}"));
    let mut groups = vec![Some(found.range())];
    groups.extend(found.captures.iter().cloned());
    template.expand(text, &groups)
}

fn replacer(pattern: &str, replacement: &str, target: ReplaceTarget) -> RegexReplacer {
    RegexReplacer::new(Arc::new(matcher(pattern)), replacement, target)
        .unwrap_or_else(|e| panic!("{replacement:?} should compile: {e}"))
}

/// Replaces everything in `text` the way Sakura's default (一括置換) does.
fn replace_all(pattern: &str, replacement: &str, text: &str) -> (String, usize) {
    replace_all_with(
        pattern,
        replacement,
        text,
        ReplaceAllMode::Batch,
        ReplaceTarget::Match,
    )
}

fn replace_all_with(
    pattern: &str,
    replacement: &str,
    text: &str,
    mode: ReplaceAllMode,
    target: ReplaceTarget,
) -> (String, usize) {
    let _limits = read_lock();
    let mut d = doc(text);
    let r = replacer(pattern, replacement, target);
    let result = r.replace_all(&mut d, mode, None, Control::new()).unwrap();
    (text_of(&d), result.edits.len())
}

// ---------------------------------------------------------------------------------------------
// Pattern pre-processing (CBregexp::MakePatternAlternate)

#[test]
fn dot_and_dollar_are_replaced_like_make_pattern_alternate() {
    // The two strings are `szDotAlternative` and `szDollarAlternative` in
    // sakura_core/extmodule/CBregexp.cpp.
    assert_eq!(make_pattern_alternate("."), r"[^\r\n]");
    assert_eq!(make_pattern_alternate("a$"), r"a(?<![\r\n])(?=\r|$)");
    assert_eq!(
        make_pattern_alternate("^.*$"),
        r"^[^\r\n]*(?<![\r\n])(?=\r|$)"
    );
}

#[test]
fn escaped_dot_and_dollar_stay() {
    assert_eq!(make_pattern_alternate(r"\.\$"), r"\.\$");
    // An escaped backslash does not escape the char after it.
    assert_eq!(make_pattern_alternate(r"\\."), r"\\[^\r\n]");
}

#[test]
fn classes_and_quotes_hide_dot_and_dollar() {
    assert_eq!(make_pattern_alternate("[.$]"), "[.$]");
    // POSIX brackets nest inside a class.
    assert_eq!(
        make_pattern_alternate("[[:alpha:].]."),
        r"[[:alpha:].][^\r\n]"
    );
    // \Q...\E
    assert_eq!(make_pattern_alternate(r"\Q.$\E."), r"\Q.$\E[^\r\n]");
    // A backslash inside \Q...\E that is not followed by E does not end it.
    assert_eq!(
        make_pattern_alternate(r"\Q\.$\E$"),
        r"\Q\.$\E(?<![\r\n])(?=\r|$)"
    );
}

#[test]
fn control_escape_takes_the_next_char() {
    // `\c.` is a control char: the dot belongs to it (state D_C of the table).
    assert_eq!(make_pattern_alternate(r"\c.."), r"\c.[^\r\n]");
    // `\c\\` : the second backslash starts an escape, as in the table.
    assert_eq!(make_pattern_alternate(r"\c\.."), r"\c\.[^\r\n]");
}

#[test]
fn comments_are_not_skipped_as_in_sakura() {
    // The state machine does not know `(?#...)`; Sakura's copy replaces the dot too.
    assert_eq!(make_pattern_alternate("(?#.)"), r"(?#[^\r\n])");
}

#[test]
fn the_alternate_pattern_matches_the_same_lines() {
    let _limits = read_lock();
    // On a line without its line ending the replacements change nothing.
    let lines = ["", "abc", "a.c", "  x  ", "日本語 123", "foo$bar", "end."];
    for pattern in [
        r"a.c", r"^$", r"c$", r".*$", r"\d+$", r"^.", r"[.$]", r"\.", r"o$", r"\s*$", r"(.)\1",
        r".$",
    ] {
        let alt = matcher(pattern);
        let plain = crate::Onigmo
            .compile(pattern, crate::Options::default())
            .unwrap();
        for line in lines {
            for start in (0..=line.len()).filter(|&i| line.is_char_boundary(i)) {
                let a = alt.find_at(line, start).unwrap().map(|m| m.range());
                let b = plain.find_at(line, start).unwrap();
                assert_eq!(a, b, "/{pattern}/ in {line:?} from {start}");
            }
        }
    }
}

#[test]
fn a_trailing_backslash_loses_the_delimiter() {
    // Sakura builds `m<D>pattern<D>k` (CBregexp.cpp). A single trailing backslash escapes the
    // closing delimiter, so bregonig.dll 4.20 refuses the pattern ("unmatch separater").
    assert!(ends_with_escape("abc\\"));
    assert!(!ends_with_escape("abc\\\\"));
    assert!(ends_with_escape("abc\\\\\\"));
    assert!(!ends_with_escape("abc"));
    assert!(!ends_with_escape(""));
    assert_eq!(
        RegexMatcher::new("abc\\", RegexSearchOptions::default()).unwrap_err(),
        PatternError::UnmatchedSeparator
    );
}

#[test]
fn bad_patterns_report_the_engine_message() {
    let _limits = read_lock();
    let err = RegexMatcher::new("(abc", RegexSearchOptions::default()).unwrap_err();
    match err {
        PatternError::Compile(e) => assert!(e.message().contains("parenthesis"), "{e}"),
        other => panic!("unexpected {other:?}"),
    }
    // The dollar becomes a look-ahead, so it cannot sit in a look-behind: Sakura fails the same way.
    assert!(RegexMatcher::new("(?<=a$)b", RegexSearchOptions::default()).is_err());
}

// ---------------------------------------------------------------------------------------------
// Matching in a document

#[test]
fn case_is_ignored_unless_asked() {
    // sakura_core/extmodule/CBregexp.cpp (MakePatternSub): `i` unless optCaseSensitive.
    let _limits = read_lock();
    let d = doc("Hello WORLD");
    let ci = matcher_ci("world");
    let found = find(
        &d,
        &ci,
        Pos::new(0, 0),
        Direction::Forward,
        false,
        &mut Control::new(),
    )
    .unwrap()
    .unwrap();
    assert_eq!((found.start, found.end), (Pos::new(0, 6), Pos::new(0, 11)));
    let cs = matcher("world");
    assert!(
        find(
            &d,
            &cs,
            Pos::new(0, 0),
            Direction::Forward,
            false,
            &mut Control::new()
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn the_help_example_of_greedy_and_lazy_repetition() {
    // help/sakura/res/HLP000089.html (量指定子): in SAKURA_COLLBACKPROC_BREXP_PROC
    // `[A-Z_]*PROC` finds the whole string and `[A-Z_]*?PROC` the first part.
    let _limits = read_lock();
    let d = doc("SAKURA_COLLBACKPROC_BREXP_PROC");
    for (pattern, end) in [("[A-Z_]*PROC", 30), ("[A-Z_]*?PROC", 19)] {
        let m = matcher(pattern);
        let f = find(
            &d,
            &m,
            Pos::new(0, 0),
            Direction::Forward,
            false,
            &mut Control::new(),
        )
        .unwrap()
        .unwrap();
        assert_eq!((f.start.col, f.end.col), (0, end), "{pattern}");
    }
}

#[test]
fn full_width_digits_match_d_and_japanese_text_is_found() {
    // HLP000089.html (文字種): `\d` includes the 2-byte (full-width) digits in the Unicode version.
    let _limits = read_lock();
    let d = doc("令和７年と2026年\n平成３０年");
    let m = matcher(r"(\d+)年");
    let all: Vec<_> = find_all(&d, &m, None, Control::new())
        .map(|f| f.unwrap())
        .collect();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].start, Pos::new(0, 6));
    assert_eq!(all[1].start, Pos::new(0, 15));
    assert_eq!(all[2].start, Pos::new(1, 6));
    // Group 1 is a byte range of the line.
    let line = d.line(1).unwrap().unwrap().text;
    assert_eq!(&line[all[2].captures[0].clone().unwrap()], "３０");
}

#[test]
fn caret_and_dollar_anchor_the_line() {
    // HLP000089.html (位置): ^ is the start and $ the end of the line.
    let _limits = read_lock();
    let d = doc("abc\n\nabc");
    let starts: Vec<_> = find_all(&d, &matcher("^"), None, Control::new())
        .map(|f| f.unwrap().start)
        .collect();
    assert_eq!(starts, vec![Pos::new(0, 0), Pos::new(1, 0), Pos::new(2, 0)]);
    let ends: Vec<_> = find_all(&d, &matcher("$"), None, Control::new())
        .map(|f| f.unwrap().start)
        .collect();
    assert_eq!(ends, vec![Pos::new(0, 3), Pos::new(1, 0), Pos::new(2, 3)]);
    // `^` does not match in the middle of a line after a search start there.
    let m = matcher("^a");
    assert!(m.find_at("aa", 1).unwrap().is_none());
}

#[test]
fn captures_at_the_start_and_end_of_a_line() {
    let _limits = read_lock();
    let m = matcher(r"^(\w)(.*)(\w)$");
    let found = m.find_at("abcd", 0).unwrap().unwrap();
    assert_eq!(found.range(), 0..4);
    assert_eq!(found.captures, vec![Some(0..1), Some(1..3), Some(3..4)]);
}

#[test]
fn backward_search_returns_the_last_match_before_the_caret() {
    // CSearchAgent::SearchWord: scan forward, keep the last hit that begins before the caret.
    let _limits = read_lock();
    let d = doc("ab ab ab\nab");
    let m = matcher("ab");
    let f = find(
        &d,
        &m,
        Pos::new(0, 6),
        Direction::Backward,
        false,
        &mut Control::new(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(f.start, Pos::new(0, 3));
    let f = find(
        &d,
        &m,
        Pos::new(1, 0),
        Direction::Backward,
        false,
        &mut Control::new(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(f.start, Pos::new(0, 6));
}

#[test]
fn empty_matches_move_on_one_char() {
    // CSearchAgent::SearchWord / Command_SEARCH_NEXT: after an empty match, one char further.
    let _limits = read_lock();
    let d = doc("a日b");
    let starts: Vec<_> = find_all(&d, &matcher("x*"), None, Control::new())
        .map(|f| f.unwrap().start.col)
        .collect();
    assert_eq!(starts, vec![0, 1, 4, 5]);
}

#[test]
fn look_behind_sees_the_text_before_the_start() {
    let _limits = read_lock();
    let m = matcher(r"(?<=a)b");
    assert_eq!(m.find_at("ab", 1).unwrap().unwrap().range(), 1..2);
    let m = matcher(r"\bfoo");
    assert!(m.find_at("xfoo", 1).unwrap().is_none());
}

// ---------------------------------------------------------------------------------------------
// The replacement syntax (bregonig.html 5.3)

#[test]
fn numbered_references() {
    // bregonig.html (置換で使える参照): $n, \n and $& ; Sakura's help: $n and \n.
    assert_eq!(
        expand(r"(\w+) (\w+)", "$2 $1", "hello world"),
        "world hello"
    );
    assert_eq!(
        expand(r"(\w+) (\w+)", r"\2 \1", "hello world"),
        "world hello"
    );
    assert_eq!(expand(r"l+", "[$&]", "hello"), "[ll]");
}

#[test]
fn braces_separate_a_reference_from_following_digits() {
    // bregonig.html: s/(hoge.)/${1}2/g
    assert_eq!(expand(r"(hoge.)", "${1}2", "hogeX"), "hogeX2");
    // Without braces all the digits are the number: group 12 does not exist.
    assert_eq!(expand(r"(hoge.)", "$12", "hogeX"), "");
    // `${0}` is the whole match (parse_digits accepts 0 inside braces).
    assert_eq!(expand(r"b+", "<${0}>", "abbc"), "<bb>");
}

#[test]
fn two_digit_references() {
    let pattern = r"(1)(2)(3)(4)(5)(6)(7)(8)(9)(a)(b)";
    assert_eq!(expand(pattern, "$10-$11-$1", "123456789ab"), "a-b-1");
    assert_eq!(expand(pattern, r"\10\11", "123456789ab"), "ab");
}

#[test]
fn dollar_zero_and_unknown_dollars_are_plain_text() {
    // bregonig.html 5.3: `$0` is just a string, and so are the other `$` forms that are not references.
    assert_eq!(expand("b", "$0", "abc"), "$0");
    assert_eq!(expand("b", "$x", "abc"), "$x");
    assert_eq!(expand("b", "a$", "abc"), "a$");
    assert_eq!(expand("b", "$", "abc"), "$");
    // bregonig.dll 4.20: a `$` that is not a reference takes the next character with it as text,
    // so `$$1` is not `$` + group 1, and `$\n` is not a line feed.
    assert_eq!(expand("(b)", "$$1", "abc"), "$$1");
    assert_eq!(expand("(b)", "$$$1", "abc"), "$$b");
    assert_eq!(expand("b", r"$\n", "abc"), r"$\n");
}

#[test]
fn a_group_that_did_not_take_part_gives_nothing() {
    assert_eq!(expand("(x)?b", "[$1]", "abc"), "[]");
    // A number past the groups gives nothing as well (and is listed as a problem).
    assert_eq!(expand("(b)", "[$2]", "abc"), "[]");
}

#[test]
fn dollar_plus_is_the_group_with_the_highest_number_that_matched() {
    // bregonig.html: `(a)|([^a])` and `$+` give whichever of $1 and $2 matched.
    assert_eq!(expand("(a)|([^a])", "<$+>", "b"), "<b>");
    assert_eq!(expand("(a)|([^a])", "<$+>", "a"), "<a>");
    // No group took part: nothing.
    assert_eq!(expand("(x)?b", "<$+>", "b"), "<>");
}

#[test]
fn named_references() {
    // bregonig.html: $+{name}, ${name}, \k<name>, \k'name', $-{name}[n]
    let p = r"(?<year>\d+)-(?<mon>\d+)";
    assert_eq!(expand(p, "$+{mon}/$+{year}", "2026-10"), "10/2026");
    assert_eq!(expand(p, "${mon}/${year}", "2026-10"), "10/2026");
    assert_eq!(expand(p, r"\k<mon>/\k'year'", "2026-10"), "10/2026");
    assert_eq!(expand(p, "$-{year}[0]", "2026-10"), "2026");
    assert_eq!(expand(p, "$1", "2026-10"), "2026");
    // Plain groups capture next to named ones (ONIG_OPTION_CAPTURE_GROUP in the Perl syntax of
    // Onigmo), so the numbers follow the parentheses.
    assert_eq!(expand(r"(a)(?<x>b)(c)", "$+{x}$1$3$2", "abc"), "bacb");
    // The Python spelling is a group too.
    assert_eq!(expand(r"(?P<x>a)(b)", "$+{x}${x}$2", "ab"), "aab");
}

#[test]
fn a_name_used_twice_means_the_leftmost_group() {
    // bregonig.html: "$+{name}: with duplicate names the leftmost result (Ver.2 and later)";
    // $-{name}[n]: n = 0 is the leftmost, -1 the rightmost.
    let p = r"(?<n>a)(?<n>b)";
    assert_eq!(expand(p, "$+{n}", "ab"), "a");
    assert_eq!(expand(p, "$-{n}[0]", "ab"), "a");
    assert_eq!(expand(p, "$-{n}[1]", "ab"), "b");
    assert_eq!(expand(p, "$-{n}[-1]", "ab"), "b");
}

#[test]
fn escapes_make_control_and_numeric_chars() {
    // HLP000089.html (文字): \t \n \r \f \a \e \b \ooo \xHH \x{HHHH}; bregonig.html 5.3.
    assert_eq!(
        expand("b", r"\t\n\r\f\a\e\b", "abc"),
        "\t\n\r\x0c\x07\x1b\x08"
    );
    assert_eq!(expand("b", r"\x41\x{3042}\o{101}", "abc"), "AあA");
    assert_eq!(expand("b", r"\012", "abc"), "\n");
    assert_eq!(expand("b", r"\0", "abc"), "\0");
    // At most 3 digits count, the 0 among them.
    assert_eq!(expand("b", r"\01012", "abc"), "\u{8}12");
    // \cX: bregonig.html "\c[ is ESC"
    assert_eq!(expand("b", r"\c[\ca", "abc"), "\x1b\x01");
    // The char itself.
    assert_eq!(expand("b", r"\\\$\/\.", "abc"), "\\$/.");
}

#[test]
fn v_is_not_an_escape() {
    // bregonig.html: \v was removed in Ver.2.
    assert_eq!(expand("b", r"\v", "abc"), "v");
}

#[test]
fn a_number_past_the_groups_is_octal_when_it_can_be() {
    // bregonig.html (pattern2 における \nnn): without that many groups, \10 and more are octal
    // chars.
    assert_eq!(expand("b", r"\12", "abc"), "\n");
    assert_eq!(expand("b", r"\101", "abc"), "A");
    // \18 : 8 is not an octal digit, so nothing.
    assert_eq!(expand("b", r"\18", "abc"), "");
    // `$12` is never octal.
    assert_eq!(expand("b", "$12", "abc"), "");
    // A single digit past the groups gives nothing.
    assert_eq!(expand("b", r"\1", "abc"), "");
}

#[test]
fn case_conversion() {
    // bregonig.html: \l \u \L \U \E (Ver.3 and later), ASCII letters only.
    assert_eq!(
        expand(r"(\w+) (\w+)", r"\u$1 \U$2\E!", "hello world"),
        "Hello WORLD!"
    );
    assert_eq!(expand(r"(\w+)", r"\L$1\E", "HeLLo"), "hello");
    assert_eq!(expand(r"(\w+)", r"\l$1", "HeLLo"), "heLLo");
    // Literal text is converted too.
    assert_eq!(expand("b", r"\Uabc\Edef", "abc"), "ABCdef");
    // `\u` wins over `\L` for the first char.
    assert_eq!(expand(r"(\w+)", r"\L\u$1", "hELLO"), "Hello");
    // Only ASCII: ß and é stay.
    assert_eq!(expand(r"(.+)", r"\U$1", "straße é"), "STRAßE é");
}

#[test]
fn a_pending_upper_case_waits_for_the_next_char_when_a_group_is_empty() {
    // Not in bregonig.html; bregonig.dll 4.20: a pending \u waits for the next written char, so an
    // empty group does not use it up and the literal text after it takes it.
    assert_eq!(expand("(x*)y", r"\u$1z", "y"), "Z");
}

#[test]
fn quote_is_skipped() {
    // Not in bregonig.html; bregonig.dll 4.20 skips `\Q` and quotes nothing.
    assert_eq!(expand("b", r"\Qa$&", "abc"), "ab");
    assert_eq!(expand("b", r"\Q$&\E", "abc"), "b");
}

#[test]
fn malformed_references_keep_the_text() {
    // Not in bregonig.html; bregonig.dll 4.20: a malformed reference keeps its text (the `$`, or
    // the letter after the backslash) and scanning goes on after it.
    assert_eq!(expand("b", "${1", "abc"), "${1");
    assert_eq!(expand("b", "$+{x", "abc"), "$+{x");
    assert_eq!(expand("b", r"\k<x", "abc"), "k<x");
    assert_eq!(expand("b", r"\x{41", "abc"), "x{41");
    // An unknown name is silently empty.
    assert_eq!(expand("b", "[$+{nope}]", "abc"), "[]");
}

#[test]
fn problems_are_listed_without_changing_the_result() {
    let _limits = read_lock();
    let m = matcher("(a)");
    let t = ReplacementTemplate::parse(r"$2 $+{x} \3 \12 ${1", m.names(), 1).unwrap();
    assert!(t.problems().contains(&ReplacementProblem::NoSuchGroup(2)));
    assert!(t.problems().contains(&ReplacementProblem::NoSuchGroup(3)));
    assert!(
        t.problems()
            .contains(&ReplacementProblem::UnknownName("x".to_owned()))
    );
    // \12 is a valid octal char, not a bad reference.
    assert!(!t.problems().contains(&ReplacementProblem::NoSuchGroup(12)));
    assert!(
        t.problems()
            .iter()
            .any(|p| matches!(p, ReplacementProblem::Malformed(_)))
    );
    let clean = ReplacementTemplate::parse("$1-$&", m.names(), 1).unwrap();
    assert!(clean.problems().is_empty());
}

#[test]
fn plain_text_is_constant() {
    let _limits = read_lock();
    let m = matcher("a");
    assert!(
        ReplacementTemplate::parse("plain", m.names(), 0)
            .unwrap()
            .is_constant()
    );
    assert!(
        ReplacementTemplate::parse("", m.names(), 0)
            .unwrap()
            .is_constant()
    );
    assert!(
        !ReplacementTemplate::parse("$&", m.names(), 0)
            .unwrap()
            .is_constant()
    );
    assert!(
        !ReplacementTemplate::parse(r"\U", m.names(), 0)
            .unwrap()
            .is_constant()
    );
}

#[test]
fn a_replacement_ending_with_a_backslash_is_an_error() {
    // A single trailing backslash escapes the delimiter that Sakura puts after the replacement
    // (`s/pattern/rep/`), so the replacement would have no end; bregonig.dll 4.20 refuses it.
    let _limits = read_lock();
    let m = matcher("a");
    assert_eq!(
        ReplacementTemplate::parse("abc\\", m.names(), 0).unwrap_err(),
        ReplacementError::UnmatchedSeparator
    );
    assert_eq!(
        ReplacementTemplate::parse("\\", m.names(), 0).unwrap_err(),
        ReplacementError::UnmatchedSeparator
    );
    assert!(ReplacementTemplate::parse("abc\\\\", m.names(), 0).is_ok());
}

// ---------------------------------------------------------------------------------------------
// Replacing in a document

#[test]
fn replace_all_expands_per_match_and_counts() {
    // Japanese text, captures, and the count Sakura shows (nReplaceNum).
    let (text, n) = replace_all(r"(\d+)年", "<$1>年", "令和７年と2026年\n平成３０年");
    assert_eq!(text, "令和<７>年と<2026>年\n平成<３０>年");
    assert_eq!(n, 3);
}

#[test]
fn replace_all_with_empty_matches() {
    // After an empty match the next search starts one char later, and the char in between is
    // kept: "abc" with x* -> "-" gives "-a-b-c-", as Perl's s///g does (bregonig.dll 4.20
    // gives the same).
    assert_eq!(replace_all("x*", "-", "abc"), ("-a-b-c-".to_owned(), 4));
    // After a non-empty match an empty match right behind it is found (s != m).
    assert_eq!(replace_all("x*", "-", "xab"), ("--a-b-".to_owned(), 4));
    // Multi-byte chars are skipped whole.
    assert_eq!(replace_all("", "|", "日本"), ("|日|本|".to_owned(), 3));
    // An empty line has one empty match.
    assert_eq!(
        replace_all("^", "> ", "a\n\nb"),
        ("> a\n> \n> b".to_owned(), 3)
    );
}

#[test]
fn replace_all_at_line_starts_and_ends() {
    assert_eq!(
        replace_all("^", "# ", "one\ntwo"),
        ("# one\n# two".to_owned(), 2)
    );
    assert_eq!(
        replace_all("$", ";", "one\ntwo"),
        ("one;\ntwo;".to_owned(), 2)
    );
    assert_eq!(
        replace_all(r"^(\s*)(.*?)\s*$", "$1[$2]", "  a b  \nc"),
        ("  [a b]\n[c]".to_owned(), 2)
    );
}

#[test]
fn batch_mode_searches_the_line_as_it_was() {
    // `(?<=a)a`: the look-behind of the second match sees the original `a`. Sakura uses the `g`
    // option (bregonig.html 5.2, "global replacement") unless 「すべて置換」は置換の繰返し is on
    // (sakura_core/cmd/CViewCommander_Search.cpp: optGlobal unless bConsecutiveAll). That a
    // look-behind sees the original text is not in bregonig.html; bregonig.dll 4.20 does (`aaa`
    // becomes `aXX`).
    let (text, n) = replace_all_with(
        "(?<=a)a",
        "X",
        "aaa",
        ReplaceAllMode::Batch,
        ReplaceTarget::Match,
    );
    assert_eq!((text.as_str(), n), ("aXX", 2));
}

#[test]
fn repeat_mode_searches_the_changed_text() {
    // 「すべて置換」は置換の繰返し: the look-behind of the next search sees the new `X`.
    let (text, n) = replace_all_with(
        "(?<=a)a",
        "X",
        "aaa",
        ReplaceAllMode::Repeat,
        ReplaceTarget::Match,
    );
    assert_eq!((text.as_str(), n), ("aXa", 1));
}

#[test]
fn both_modes_agree_when_the_context_does_not_matter() {
    for text in ["a1b22c333", "日本１２３語\n45"] {
        let batch = replace_all_with(
            r"(\d+)",
            "<$1>",
            text,
            ReplaceAllMode::Batch,
            ReplaceTarget::Match,
        );
        let repeat = replace_all_with(
            r"(\d+)",
            "<$1>",
            text,
            ReplaceAllMode::Repeat,
            ReplaceTarget::Match,
        );
        assert_eq!(batch, repeat);
    }
    // Empty matches too.
    let batch = replace_all_with(
        "x*",
        "-",
        "abc",
        ReplaceAllMode::Batch,
        ReplaceTarget::Match,
    );
    let repeat = replace_all_with(
        "x*",
        "-",
        "abc",
        ReplaceAllMode::Repeat,
        ReplaceTarget::Match,
    );
    assert_eq!(batch, repeat);
    assert_eq!(batch.0, "-a-b-c-");
}

#[test]
fn a_line_break_in_the_replacement_splits_the_line() {
    // bregonig.html 5.3: \n is LF; Sakura's Command_INSTEXT inserts it as it is, so the line is
    // split.
    let (text, n) = replace_all(",", r"\n", "a,b,c");
    assert_eq!(text, "a\nb\nc");
    assert_eq!(n, 2);
    let (text, _) = replace_all(r"\s+", r"\r\n", "a b");
    assert_eq!(text, "a\r\nb");
    // The new lines are lines of the document.
    let _limits = read_lock();
    let mut d = doc("a,b");
    let r = replacer(",", r"\n", ReplaceTarget::Match);
    r.replace_all(&mut d, ReplaceAllMode::Batch, None, Control::new())
        .unwrap();
    assert_eq!(d.line_count(), 2);
}

#[test]
fn insert_before_and_after_keep_the_match() {
    // CViewCommander_Search.cpp (Nov. 9, 2005 かろと): "rep$&" and "$&rep".
    let _limits = read_lock();
    let (text, n) = replace_all_with(
        "b+",
        "<",
        "abbc",
        ReplaceAllMode::Batch,
        ReplaceTarget::InsertBefore,
    );
    assert_eq!((text.as_str(), n), ("a<bbc", 1));
    let (text, _) = replace_all_with(
        "b+",
        ">",
        "abbc",
        ReplaceAllMode::Batch,
        ReplaceTarget::InsertAfter,
    );
    assert_eq!(text, "abb>c");
    // The kept match goes through the case conversion like the rest (from the source).
    let (text, _) = replace_all_with(
        "b",
        r"\Ux",
        "abc",
        ReplaceAllMode::Batch,
        ReplaceTarget::InsertBefore,
    );
    assert_eq!(text, "aXBc");
}

#[test]
fn delete_line_does_not_read_the_replacement() {
    let _limits = read_lock();
    // A trailing backslash would be an error for any other target.
    let r = RegexReplacer::new(Arc::new(matcher("a")), "x\\", ReplaceTarget::DeleteLine);
    assert!(r.is_ok());
    let r = RegexReplacer::new(Arc::new(matcher("a")), "x\\", ReplaceTarget::Match);
    assert!(r.is_err());
}

#[test]
fn delete_line_with_an_empty_matching_pattern_terminates() {
    // `^` matches in every line, including an empty last line that has no line ending to
    // delete; Repeat must not loop on it.
    for (text, want, n) in [
        ("a\nb\nc", "", 3),
        ("a\nb\n", "", 2),
        ("a\n\nb", "", 3),
        ("", "", 0),
    ] {
        for mode in [ReplaceAllMode::Batch, ReplaceAllMode::Repeat] {
            let (got, count) = replace_all_with("^", "", text, mode, ReplaceTarget::DeleteLine);
            assert_eq!((got.as_str(), count), (want, n), "{text:?} {mode:?}");
        }
    }
}

#[test]
fn an_empty_match_replaced_with_nothing_does_not_skip_the_line() {
    for mode in [ReplaceAllMode::Batch, ReplaceAllMode::Repeat] {
        let (got, _) = replace_all_with("x*", "", "axb", mode, ReplaceTarget::Match);
        assert_eq!(got, "ab", "{mode:?}");
    }
}

#[test]
fn repeat_mode_stays_inside_the_selection_and_follows_its_end() {
    let _limits = read_lock();
    let mut d = doc("a a a\na a");
    let r = replacer("a", "bb", ReplaceTarget::Match);
    // Only the first two `a` of line 0: the range end moves as the text grows.
    r.replace_all(
        &mut d,
        ReplaceAllMode::Repeat,
        Some(Pos::new(0, 0)..Pos::new(0, 3)),
        Control::new(),
    )
    .unwrap();
    assert_eq!(text_of(&d), "bb bb a\na a");
}

#[test]
fn repeat_mode_in_a_selection_can_be_cancelled() {
    let _limits = read_lock();
    let mut d = doc("a\na\na");
    let token = ume_core::search::CancelToken::new();
    token.cancel();
    let r = replacer("a", "b", ReplaceTarget::Match);
    let err = r
        .replace_all(
            &mut d,
            ReplaceAllMode::Repeat,
            Some(Pos::new(0, 0)..Pos::new(2, 1)),
            Control::new().with_cancel(token),
        )
        .unwrap_err();
    assert!(matches!(err, ume_core::search::SearchError::Cancelled));
    assert_eq!(text_of(&d), "a\na\na");
}

#[test]
fn conditional_patterns_keep_their_names() {
    // The engine counts three groups; the name scan must agree.
    assert_eq!(expand(r"(a)?(?(1)(?<yes>b)|(?<no>c))", "${yes}", "ab"), "b");
}

#[test]
fn delete_line_removes_each_line_once() {
    let (text, n) = replace_all_with(
        "a",
        "ignored",
        "a a\nb\nxa",
        ReplaceAllMode::Batch,
        ReplaceTarget::DeleteLine,
    );
    assert_eq!((text.as_str(), n), ("b\n", 2));
    let (text, n) = replace_all_with(
        "a",
        "",
        "a a\nb\nxa",
        ReplaceAllMode::Repeat,
        ReplaceTarget::DeleteLine,
    );
    assert_eq!((text.as_str(), n), ("b\n", 2));
}

#[test]
fn replace_in_a_range() {
    let _limits = read_lock();
    let mut d = doc("a1\na2\na3");
    let r = replacer(r"a(\d)", "<$1>", ReplaceTarget::Match);
    // From the start of line 1 to the end of line 1: only the second line.
    let range = Pos::new(1, 0)..Pos::new(1, 2);
    let result = r
        .replace_all(
            &mut d,
            ReplaceAllMode::Batch,
            Some(range.clone()),
            Control::new(),
        )
        .unwrap();
    assert_eq!(text_of(&d), "a1\n<2>\na3");
    assert_eq!(result.edits.len(), 1);
    // Repeat mode follows the same range.
    let mut d = doc("a1\na2\na3");
    r.replace_all(&mut d, ReplaceAllMode::Repeat, Some(range), Control::new())
        .unwrap();
    assert_eq!(text_of(&d), "a1\n<2>\na3");
    // A range that ends at column 0 leaves that last line out (find_all).
    let mut d = doc("a1\na2\na3");
    r.replace_all(
        &mut d,
        ReplaceAllMode::Repeat,
        Some(Pos::new(0, 0)..Pos::new(2, 0)),
        Control::new(),
    )
    .unwrap();
    assert_eq!(text_of(&d), "<1>\n<2>\na3");
}

#[test]
fn replace_and_find_next_steps_through_the_document() {
    let _limits = read_lock();
    let mut d = doc("k1 k2\nk3");
    let r = replacer(r"k(\d)", "v$1$1", ReplaceTarget::Match);
    let mut ctl = Control::new();
    let first = find(
        &d,
        r.matcher(),
        Pos::new(0, 0),
        Direction::Forward,
        false,
        &mut ctl,
    )
    .unwrap()
    .unwrap();
    let step = r
        .replace_and_find_next(&mut d, &first, false, &mut ctl)
        .unwrap();
    assert_eq!(step.edit.removed, "k1");
    assert_eq!(step.edit.inserted, "v11");
    let second = step.next.unwrap().unwrap();
    assert_eq!(second.start, Pos::new(0, 4));
    let step = r
        .replace_and_find_next(&mut d, &second, false, &mut ctl)
        .unwrap();
    let third = step.next.unwrap().unwrap();
    assert_eq!(third.start, Pos::new(1, 0));
    let step = r
        .replace_and_find_next(&mut d, &third, false, &mut ctl)
        .unwrap();
    assert!(step.next.unwrap().is_none());
    assert_eq!(text_of(&d), "v11 v22\nv33");
}

#[test]
fn replace_and_find_next_moves_past_an_empty_match() {
    let _limits = read_lock();
    let mut d = doc("ab");
    let r = replacer("x*", "-", ReplaceTarget::Match);
    let mut ctl = Control::new();
    let mut found = find(
        &d,
        r.matcher(),
        Pos::new(0, 0),
        Direction::Forward,
        false,
        &mut ctl,
    )
    .unwrap();
    let mut count = 0;
    while let Some(f) = found {
        count += 1;
        assert!(count < 10, "does not end");
        found = r
            .replace_and_find_next(&mut d, &f, false, &mut ctl)
            .unwrap()
            .next
            .unwrap();
    }
    assert_eq!(text_of(&d), "-a-b-");
}

#[test]
fn replace_all_can_be_cancelled_without_changing_the_document() {
    let _limits = read_lock();
    let mut d = doc("a\na\na");
    let token = ume_core::search::CancelToken::new();
    token.cancel();
    let r = replacer("a", "b", ReplaceTarget::Match);
    let err = r
        .replace_all(
            &mut d,
            ReplaceAllMode::Batch,
            None,
            Control::new().with_cancel(token),
        )
        .unwrap_err();
    assert!(matches!(err, ume_core::search::SearchError::Cancelled));
    assert_eq!(text_of(&d), "a\na\na");
}

#[test]
fn a_large_document() {
    let _limits = read_lock();
    let n = 50_000;
    let mut text = String::new();
    for i in 0..n {
        text.push_str(&format!("行{i}: value={i} (終)\n"));
    }
    let mut d = doc(&text);
    let r = replacer(r"value=(\d+)", "v=$1;", ReplaceTarget::Match);
    let result = r
        .replace_all(&mut d, ReplaceAllMode::Batch, None, Control::new())
        .unwrap();
    assert_eq!(result.edits.len(), n);
    assert_eq!(d.line(0).unwrap().unwrap().text, "行0: v=0; (終)");
    assert_eq!(
        d.line(n - 1).unwrap().unwrap().text,
        format!("行{}: v={}; (終)", n - 1, n - 1)
    );
    let found = find_all(&d, r.matcher(), None, Control::new()).count();
    assert_eq!(found, 0);
}
