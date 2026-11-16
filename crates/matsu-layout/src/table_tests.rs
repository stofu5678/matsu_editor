//! Tests of the table form of [`Widths`] (ADR-0034, stage 4): the table itself, and the layout
//! and the mapping of positions when each character has the width of a proportional font.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use matsu_core::buffer::Document;

use super::*;

fn x(n: u32) -> X {
    X::from_raw(n)
}

/// A font like Noto Sans JP at the ADR's unit (a half-width `0` is 64): `i` and `l` narrow, `m`
/// and `W` wide, kana 115 and kanji 128, whatever else is 64.
fn proportional_advance(c: char) -> u32 {
    match c {
        'i' | 'l' | '.' => 32,
        ' ' => 26,
        'm' => 107,
        'W' => 101,
        '\u{3000}'..='\u{30FF}' => 115,
        '\u{4E00}'..='\u{9FFF}' => 128,
        '\u{FF61}'..='\u{FF9F}' => 58,
        _ => 64,
    }
}

fn table(space_units: u32) -> Widths {
    Widths::table(64, space_units, proportional_advance)
}

fn config_with(width: Widths, columns: u32) -> LayoutConfig {
    LayoutConfig {
        wrap: Wrap::Columns(columns),
        width,
        ..LayoutConfig::default()
    }
}

fn rows_of(text: &str, has_eol: bool, cfg: &LayoutConfig) -> Vec<Row> {
    let mut rows = Vec::new();
    layout_line(text, has_eol, cfg, &mut rows);
    rows
}

fn texts<'a>(text: &'a str, rows: &[Row]) -> Vec<&'a str> {
    rows.iter().map(|r| r.text(text)).collect()
}

// ---- the table ----

#[test]
fn a_character_is_measured_once_and_then_remembered() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    let t = WidthTable::new(64, 0, move |c| {
        counted.fetch_add(1, Ordering::Relaxed);
        proportional_advance(c)
    });
    assert_eq!(
        calls.load(Ordering::Relaxed),
        0,
        "nothing is measured on creation"
    );
    assert_eq!(t.measured_in_bmp(), 0);
    assert_eq!(t.width_of('m'), x(107));
    assert_eq!(t.width_of('m'), x(107));
    assert_eq!(t.width_of('i'), x(32));
    assert_eq!(calls.load(Ordering::Relaxed), 2);
    assert_eq!(t.measured_in_bmp(), 2);
}

#[test]
fn the_bmp_is_kept_in_pages_that_exist_when_they_are_used() {
    let t = WidthTable::new(64, 0, proportional_advance);
    assert_eq!(t.pages(), 0);
    t.width_of('a');
    t.width_of('z');
    // 'a' and 'z' are in the page of U+0000..=U+00FF.
    assert_eq!(t.pages(), 1);
    t.width_of('あ');
    t.width_of('い');
    // U+3042 and U+3044 share a page.
    assert_eq!(t.pages(), 2);
    t.width_of('\u{FFFF}');
    assert_eq!(t.pages(), 3);
}

#[test]
fn characters_outside_the_bmp_have_their_own_map_and_take_two_columns_of_spacing() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    let t = WidthTable::new(64, 16, move |_| {
        counted.fetch_add(1, Ordering::Relaxed);
        100
    });
    // A width of 100 is wider than a half-width letter: two columns of spacing, and outside the
    // BMP it is two columns whatever the width (`GetKetaOfChar`).
    assert_eq!(t.width_of('😀'), x(100 + 2 * 16));
    assert_eq!(t.width_of('😀'), x(132));
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(t.measured_in_bmp(), 0, "the BMP was not touched");
    assert_eq!(t.pages(), 0);
    let narrow = WidthTable::new(64, 16, |_| 10);
    assert_eq!(narrow.width_of('😀'), x(10 + 32));
    assert_eq!(narrow.columns_of('😀'), 2);
}

