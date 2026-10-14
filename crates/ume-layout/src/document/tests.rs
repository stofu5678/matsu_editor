//! Tests of the layout index. The reference for every check is the plain way: `layout_line` on
//! every line of the document, with no shortcut and no incremental update. The EOF row follows
//! `CLayoutMgr` (no `CLayout` for the empty text after the last line ending; the view puts the
//! EOF at row `GetLineCount()`).

use std::time::Duration;

use ume_core::buffer::{Document, Pos};

use super::*;
use crate::config::{Wrap, WrapIndent};
use crate::kinsoku::Kinsoku;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn doc(text: &str) -> Document {
    Document::from_utf8(text.as_bytes().to_vec()).unwrap()
}

fn wrap(columns: u32) -> LayoutConfig {
    LayoutConfig {
        wrap: Wrap::Columns(columns),
        ..LayoutConfig::default()
    }
}

/// The configurations the random tests go through.
fn configs() -> Vec<LayoutConfig> {
    vec![
        wrap(10),
        wrap(4),
        LayoutConfig {
            word_wrap: true,
            kinsoku: Kinsoku {
                head: true,
                tail: true,
                kuto: true,
                ..Kinsoku::default()
            },
            indent: WrapIndent::leading_white_space(),
            ..wrap(12)
        },
        LayoutConfig {
            tab_width: 3,
            indent: WrapIndent::AfterLastTab,
            ..wrap(7)
        },
        LayoutConfig::default(),
    ]
}

fn lines_of(doc: &Document) -> Vec<(String, bool)> {
    doc.lines(0..doc.line_count())
        .map(|l| {
            let l = l.unwrap();
            (l.text, l.eol.is_some())
        })
        .collect()
}

/// The rows of every line, the plain way.
fn reference(doc: &Document, cfg: &LayoutConfig) -> Vec<Vec<Row>> {
    lines_of(doc)
        .iter()
        .map(|(text, eol)| {
            let mut rows = Vec::new();
            layout_line(text, *eol, cfg, &mut rows);
            rows
        })
        .collect()
}

fn ends_with_eol_or_empty(doc: &Document) -> bool {
    match lines_of(doc).last() {
        None => true,
        Some((_, eol)) => *eol,
    }
}

/// Everything the index says must agree with the reference.
fn assert_matches(idx: &DocumentLayout, doc: &Document, cfg: &LayoutConfig) {
    let want = reference(doc, cfg);
    assert_eq!(idx.line_count(), want.len());
    assert!(idx.is_complete());
    let mut row = 0;
    for (line, rows) in want.iter().enumerate() {
        assert_eq!(idx.rows_in_line(line), Some(rows.len()), "line {line}");
        assert_eq!(idx.first_row_of_line(line), row, "line {line}");
        for k in 0..rows.len() {
            assert_eq!(
                idx.locate_row(row + k),
                RowRef {
                    line,
                    row_in_line: k
                }
            );
        }
        row += rows.len();
    }
    assert_eq!(idx.rows_in_line(want.len()), None);
    assert_eq!(idx.has_eof_row(), ends_with_eol_or_empty(doc));
    assert_eq!(idx.row_count(), row + usize::from(idx.has_eof_row()));
    assert_eq!(idx.eof_row(), idx.row_count() - 1);
    let eof = idx.locate_row(idx.row_count() + 5);
    if idx.has_eof_row() {
        assert_eq!(
            eof,
            RowRef {
                line: want.len(),
                row_in_line: 0
            }
        );
    } else {
        assert_eq!(eof.line, want.len() - 1);
    }

    // The rows as the painter gets them.
    let infos = idx.rows(doc, 0, usize::MAX).unwrap();
    assert_eq!(infos.len(), idx.row_count());
    let texts = lines_of(doc);
    let mut i = 0;
    for (line, rows) in want.iter().enumerate() {
        for (k, r) in rows.iter().enumerate() {
            let info = &infos[i];
            assert_eq!((info.row, info.line, info.row_in_line), (i, line, k));
            assert_eq!(info.start, Pos::new(line, r.range.start));
            assert_eq!(info.end, Pos::new(line, r.range.end));
            assert_eq!(info.wrapped, r.wrapped);
            assert!(!info.is_eof_row);
            assert_eq!(info.text(), &texts[line].0[r.range.clone()]);
            i += 1;
        }
    }
    if idx.has_eof_row() {
        let last = infos.last().unwrap();
        assert!(last.is_eof_row);
        assert_eq!(last.start, Pos::new(want.len(), 0));
    }

    // Positions: every char boundary maps to a row and back to the same position.
    for (line, (text, _)) in texts.iter().enumerate() {
        let mut offsets: Vec<usize> = text.char_indices().map(|(o, _)| o).collect();
        offsets.push(text.len());
        // Every position of a very long line would take quadratic time; take a sample.
        let step = offsets.len() / 400 + 1;
        for col in offsets.into_iter().step_by(step).chain([text.len()]) {
            let pos = Pos::new(line, col);
            let at = idx.pos_to_row(doc, pos).unwrap();
            let back = idx.row_to_pos(doc, at.row, at.column).unwrap();
            assert_eq!(back.pos, pos, "{pos:?} -> {at:?}");
            assert_eq!(back.extra_columns, 0);
            // The row is a row of this line.
            let r = idx.locate_row(at.row);
            assert_eq!(r.line, line);
        }
    }
    if idx.has_eof_row() {
        let end = doc.end_pos().unwrap();
        let at = idx.pos_to_row(doc, end).unwrap();
        assert_eq!(
            at,
            RowPos {
                row: idx.eof_row(),
                column: 0
            }
        );
        let back = idx.row_to_pos(doc, at.row, 7).unwrap();
        assert_eq!((back.pos, back.extra_columns), (end, 7));
    }
}

