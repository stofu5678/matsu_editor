//! Tests of the stored states: across lines, after edits, and in a large document.

use std::time::Duration;

use ume_config::settings::DocumentTypes;
use ume_core::buffer::{Document, Pos};

use super::*;
use crate::scan::{ColorState, QuoteKind};

const FOREVER: Duration = Duration::from_secs(3600);

fn c_rules() -> ColorRules {
    let types = DocumentTypes::builtin();
    let index = types.index_of_name("C/C++").unwrap();
    ColorRules::from_type(types.get(index).unwrap(), types.keywords())
}

fn doc(text: &str) -> Document {
    Document::from_utf8(text.as_bytes().to_vec()).unwrap()
}

/// The states of all lines, made from scratch.
fn all_states(doc: &Document, rules: &ColorRules) -> Vec<ColorState> {
    let mut states = ColorStates::new();
    assert!(states.advance(doc, rules, FOREVER));
    (0..=doc.line_count())
        .map(|l| states.state_at(doc, rules, l).0)
        .collect()
}

#[test]
fn states_follow_a_block_comment_over_lines() {
    let rules = c_rules();
    let d = doc("a\n/* b\nc\nd */ e\nf\n");
    let mut states = ColorStates::new();
    assert!(states.advance(&d, &rules, FOREVER));
    let at = |l| states.state_at(&d, &rules, l);
    assert_eq!(at(0), (ColorState::Normal, true));
    assert_eq!(at(1), (ColorState::Normal, true));
    assert_eq!(at(2), (ColorState::Block(0), true));
    assert_eq!(at(3), (ColorState::Block(0), true));
    assert_eq!(at(4), (ColorState::Normal, true));
    assert_eq!(at(5), (ColorState::Normal, true));
    assert_eq!(states.known_lines(), 5);
}

#[test]
fn states_carry_strings_and_crlf_lines() {
    let rules = c_rules();
    let d = doc("s = \"a\\\r\nb\"; x\r\n/* c\r\n*/\r\n");
    let states = all_states(&d, &rules);
    assert_eq!(
        states,
        [
            ColorState::Normal,
            ColorState::Quote {
                double: true,
                kind: QuoteKind::Plain
            },
            ColorState::Normal,
            ColorState::Block(0),
            ColorState::Normal,
        ]
    );
}

#[test]
fn a_long_comment_is_one_entry() {
    let rules = c_rules();
    let text = format!("/*\n{}*/\nx\n", "comment\n".repeat(10_000));
    let d = doc(&text);
    let mut states = ColorStates::new();
    states.advance(&d, &rules, FOREVER);
    assert_eq!(states.points.len(), 2);
    assert_eq!(
        states.state_at(&d, &rules, 5_000),
        (ColorState::Block(0), true)
    );
    assert_eq!(states.state_at(&d, &rules, 10_002).0, ColorState::Normal);
}

#[test]
fn rules_without_range_strategies_need_no_scan() {
    let rules = ColorRules::none();
    let d = doc("/* a\nb\n");
    let mut states = ColorStates::new();
    assert!(states.advance(&d, &rules, FOREVER));
    assert_eq!(states.scanned_lines(), 0);
    assert_eq!(states.state_at(&d, &rules, 1), (ColorState::Normal, true));
    assert!(states.is_complete(&d, &rules));
}

#[test]
fn advance_stops_at_the_budget_and_goes_on() {
    let rules = c_rules();
    let d = doc(&"int a;\n".repeat(5_000));
    let mut states = ColorStates::new();
    assert!(!states.advance(&d, &rules, Duration::ZERO));
    let first = states.known_lines();
    assert!(first > 0 && first < 5_000, "{first}");
    assert!(states.advance(&d, &rules, FOREVER));
    assert_eq!(states.known_lines(), 5_000);
}

/// A document of `n` lines of code, with a stray `*/` at line `close`.
fn code_with_close(n: usize, close: usize) -> String {
    (0..n)
        .map(|i| {
            if i == close {
                "*/ int a;\n"
            } else {
                "int a;\n"
            }
        })
        .collect()
}

#[test]
fn an_edit_that_opens_a_comment_scans_only_up_to_where_it_closes() {
    let rules = c_rules();
    let mut d = doc(&code_with_close(20_000, 300));
    let mut states = ColorStates::new();
    states.advance(&d, &rules, FOREVER);
    assert_eq!(states.known_lines(), 20_000);
    let before = states.scanned_lines();

    // Type `/*` at the start of line 100.
    let edit = d.insert(Pos::new(100, 0), "/*").unwrap();
    states.edited(
        &d,
        &rules,
        LineEdit {
            first: edit.start.line,
            old_last: 100,
            new_last: edit.end.line,
        },
        FOREVER,
    );
    let scanned = states.scanned_lines() - before;
    // Lines 100 to 300 are scanned (101 to 300 changed their state); the state of line 301 is as it was.
    assert_eq!(scanned, 201, "{scanned}");
    assert_eq!(states.known_lines(), 20_000);
    assert_eq!(states.state_at(&d, &rules, 100).0, ColorState::Normal);
    assert_eq!(states.state_at(&d, &rules, 101).0, ColorState::Block(0));
    assert_eq!(states.state_at(&d, &rules, 300).0, ColorState::Block(0));
    assert_eq!(states.state_at(&d, &rules, 301).0, ColorState::Normal);
    assert_eq!(states.state_at(&d, &rules, 19_999).0, ColorState::Normal);

    // Take it away again: the same lines go back.
    let before = states.scanned_lines();
    let edit = d.delete(Pos::new(100, 0), Pos::new(100, 2)).unwrap();
    states.edited(
        &d,
        &rules,
        LineEdit {
            first: edit.start.line,
            old_last: 100,
            new_last: edit.end.line,
        },
        FOREVER,
    );
    assert_eq!(states.scanned_lines() - before, 201);
    assert_eq!(states.state_at(&d, &rules, 200).0, ColorState::Normal);
}

