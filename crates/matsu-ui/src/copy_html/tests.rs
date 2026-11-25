//! Tests of the colored HTML copy without a window: the rows, the colors of a C file, special
//! characters, tabs, Japanese text, the line numbers and the box selection. The tags
//! are checked in `builder_tests` against Sakura's `AppendHTMLColor`.

use matsu_coloring::{ColorKind, ColorRules, ColorStates};
use matsu_config::settings::color::ColorEntry;
use matsu_core::buffer::{Document, Pos};
use matsu_core::cursor::Cursor;
use matsu_core::search::{Matcher, PlainMatcher, PlainOptions};
use matsu_layout::{DocumentLayout, LayoutConfig, Wrap};

use super::builder::{Attr, Html, Row};
use super::collect::{Found, Source, color_html};
use crate::settings::UserSettings;
use crate::settings::view_settings::{FoundColors, rgb_of};

/// What a test sets up: the document, the layout, and the rules of a file called `file`.
struct Fixture {
    doc: Document,
    layout: DocumentLayout,
    rules: std::sync::Arc<ColorRules>,
    states: ColorStates,
    cursor: Cursor,
}

impl Fixture {
    fn new(text: &str, file: &str) -> Self {
        Self::wrapped(text, file, LayoutConfig::default())
    }

    fn wrapped(text: &str, file: &str, cfg: LayoutConfig) -> Self {
        let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
        let layout = DocumentLayout::laid_out(&doc, cfg).unwrap();
        let rules = UserSettings::none().view_settings(Some(file)).coloring;
        Self {
            doc,
            layout,
            rules,
            states: ColorStates::default(),
            cursor: Cursor::default(),
        }
    }

    fn select(&mut self, from: (usize, usize), to: (usize, usize)) {
        self.cursor.move_to(Pos::new(from.0, from.1), false);
        self.cursor.move_to(Pos::new(to.0, to.1), true);
    }

    /// A box with the corners `(line, column)` of the display (the lines are not wrapped).
    fn select_box(&mut self, from: (usize, usize), to: (usize, usize)) {
        self.cursor.move_to(self.pos_at(from), false);
        self.cursor.begin_box(from.1, true);
        self.cursor.move_box_to(self.pos_at(to), to.1);
    }

    fn pos_at(&self, (line, col): (usize, usize)) -> Pos {
        let row = self.layout.first_row_of_line(line);
        self.layout
            .row_to_pos(&self.doc, row, self.layout.config().width.to_x(col as u32))
            .unwrap()
            .pos
    }

    fn html(&self, line_numbers: bool, line_num_is_crlf: bool) -> Option<String> {
        color_html(&Source {
            doc: &self.doc,
            layout: &self.layout,
            rules: &self.rules,
            states: &self.states,
            cursor: &self.cursor,
            line_num_is_crlf,
            line_numbers,
            found: None,
        })
    }

    /// The HTML with the matches of `pattern` colored in `colors` (`CColor_Found`).
    fn html_found(&self, pattern: &str, whole_word: bool, colors: &[ColorEntry]) -> Option<String> {
        let matcher = PlainMatcher::new(
            pattern,
            PlainOptions {
                whole_word,
                ..PlainOptions::default()
            },
        );
        self.html_with(&matcher, whole_word.then_some(&matcher), colors)
    }

    fn html_with(
        &self,
        matcher: &dyn Matcher,
        words: Option<&PlainMatcher>,
        colors: &[ColorEntry],
    ) -> Option<String> {
        color_html(&Source {
            doc: &self.doc,
            layout: &self.layout,
            rules: &self.rules,
            states: &self.states,
            cursor: &self.cursor,
            line_num_is_crlf: true,
            line_numbers: false,
            found: Some(Found {
                matcher,
                colors: FoundColors::new(colors.iter().copied()),
                words,
            }),
        })
    }