// ---- the EOF row ----

#[test]
fn empty_document_has_only_the_eof_row() {
    let d = doc("");
    let idx = DocumentLayout::laid_out(&d, wrap(10)).unwrap();
    assert_eq!(idx.line_count(), 0);
    assert_eq!(idx.row_count(), 1);
    assert!(idx.has_eof_row());
    assert_eq!(idx.eof_row(), 0);
    assert_eq!(idx.locate_row(0).line, 0);
    assert_eq!(
        idx.pos_to_row(&d, Pos::new(0, 0)).unwrap(),
        RowPos { row: 0, column: 0 }
    );
    assert_matches(&idx, &d, &wrap(10));
}

#[test]
fn eof_row_exists_only_after_a_line_ending() {
    // CLayoutMgr has no row for the empty text after the last line ending; the view draws the
    // EOF at row GetLineCount(). The index counts that row.
    for (text, lines, rows, eof_row, trailing) in [
        ("abc", 1, 1, 0, false),
        ("abc\n", 1, 2, 1, true),
        ("abc\r\n", 1, 2, 1, true),
        ("abc\n\n", 2, 3, 2, true),
        ("\n", 1, 2, 1, true),
        ("a\nb", 2, 2, 1, false),
    ] {
        let d = doc(text);
        let idx = DocumentLayout::laid_out(&d, wrap(10)).unwrap();
        assert_eq!(idx.line_count(), lines, "{text:?}");
        assert_eq!(idx.row_count(), rows, "{text:?}");
        assert_eq!(idx.eof_row(), eof_row, "{text:?}");
        assert_eq!(idx.has_eof_row(), trailing, "{text:?}");
        assert_matches(&idx, &d, &wrap(10));
    }
}

#[test]
fn a_full_line_puts_its_line_ending_on_its_own_row() {
    // layout_line counts the line ending as one column (CNativeW::GetHabaOfChar).
    let d = doc("0123456789\nabc");
    let idx = DocumentLayout::laid_out(&d, wrap(10)).unwrap();
    assert_eq!(idx.rows_in_line(0), Some(2));
    assert_eq!(idx.rows_in_line(1), Some(1));
    let row = idx.row(&d, 1).unwrap();
    assert_eq!((row.start, row.end), (Pos::new(0, 10), Pos::new(0, 10)));
    assert_matches(&idx, &d, &wrap(10));
}

// ---- rows, lines and positions ----

