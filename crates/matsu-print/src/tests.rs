//! Tests of the page layout. The numbers come from working out Sakura's formulas by hand
//! (`CPrint::CalculatePrintableColumns` and `CalculatePrintableLines` in
//! `sakura_core/print/CPrint.cpp`, `CPrintPreview::DrawPageText` and `DrawHeaderFooter` in
//! `sakura_core/print/CPrintPreview.cpp`), and from the measurements of the print PoC
//! (`docs/poc/printing.md`: A4 portrait 150 cells and 82 rows, A4 landscape 222 cells and 54
//! rows, 12,196 pages for a million lines). Sakura has no unit tests for printing.

use super::*;
use matsu_layout::{CharSet, WidthRule, WrapIndent};

fn no_header_footer() -> PrintSetting {
    PrintSetting {
        header_form: Default::default(),
        footer_form: Default::default(),
        ..PrintSetting::default()
    }
}

/// A setting with `cells` cells in a row, no header and no footer, and the paper for it.
fn narrow(cells: i32) -> (PrintSetting, Paper) {
    let s = no_header_footer();
    let width = s.print_margin_lx + s.print_margin_rx + cells * s.print_font_width;
    (s, Paper { width, ..Paper::A4 })
}

/// The same, with room for `rows` rows (the height is 0.1 mm more than needed).
fn small(cells: i32, rows: i32) -> (PrintSetting, Paper) {
    let (s, paper) = narrow(cells);
    // rows = (height - margins + space) / (font height + space), with space 7.
    let height = s.print_margin_ty + s.print_margin_by + rows * 31 - 7;
    (s, Paper { height, ..paper })
}

fn layout<S: LineSource + ?Sized>(src: &S, s: &PrintSetting, paper: Paper) -> Layout {
    Layout::new(src, s, paper, &TypeOptions::default()).unwrap()
}

fn page<S: LineSource + ?Sized>(l: &Layout, i: usize, src: &S) -> Page {
    l.page(i, src, &HeaderContext::default()).unwrap()
}

/// The text of a page by row: the pieces on one baseline, joined (a row of mixed fonts is
/// several pieces).
fn texts(p: &Page) -> Vec<String> {
    let mut rows: Vec<(i32, String)> = Vec::new();
    for t in &p.texts {
        match rows.last_mut() {
            Some((baseline, text)) if *baseline == t.baseline => text.push_str(&t.text),
            _ => rows.push((t.baseline, t.text.clone())),
        }
    }
    rows.into_iter().map(|(_, text)| text).collect()
}

#[test]
fn sakura_defaults_on_a4() {
    let l = layout(&["a"], &PrintSetting::default(), Paper::A4);
    // (2100 - 200 - 100) / 12, and (2970 - 300 - 31 * 4 + 7) / 31.
    assert_eq!((l.cells_per_row(), l.rows_per_column()), (150, 82));
    assert_eq!(l.number_cells(), 0);
}

#[test]
fn landscape_swaps_the_paper() {
    let s = PrintSetting {
        print_paper_orientation: Orientation::Landscape,
        ..PrintSetting::default()
    };
    let paper = Paper::of_setting(&s).unwrap();
    let l = layout(&["a"], &s, paper);
    // The PoC's A4 landscape: 222 characters in a row and 54 rows.
    assert_eq!((l.cells_per_row(), l.rows_per_column()), (222, 54));
}

#[test]
fn pages_break_at_the_exact_boundary() {
    let s = PrintSetting::default();
    let lines = |n: usize| (1..=n).map(|i| i.to_string()).collect::<Vec<_>>();
    for (n, pages) in [(1, 1), (82, 1), (83, 2), (164, 2), (165, 3)] {
        let l = lines(n);
        assert_eq!(layout(&l, &s, Paper::A4).page_count(), pages, "{n} lines");
    }
    // The first row of the second page is line 83, just under the header.
    let l = lines(83);
    let lay = layout(&l, &s, Paper::A4);
    let p = page(&lay, 1, &l);
    assert_eq!(texts(&p), ["(無題)", "83", "- 2 -"]);
    assert_eq!((p.texts[3].x, p.texts[3].baseline), (200, 162 + 24));
    // Every row of the first page is there: the header (in 3 pieces: the parentheses are in
    // the other font), 82 rows, and the footer.
    assert_eq!(texts(&page(&lay, 0, &l)).len(), 84);
    assert!(lay.page(2, &l, &HeaderContext::default()).is_none());
}