    /// `<span style="...">` of the colors of a kind.
    fn open(&self, kind: ColorKind) -> String {
        let entry = self.rules.entry(kind);
        assert!(!entry.bold && !entry.underline, "{kind:?} has a font style");
        format!(
            "<span style=\"color:#{:06x};background-color:#{:06x}\">",
            rgb_of(entry.text),
            rgb_of(entry.back)
        )
    }

    fn pre(&self) -> String {
        format!(
            "<pre style=\"background-color:#{:06x}\">",
            rgb_of(self.rules.entry(ColorKind::Text).back)
        )
    }
}

#[test]
fn plain_text_is_one_span_in_the_text_color() {
    let mut f = Fixture::new("hello world\r\n", "a.txt");
    f.select((0, 0), (0, 5));
    let text = f.open(ColorKind::Text);
    assert_eq!(
        f.html(false, true).unwrap(),
        format!("{}{text}hello</span></pre>", f.pre())
    );
}

#[test]
fn nothing_is_copied_without_a_selection() {
    let f = Fixture::new("hello\r\n", "a.txt");
    assert_eq!(f.html(false, true), None);
    let mut f = Fixture::new("hello\r\n", "a.txt");
    // an anchor at the caret selects nothing
    f.select((0, 2), (0, 2));
    assert_eq!(f.html(false, true), None);
}

#[test]
fn keywords_comments_and_strings_have_the_colors_of_the_type() {
    let mut f = Fixture::new("int a = 1; // note\r\nchar *s = \"x\";\r\n", "a.c");
    f.select((0, 0), (1, 14));
    let keyword = f.open(ColorKind::Keyword(0));
    let number = f.open(ColorKind::Number);
    let comment = f.open(ColorKind::Comment);
    let string = f.open(ColorKind::DoubleQuote);
    let text = f.open(ColorKind::Text);
    // the line ending after a comment is in the text color again
    assert_eq!(
        f.html(false, true).unwrap(),
        format!(
            "{pre}{keyword}int</span>{text} a = </span>{number}1</span>{text}; </span>\
             {comment}// note</span>{text}\r\n</span>{keyword}char</span>{text} *s = </span>\
             {string}\"x\"</span>{text};</span></pre>",
            pre = f.pre()
        )
    );
}

#[test]
fn a_selection_that_starts_in_the_middle_keeps_the_color_of_its_place() {
    // the selection starts inside the block comment: it is a comment from the first character
    let mut f = Fixture::new("/* one\r\n two */ int\r\n", "a.c");
    f.select((1, 1), (1, 7));
    let html = f.html(false, true).unwrap();
    let comment = f.open(ColorKind::Comment);
    assert_eq!(
        html,
        format!("{}{comment}two */</span></pre>", f.pre()),
        "{html}"
    );
}

#[test]
fn special_characters_are_escaped_and_tabs_and_japanese_text_are_kept() {
    let mut f = Fixture::new("a<b && c>d\t日本語 \"q\"\r\n", "a.txt");
    f.select((0, 0), (1, 0));
    let text = f.open(ColorKind::Text);
    let html = f.html(false, true).unwrap();
    assert_eq!(
        html,
        format!(
            "{}{text}a&lt;b &amp;&amp; c&gt;d\t日本語 \"q\"\r\n</span></pre>",
            f.pre()
        )
    );
}

#[test]
fn the_selection_covers_several_lines_with_their_line_endings() {
    let mut f = Fixture::new("one\r\ntwo\r\nthree\r\n", "a.txt");
    f.select((0, 1), (2, 2));
    let text = f.open(ColorKind::Text);
    assert_eq!(
        f.html(false, true).unwrap(),
        format!("{}{text}ne\r\ntwo\r\nth</span></pre>", f.pre())
    );
}

#[test]
fn a_selection_that_ends_at_the_start_of_a_line_does_not_take_that_line() {
    let mut f = Fixture::new("one\r\ntwo\r\n", "a.txt");
    f.select((0, 0), (1, 0));
    let text = f.open(ColorKind::Text);
    assert_eq!(
        f.html(false, true).unwrap(),
        format!("{}{text}one\r\n</span></pre>", f.pre())
    );
    // and with the line numbers there is no number for the line after it
    let html = f.html(true, true).unwrap();
    assert_eq!(html, format!("{}1:{text}one\r\n</span></pre>", f.pre()));
}

