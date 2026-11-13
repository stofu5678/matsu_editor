//! Links on the screen (ADR-0028 point 5): the link under a cell, and whether it may be opened.
//!
//! Two kinds are found: the links a program wrote with OSC 8 (the address is in the cells), and plain
//! `http://` and `https://` addresses that a program only printed. The plain ones are found by a scan of the
//! text around the cell, on demand (a hover or a click), not by a search over the whole screen with
//! `alacritty_terminal`'s regex search, which compiles an automaton for every terminal and scans every line
//! that is shown; the scan costs one logical line. A line that wraps (the program wrote past the last column)
//! is one line for the scan, so an address that wraps is found whole.
//!
//! Only `http` and `https` are opened ([`is_openable`]); the UI shows the address and asks first.

use std::ops::Range;

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::Line;
use alacritty_terminal::term::Term;
use alacritty_terminal::term::cell::Flags;

use crate::screen::{Cell, Row};

/// A run of cells of one row that belong to a link: the columns from `first` to `last`, both included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkSpan {
    pub row: usize,
    pub first: usize,
    pub last: usize,
}

/// A link: its address and the cells it covers (several spans when it wraps).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub uri: String,
    pub spans: Vec<LinkSpan>,
}

impl Link {
    /// Whether the cell at `row` and `column` is part of the link.
    pub fn contains(&self, row: usize, column: usize) -> bool {
        self.spans
            .iter()
            .any(|span| span.row == row && (span.first..=span.last).contains(&column))
    }
}

/// How many rows of a line that wraps are looked at on each side of the row of the cell: an address is not
/// longer than that, and a very long line of output must not be copied for every hover.
const MAX_WRAPPED_ROWS: i32 = 100;

/// The link at a cell of the view of `term`. The rows of the logical line of that row are copied, also the ones
/// above or below the view when the view is scrolled back (an address may wrap out of it); the spans that
/// come back are the ones in the view.
pub(crate) fn in_term<T>(term: &Term<T>, row: usize, column: usize) -> Option<Link> {
    let screen_lines = i32::try_from(term.screen_lines()).ok()?;
    let row = i32::try_from(row).ok().filter(|&row| row < screen_lines)?;
    let offset = i32::try_from(term.grid().display_offset()).unwrap_or(0);
    let top = -i32::try_from(term.history_size()).unwrap_or(0);
    let wraps = |line: i32| {
        term.grid()[Line(line)]
            .last()
            .is_some_and(|cell| cell.flags.contains(Flags::WRAPLINE))
    };
    let at = row - offset;
    let mut first = at;
    while first > top && at - first < MAX_WRAPPED_ROWS && wraps(first - 1) {
        first -= 1;
    }
    let mut last = at;
    while last + 1 < screen_lines && last - at < MAX_WRAPPED_ROWS && wraps(last) {
        last += 1;
    }
    let rows: Vec<Row> = (first..=last)
        .map(|line| Row::from_grid(&term.grid()[Line(line)]))
        .collect();
    let mut link = link_at(&rows, usize::try_from(at - first).ok()?, column)?;
    // From rows of the copy to rows of the view; the ones out of the view are not drawn.
    link.spans.retain_mut(|span| {
        let view_row = i32::try_from(span.row).unwrap_or(i32::MAX) + first + offset;
        match usize::try_from(view_row) {
            Ok(view_row) if view_row < term.screen_lines() => {
                span.row = view_row;
                true
            }
            _ => false,
        }
    });
    Some(link)
}

/// A cell that is drawn: not the right half of a wide character, not the blank before one that wrapped.
struct Item<'a> {
    row: usize,
    column: usize,
    span: usize,
    cell: &'a Cell,
}

