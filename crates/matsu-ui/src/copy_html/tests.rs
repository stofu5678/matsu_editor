//! Tests of the colored HTML copy without a window: the rows, the colors of a C file, special
//! characters, tabs, Japanese text, the line numbers and the box selection. The tags
//! are checked in `builder_tests` against Sakura's `AppendHTMLColor`.

use matsu_coloring::{ColorKind, ColorRules, ColorStates};
use matsu_core::buffer::{Document, Pos};
use matsu_core::cursor::Cursor;
use matsu_layout::{DocumentLayout, LayoutConfig, Wrap};

use super::builder::{Attr, Html, Row};
use super::collect::{Source, color_html};
use crate::settings::UserSettings;
use crate::settings::view_settings::rgb_of;

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
            .row_to_pos(&self.doc, row, col as u32)
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