#[test]
fn rows_and_positions_of_a_wrapped_document() {
    let d = doc("abcdefghijklmnopqrstuvwxyz\nx\n日本語のテキストです。\n");
    let cfg = wrap(10);
    let idx = DocumentLayout::laid_out(&d, cfg.clone()).unwrap();
    // 26 chars: 10 + 10 + 6 (+ the line ending fits), "x", 12 full-width chars: 20 + 4 columns...
    assert_eq!(idx.rows_in_line(0), Some(3));
    assert_eq!(idx.rows_in_line(1), Some(1));
    assert_eq!(idx.first_row_of_line(1), 3);
    assert_eq!(idx.first_row_of_line(2), 4);
    assert_eq!(
        idx.pos_to_row(&d, Pos::new(0, 12)).unwrap(),
        RowPos { row: 1, column: 2 }
    );
    // The start of a wrapped row is at the left of that row, not the right end of the previous.
    assert_eq!(
        idx.pos_to_row(&d, Pos::new(0, 10)).unwrap(),
        RowPos { row: 1, column: 0 }
    );
    let t = idx.row_to_pos(&d, 1, 2).unwrap();
    assert_eq!(t.pos, Pos::new(0, 12));
    // Full-width chars: a column inside one snaps to its left edge.
    let t = idx.row_to_pos(&d, 4, 3).unwrap();
    assert_eq!(t.pos, Pos::new(2, 3));
    // Past the end of the row: the end and the extra columns.
    let t = idx.row_to_pos(&d, 3, 5).unwrap();
    assert_eq!((t.pos, t.extra_columns), (Pos::new(1, 1), 4));
    // A row past the last one is the last one.
    let t = idx.row_to_pos(&d, 999, 0).unwrap();
    assert_eq!(t.row, idx.eof_row());
    assert_matches(&idx, &d, &cfg);
}

#[test]
fn moving_up_and_down_keeps_the_goal_column() {
    let d = doc("abcdef\n日本語\nx\n");
    let idx = DocumentLayout::laid_out(&d, wrap(80)).unwrap();
    let down = idx.move_rows(&d, Pos::new(0, 3), 1, 3).unwrap();
    // Column 3 is inside the second full-width char (columns 2-3): its left edge.
    assert_eq!(down.pos, Pos::new(1, 3));
    let down = idx.move_rows(&d, down.pos, 1, 3).unwrap();
    assert_eq!((down.pos, down.extra_columns), (Pos::new(2, 1), 2));
    // Down from the last line goes to the EOF row, and not further.
    let eof = idx.move_rows(&d, down.pos, 1, 3).unwrap();
    assert_eq!(eof.pos, Pos::new(3, 0));
    let stay = idx.move_rows(&d, eof.pos, 5, 0).unwrap();
    assert_eq!(stay.pos, Pos::new(3, 0));
    let up = idx.move_rows(&d, eof.pos, -100, 0).unwrap();
    assert_eq!(up.pos, Pos::new(0, 0));
}

#[test]
fn vertical_moves_across_wrapped_rows() {
    let d = doc("abcdefghijklmnopqrst\nuvw");
    let idx = DocumentLayout::laid_out(&d, wrap(8)).unwrap();
    // Rows: abcdefgh / ijklmnop / qrst, uvw.
    let t = idx.move_rows(&d, Pos::new(0, 2), 1, 2).unwrap();
    assert_eq!(t.pos, Pos::new(0, 10));
    let t = idx.move_rows(&d, t.pos, 2, 2).unwrap();
    assert_eq!(t.pos, Pos::new(1, 2));
}

// ---- long lines ----

#[test]
fn no_wrap_still_breaks_very_long_lines() {
    // MAXLINEKETAS = 10240 (CEditDoc.cpp:781, config/app_constants.h).
    let long = "a".repeat(25_000);
    let text = format!("short\n{long}\nend\n");
    let d = doc(&text);
    let cfg = LayoutConfig::default();
    let idx = DocumentLayout::laid_out(&d, cfg.clone()).unwrap();
    assert_eq!(idx.rows_in_line(0), Some(1));
    assert_eq!(idx.rows_in_line(1), Some(3));
    assert_eq!(idx.rows_in_line(2), Some(1));
    assert_eq!(idx.first_row_of_line(2), 4);
    assert_matches(&idx, &d, &cfg);
}

#[test]
fn the_shortcut_is_exact_at_the_wrap_column() {
    // Lines around the limit, with the line ending, tabs and full-width chars.
    let cfg = wrap(40);
    for n in 0..60 {
        for unit in ["a", "日", "\t", "😀"] {
            let text = format!("{}\n{}x", unit.repeat(n), unit.repeat(n));
            let d = doc(&text);
            let idx = DocumentLayout::laid_out(&d, cfg.clone()).unwrap();
            let want = reference(&d, &cfg);
            assert_eq!(idx.rows_in_line(0), Some(want[0].len()), "{unit:?} x {n}");
            assert_eq!(idx.rows_in_line(1), Some(want[1].len()), "{unit:?} x {n}");
        }
    }
}