/// The link at the cell (`row`, `column`) of the view `rows`, if the cell is in one. An OSC 8 link wins over
/// an address that happens to be written in its text.
pub fn link_at(rows: &[Row], row: usize, column: usize) -> Option<Link> {
    if row >= rows.len() {
        return None;
    }
    let mut first_row = row;
    while first_row > 0 && rows[first_row - 1].wrapped {
        first_row -= 1;
    }
    let mut last_row = row;
    while rows[last_row].wrapped && last_row + 1 < rows.len() {
        last_row += 1;
    }
    let mut items: Vec<Item> = Vec::new();
    for (index, line) in rows.iter().enumerate().take(last_row + 1).skip(first_row) {
        for (column, cell) in line.cells.iter().enumerate() {
            if cell.wide_spacer || cell.leading_wide_spacer {
                continue;
            }
            items.push(Item {
                row: index,
                column,
                span: if cell.wide { 2 } else { 1 },
                cell,
            });
        }
    }
    let hit = items.iter().position(|item| {
        item.row == row && (item.column..item.column + item.span).contains(&column)
    })?;

    if let Some(uri) = &items[hit].cell.hyperlink {
        let same = |item: &Item| item.cell.hyperlink.as_deref() == Some(&**uri);
        let mut start = hit;
        while start > 0 && same(&items[start - 1]) {
            start -= 1;
        }
        let mut end = hit + 1;
        while end < items.len() && same(&items[end]) {
            end += 1;
        }
        return Some(Link {
            uri: uri.to_string(),
            spans: spans_of(&items, start..end),
        });
    }

    // The text of the logical line, and the item each character comes from.
    let mut chars: Vec<char> = Vec::new();
    let mut owner: Vec<usize> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        for c in std::iter::once(item.cell.ch).chain(item.cell.zerowidth.iter().copied()) {
            chars.push(c);
            owner.push(index);
        }
    }
    let at = owner.iter().position(|&index| index == hit)?;
    let range = url_around(&chars, at)?;
    let uri: String = chars[range.clone()].iter().collect();
    Some(Link {
        uri,
        spans: spans_of(&items, owner[range.start]..owner[range.end - 1] + 1),
    })
}

/// The spans of the rows that the items in `range` cover.
fn spans_of(items: &[Item], range: Range<usize>) -> Vec<LinkSpan> {
    let mut spans: Vec<LinkSpan> = Vec::new();
    for item in &items[range] {
        let last = item.column + item.span - 1;
        match spans.last_mut() {
            Some(span) if span.row == item.row => span.last = last,
            _ => spans.push(LinkSpan {
                row: item.row,
                first: item.column,
                last,
            }),
        }
    }
    spans
}

/// The characters an address is made of: ASCII, printable, and none of the ones that end an address in
/// running text (`<>"'` and the backtick).
fn is_url_char(c: char) -> bool {
    c.is_ascii_graphic() && !matches!(c, '<' | '>' | '"' | '\'' | '`')
}

/// The longest address that is found, in characters. Longer ones are not links: the scan of a line made of
/// addresses end to end would otherwise look at the rest of the line from every scheme in it.
const MAX_URL_CHARS: usize = 2000;

/// The range of the `http://` or `https://` address that has the character `at`, if there is one.
fn url_around(chars: &[char], at: usize) -> Option<Range<usize>> {
    // Walk back to the start of the run of address characters, then try each start of a scheme in it.
    let mut run_start = at;
    while run_start > 0 && at - run_start < MAX_URL_CHARS && is_url_char(chars[run_start - 1]) {
        run_start -= 1;
    }
    if !is_url_char(chars[at]) {
        return None;
    }
    for start in run_start..=at {
        let Some(scheme) = scheme_at(chars, start) else {
            continue;
        };
        // A scheme in the middle of a word (`xhttp://`) is not one.
        if start > 0 && chars[start - 1].is_ascii_alphanumeric() {
            continue;
        }
        let limit = chars.len().min(start + MAX_URL_CHARS);
        let mut end = start + scheme;
        while end < limit && is_url_char(chars[end]) {
            end += 1;
        }
        // An address that goes on past the limit is not one that is found: its first 2000 characters would be
        // another address.
        if end == limit && end < chars.len() && is_url_char(chars[end]) {
            continue;
        }
        end = trim_end(chars, start + scheme, end);
        if end > start + scheme && (start..end).contains(&at) {
            return Some(start..end);
        }
    }
    None
}