#[test]
fn a_million_lines() {
    struct Many;
    impl LineSource for Many {
        fn line_count(&self) -> usize {
            1_000_000
        }
        fn read_line(&self, index: usize, buf: &mut String) -> bool {
            buf.clear();
            buf.push_str(&index.to_string());
            true
        }
    }
    let lay = layout(&Many, &PrintSetting::default(), Paper::A4);
    // The PoC counted 12,196 pages for this: 1,000,000 / 82 rounded up.
    assert_eq!(lay.page_count(), 12_196);
    assert_eq!(lay.row_count(), 1_000_000);
    let last = lay.page(12_195, &Many, &HeaderContext::default()).unwrap();
    // 1,000,000 - 12,195 * 82 = 10 rows, between the header and the footer.
    let rows = texts(&last);
    assert_eq!(rows.len(), 10 + 2);
    assert_eq!(rows[1], (12_195 * 82).to_string());
    assert_eq!(rows[10], (12_195 * 82 + 9).to_string());
}

#[test]
fn empty_documents_have_no_pages() {
    let s = PrintSetting::default();
    let empty = layout(&[""], &s, Paper::A4);
    assert_eq!(empty.page_count(), 0);
    assert!(empty.page(0, &[""], &HeaderContext::default()).is_none());
    assert_eq!(layout(&Vec::<String>::new(), &s, Paper::A4).page_count(), 0);
    // A final line ending is not a line of its own.
    let l = layout(&["a", ""], &s, Paper::A4);
    assert_eq!((l.page_count(), l.row_count()), (1, 1));
    // One line ending is one empty line: a page with the header and the footer only.
    let one = layout(&["", ""], &s, Paper::A4);
    assert_eq!(one.page_count(), 1);
    assert_eq!(texts(&page(&one, 0, &["", ""])), ["(無題)", "- 1 -"]);
}

#[test]
fn wraps_full_width_characters_on_cells() {
    let (s, paper) = narrow(5);
    // "ab" takes 2 cells and "あ" 2 more; "い" would end at the 6th.
    let src = ["abあい"];
    let p = page(&layout(&src, &s, paper), 0, &src);
    assert_eq!(texts(&p), ["abあ", "い"]);
    // The next row is 31 lower (24 and the 30 % space), and starts at the left margin.
    assert_eq!((p.texts[2].x, p.texts[2].baseline), (200, 24 + 100 + 31));
    assert_eq!(p.texts.len(), 3);
    // "ab" and "あ" are on a grid: あ starts after the 2 cells of "ab".
    assert_eq!(p.texts[1].x, 200 + 24);
    assert_eq!(p.texts[1].font, FontKind::Zen);
    assert_eq!(p.texts[0].font, FontKind::Han);
}

#[test]
fn the_rows_wrap_like_matsu_layout() {
    let (s, paper) = narrow(10);
    let src = ["one twelve"];
    // _DoWordWrap breaks before "twelve", which would end at the last cell.
    let p = page(&layout(&src, &s, paper), 0, &src);
    assert_eq!(texts(&p), ["one ", "twelve"]);
    let no_wrap = PrintSetting {
        print_word_wrap: false,
        ..s.clone()
    };
    let p = page(&layout(&src, &no_wrap, paper), 0, &src);
    assert_eq!(texts(&p), ["one twelve"]);
    // A word longer than a row breaks where the row ends.
    let src = ["abcdefghijkl"];
    let p = page(&layout(&src, &s, paper), 0, &src);
    assert_eq!(texts(&p), ["abcdefghij", "kl"]);
}

#[test]
fn a_full_row_puts_its_line_ending_on_the_next_row() {
    let (s, paper) = narrow(4);
    let src = ["あい", "あい"];
    let lay = layout(&src, &s, paper);
    // The line ending of the first line takes a cell, so it has a row of its own, which prints
    // nothing; the last line has no line ending.
    assert_eq!(lay.row_count(), 3);
    let p = page(&lay, 0, &src);
    assert_eq!(texts(&p), ["あい", "あい"]);
    assert_eq!(p.texts[1].baseline - p.texts[0].baseline, 2 * 31);
}