#[test]
fn an_edit_that_changes_no_state_scans_the_edited_line_only() {
    let rules = c_rules();
    let mut d = doc(&code_with_close(20_000, 300));
    let mut states = ColorStates::new();
    states.advance(&d, &rules, FOREVER);
    let before = states.scanned_lines();
    let edit = d.insert(Pos::new(10_000, 3), "xyz").unwrap();
    states.edited(
        &d,
        &rules,
        LineEdit {
            first: 10_000,
            old_last: 10_000,
            new_last: edit.end.line,
        },
        FOREVER,
    );
    assert_eq!(states.scanned_lines() - before, 1);
    assert_eq!(states.known_lines(), 20_000);
}

#[test]
fn an_edit_that_adds_lines_moves_the_states_after_it() {
    let rules = c_rules();
    let mut d = doc("a\n/* b\nc\nd */ e\nf\n");
    let mut states = ColorStates::new();
    states.advance(&d, &rules, FOREVER);
    // Two new lines before the comment.
    let edit = d.insert(Pos::new(0, 1), "\nx\ny").unwrap();
    assert_eq!(edit.end.line, 2);
    states.edited(
        &d,
        &rules,
        LineEdit {
            first: 0,
            old_last: 0,
            new_last: 2,
        },
        FOREVER,
    );
    assert_eq!(states.known_lines(), 7);
    assert_eq!(
        (0..=7)
            .map(|l| states.state_at(&d, &rules, l).0)
            .collect::<Vec<_>>(),
        all_states(&d, &rules)
    );
    assert_eq!(states.state_at(&d, &rules, 4), (ColorState::Block(0), true));
}

/// A tiny generator of numbers, so that the test needs no crate and repeats.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n
    }
}

#[test]
fn edits_keep_the_states_equal_to_the_ones_made_from_scratch() {
    let rules = c_rules();
    let pieces = [
        "/*", "*/", "\n", "\"", "'", "//", "x", "\\", "\r\n", "#if 0", "#endif", "R\"(", ")\"",
    ];
    let mut random = Lcg(7);
    for round in 0..40 {
        let mut text = String::new();
        for _ in 0..30 {
            text.push_str(pieces[random.next(pieces.len())]);
            if random.next(3) == 0 {
                text.push('\n');
            }
        }
        let mut d = doc(&text);
        let mut states = ColorStates::new();
        states.advance(&d, &rules, FOREVER);
        for step in 0..25 {
            let count = d.line_count().max(1);
            let line_a = random.next(count);
            let line_b = (line_a + random.next(3)).min(count - 1);
            let len_a = d.line_len(line_a).unwrap().unwrap_or(0);
            let len_b = d.line_len(line_b).unwrap().unwrap_or(0);
            let a = Pos::new(line_a, 0);
            let b = Pos::new(line_b, if line_a == line_b { len_a } else { len_b });
            let a = if random.next(2) == 0 {
                a
            } else {
                Pos::new(line_a, len_a)
            };
            let (a, b) = if a <= b { (a, b) } else { (b, a) };
            let insert = if random.next(4) == 0 {
                ""
            } else {
                pieces[random.next(pieces.len())]
            };
            let old_last = b.line;
            let Ok(edit) = d.replace(a, b, insert) else {
                continue;
            };
            states.edited(
                &d,
                &rules,
                LineEdit {
                    first: edit.start.line,
                    old_last,
                    new_last: edit.end.line,
                },
                FOREVER,
            );
            let want = all_states(&d, &rules);
            let got: Vec<ColorState> = (0..=d.line_count())
                .map(|l| states.state_at(&d, &rules, l).0)
                .collect();
            assert_eq!(
                got, want,
                "round {round} step {step}: replaced {a:?}..{b:?} by {insert:?}"
            );
        }
    }
}