// ---- settings ----

#[test]
fn a_change_of_the_settings_lays_everything_out_again() {
    let d = doc("abcdefghijklmnopqrstuvwxyz\n\tabc\n");
    let mut idx = DocumentLayout::laid_out(&d, wrap(10)).unwrap();
    assert_eq!(idx.rows_in_line(0), Some(3));
    idx.set_config(&d, wrap(5)).unwrap();
    assert!(!idx.is_complete());
    idx.layout_all(&d).unwrap();
    assert_matches(&idx, &d, &wrap(5));
    assert_eq!(idx.rows_in_line(0), Some(6));
    idx.set_config(&d, LayoutConfig::default()).unwrap();
    idx.layout_all(&d).unwrap();
    assert_eq!(idx.rows_in_line(0), Some(1));
    // The same settings again: nothing is thrown away.
    idx.set_config(&d, LayoutConfig::default()).unwrap();
    assert!(idx.is_complete());
}

// ---- the first layout in chunks ----

#[test]
fn the_first_layout_goes_in_chunks_and_the_done_part_is_exact() {
    let mut text = String::new();
    for i in 0..1000 {
        text.push_str(&"x".repeat(i % 37));
        text.push('\n');
    }
    let d = doc(&text);
    let cfg = wrap(10);
    let mut idx = DocumentLayout::new(&d, cfg.clone()).unwrap();
    assert_eq!(
        idx.progress(),
        Progress {
            done: 0,
            total: 1000
        }
    );
    assert_eq!(idx.row_count(), 1001);
    let want = reference(&d, &cfg);
    let mut last = 0;
    while !idx.is_complete() {
        let p = idx.layout_chunk(&d, 123).unwrap();
        assert!(p.done > last);
        last = p.done;
        for (line, rows) in want.iter().enumerate().take(p.done) {
            assert_eq!(idx.rows_in_line(line), Some(rows.len()));
        }
        // A line that is not done still has its own rows right.
        let probe = (p.done + 5).min(999);
        let rows = idx.line_layout(&d, probe).unwrap().unwrap();
        assert_eq!(rows.rows.len(), want[probe].len());
    }
    assert_matches(&idx, &d, &cfg);
    // The time-boxed version gets there too.
    let mut idx = DocumentLayout::new(&d, cfg.clone()).unwrap();
    while !idx
        .layout_for(&d, Duration::from_micros(50))
        .unwrap()
        .is_complete()
    {}
    assert_matches(&idx, &d, &cfg);
}

#[test]
fn queries_agree_with_the_counts_before_the_layout_reaches_a_line() {
    let d = doc("abcdefghijkl\nxy\nmnopqrstuvwx\n");
    let cfg = wrap(4);
    let mut idx = DocumentLayout::new(&d, cfg.clone()).unwrap();
    // Nothing is laid out: one row a line, plus the EOF row.
    assert_eq!(idx.row_count(), 4);
    for row in 0..idx.row_count() {
        let info = idx.row(&d, row).unwrap();
        assert_eq!(info.row, row);
        assert_eq!(idx.row_to_pos(&d, row, 0).unwrap().pos, info.start);
        assert_eq!(idx.pos_to_row(&d, info.start).unwrap().row, row);
    }
    assert_eq!(idx.row(&d, 0).unwrap().text(), "abcd");
    // The rows of the second line are made exact; the rows before it do not move.
    idx.layout_rows(&d, 1, 1).unwrap();
    assert_eq!(idx.rows_in_line(0), Some(1));
    assert_eq!(idx.rows_in_line(1), Some(1));
    // Line 0 has 4 rows (12 columns and the line ending on its own row); the window grows as the
    // lines in it turn out to be longer.
    idx.layout_rows(&d, 0, 7).unwrap();
    assert_eq!(idx.rows_in_line(0), Some(4));
    assert_eq!(idx.rows_in_line(1), Some(1));
    assert_eq!(idx.rows_in_line(2), Some(4));
    assert_eq!(idx.row(&d, 2).unwrap().text(), "ijkl");
    assert_eq!(idx.row(&d, 3).unwrap().text(), "");
    assert_eq!(idx.row(&d, 4).unwrap().text(), "xy");
    idx.layout_all(&d).unwrap();
    assert_matches(&idx, &d, &cfg);
}