/// The length of `http://` or `https://` (any case) at `start`.
fn scheme_at(chars: &[char], start: usize) -> Option<usize> {
    let matches = |text: &str| {
        chars.len() >= start + text.len()
            && chars[start..start + text.len()]
                .iter()
                .zip(text.chars())
                .all(|(c, t)| c.eq_ignore_ascii_case(&t))
    };
    if matches("https://") {
        Some(8)
    } else if matches("http://") {
        Some(7)
    } else {
        None
    }
}

/// Where an address ends once the punctuation that belongs to the sentence is taken off: `. , ; : ! ?`, and a
/// closing bracket that has no opening one in the address (`(see http://a.example/b)`).
fn trim_end(chars: &[char], body_start: usize, mut end: usize) -> usize {
    // Open and close brackets of the address, kept as `end` moves so that the work stays linear: `(` `)`,
    // `[` `]`, `{` `}`.
    let mut count = [0usize; 6];
    let slot = |c: char| "()[]{}".find(c);
    for &c in &chars[body_start..end] {
        if let Some(slot) = slot(c) {
            count[slot] += 1;
        }
    }
    while end > body_start {
        let last = chars[end - 1];
        let trim = match last {
            '.' | ',' | ';' | ':' | '!' | '?' => true,
            ')' => count[1] > count[0],
            ']' => count[3] > count[2],
            '}' => count[5] > count[4],
            _ => false,
        };
        if !trim {
            break;
        }
        if let Some(slot) = slot(last) {
            count[slot] -= 1;
        }
        end -= 1;
    }
    end
}

/// The longest host (with its port) that an address may have to be opened: a host name is at most 253
/// characters, and the question shows the first [`display_uri`] characters, so the whole host is always shown.
const MAX_AUTHORITY: usize = 255;

/// Whether the address may be opened: `http` or `https` (any case), a host after `://` that has no user
/// information (`user@`, which makes `https://trusted.example@evil.example` look like another host) and is not
/// longer than [`MAX_AUTHORITY`], and neither control characters nor blanks (a program can write anything into
/// an OSC 8 address).
pub fn is_openable(uri: &str) -> bool {
    let lower = uri.to_ascii_lowercase();
    let Some(rest) = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
    else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    !authority.is_empty()
        && authority.len() <= MAX_AUTHORITY
        && !authority.contains('@')
        && !uri.chars().any(|c| c.is_control() || c.is_whitespace())
}