#[test]
fn a_width_is_at_least_one_x() {
    // `QueryPixelWidth`: std::max(1, size.cx), so a combining mark is not 0.
    let t = WidthTable::new(64, 0, |_| 0);
    assert_eq!(t.width_of('\u{0301}'), x(1));
    assert_eq!(t.width_of('😀'), x(1));
    // Never 0 either way, so that "0 = not measured yet" holds.
    assert!(
        (0..0x3000u32)
            .filter_map(char::from_u32)
            .all(|c| t.width_of(c).raw() >= 1)
    );
}

#[test]
fn the_spacing_is_added_for_each_column_of_the_character() {
    // Half 64, spacing 16: per_column 80. A narrow letter takes one column of spacing, a wide one
    // two (`GetLayoutXOfChar`: GetKetaOfChar * spacing), so a full-width kana takes it twice.
    let w = table(16);
    assert_eq!(w.per_column(), 80);
    assert_eq!(w.width_of('0'), x(64 + 16));
    assert_eq!(w.width_of('i'), x(32 + 16));
    // exactly a half-width letter: still one column
    assert_eq!(w.width_of('a'), x(80));
    assert_eq!(w.width_of('m'), x(107 + 32));
    assert_eq!(w.width_of('あ'), x(115 + 32));
    assert_eq!(
        (
            w.columns('i'),
            w.columns('a'),
            w.columns('m'),
            w.columns('あ')
        ),
        (1, 1, 2, 2)
    );
}

#[test]
fn without_spacing_the_widths_are_the_advances() {
    let w = table(0);
    assert_eq!(w.per_column(), 64);
    for c in ['0', 'i', 'm', 'W', ' ', 'あ', '漢', 'ｱ'] {
        assert_eq!(w.width_of(c), x(proportional_advance(c)), "{c:?}");
    }
    assert_eq!((w.to_x(3), w.to_columns(x(130))), (x(192), 2));
}

#[test]
fn tables_are_equal_by_generation_not_by_content() {
    let a = table(0);
    let b = a.clone();
    assert_eq!(a, b);
    // Measuring changes what a table holds, not what it is.
    a.width_of('m');
    assert_eq!(a, b);
    // The same font and spacing made again is another generation.
    let c = table(0);
    assert_ne!(a, c);
    assert_ne!(table(0), table(16));
    // A grid is not a table, and `with_per_column` leaves a table alone.
    assert_ne!(Widths::default(), a);
    assert_eq!(a.clone().with_per_column(7), a);
    let (Widths::Table(ta), Widths::Table(tb)) = (&a, &c) else {
        panic!("tables");
    };
    assert!(ta.generation() < tb.generation());
}

#[test]
fn a_config_with_the_same_table_is_equal_and_a_new_table_is_not() {
    let w = table(0);
    let a = config_with(w.clone(), 20);
    assert_eq!(a, config_with(w, 20));
    assert_ne!(a, config_with(table(0), 20));
}

#[test]
fn a_table_has_no_widest_character() {
    assert_eq!(table(0).widest_char(), None);
    assert_eq!(config_with(table(0), 20).widest_character(), None);
    assert!(LayoutConfig::default().widest_character().is_some());
}

#[test]
fn threads_can_use_one_table_together() {
    let t = WidthTable::new(64, 0, proportional_advance);
    let sums: Vec<u32> = std::thread::scope(|scope| {
        let work = || {
            (0x20u32..0x2000)
                .filter_map(char::from_u32)
                .map(|c| t.width_of(c).raw())
                .sum::<u32>()
        };
        let handles = [scope.spawn(work), scope.spawn(work), scope.spawn(work)];
        handles.map(|h| h.join().unwrap()).to_vec()
    });
    assert!(sums.iter().all(|s| *s == sums[0]));
}

// ---- layout ----

#[test]
fn letters_take_their_own_width_in_a_row() {
    let cfg = config_with(table(0), 100);
    let rows = rows_of("mmii", false, &cfg);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].end_x, x(107 + 107 + 32 + 32));
    // 8 columns would be 512; the same eight characters of a grid are 8 columns.
    let grid = rows_of(
        "mmii",
        false,
        &config_with(Widths::grid_scaled(WidthRule::default(), 64), 100),
    );
    assert_eq!(grid[0].end_x, x(4 * 64));
}