// ---- random edits ----

const PIECES: [&str; 14] = [
    "a",
    "bc",
    " ",
    "\t",
    "日本",
    "\n",
    "\r\n",
    "\r",
    "😀",
    "word ",
    "、",
    "。」",
    "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    "\n\n",
];

fn random_text(rng: &mut Rng) -> String {
    (0..rng.below(5))
        .map(|_| PIECES[rng.below(PIECES.len())])
        .collect()
}

fn random_pos(rng: &mut Rng, d: &Document) -> Pos {
    Pos::new(rng.below(d.line_count() + 2), rng.below(40))
}

fn random_edit(rng: &mut Rng, d: &mut Document) -> Edit {
    let (a, b) = (random_pos(rng, d), random_pos(rng, d));
    // Mostly small edits; sometimes a deletion across many lines or a big paste.
    let text = match rng.below(12) {
        0 => PIECES[5].repeat(rng.below(600)),
        1 => String::new(),
        _ => random_text(rng),
    };
    let b = if rng.below(4) == 0 { a } else { b };
    d.replace(a, b, &text).unwrap()
}

#[test]
fn random_edits_match_a_full_layout_after_every_step() {
    for (n, cfg) in configs().into_iter().enumerate() {
        for seed in 1..=3u64 {
            let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ (seed * 7919 + n as u64 * 104_729));
            let mut d = doc(&random_text(&mut rng).repeat(3));
            let mut idx = DocumentLayout::laid_out(&d, cfg.clone()).unwrap();
            for step in 0..150 {
                let edit = random_edit(&mut rng, &mut d);
                idx.edited(&d, &edit).unwrap();
                if !idx.is_complete() {
                    // A big paste is counted as one row per line until the chunks reach it.
                    assert_eq!(idx.line_count(), d.line_count());
                    idx.layout_all(&d).unwrap();
                }
                let fresh = DocumentLayout::laid_out(&d, cfg.clone()).unwrap();
                assert_eq!(
                    idx.row_count(),
                    fresh.row_count(),
                    "cfg {n} seed {seed} step {step}"
                );
                assert_eq!(idx.line_count(), fresh.line_count());
                if step % 5 == 0 {
                    assert_matches(&idx, &d, &cfg);
                } else {
                    for line in 0..d.line_count() {
                        assert_eq!(
                            idx.rows_in_line(line),
                            fresh.rows_in_line(line),
                            "cfg {n} seed {seed} step {step} line {line}"
                        );
                    }
                    assert_eq!(idx.has_eof_row(), fresh.has_eof_row());
                }
            }
        }
    }
}

#[test]
fn random_edits_while_the_first_layout_is_going_on() {
    for (n, cfg) in configs().into_iter().enumerate() {
        let mut rng = Rng(0xD1B5_4A32_D192_ED03 ^ n as u64);
        let mut d = doc(&PIECES.join("").repeat(40));
        let mut idx = DocumentLayout::new(&d, cfg.clone()).unwrap();
        for step in 0..200 {
            let edit = random_edit(&mut rng, &mut d);
            idx.edited(&d, &edit).unwrap();
            assert_eq!(idx.line_count(), d.line_count(), "cfg {n} step {step}");
            if rng.below(3) == 0 {
                idx.layout_chunk(&d, 1 + rng.below(30)).unwrap();
            }
            // Whatever is done is exact.
            let done = idx.progress().done;
            let want = reference(&d, &cfg);
            for (line, rows) in want.iter().enumerate().take(done) {
                assert_eq!(
                    idx.rows_in_line(line),
                    Some(rows.len()),
                    "cfg {n} step {step}"
                );
            }
            // The rest counts one row each.
            for line in done..want.len() {
                assert_eq!(idx.rows_in_line(line), Some(1));
            }
        }
        idx.layout_all(&d).unwrap();
        assert_matches(&idx, &d, &cfg);
    }
}