#[test]
fn a_budget_that_runs_out_after_an_edit_leaves_the_rest_to_advance() {
    let rules = c_rules();
    let mut d = doc(&code_with_close(5_000, 4_000));
    let mut states = ColorStates::new();
    states.advance(&d, &rules, FOREVER);
    let edit = d.insert(Pos::new(10, 0), "/*").unwrap();
    states.edited(
        &d,
        &rules,
        LineEdit {
            first: 10,
            old_last: 10,
            new_last: edit.end.line,
        },
        Duration::ZERO,
    );
    // The edited line is done, the lines after it are not all known yet.
    assert!(states.known_lines() >= 11);
    assert!(states.known_lines() < 5_000);
    // A line past what is known is asked by a scan from the last known line.
    let (state, exact) = states.state_at(&d, &rules, states.known_lines() + 5);
    assert_eq!(state, ColorState::Block(0));
    assert!(exact);
    assert!(states.advance(&d, &rules, FOREVER));
    assert_eq!(states.state_at(&d, &rules, 3_999).0, ColorState::Block(0));
    assert_eq!(states.state_at(&d, &rules, 4_001).0, ColorState::Normal);
}

#[test]
fn a_paste_of_many_lines_is_scanned_within_the_budget() {
    let rules = c_rules();
    let mut d = doc(&"int a;\n".repeat(1_000));
    let mut states = ColorStates::new();
    states.advance(&d, &rules, FOREVER);
    let before = states.scanned_lines();
    let pasted = "x /* y\n".repeat(50_000);
    let edit = d.insert(Pos::new(10, 0), &pasted).unwrap();
    states.edited(
        &d,
        &rules,
        LineEdit {
            first: 10,
            old_last: 10,
            new_last: edit.end.line,
        },
        Duration::ZERO,
    );
    // It stopped at the first look at the clock, with the states exact up to there.
    assert_eq!(states.scanned_lines() - before, 64);
    assert_eq!(states.known_lines(), 10 + 64);
    assert_eq!(
        states.state_at(&d, &rules, 40),
        (ColorState::Block(0), true)
    );
    // The rest comes from `advance`, and agrees with a scan from the top.
    assert!(states.advance(&d, &rules, FOREVER));
    let want = all_states(&d, &rules);
    let got: Vec<ColorState> = (0..=d.line_count())
        .map(|l| states.state_at(&d, &rules, l).0)
        .collect();
    assert_eq!(got, want);
}

#[test]
fn a_far_line_is_scanned_from_a_window_above_it_when_the_states_are_not_known() {
    let rules = c_rules();
    // A comment that begins at line 50 and never ends, in 100 000 lines.
    let text = format!(
        "{}/*\n{}",
        "int a;\n".repeat(50),
        "int a;\n".repeat(100_000)
    );
    let d = doc(&text);
    let mut states = ColorStates::new();

    // Near the top the scan goes from the top and is exact.
    let (state, exact) = states.state_at(&d, &rules, 100);
    assert_eq!((state, exact), (ColorState::Block(0), true));

    // Far down it looks at a window only: it does not see the comment, and says so.
    let before = states.scanned_lines();
    let (state, exact) = states.state_at(&d, &rules, 90_000);
    assert_eq!(state, ColorState::Normal);
    assert!(!exact);
    let scanned = states.scanned_lines() - before;
    assert_eq!(scanned, LOCAL_WINDOW as u64);

    // Once the states have come that far it is exact.
    assert!(states.advance(&d, &rules, FOREVER));
    assert_eq!(
        states.state_at(&d, &rules, 90_000),
        (ColorState::Block(0), true)
    );
}

#[test]
fn painting_the_last_screen_of_a_large_file_does_not_visit_every_line() {
    let rules = c_rules();
    let lines = 500_000;
    let text = format!(
        "/* a comment that is closed far below */\n{}x = \"string\"; // end\n",
        "int a = 1; /* b */\n".repeat(lines)
    );
    let d = doc(&text);
    let states = ColorStates::new();
    assert_eq!(d.line_count(), lines + 2);

    // What a paint of the last 60 lines does: the state of the first, then the lines one by one.
    let first = d.line_count() - 60;
    let (mut state, exact) = states.state_at(&d, &rules, first);
    assert!(!exact, "the states are not made yet");
    assert_eq!(state, ColorState::Normal);
    let mut colored = 0;
    for line in first..d.line_count() {
        let l = d.line(line).unwrap().unwrap();
        let runs = crate::line_runs(&rules, &l.text, l.eol, &state, l.text.len());
        colored += runs.len();
        state = line_end_state(&rules, &l.text, l.eol, &state);
    }
    assert!(colored > 60 * 3);
    assert_eq!(state, ColorState::Normal);
    // Only the window above the screen was scanned, not the 500 000 lines.
    assert_eq!(states.scanned_lines(), LOCAL_WINDOW as u64);
    assert_eq!(states.known_lines(), 0);
}

#[test]
fn invalidating_after_a_line_keeps_the_lines_above() {
    let rules = c_rules();
    let d = doc("a\n/* b\nc\nd */ e\nf\n");
    let mut states = ColorStates::new();
    states.advance(&d, &rules, FOREVER);
    states.invalidate_after(2);
    assert_eq!(states.known_lines(), 2);
    assert_eq!(states.state_at(&d, &rules, 2), (ColorState::Block(0), true));
    // Past what is known: a scan from line 2.
    assert_eq!(states.state_at(&d, &rules, 4), (ColorState::Normal, true));
    states.reset();
    assert_eq!(states.known_lines(), 0);
}