#[test]
fn tabs_move_to_the_next_stop_and_print_nothing() {
    let (s, paper) = narrow(20);
    let src = ["a\tbc\td"];
    let p = page(&layout(&src, &s, paper), 0, &src);
    let placed: Vec<_> = p.texts.iter().map(|t| (t.x, t.text.as_str())).collect();
    // Tab stops every 4 cells: "bc" at the 4th, "d" at the 8th.
    assert_eq!(
        placed,
        [(200, "a"), (200 + 4 * 12, "bc"), (200 + 8 * 12, "d")]
    );
    // A tab on a stop takes the whole width.
    let src = ["abcd\te"];
    let p = page(&layout(&src, &s, paper), 0, &src);
    assert_eq!(p.texts[1].x, 200 + 8 * 12);
    // With another tab width.
    let options = TypeOptions {
        tab_width: 8,
        ..TypeOptions::default()
    };
    let src = ["a\tb"];
    let lay = Layout::new(&src, &s, paper, &options).unwrap();
    assert_eq!(page(&lay, 0, &src).texts[1].x, 200 + 8 * 12);
}

#[test]
fn text_splits_where_the_font_changes() {
    let (s, paper) = narrow(20);
    let src = ["ab日本c"];
    let p = page(&layout(&src, &s, paper), 0, &src);
    let runs: Vec<_> = p
        .texts
        .iter()
        .map(|t| (t.x, t.text.as_str(), t.font))
        .collect();
    assert_eq!(
        runs,
        [
            (200, "ab", FontKind::Han),
            (224, "日本", FontKind::Zen),
            (272, "c", FontKind::Han)
        ]
    );
    // The cells of each character, for the drawer.
    let xs: Vec<_> = p.texts[1].positions(12, WidthRule::default()).collect();
    assert_eq!(xs, [(0, '日', 224), (3, '本', 248)]);
}

#[test]
fn kinsoku_in_print() {
    // "あい。" in 4 cells: the 。 would start the second row.
    let src = ["あい。"];
    let (mut s, paper) = narrow(4);
    assert_eq!(
        texts(&page(&layout(&src, &s, paper), 0, &src)),
        ["あい", "。"]
    );
    // 行頭禁則: い goes down with it.
    s.print_kinsoku_head = true;
    assert_eq!(
        texts(&page(&layout(&src, &s, paper), 0, &src)),
        ["あ", "い。"]
    );
    // 句読点のぶら下げ: 。 hangs over the edge of the first row.
    s.print_kinsoku_head = false;
    s.print_kinsoku_kuto = true;
    let p = page(&layout(&src, &s, paper), 0, &src);
    assert_eq!(texts(&p), ["あい。"]);
    // 行末禁則: 「 does not end a row.
    let src = ["あ「い"];
    let mut s = narrow(4).0;
    assert_eq!(
        texts(&page(&layout(&src, &s, paper), 0, &src)),
        ["あ「", "い"]
    );
    s.print_kinsoku_tail = true;
    assert_eq!(
        texts(&page(&layout(&src, &s, paper), 0, &src)),
        ["あ", "「い"]
    );
}

#[test]
fn kinsoku_characters_come_from_the_document_type() {
    let (mut s, paper) = narrow(4);
    s.print_kinsoku_head = true;
    let src = ["あい。"];
    let options = TypeOptions {
        // 。 is no longer prohibited at the head.
        kinsoku_head: CharSet::new("、"),
        ..TypeOptions::default()
    };
    let lay = Layout::new(&src, &s, paper, &options).unwrap();
    assert_eq!(texts(&page(&lay, 0, &src)), ["あい", "。"]);
}