/// The address as a message shows it: control characters taken out and cut to `max` characters with `…`.
pub fn display_uri(uri: &str, max: usize) -> String {
    let clean: String = uri.chars().filter(|c| !c.is_control()).collect();
    if clean.chars().count() <= max {
        clean
    } else {
        let mut cut: String = clean.chars().take(max).collect();
        cut.push('…');
        cut
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    /// A row of `width` cells with `text` from column 0 (a wide character takes two cells).
    fn row(text: &str, width: usize, wrapped: bool) -> Row {
        let blank = || Cell {
            ch: ' ',
            zerowidth: Vec::new(),
            fg: crate::Color::Default,
            bg: crate::Color::Default,
            underline_color: None,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            strikeout: false,
            inverse: false,
            hidden: false,
            wide: false,
            wide_spacer: false,
            leading_wide_spacer: false,
            hyperlink: None,
        };
        let mut cells = Vec::new();
        for c in text.chars() {
            let wide = c.len_utf8() > 1;
            cells.push(Cell {
                ch: c,
                wide,
                ..blank()
            });
            if wide {
                cells.push(Cell {
                    wide_spacer: true,
                    ..blank()
                });
            }
        }
        while cells.len() < width {
            cells.push(blank());
        }
        Row { cells, wrapped }
    }

    fn uri_at(rows: &[Row], row: usize, column: usize) -> Option<String> {
        link_at(rows, row, column).map(|link| link.uri)
    }

    #[test]
    fn a_printed_address_is_a_link_on_all_of_its_cells() {
        let rows = [row("see https://example.com/a?b=1 now", 40, false)];
        let link = link_at(&rows, 0, 10).unwrap();
        assert_eq!(link.uri, "https://example.com/a?b=1");
        assert_eq!(
            link.spans,
            vec![LinkSpan {
                row: 0,
                first: 4,
                last: 28
            }]
        );
        assert!(link.contains(0, 4) && link.contains(0, 28));
        assert!(!link.contains(0, 3) && !link.contains(0, 29));
        // the text around it is not
        assert_eq!(uri_at(&rows, 0, 0), None);
        assert_eq!(uri_at(&rows, 0, 30), None);
        assert_eq!(uri_at(&rows, 0, 39), None);
    }

    #[test]
    fn only_http_and_https_are_found_in_text() {
        let rows = [row("ftp://example.com and file:///etc/passwd", 50, false)];
        assert_eq!(uri_at(&rows, 0, 5), None);
        assert_eq!(uri_at(&rows, 0, 30), None);
        let rows = [row("HTTP://EXAMPLE.COM/x", 30, false)];
        assert_eq!(uri_at(&rows, 0, 3).as_deref(), Some("HTTP://EXAMPLE.COM/x"));
        // a scheme inside a word, or with nothing after it, is not one
        let rows = [row("xhttp://a.example http://", 30, false)];
        assert_eq!(uri_at(&rows, 0, 8), None);
        assert_eq!(uri_at(&rows, 0, 20), None);
    }

    #[test]
    fn punctuation_of_the_sentence_is_not_the_address() {
        let cases = [
            ("go to http://a.example/b.", "http://a.example/b"),
            ("(see http://a.example/b)", "http://a.example/b"),
            ("http://a.example/w_(x)", "http://a.example/w_(x)"),
            ("is it http://a.example/b?!", "http://a.example/b"),
            ("[http://a.example/b], then", "http://a.example/b"),
            ("\"http://a.example/b\"", "http://a.example/b"),
            ("<http://a.example/b>", "http://a.example/b"),
        ];
        for (text, want) in cases {
            let rows = [row(text, 40, false)];
            let at = text.find("http").unwrap() + 2;
            assert_eq!(uri_at(&rows, 0, at).as_deref(), Some(want), "{text}");
        }
    }

    #[test]
    fn an_address_stops_at_a_wide_character() {
        let rows = [row("http://a.example/はい", 30, false)];
        let link = link_at(&rows, 0, 3).unwrap();
        assert_eq!(link.uri, "http://a.example/");
        // the wide characters are not part of it
        assert_eq!(uri_at(&rows, 0, 17), None);
    }

    #[test]
    fn an_address_after_wide_characters_is_found_by_its_columns() {
        // the wide characters take two columns each: the address starts at column 6
        let rows = [row("日本語http://a.example", 30, false)];
        let link = link_at(&rows, 0, 7).unwrap();
        assert_eq!(link.uri, "http://a.example");
        assert_eq!(link.spans[0].first, 6);
        assert_eq!(link.spans[0].last, 21);
    }

    #[test]
    fn an_address_that_wraps_is_one_link_over_both_rows() {
        let rows = [
            row("see http://a.example/", 21, true),
            row("long/path now", 21, false),
        ];
        let first = link_at(&rows, 0, 10).unwrap();
        assert_eq!(first.uri, "http://a.example/long/path");
        assert_eq!(
            first.spans,
            vec![
                LinkSpan {
                    row: 0,
                    first: 4,
                    last: 20
                },
                LinkSpan {
                    row: 1,
                    first: 0,
                    last: 8
                }
            ]
        );
        assert_eq!(link_at(&rows, 1, 3), Some(first));
        // a row that did not wrap does not continue on the next
        let rows = [
            row("see http://a.example/", 21, false),
            row("long/path now", 21, false),
        ];
        assert_eq!(uri_at(&rows, 0, 10).as_deref(), Some("http://a.example/"));
        assert_eq!(uri_at(&rows, 1, 3), None);
    }

    #[test]
    fn an_osc_8_link_is_the_cells_with_its_address_and_the_address_may_be_any() {
        let mut line = row("click here, not here", 30, false);
        let uri: Arc<str> = Arc::from("ftp://files.example/x");
        for cell in &mut line.cells[0..10] {
            cell.hyperlink = Some(Arc::clone(&uri));
        }
        let rows = [line];
        let link = link_at(&rows, 0, 3).unwrap();
        assert_eq!(link.uri, "ftp://files.example/x");
        assert_eq!(
            link.spans,
            vec![LinkSpan {
                row: 0,
                first: 0,
                last: 9
            }]
        );
        assert_eq!(uri_at(&rows, 0, 12), None);
        // it is not openable, but it is a link: the UI says why
        assert!(!is_openable(&link.uri));
    }

    #[test]
    fn two_osc_8_links_next_to_each_other_are_two_links() {
        let mut line = row("onetwo", 10, false);
        for cell in &mut line.cells[0..3] {
            cell.hyperlink = Some(Arc::from("http://one.example/"));
        }
        for cell in &mut line.cells[3..6] {
            cell.hyperlink = Some(Arc::from("http://two.example/"));
        }
        let rows = [line];
        assert_eq!(uri_at(&rows, 0, 1).as_deref(), Some("http://one.example/"));
        assert_eq!(uri_at(&rows, 0, 4).as_deref(), Some("http://two.example/"));
        assert_eq!(link_at(&rows, 0, 4).unwrap().spans[0].first, 3);
    }

    #[test]
    fn a_cell_outside_the_rows_has_no_link() {
        let rows = [row("http://a.example", 20, false)];
        assert_eq!(link_at(&rows, 1, 0), None);
        assert_eq!(link_at(&rows, 0, 20), None);
        assert_eq!(link_at(&[], 0, 0), None);
    }

    #[test]
    fn only_http_and_https_addresses_without_blanks_or_controls_open() {
        assert!(is_openable("http://example.com"));
        assert!(is_openable("https://example.com/a?b#c"));
        assert!(is_openable("HTTPS://EXAMPLE.COM"));
        assert!(!is_openable("http://"));
        assert!(!is_openable("http:///x"));
        assert!(!is_openable("http://?x"));
        // user information makes the host look like another one
        assert!(!is_openable("https://trusted.example@evil.example/"));
        assert!(!is_openable("https://trusted.example:pw@evil.example/"));
        assert!(is_openable("https://example.com/a@b"));
        assert!(is_openable("https://example.com?x=a@b"));
        assert!(is_openable("https://example.com:8080/x"));
        assert!(is_openable(&format!("https://{}/x", "a".repeat(255))));
        assert!(!is_openable(&format!("https://{}/x", "a".repeat(256))));
        assert!(!is_openable("ftp://example.com"));
        assert!(!is_openable("file:///etc/passwd"));
        assert!(!is_openable("javascript:alert(1)"));
        assert!(!is_openable("mailto:a@example.com"));
        assert!(!is_openable("example.com"));
        assert!(!is_openable("https://example.com/a b"));
        assert!(!is_openable("https://example.com/\u{1b}[31m"));
        assert!(!is_openable("https://example.com/\n"));
        assert!(!is_openable(""));
    }

    /// A terminal of 20 columns and 3 rows, with 100 lines of scrollback, fed with `output`.
    fn term_of(output: &str) -> Term<alacritty_terminal::event::VoidListener> {
        use alacritty_terminal::term::Config;
        let config = Config {
            scrolling_history: 100,
            ..Config::default()
        };
        let size = crate::terminal::TerminalSize::new(20, 3);
        let mut term = Term::new(config, &size, alacritty_terminal::event::VoidListener);
        let mut processor: alacritty_terminal::vte::ansi::Processor =
            alacritty_terminal::vte::ansi::Processor::new();
        processor.advance(&mut term, output.as_bytes());
        term
    }

    #[test]
    fn an_address_that_wraps_out_of_the_view_is_found_whole() {
        // "see http://a.example" / "/" and 19 b / 11 b and " end": the first row is in the scrollback after
        // one more line
        let url = format!("http://a.example/{}", "b".repeat(30));
        let term = term_of(&format!("see {url} end\r\nx"));
        // the view is [row 1, row 2, "x"]: row 0 of the view continues an address that starts above it
        assert_eq!(term.grid().display_offset(), 0);
        let link = in_term(&term, 0, 3).expect("the address, though its start is out of the view");
        assert_eq!(link.uri, url);
        assert_eq!(
            link.spans,
            vec![
                LinkSpan {
                    row: 0,
                    first: 0,
                    last: 19
                },
                LinkSpan {
                    row: 1,
                    first: 0,
                    last: 10
                }
            ]
        );
        assert_eq!(in_term(&term, 1, 5), Some(link));
        assert_eq!(in_term(&term, 2, 0), None);
        assert_eq!(in_term(&term, 3, 0), None);
    }

    #[test]
    fn an_address_that_wraps_below_the_view_is_found_whole_when_scrolled_back() {
        // four rows of address: "see http://a.example" / "/" and 19 b / 20 b / 11 b and " end"; then three
        // lines, so that the four rows are in the scrollback
        let url = format!("http://a.example/{}", "b".repeat(50));
        let mut term = term_of(&format!("see {url} end\r\nx\r\ny\r\nz"));
        term.scroll_display(alacritty_terminal::grid::Scroll::Delta(4));
        assert_eq!(term.grid().display_offset(), 4);
        // the view is the first three rows: the last of them continues below the view
        let link = in_term(&term, 0, 5).expect("the address");
        assert_eq!(link.uri, url);
        let rows: Vec<usize> = link.spans.iter().map(|span| span.row).collect();
        assert_eq!(rows, vec![0, 1, 2], "the row below the view is not drawn");
        assert_eq!(in_term(&term, 2, 19), Some(link));
    }

    #[test]
    fn closing_brackets_are_trimmed_in_linear_time() {
        let text = format!("http://a.example/b{}", ")".repeat(20_000));
        let rows = [row(&text, text.len(), false)];
        let started = std::time::Instant::now();
        assert_eq!(uri_at(&rows, 0, 3).as_deref(), Some("http://a.example/b"));
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
    }

    #[test]
    fn a_line_of_addresses_end_to_end_is_scanned_in_bounded_time() {
        let text = format!("{}.", "http://a/".repeat(5_000));
        let rows = [row(&text, text.chars().count(), false)];
        let started = std::time::Instant::now();
        // the last full stop is not part of an address
        assert_eq!(uri_at(&rows, 0, text.len() - 1), None);
        // the whole line is one run of address characters, longer than any address that is found
        assert_eq!(uri_at(&rows, 0, 20_000), None);
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
    }

    #[test]
    fn an_address_longer_than_the_limit_is_not_a_link_and_one_at_the_limit_is() {
        let long = |extra: usize| {
            format!(
                "http://a.example/{} end",
                "b".repeat(MAX_URL_CHARS - "http://a.example/".len() + extra)
            )
        };
        let text = long(0);
        let rows = [row(&text, text.chars().count(), false)];
        assert_eq!(
            uri_at(&rows, 0, 3).map(|uri| uri.chars().count()),
            Some(MAX_URL_CHARS)
        );
        let text = long(1);
        let rows = [row(&text, text.chars().count(), false)];
        assert_eq!(uri_at(&rows, 0, 3), None, "not its first 2000 characters");
    }

    #[test]
    fn an_address_is_shown_without_controls_and_cut_when_long() {
        assert_eq!(display_uri("http://a/\u{1b}[31mb", 50), "http://a/[31mb");
        assert_eq!(display_uri("abcdef", 3), "abc…");
        assert_eq!(display_uri("abc", 3), "abc");
    }
}