#[test]
fn line_numbers_are_in_front_of_each_line_and_outside_the_spans() {
    let mut f = Fixture::new("int a;\r\n// c\r\nb\r\n", "a.c");
    f.select((0, 0), (2, 1));
    let keyword = f.open(ColorKind::Keyword(0));
    let comment = f.open(ColorKind::Comment);
    let text = f.open(ColorKind::Text);
    // the number is as wide as the digits of the last line index (2): one digit
    assert_eq!(
        f.html(true, true).unwrap(),
        format!(
            "{pre}1:{keyword}int</span>{text} a;\r\n</span>2:{comment}// c</span>{text}\r\n</span>\
             3:{text}b</span></pre>",
            pre = f.pre()
        )
    );
}

#[test]
fn line_numbers_are_as_wide_as_the_last_one() {
    let lines: String = (0..12).map(|i| format!("l{i}\r\n")).collect();
    let mut f = Fixture::new(&lines, "a.txt");
    f.select((0, 0), (11, 2));
    let html = f.html(true, true).unwrap();
    let text = f.open(ColorKind::Text);
    // the last line index is 11: two digits
    assert!(
        html.starts_with(&format!("{}{text}", f.pre())) || html.contains(" 1:"),
        "{html}"
    );
    assert!(html.contains(" 1:"), "{html}");
    assert!(html.contains("12:"), "{html}");
    assert!(html.contains(" 9:"), "{html}");
}

#[test]
fn a_box_takes_the_columns_of_each_row_and_ends_each_with_crlf() {
    let mut f = Fixture::new("abcdef\r\nxyz\r\n1234567\r\n", "a.txt");
    f.select_box((0, 1), (2, 4));
    let text = f.open(ColorKind::Text);
    // row 1 is shorter than the box: only what is there
    assert_eq!(
        f.html(false, true).unwrap(),
        format!("{}{text}bcd\r\nyz\r\n234\r\n</span></pre>", f.pre())
    );
}

#[test]
fn a_box_with_line_numbers_numbers_every_row() {
    let mut f = Fixture::new("abcdef\r\nxyz\r\n", "a.txt");
    f.select_box((0, 1), (1, 3));
    let text = f.open(ColorKind::Text);
    assert_eq!(
        f.html(true, true).unwrap(),
        format!(
            "{}1:{text}bc</span>\r\n2:{text}yz</span>\r\n</pre>",
            f.pre()
        )
    );
}

#[test]
fn wrapped_rows_are_numbered_by_line_or_by_row() {
    let cfg = LayoutConfig {
        wrap: Wrap::Columns(5),
        ..LayoutConfig::default()
    };
    let mut f = Fixture::wrapped("abcdefgh\r\nij\r\n", "a.txt", cfg);
    f.select((0, 0), (1, 2));
    let text = f.open(ColorKind::Text);
    // `bLineNumIsCRLF`: the second row of a line has no number, and the tags close after every row
    assert_eq!(
        f.html(true, true).unwrap(),
        format!(
            "{}1:{text}abcde</span>{text}fgh\r\n</span>2:{text}ij</span></pre>",
            f.pre()
        )
    );
    // otherwise the number is the number of the row, and the line is one piece
    assert_eq!(
        f.html(true, false).unwrap(),
        format!(
            "{}1:{text}abcdefgh\r\n</span>3:{text}ij</span></pre>",
            f.pre()
        )
    );
}

#[test]
fn rows_of_a_wrapped_line_after_the_selection_are_not_copied() {
    let cfg = LayoutConfig {
        wrap: Wrap::Columns(4),
        ..LayoutConfig::default()
    };
    let mut f = Fixture::wrapped("abcdefgh\r\n", "a.txt", cfg);
    f.select((0, 0), (0, 2));
    let text = f.open(ColorKind::Text);
    assert_eq!(
        f.html(false, true).unwrap(),
        format!("{}{text}ab</span></pre>", f.pre())
    );
}