#[test]
fn edits_next_to_line_endings_that_join_and_split() {
    // CR + LF becoming one line ending, and back; the neighbours must follow.
    let cfg = wrap(6);
    let mut d = doc("abc\rdef\nghi\r\n");
    let mut idx = DocumentLayout::laid_out(&d, cfg.clone()).unwrap();
    for (from, to, text) in [
        (Pos::new(0, 3), Pos::new(0, 3), "\n"),
        (Pos::new(0, 3), Pos::new(1, 0), ""),
        (Pos::new(2, 3), Pos::new(2, 3), "\n"),
        (Pos::new(1, 3), Pos::new(2, 0), "xxxxxxxx"),
        (Pos::new(0, 0), Pos::new(9, 0), ""),
        (Pos::new(0, 0), Pos::new(0, 0), "\r"),
        (Pos::new(1, 0), Pos::new(1, 0), "\nabcdefghijkl"),
    ] {
        let edit = d.replace(from, to, text).unwrap();
        idx.edited(&d, &edit).unwrap();
        idx.layout_all(&d).unwrap();
        assert_matches(&idx, &d, &cfg);
    }
}

#[test]
fn lines_changed_without_an_edit_value() {
    let cfg = wrap(8);
    let mut d = doc("one\ntwo\nthree\nfour\n");
    let mut idx = DocumentLayout::laid_out(&d, cfg.clone()).unwrap();
    // Replace lines 1..3 with three longer lines.
    d.replace(
        Pos::new(1, 0),
        Pos::new(3, 0),
        "aaaaaaaaaaaaaaaaaaaa\nb\nccccccccccc\n",
    )
    .unwrap();
    idx.lines_changed(&d, 1, 2).unwrap();
    idx.layout_all(&d).unwrap();
    assert_matches(&idx, &d, &cfg);
    // A wrong claim does not corrupt the index: it starts again from the document.
    idx.lines_changed(&d, 10, 5).unwrap();
    idx.layout_all(&d).unwrap();
    assert_matches(&idx, &d, &cfg);
}

#[test]
fn the_cache_is_dropped_for_the_lines_that_changed() {
    let cfg = wrap(5);
    let mut d = doc("aaaaaaaa\nbbbbbbbb\ncccccccc");
    let mut idx = DocumentLayout::laid_out(&d, cfg.clone()).unwrap();
    // Fill the cache with every line, then edit the middle one and insert a line before it.
    idx.rows(&d, 0, usize::MAX).unwrap();
    let edit = d.replace(Pos::new(1, 0), Pos::new(1, 8), "z").unwrap();
    idx.edited(&d, &edit).unwrap();
    let edit = d.insert(Pos::new(0, 0), "new\n").unwrap();
    idx.edited(&d, &edit).unwrap();
    idx.layout_all(&d).unwrap();
    assert_eq!(idx.row(&d, 1).unwrap().text(), "aaaaa");
    assert_eq!(idx.row(&d, 2).unwrap().text(), "aaa");
    assert_eq!(idx.row(&d, 3).unwrap().text(), "z");
    assert_matches(&idx, &d, &cfg);
}

#[test]
fn many_lines_in_a_big_document_stay_consistent() {
    // More than a few blocks of the sequence, with edits at the start, middle and end.
    let text: String = (0..5000)
        .map(|i| format!("{}\n", "w".repeat(i % 23)))
        .collect();
    let cfg = wrap(10);
    let mut d = doc(&text);
    let mut idx = DocumentLayout::laid_out(&d, cfg.clone()).unwrap();
    for (line, ins) in [
        (0, "q\n"),
        (2500, "qqqqqqqqqqqqqqqqqqqqqqqqqqq\n"),
        (5000, "tail"),
    ] {
        let edit = d.insert(Pos::new(line, 0), ins).unwrap();
        idx.edited(&d, &edit).unwrap();
        assert_eq!(
            idx.row_count(),
            DocumentLayout::laid_out(&d, cfg.clone())
                .unwrap()
                .row_count()
        );
    }
    let edit = d.delete(Pos::new(10, 0), Pos::new(4000, 0)).unwrap();
    idx.edited(&d, &edit).unwrap();
    idx.layout_all(&d).unwrap();
    assert_matches(&idx, &d, &cfg);
}

#[test]
fn a_huge_paste_is_counted_provisionally_and_finished_in_chunks() {
    let cfg = wrap(10);
    let mut d = doc("first line is long enough to wrap\nsecond\n");
    let mut idx = DocumentLayout::laid_out(&d, cfg.clone()).unwrap();
    let paste = "0123456789abcdef\nx\n".repeat(EDIT_EXACT_LIMIT);
    let edit = d.insert(Pos::new(1, 0), &paste).unwrap();
    idx.edited(&d, &edit).unwrap();
    // The pasted lines (and the line before them, which the edit may have changed) are not laid
    // out yet: they count one row each, and the layout goes back to the line before the paste.
    assert_eq!(idx.line_count(), d.line_count());
    assert!(!idx.is_complete());
    assert_eq!(idx.progress().done, 0);
    while !idx.layout_chunk(&d, 3000).unwrap().is_complete() {}
    assert_counts_match(&idx, &d, &cfg);
}