#[test]
fn wrapped_rows_keep_the_indent() {
    let (mut s, paper) = narrow(8);
    s.print_word_wrap = false;
    let options = TypeOptions {
        indent: WrapIndent::leading_white_space(),
        ..TypeOptions::default()
    };
    let src = ["  abcdefghij"];
    let lay = Layout::new(&src, &s, paper, &options).unwrap();
    let p = page(&lay, 0, &src);
    // The leading spaces are text too; the second row starts under "a".
    assert_eq!(texts(&p), ["  abcdef", "ghij"]);
    assert_eq!(p.texts[1].x, 200 + 2 * 12);
    // With English word wrap the long word goes to the next row whole, after the indent.
    s.print_word_wrap = true;
    let lay = Layout::new(&src, &s, paper, &options).unwrap();
    assert_eq!(texts(&page(&lay, 0, &src)), ["  ", "abcdef", "ghij"]);
}

#[test]
fn a_page_may_begin_in_the_middle_of_a_line() {
    // 3 rows of 4 cells (2 full-width characters) on a page.
    let (s, paper) = small(4, 3);
    let src = ["a", "あああああ"];
    let lay = layout(&src, &s, paper);
    assert_eq!(
        (lay.rows_per_column(), lay.row_count(), lay.page_count()),
        (3, 4, 2)
    );
    assert_eq!(texts(&page(&lay, 0, &src)), ["a", "ああ", "ああ"]);
    // The third row of the second line is the first of the second page.
    let p = page(&lay, 1, &src);
    assert_eq!(texts(&p), ["あ"]);
    assert_eq!((p.texts[0].x, p.texts[0].baseline), (200, 100 + 24));
}

#[test]
fn line_numbers() {
    let src: Vec<String> = (0..120).map(|_| "あ".repeat(80)).collect();
    let s = PrintSetting {
        print_line_number: true,
        header_form: Default::default(),
        footer_form: Default::default(),
        ..PrintSetting::default()
    };
    let lay = layout(&src, &s, Paper::A4);
    // Three digits for 120 lines and a space, out of 150 cells.
    assert_eq!((lay.number_cells(), lay.cells_per_row()), (4, 146));
    let p = page(&lay, 0, &src);
    let numbers: Vec<_> = p
        .texts
        .iter()
        .filter(|t| t.text.chars().all(|c| c.is_ascii_digit()))
        .map(|t| (t.x, t.baseline, t.text.as_str()))
        .collect();
    // Each line wraps once (160 cells in 146), and the wrapped row gets no number. "1" and its
    // space end where the text starts, after the 4 cells of the numbers. Without a header
    // there are (2970 - 300 + 7) / 31 = 86 rows on a page: 43 lines.
    let base_x = 200 + 4 * 12;
    assert_eq!(lay.rows_per_column(), 86);
    assert_eq!(numbers.len(), 43);
    assert_eq!(numbers[0], (base_x - 2 * 12, 124, "1"));
    assert_eq!(numbers[1], (base_x - 2 * 12, 124 + 2 * 31, "2"));
    assert_eq!(numbers[42].2, "43");
    // The vertical line is in the middle of the cell before the text, as long as the rows.
    assert_eq!(
        p.rules,
        [Rule {
            from: (base_x - 6, 100),
            to: (base_x - 6, 100 + 31 * 86),
        }]
    );
    // The second page goes on with the numbers of the lines: row 87 is the 44th line.
    let p = page(&lay, 1, &src);
    let first = p
        .texts
        .iter()
        .find(|t| t.text.chars().all(|c| c.is_ascii_digit()));
    assert_eq!(first.map(|t| t.text.as_str()), Some("44"));
}

#[test]
fn line_number_width_grows_with_the_lines() {
    let s = PrintSetting {
        print_line_number: true,
        ..PrintSetting::default()
    };
    let width = |n: usize| {
        let src = vec!["x"; n];
        layout(&src, &s, Paper::A4).number_cells()
    };
    // The digits of the number of lines, at least 2, and a space.
    assert_eq!((width(1), width(9), width(10), width(99)), (3, 3, 3, 3));
    assert_eq!((width(100), width(999), width(1000)), (4, 4, 5));
    // The setting of the type sets the fewest digits.
    let options = TypeOptions {
        line_number_width: 5,
        ..TypeOptions::default()
    };
    let src = ["x"];
    assert_eq!(
        Layout::new(&src, &s, Paper::A4, &options)
            .unwrap()
            .number_cells(),
        6
    );
}