#[test]
fn kana_and_kanji_come_from_the_table_and_not_from_two_columns() {
    let cfg = config_with(table(0), 100);
    let rows = rows_of("あ漢ｱ", false, &cfg);
    assert_eq!(rows[0].end_x, x(115 + 128 + 58));
    assert_ne!(rows[0].end_x, x(5 * 64), "not 2 + 2 + 1 columns");
}

#[test]
fn the_wrap_column_is_columns_of_x_so_narrow_letters_fit_more_of_them() {
    // 10 columns are 640 X: 20 `i` (32 each) fit, 10 `m` do not (107 each: 5 fit, 640 / 107).
    let cfg = config_with(table(0), 10);
    let line = "i".repeat(25);
    let rows = rows_of(&line, false, &cfg);
    assert_eq!(texts(&line, &rows), [&line[..20], &line[20..]]);
    assert_eq!(rows[0].end_x, x(640));
    let line = "m".repeat(7);
    let rows = rows_of(&line, false, &cfg);
    assert_eq!(texts(&line, &rows), ["mmmmm", "mm"]);
    assert_eq!(rows[0].end_x, x(535));
}

#[test]
fn a_tab_goes_to_the_next_stop_in_x_whatever_the_letters_before_it_are() {
    // Tab width 4 columns: stops every 256 X.
    let cfg = config_with(table(0), 100);
    let rows = rows_of("m\tx", false, &cfg);
    // m = 107, the tab to 256, x = 64
    assert_eq!(rows[0].end_x, x(256 + 64));
    assert_eq!(cfg.advance('\t', x(107)), x(149));
}

#[test]
fn the_line_ending_is_one_column() {
    let cfg = config_with(table(0), 10);
    // 10 columns hold 20 `i`: the line ending (one column of 64 X) does not fit after them, so
    // it goes on a row of its own as with a grid.
    let line = "i".repeat(20);
    let rows = rows_of(&line, true, &cfg);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].range, 20..20);
    // 19 `i` leave 32 X: not enough for it either
    let rows = rows_of(&"i".repeat(19), true, &cfg);
    assert_eq!(rows.len(), 2);
    // 18 `i` leave 64 X: it fits, and the row ends where the text does
    let rows = rows_of(&"i".repeat(18), true, &cfg);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].end_x, x(18 * 32));
}

#[test]
fn word_wrap_adds_up_the_widths_of_the_letters_of_the_word() {
    let cfg = LayoutConfig {
        word_wrap: true,
        ..config_with(table(0), 10)
    };
    // 640 X per row. "iiii " = 4 * 32 + 26 = 154; the word "mmmm" is 428: 154 + 428 = 582 < 640
    // fits. A longer word would not: "mmmmm" is 535, and 154 + 535 = 689 >= 640 moves it down.
    let fits = "iiii mmmm";
    assert_eq!(texts(fits, &rows_of(fits, false, &cfg)), ["iiii mmmm"]);
    let breaks = "iiii mmmmm";
    assert_eq!(
        texts(breaks, &rows_of(breaks, false, &cfg)),
        ["iiii ", "mmmmm"]
    );
}

#[test]
fn hanging_punctuation_and_kinsoku_use_the_widths_of_the_table() {
    let cfg = LayoutConfig {
        kinsoku: Kinsoku {
            kuto: true,
            ..Kinsoku::default()
        },
        ..config_with(table(0), 10)
    };
    // 5 `m` are 535 X, 105 are left, and "。" is 115 in this font: it does not fit, so it hangs
    // over the edge instead of starting the next row.
    let line = "mmmmm。m";
    assert_eq!(texts(line, &rows_of(line, false, &cfg)), ["mmmmm。", "m"]);
}

// ---- mapping positions ----