// ---- the tags, as `AppendHTMLColor` writes them ----

const RED: Option<(u32, u32)> = Some((0xff0000, 0xffffff));
const BLUE: Option<(u32, u32)> = Some((0x0000ff, 0xffffff));

fn attr(color: Option<(u32, u32)>, bold: bool, underline: bool) -> Attr {
    Attr {
        color,
        bold,
        underline,
    }
}

fn write(parts: &[(&str, Attr)]) -> String {
    let mut chars = Vec::new();
    let mut attrs = Vec::new();
    for (text, a) in parts {
        for c in text.chars() {
            chars.push(c);
            attrs.push(*a);
        }
    }
    let mut html = Html::new(0xffffff);
    let len = chars.len();
    html.row(&Row {
        chars,
        attrs,
        from: Some(0),
        to: len,
    });
    html.finish()
}

const OPEN_RED: &str = "<span style=\"color:#ff0000;background-color:#ffffff\">";
const OPEN_BLUE: &str = "<span style=\"color:#0000ff;background-color:#ffffff\">";
const PRE: &str = "<pre style=\"background-color:#ffffff\">";

#[test]
fn a_change_of_color_closes_the_span_and_opens_another() {
    let html = write(&[
        ("ab", attr(RED, false, false)),
        ("cd", attr(BLUE, false, false)),
    ]);
    assert_eq!(
        html,
        format!("{PRE}{OPEN_RED}ab</span>{OPEN_BLUE}cd</span></pre>")
    );
}

#[test]
fn bold_and_underline_nest_inside_the_span_and_close_in_reverse() {
    let html = write(&[
        ("ab", attr(RED, true, true)),
        ("cd", attr(RED, false, false)),
    ]);
    assert_eq!(
        html,
        format!("{PRE}{OPEN_RED}<u><b>ab</b></u>cd</span></pre>")
    );
}

#[test]
fn a_change_of_bold_alone_keeps_the_underline_open() {
    let html = write(&[
        ("ab", attr(RED, true, true)),
        ("cd", attr(RED, false, true)),
    ]);
    assert_eq!(
        html,
        format!("{PRE}{OPEN_RED}<u><b>ab</b>cd</u></span></pre>")
    );
}

#[test]
fn the_same_style_is_one_piece() {
    let html = write(&[
        ("ab", attr(RED, false, false)),
        ("cd", attr(RED, false, false)),
    ]);
    assert_eq!(html, format!("{PRE}{OPEN_RED}abcd</span></pre>"));
}

#[test]
fn the_background_of_the_pre_is_written_in_lowercase_hex_with_six_digits() {
    let html = Html::new(0x00abcd).finish();
    assert_eq!(html, "<pre style=\"background-color:#00abcd\"></pre>");
}

/// A search color for a test.
fn found_entry(text: u32, back: u32, bold: bool) -> ColorEntry {
    ColorEntry {
        disp: true,
        bold,
        underline: false,
        text,
        back,
    }
}

fn span(entry: ColorEntry) -> String {
    format!(
        "<span style=\"color:#{:06x};background-color:#{:06x}\">",
        rgb_of(entry.text),
        rgb_of(entry.back)
    )
}

#[test]
fn a_match_of_the_search_has_the_search_color_in_the_copy() {
    let mut f = Fixture::new("one two one\r\n", "a.txt");
    f.select((0, 0), (1, 0));
    let text = f.open(ColorKind::Text);
    let found = found_entry(0x0000ff, 0x00ffff, false);
    let html = f.html_found("two", false, &[found]).unwrap();
    assert_eq!(
        html,
        format!(
            "{}{text}one </span>{}two</span>{text} one\r\n</span></pre>",
            f.pre(),
            span(found)
        )
    );
}