#[test]
fn line_number_terminators() {
    let mut s = narrow(20).0;
    s.print_line_number = true;
    let src = ["a"];
    let base_x = 200 + 3 * 12;
    // A character instead of the line.
    let options = TypeOptions {
        line_term: LineTerm::Char(':'),
        ..TypeOptions::default()
    };
    let lay = Layout::new(&src, &s, paper_for(&s, 20, 3), &options).unwrap();
    let p = page(&lay, 0, &src);
    assert_eq!(
        (p.texts[0].text.as_str(), p.texts[0].x),
        ("1:", base_x - 2 * 12)
    );
    assert!(p.rules.is_empty());
    // A wrapped row has a blank in place of the number, and the character after it
    // (DrawPageText: " " and then the terminator).
    let src = ["aaaaaaaaaaaaaaaaaaaaaaaaa"];
    let lay = Layout::new(&src, &s, paper_for(&s, 20, 3), &options).unwrap();
    let p = page(&lay, 0, &src);
    let seps: Vec<_> = p
        .texts
        .iter()
        .filter(|t| t.text.ends_with(':'))
        .map(|t| (t.x, t.text.as_str()))
        .collect();
    assert_eq!(seps, [(base_x - 2 * 12, "1:"), (base_x - 2 * 12, " :")]);
    // Nothing: no line either.
    let options = TypeOptions {
        line_term: LineTerm::None,
        ..TypeOptions::default()
    };
    let lay = Layout::new(&src, &s, paper_for(&s, 20, 3), &options).unwrap();
    assert!(page(&lay, 0, &src).rules.is_empty());
}

/// The paper for `cells` cells of text and `numbers` cells of line numbers.
fn paper_for(s: &PrintSetting, cells: i32, numbers: i32) -> Paper {
    Paper {
        width: s.print_margin_lx + s.print_margin_rx + (cells + numbers) * s.print_font_width,
        ..Paper::A4
    }
}

#[test]
fn line_numbers_by_rows() {
    // m_bLineNumIsCRLF off: every printed row has its own number.
    let (mut s, _) = narrow(4);
    s.print_line_number = true;
    let paper = paper_for(&s, 4, 3);
    let options = TypeOptions {
        line_number_is_crlf: false,
        ..TypeOptions::default()
    };
    let src = ["ああああ", "b"];
    let lay = Layout::new(&src, &s, paper, &options).unwrap();
    // The first line fills two rows, and its line ending takes a third.
    assert_eq!(lay.row_count(), 4);
    let p = page(&lay, 0, &src);
    let numbers: Vec<_> = p
        .texts
        .iter()
        .filter(|t| t.text.chars().all(|c| c.is_ascii_digit()))
        .map(|t| t.text.as_str())
        .collect();
    assert_eq!(numbers, ["1", "2", "3", "4"]);
}

#[test]
fn two_columns() {
    let src: Vec<String> = (1..=200).map(|i| i.to_string()).collect();
    let s = PrintSetting {
        print_dansuu: 2,
        ..PrintSetting::default()
    };
    let lay = layout(&src, &s, Paper::A4);
    // (1800 - 70) / 12 / 2 cells in each column, and 164 rows on a page.
    assert_eq!((lay.cells_per_row(), lay.rows_per_column()), (72, 82));
    assert_eq!(lay.page_count(), 2);
    let p = page(&lay, 0, &src);
    let find = |s: &str| p.texts.iter().find(|t| t.text == s).unwrap();
    // Line 82 is the last of the first column; line 83 starts the second, 72 cells and the
    // space to the right (CPrintPreview::DrawPageText, nDanWidth).
    assert_eq!(
        (find("82").x, find("82").baseline),
        (200, 162 + 81 * 31 + 24)
    );
    assert_eq!(
        (find("83").x, find("83").baseline),
        (200 + 72 * 12 + 70, 162 + 24)
    );
    // Page 2 begins with line 165.
    let p = page(&lay, 1, &src);
    let rows = texts(&p);
    assert_eq!(rows[1], "165");
    assert_eq!(rows.len(), 2 + 36);
}