/// `assert_matches` is quadratic in places; for a big document compare the counts only.
fn assert_counts_match(idx: &DocumentLayout, d: &Document, cfg: &LayoutConfig) {
    let want = reference(d, cfg);
    assert_eq!(idx.line_count(), want.len());
    let mut row = 0;
    for (line, rows) in want.iter().enumerate() {
        assert_eq!(idx.rows_in_line(line), Some(rows.len()));
        if line % 97 == 0 {
            assert_eq!(idx.first_row_of_line(line), row);
            assert_eq!(idx.locate_row(row).line, line);
        }
        row += rows.len();
    }
    assert_eq!(idx.row_count(), row + usize::from(idx.has_eof_row()));
}

/// The numbers for the PR (`cargo test -p ume-layout --release -- --ignored --nocapture perf`).
#[test]
#[ignore = "a report, not a test"]
fn perf_report() {
    use std::time::Instant;
    // About 100 MB and 1M lines: 100 bytes a line on average, ASCII with some Japanese and tabs.
    let mut rng = Rng(42);
    let mut text = String::with_capacity(110_000_000);
    while text.len() < 100_000_000 {
        let len = rng.below(160);
        for _ in 0..len {
            match rng.below(40) {
                0 => text.push_str("日本語"),
                1 => text.push('\t'),
                2..=4 => text.push(' '),
                _ => text.push(char::from(b'a' + rng.below(26) as u8)),
            }
        }
        text.push('\n');
    }
    let t = Instant::now();
    let mut d = doc(&text);
    println!(
        "document: {} MB, {} lines, built in {:?}",
        text.len() / 1_000_000,
        d.line_count(),
        t.elapsed()
    );
    drop(text);
    for (name, cfg) in [("wrap 80", wrap(80)), ("no wrap", LayoutConfig::default())] {
        let t = Instant::now();
        let mut idx = DocumentLayout::new(&d, cfg.clone()).unwrap();
        println!("[{name}] new (nothing laid out): {:?}", t.elapsed());
        let t = Instant::now();
        let p = idx.layout_for(&d, Duration::from_millis(16)).unwrap();
        println!("[{name}] one 16 ms slice: {} of {} lines", p.done, p.total);
        let mut slices = 1;
        while !idx
            .layout_for(&d, Duration::from_millis(16))
            .unwrap()
            .is_complete()
        {
            slices += 1;
        }
        println!(
            "[{name}] whole layout: {:?} in {slices} slices of ~16 ms ({} rows)",
            t.elapsed(),
            idx.row_count()
        );

        let t = Instant::now();
        let mut sink = 0;
        for i in 0..100_000 {
            sink += idx.locate_row((i * 7919) % idx.row_count()).line;
        }
        println!(
            "[{name}] locate_row: {:?} each ({sink})",
            t.elapsed() / 100_000
        );
        let t = Instant::now();
        let n = 2000;
        for i in 0..n {
            let rows = idx.rows(&d, (i * 104_729) % idx.row_count(), 60).unwrap();
            sink += rows.len();
        }
        println!(
            "[{name}] 60 visible rows, cold cache: {:?} each ({sink})",
            t.elapsed() / n as u32
        );
        let t = Instant::now();
        for _ in 0..n {
            sink += idx.rows(&d, idx.row_count() / 2, 60).unwrap().len();
        }
        println!(
            "[{name}] 60 visible rows, warm cache: {:?} each ({sink})",
            t.elapsed() / n as u32
        );

        let mut total = Duration::ZERO;
        let mut worst = Duration::ZERO;
        let edits = 2000;
        for _ in 0..edits {
            let pos = Pos::new(rng.below(d.line_count()), rng.below(50));
            let text = ["x", "\n", "日本語", "ab\ncd"][rng.below(4)];
            let edit = d.insert(pos, text).unwrap();
            let t = Instant::now();
            idx.edited(&d, &edit).unwrap();
            let e = t.elapsed();
            total += e;
            worst = worst.max(e);
        }
        println!(
            "[{name}] edited(): {:?} on average, {worst:?} at worst ({edits} edits)",
            total / edits
        );
    }
}