#[test]
fn a_match_takes_over_from_the_color_of_the_syntax() {
    let mut f = Fixture::new("int a; // int\r\n", "a.c");
    f.select((0, 0), (1, 0));
    let found = found_entry(0x0000ff, 0x00ffff, true);
    let html = f.html_found("int", false, &[found]).unwrap();
    // the keyword and the word in the comment are both in the search color, bold
    assert_eq!(html.matches("<b>int</b>").count(), 2, "{html}");
    assert!(!html.contains(&f.open(ColorKind::Keyword(0))), "{html}");
}

#[test]
fn the_words_of_a_whole_word_search_have_the_colors_in_turn() {
    let mut f = Fixture::new("aa bb cc aa\r\n", "a.txt");
    f.select((0, 0), (0, 11));
    let first = found_entry(0x000001, 0x000010, false);
    let second = found_entry(0x000002, 0x000020, false);
    let html = f.html_found("aa bb", true, &[first, second]).unwrap();
    assert!(
        html.contains(&format!("{}bb</span>", span(second))),
        "{html}"
    );
    // the first word of the key has the first color wherever it comes
    assert_eq!(
        html.matches(&format!("{}aa</span>", span(first))).count(),
        2,
        "{html}"
    );
}

#[test]
fn no_search_color_shown_leaves_the_copy_as_it_is() {
    let mut f = Fixture::new("one two\r\n", "a.txt");
    f.select((0, 0), (1, 0));
    assert_eq!(f.html_found("two", false, &[]), f.html(false, true));
}

#[test]
fn a_match_over_the_line_ending_colors_it_too() {
    let mut f = Fixture::new("ab\r\ncd\r\n", "a.txt");
    f.select((0, 0), (2, 0));
    let found = found_entry(0x0000ff, 0x00ffff, false);
    // a regular expression sees the line ending, but never the next line
    let regex =
        matsu_regex::RegexMatcher::new(r"b\r\n", matsu_regex::RegexSearchOptions::default())
            .unwrap();
    let html = f.html_with(&regex, None, &[found]).unwrap();
    let text = f.open(ColorKind::Text);
    assert_eq!(
        html,
        format!(
            "{}{text}a</span>{}b\r\n</span>{text}cd\r\n</span></pre>",
            f.pre(),
            span(found)
        )
    );
}

// ---- a proportional font (ADR-0034, stage 5) ----

/// A narrow `i`, a wide `m`, the rest a half-width letter.
fn proportional() -> LayoutConfig {
    LayoutConfig {
        width: matsu_layout::Widths::table(64, 0, |c| match c {
            'i' | 'l' => 32,
            'm' => 107,
            _ => 64,
        }),
        ..LayoutConfig::default()
    }
}

#[test]
fn a_tab_stays_a_tab_in_the_copy_with_a_proportional_font() {
    // Sakura writes the tab as it is (`Command_COPY_COLOR_HTML` has no tab expansion, and the
    // `<pre>` has no font): the browser puts it where its own font and tab size say, so the
    // widths of the layout do not change the text of the copy.
    let mut f = Fixture::wrapped("mi\tim\r\n", "a.txt", proportional());
    f.select((0, 0), (1, 0));
    let text = f.open(ColorKind::Text);
    assert_eq!(
        f.html(false, true).unwrap(),
        format!("{}{text}mi\tim\r\n</span></pre>", f.pre())
    );
}

#[test]
fn a_match_after_a_tab_is_colored_by_its_characters_with_a_proportional_font() {
    // `mi` ends at 139 of a 256 stop, so the tab takes 117 X and the match starts at 256.
    let mut f = Fixture::wrapped("mi\tim mim\r\n", "a.txt", proportional());
    f.select((0, 0), (1, 0));
    let text = f.open(ColorKind::Text);
    let found = found_entry(0x0000ff, 0x00ffff, false);
    let html = f.html_found("mim", false, &[found]).unwrap();
    assert_eq!(
        html,
        format!(
            "{}{text}mi\tim </span>{}mim</span>{text}\r\n</span></pre>",
            f.pre(),
            span(found)
        )
    );
}