#[test]
fn two_columns_with_line_numbers() {
    let src: Vec<String> = (1..=100).map(|i| i.to_string()).collect();
    let s = PrintSetting {
        print_dansuu: 2,
        print_line_number: true,
        header_form: Default::default(),
        footer_form: Default::default(),
        ..PrintSetting::default()
    };
    let lay = layout(&src, &s, Paper::A4);
    // 100 lines: 3 digits and a space, in both columns: (1800 - 70 - 2 * 4 * 12) / 12 / 2.
    // Without a header there are 86 rows in a column.
    assert_eq!((lay.number_cells(), lay.cells_per_row()), (4, 68));
    assert_eq!(lay.rows_per_column(), 86);
    let p = page(&lay, 0, &src);
    // Line 87 is the first of the second column. Its text starts at
    // 200 + (68 * 12 + 70) + 2 * 4 * 12 and its number "87 " ends where it starts.
    let base_x = 200 + (68 * 12 + 70) + 2 * 4 * 12;
    let mut at_87 = p.texts.iter().filter(|t| t.text == "87");
    let (number, text) = (at_87.next().unwrap(), at_87.next().unwrap());
    assert_eq!((number.x, text.x), (base_x - 3 * 12, base_x));
    assert_eq!(number.baseline, text.baseline);
    // A line for each column, the second 100 - 86 = 14 rows long.
    assert_eq!(p.rules.len(), 2);
    assert_eq!(p.rules[1].from.0, base_x - 6);
    assert_eq!(p.rules[1].to.1 - p.rules[1].from.1, 14 * 31);
    assert_eq!(p.rules[0].to.1 - p.rules[0].from.1, 86 * 31);
}

#[test]
fn header_and_footer() {
    let ctx = HeaderContext {
        file_path: Some("/home/me/memo.txt".into()),
        ..HeaderContext::default()
    };
    let src = ["a"];
    let lay = layout(&src, &PrintSetting::default(), Paper::A4);
    let p = lay.page(0, &src, &ctx).unwrap();
    let header = &p.texts[0];
    // The header is 0.5 mm inside the margins; its baseline is a font height below the top.
    assert_eq!(
        (header.x, header.baseline, header.text.as_str()),
        (205, 100 + 5 + 24, "memo.txt")
    );
    // The text starts below the header and the blank row after it: 10 mm and 2 * 3.1 mm.
    let body = &p.texts[1];
    assert_eq!(
        (body.x, body.baseline, body.text.as_str()),
        (200, 162 + 24, "a")
    );
    // "- 1 -" is 5 cells, centered between 205 and 1995, on the baseline 0.5 mm inside the
    // bottom margin.
    let footer = &p.texts[2];
    assert_eq!(footer.text, "- 1 -");
    assert_eq!(
        (footer.x, footer.baseline),
        ((205 + 1995 - 60) / 2, 2970 - 205)
    );
}

#[test]
fn header_parts_and_variables() {
    let s = PrintSetting {
        name: "Setting".into(),
        header_form: ["$f".into(), "$Q".into(), "$p/$P".into()],
        footer_form: ["${U?x$:ok$}".into(), String::new(), "$$".into()],
        ..PrintSetting::default()
    };
    let src: Vec<String> = (0..165).map(|i| i.to_string()).collect();
    let lay = layout(&src, &s, Paper::A4);
    assert_eq!(lay.page_count(), 3);
    let ctx = HeaderContext {
        file_path: Some("C:/a/b.txt".into()),
        ..HeaderContext::default()
    };
    let p = lay.page(1, &src, &ctx).unwrap();
    let head: Vec<_> = p
        .texts
        .iter()
        .take(3)
        .map(|t| (t.x, t.text.as_str()))
        .collect();
    // "Setting" is 7 cells (84): centered; "2/3" is 3 cells (36): at the right edge.
    assert_eq!(
        head,
        [
            (205, "b.txt"),
            ((205 + 1995 - 84) / 2, "Setting"),
            (1995 - 36, "2/3")
        ]
    );
    let foot: Vec<_> = p
        .texts
        .iter()
        .rev()
        .take(2)
        .map(|t| (t.x, t.text.as_str()))
        .collect();
    assert_eq!(foot, [(1995 - 12, "$"), (205, "ok")]);
}