#[test]
fn a_position_is_the_sum_of_the_widths_before_it() {
    let cfg = config_with(table(0), 100);
    let text = "imあ";
    let rows = rows_of(text, false, &cfg);
    let at = |offset| offset_to_position(text, &rows, &cfg, offset).x;
    assert_eq!(at(0), x(0));
    assert_eq!(at(1), x(32));
    assert_eq!(at(2), x(32 + 107));
    assert_eq!(at(text.len()), x(32 + 107 + 115));
}

#[test]
fn a_click_between_letters_finds_the_character_whose_width_holds_it() {
    let cfg = config_with(table(0), 100);
    let text = "imあ";
    let rows = rows_of(text, false, &cfg);
    let offset = |px| position_to_offset(text, &rows, &cfg, 0, x(px)).offset;
    // i: 0..32, m: 32..139, あ: 139..254
    assert_eq!(offset(0), 0);
    assert_eq!(offset(31), 0);
    assert_eq!(offset(32), 1);
    assert_eq!(offset(138), 1);
    assert_eq!(offset(139), 2);
    assert_eq!(offset(253), 2);
    let past = position_to_offset(text, &rows, &cfg, 0, x(300));
    assert_eq!((past.offset, past.extra_x), (text.len(), x(46)));
}

#[test]
fn positions_round_trip_with_spacing() {
    let cfg = config_with(table(16), 12);
    let text = "mm iii あい漢字 mWi wrapped around the edge";
    let rows = rows_of(text, true, &cfg);
    for (offset, _) in text.char_indices() {
        let p = offset_to_position(text, &rows, &cfg, offset);
        let back = position_to_offset(text, &rows, &cfg, p.row, p.x);
        assert_eq!(back.offset, offset, "{offset}");
    }
}

#[test]
fn the_columns_of_a_document_layout_follow_the_table() {
    let doc = Document::from_utf8("mmmmmmmmm\niiiiiiiiiiiiiiiiii\n".as_bytes().to_vec()).unwrap();
    let cfg = config_with(table(0), 10);
    let layout = DocumentLayout::laid_out(&doc, cfg).unwrap();
    // Line 0: 9 `m` are 963 X over a row of 640: two rows. Line 1: 18 `i` and the line ending are
    // 640: one row.
    assert_eq!(
        layout.row_count(),
        2 + 1 + 1,
        "the empty row after the last line ending too"
    );
}

#[test]
fn a_huge_line_is_not_taken_for_one_row_without_looking_at_it() {
    // With a grid the longest a character can be bounds the row count; with a table there is no
    // such bound, so a line of 200 bytes (at most 200 * 2 columns is 400 columns > 10) is laid out.
    let text = "m".repeat(200);
    let doc = Document::from_utf8(text.into_bytes()).unwrap();
    let layout = DocumentLayout::laid_out(&doc, config_with(table(0), 10)).unwrap();
    assert_eq!(layout.row_count(), 200usize.div_ceil(5));
    // A table of narrow letters makes few rows from the same bytes: the answer comes from the
    // widths, which the shortcut of a grid would have got wrong.
    let narrow = "i".repeat(200);
    let doc = Document::from_utf8(narrow.into_bytes()).unwrap();
    let layout = DocumentLayout::laid_out(&doc, config_with(table(0), 10)).unwrap();
    assert_eq!(layout.row_count(), 200usize.div_ceil(20));
}

#[test]
fn a_table_measures_only_the_characters_the_layout_reads() {
    let seen = Arc::new(Mutex::new(std::collections::BTreeSet::new()));
    let log = Arc::clone(&seen);
    let w = Widths::table(64, 0, move |c| {
        log.lock().unwrap().insert(c);
        64
    });
    let doc = Document::from_utf8("abc\nabd\n".as_bytes().to_vec()).unwrap();
    let _ = DocumentLayout::laid_out(&doc, config_with(w, 5)).unwrap();
    let seen: String = seen.lock().unwrap().iter().collect();
    assert_eq!(seen, "abcd");
}