#[test]
fn the_html_is_not_built_past_its_limit() {
    use super::collect::{OutputFull, color_html_within};

    let text = "int main(void) { return 0; }\r\n".repeat(200);
    let mut f = Fixture::new(&text, "a.c");
    f.select((0, 0), (199, 0));
    let source = Source {
        doc: &f.doc,
        layout: &f.layout,
        rules: &f.rules,
        states: &f.states,
        cursor: &f.cursor,
        line_num_is_crlf: true,
        line_numbers: false,
        found: None,
    };
    let whole = color_html_within(&source, usize::MAX).unwrap().unwrap();
    assert!(whole.len() > 2000);
    // under the limit: the same HTML; over it: the size so far, which is a row past the limit at
    // most, not the whole HTML
    assert_eq!(
        color_html_within(&source, whole.len() + 1)
            .unwrap()
            .unwrap(),
        whole
    );
    let OutputFull(written) = color_html_within(&source, 1000).unwrap_err();
    assert!((1000..2000).contains(&written), "{written}");
}

#[test]
fn a_row_that_grows_when_escaped_stops_near_the_limit_inside_the_row() {
    use super::collect::{OutputFull, color_html_within};

    // 400 000 chars are 2 000 000 bytes of `&amp;`; the limit is 100 000
    for ch in ['&', '<'] {
        let text = format!("{}\r\nx", ch.to_string().repeat(400_000));
        let mut f = Fixture::new(&text, "a.txt");
        f.select((0, 0), (1, 0));
        let source = Source {
            doc: &f.doc,
            layout: &f.layout,
            rules: &f.rules,
            states: &f.states,
            cursor: &f.cursor,
            line_num_is_crlf: true,
            line_numbers: false,
            found: None,
        };
        let OutputFull(written) = color_html_within(&source, 100_000).unwrap_err();
        // within a few bytes of the limit, not a whole row of 2 MB
        assert!((100_000..100_100).contains(&written), "{ch}: {written}");
        // with room enough the whole row is written, and the closing tags count in the size
        let whole = color_html_within(&source, usize::MAX).unwrap().unwrap();
        assert!(whole.len() > 1_000_000, "{}", whole.len());
        assert!(color_html_within(&source, whole.len()).is_err());
        assert!(color_html_within(&source, whole.len() + 1).is_ok());
    }
}

#[test]
fn a_selection_with_a_very_long_line_is_refused_before_it_is_laid_out() {
    use super::LINE_LIMIT;
    use super::collect::line_over;

    // one line of LINE_LIMIT + 100 bytes, between two short ones
    let long = "x".repeat(LINE_LIMIT + 100);
    let text = format!("short\r\n{long}\r\nshort\r\n");
    let mut f = Fixture::new(&text, "a.txt");
    let over = |f: &Fixture, limit: usize| line_over(&f.doc, &f.cursor, limit);
    // a selection that touches the long line, as a part of it or with the lines around it
    for (from, to) in [((0, 0), (3, 0)), ((1, 10), (1, 20)), ((1, 0), (2, 0))] {
        f.select(from, to);
        assert_eq!(
            over(&f, LINE_LIMIT),
            Some(LINE_LIMIT + 100),
            "{from:?} {to:?}"
        );
    }
    // a selection that ends at the head of the long line is refused too: its row is laid out
    f.select((0, 0), (1, 0));
    assert_eq!(over(&f, LINE_LIMIT), Some(LINE_LIMIT + 100));
    // short lines are not
    for (from, to) in [((2, 0), (3, 0)), ((0, 0), (0, 3))] {
        f.select(from, to);
        assert_eq!(over(&f, LINE_LIMIT), None, "{from:?} {to:?}");
    }
    // a line of exactly the limit is refused, one byte less is not
    f.select((1, 0), (2, 0));
    assert_eq!(over(&f, LINE_LIMIT + 100), Some(LINE_LIMIT + 100));
    assert_eq!(over(&f, LINE_LIMIT + 101), None);
    // a box takes the lines of its rows
    f.select_box((0, 0), (1, 2));
    assert_eq!(over(&f, LINE_LIMIT), Some(LINE_LIMIT + 100));
    // no selection, nothing to refuse
    f.cursor.clear_selection();
    assert_eq!(over(&f, LINE_LIMIT), None);
}