#[test]
fn an_empty_header_takes_no_room() {
    let (s, _) = narrow(10);
    let paper = Paper::A4;
    let src = ["a"];
    let lay = layout(&src, &s, paper);
    // No header and no footer: the rows are (2970 - 300 + 7) / 31 = 86, from the top margin.
    assert_eq!(lay.rows_per_column(), 86);
    let p = page(&lay, 0, &src);
    assert_eq!(p.texts[0].baseline, 100 + 24);
    // Only a footer.
    let s = PrintSetting {
        header_form: Default::default(),
        ..PrintSetting::default()
    };
    let lay = layout(&src, &s, paper);
    assert_eq!(lay.rows_per_column(), (2670 - 62 + 7) / 31);
}

#[test]
fn tabs_in_a_header_are_laid_out_on_the_grid() {
    let s = PrintSetting {
        header_form: ["a\tb".into(), String::new(), String::new()],
        footer_form: Default::default(),
        ..PrintSetting::default()
    };
    let src = ["x"];
    let p = page(&layout(&src, &s, Paper::A4), 0, &src);
    let head: Vec<_> = p
        .texts
        .iter()
        .take(2)
        .map(|t| (t.x, t.text.as_str()))
        .collect();
    assert_eq!(head, [(205, "a"), (205 + 4 * 12, "b")]);
}

#[test]
fn headers_with_a_font_of_their_own() {
    let s = PrintSetting {
        header_form: [String::new(), "$f".into(), "R".into()],
        header_font: Some(HeaderFooterFont {
            face: "Arial".into(),
            point_size: 100,
        }),
        footer_form: ["L".into(), String::new(), String::new()],
        footer_font: Some(HeaderFooterFont {
            face: "Arial".into(),
            point_size: 100,
        }),
        ..PrintSetting::default()
    };
    let src = ["a"];
    let lay = layout(&src, &s, Paper::A4);
    // 10 pt is 35 (0.1 mm), and 45 with the line spacing, twice for the blank row.
    // (2970 - 300 - 90 - 90 + 7) / 31 rows.
    assert_eq!(lay.rows_per_column(), (2670 - 180 + 7) / 31);
    let p = page(&lay, 0, &src);
    let by_text = |t: &str| p.texts.iter().find(|p| p.text == t).unwrap();
    let center = by_text("(無題)");
    assert_eq!(
        (center.x, center.baseline, center.font, center.align),
        (
            (205 + 1995) / 2,
            100 + 5 + 35,
            FontKind::Header,
            HAlign::Center
        )
    );
    let right = by_text("R");
    assert_eq!((right.x, right.align), (1995, HAlign::Right));
    let left = by_text("L");
    assert_eq!(
        (left.x, left.baseline, left.font, left.align),
        (205, 2970 - 205, FontKind::Footer, HAlign::Left)
    );
    // The body starts below the header of 90.
    assert_eq!(by_text("a").baseline, 100 + 90 + 24);
}

#[test]
fn a_tab_in_a_centered_header_is_measured_as_sakura_does() {
    // CalcTextWidth2 gives GenerateDxArray the tab width 8 (0.1 mm): after "a" of 12, a tab is
    // (8 + 12 - 1) - ((12 + 12 - 1) % 8) = 19 - 7 = 12.
    let s = PrintSetting {
        header_form: [String::new(), "a\tb".into(), String::new()],
        footer_form: Default::default(),
        ..PrintSetting::default()
    };
    let src = ["x"];
    let p = page(&layout(&src, &s, Paper::A4), 0, &src);
    // 12 + 12 + 12 = 36 wide.
    assert_eq!(p.texts[0].x, (205 + 1995 - 36) / 2);
}

#[test]
fn page_ranges() {
    let src: Vec<String> = (1..=300).map(|i| i.to_string()).collect();
    let lay = layout(&src, &PrintSetting::default(), Paper::A4);
    assert_eq!(lay.page_count(), 4);
    let ctx = HeaderContext::default();
    let got: Vec<_> = lay.pages(1..3, &src, &ctx).map(|p| p.index).collect();
    assert_eq!(got, [1, 2]);
    // The range is clipped to the pages there are.
    let got: Vec<_> = lay.pages(2..99, &src, &ctx).map(|p| p.index).collect();
    assert_eq!(got, [2, 3]);
    assert_eq!(lay.pages(9..99, &src, &ctx).count(), 0);
}

#[test]
fn no_room_for_text() {
    let src = ["a"];
    let too_wide = PrintSetting {
        print_margin_lx: 1500,
        print_margin_rx: 600,
        ..PrintSetting::default()
    };
    let err = Layout::new(&src, &too_wide, Paper::A4, &TypeOptions::default()).unwrap_err();
    assert_eq!(err, PrintError::NoPrintableArea);
    assert_eq!(
        err.to_string(),
        "印刷ページ設定エラー:印字可能領域がありません"
    );
    // Margins that use all the height.
    let too_tall = PrintSetting {
        print_margin_ty: 1500,
        print_margin_by: 1500,
        ..PrintSetting::default()
    };
    assert_eq!(
        Layout::new(&src, &too_tall, Paper::A4, &TypeOptions::default()).unwrap_err(),
        PrintError::NoPrintableArea
    );
    // Two columns with a gap wider than the paper.
    let gap = PrintSetting {
        print_dansuu: 4,
        print_dan_space: 900,
        ..PrintSetting::default()
    };
    assert_eq!(
        Layout::new(&src, &gap, Paper::A4, &TypeOptions::default()).unwrap_err(),
        PrintError::NoPrintableArea
    );
    // Line numbers that leave no room for the text.
    let (mut s, paper) = narrow(2);
    s.print_line_number = true;
    assert_eq!(
        Layout::new(&src, &s, paper, &TypeOptions::default()).unwrap_err(),
        PrintError::NoPrintableArea
    );
}

#[test]
fn invalid_numbers_are_errors() {
    let src = ["a"];
    for s in [
        PrintSetting {
            print_font_width: 0,
            ..PrintSetting::default()
        },
        PrintSetting {
            print_font_height: -1,
            ..PrintSetting::default()
        },
        PrintSetting {
            print_dansuu: 0,
            ..PrintSetting::default()
        },
    ] {
        assert!(matches!(
            Layout::new(&src, &s, Paper::A4, &TypeOptions::default()),
            Err(PrintError::InvalidSetting(_))
        ));
    }
    // A line spacing that cancels the font height would divide by zero.
    let s = PrintSetting {
        print_line_spacing: -100,
        ..PrintSetting::default()
    };
    assert_eq!(
        Layout::new(&src, &s, Paper::A4, &TypeOptions::default()).unwrap_err(),
        PrintError::InvalidSetting("print_line_spacing")
    );
    let options = TypeOptions {
        tab_width: 0,
        ..TypeOptions::default()
    };
    assert!(Layout::new(&src, &PrintSetting::default(), Paper::A4, &options).is_err());
}

#[test]
fn grid_positions() {
    let text = PlacedText {
        x: 100,
        baseline: 0,
        text: "aあb".into(),
        font: FontKind::Han,
        align: HAlign::Left,
    };
    let xs: Vec<_> = text
        .positions(12, WidthRule::default())
        .map(|(i, _, x)| (i, x))
        .collect();
    assert_eq!(xs, [(0, 100), (1, 112), (4, 136)]);
}

#[test]
fn a_line_of_many_pages() {
    // One line of 20 rows on pages of 3 rows: the pages go on with the same line, the same
    // with and without the cache that `pages` keeps.
    let (s, paper) = small(4, 3);
    let src = ["あ".repeat(40)];
    let lay = layout(&src, &s, paper);
    assert_eq!((lay.row_count(), lay.page_count()), (20, 7));
    let ctx = HeaderContext::default();
    let all: Vec<_> = lay.pages(0..99, &src, &ctx).collect();
    assert_eq!(all.len(), 7);
    for (i, p) in all.iter().enumerate() {
        assert_eq!(Some(p), lay.page(i, &src, &ctx).as_ref());
    }
    // 2 characters in each of 3 rows, then 2 rows of 2 characters on the last page.
    assert_eq!(texts(&all[0]), ["ああ"; 3]);
    assert_eq!(texts(&all[6]), ["ああ"; 2]);
}