#[test]
fn a_document_read_by_ranges_has_no_line_limit_for_the_html() {
    use super::collect::line_over;
    use matsu_core::buffer::{EncodingCodec, MemorySource, ScanOptions, scan_blocks};
    use matsu_core::encoding::Encoding;
    use std::ops::ControlFlow;
    use std::sync::Arc;

    let text = format!("{}\nend\n", "x".repeat(5000));
    let index = scan_blocks(
        Arc::new(MemorySource::new(text.into_bytes())),
        Arc::new(EncodingCodec(Encoding::Utf8)),
        ScanOptions {
            block_size: 64,
            window_blocks: 3,
            long_line: 1000,
            ..ScanOptions::default()
        },
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    let doc = Document::from_block_index(index).unwrap();
    assert!(doc.is_long_line(0));
    let mut cursor = Cursor::default();
    cursor.move_to(Pos::new(0, 0), false);
    cursor.move_to(Pos::new(1, 0), true);
    assert_eq!(line_over(&doc, &cursor, 1000), None);
}

#[test]
fn the_message_of_a_long_line_says_the_size_and_the_limit() {
    let message = super::long_line_message(3 * 1024 * 1024 + 1);
    assert!(
        message.contains("4 MB") && message.contains("上限は 1 MB"),
        "{message}"
    );
}

/// Not a test: the peak memory of the colored HTML copy of one long line, for the PR. Run it in a
/// release build, one step at a time: `MATSU_HTML_LINE_MB=100 MATSU_HTML_STEP=before|after cargo test
/// -p matsu-ui --release --lib measure_the_html_of_a_long_line -- --ignored --nocapture`.
/// "before" turns the refusal of a long line off (the arrays of the whole row are built; the
/// output stops at 256 MiB as in the app).
#[test]
#[ignore = "measures a row of MATSU_HTML_LINE_MB megabytes; run in a release build"]
fn measure_the_html_of_a_long_line() {
    use super::collect::{color_html_within, line_over};

    let Ok(mb) = std::env::var("MATSU_HTML_LINE_MB") else {
        return;
    };
    let mb: usize = mb.parse().unwrap();
    let step = std::env::var("MATSU_HTML_STEP").unwrap_or_default();
    let peak = || {
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find_map(|l| l.strip_prefix("VmHWM:"))
                    .and_then(|l| l.split_whitespace().next()?.parse::<u64>().ok())
            })
            .map_or(0, |kb| kb / 1024)
    };
    let text = format!(
        "{}\r\nend",
        "int x = 1; /* a */ ".repeat(mb * 1024 * 1024 / 19)
    );
    let mut f = Fixture::new(&text, "a.c");
    f.select((0, 0), (1, 0));
    let source = Source {
        doc: &f.doc,
        layout: &f.layout,
        rules: &f.rules,
        states: &f.states,
        cursor: &f.cursor,
        line_num_is_crlf: true,
        line_numbers: false,
        found: None,
    };
    println!(
        "{mb} MB line, step {step}: memory before the copy {} MB",
        peak()
    );
    let started = std::time::Instant::now();
    // "before": no refusal of a long line (the check comes before the layout, in the command);
    // "after": the check of the command
    let refused = (step != "before")
        .then(|| line_over(&f.doc, &f.cursor, super::LINE_LIMIT))
        .flatten();
    let result = match refused {
        Some(bytes) => Err(format!("a line of {bytes} bytes is refused")),
        None => color_html_within(&source, 256 * 1024 * 1024)
            .map_err(|full| format!("the output is full at {} bytes", full.0)),
    };
    println!(
        "{}: {:.2} s, peak memory {} MB",
        match &result {
            Ok(Some(html)) => format!("{} bytes of HTML", html.len()),
            Ok(None) => "nothing".to_owned(),
            Err(why) => format!("refused: {why}"),
        },
        started.elapsed().as_secs_f64(),
        peak()
    );
}
