//! Tests of the strategies: each rule of Sakura's color strategies with the cases that are
//! hard to get right (escapes, a comment start inside a string and the reverse, a keyword inside an
//! identifier, case, numbers, URLs at the end of a line).

use ume_config::settings::{DocumentTypes, TypeSettings};
use ume_core::buffer::Eol;

use super::*;
use crate::{ColorKind as K, line_end_state, line_runs};

const COMMENT: K = K::Comment;
const SQ: K = K::SingleQuote;
const DQ: K = K::DoubleQuote;

fn type_rules(name: &str, edit: impl FnOnce(&mut TypeSettings)) -> ColorRules {
    let types = DocumentTypes::builtin();
    let index = types.index_of_name(name).expect("a built-in type");
    let mut t = types.get(index).unwrap().clone();
    edit(&mut t);
    ColorRules::from_type(&t, types.keywords())
}

fn rules(name: &str) -> ColorRules {
    type_rules(name, |_| {})
}

/// The type for plain text: URLs are colored, there is no comment to take them.
fn text_rules() -> ColorRules {
    rules("テキスト")
}

/// The pieces of the text of a line (without its line ending) with their colors, from the state.
fn pieces_from(rules: &ColorRules, text: &str, state: &ColorState) -> Vec<(String, ColorKind)> {
    let runs = line_runs(rules, text, Some(Eol::Lf), state, text.len());
    runs.iter()
        .filter_map(|r| {
            let end = r.end.min(text.len());
            (r.start < end).then(|| (text[r.start..end].to_owned(), r.kind))
        })
        .collect()
}

fn pieces(rules: &ColorRules, text: &str) -> Vec<(String, ColorKind)> {
    pieces_from(rules, text, &ColorState::Normal)
}

fn p(text: &str, kind: ColorKind) -> (String, ColorKind) {
    (text.to_owned(), kind)
}

fn end_state(rules: &ColorRules, text: &str, state: &ColorState) -> ColorState {
    line_end_state(rules, text, Some(Eol::Lf), state)
}

#[test]
fn a_line_comment_runs_to_the_end_of_the_line() {
    let c = rules("C/C++");
    assert_eq!(
        pieces(&c, "x = 1; // note \"a"),
        [
            p("x = ", K::Text),
            p("1", K::Number),
            p("; ", K::Text),
            p("// note \"a", COMMENT),
        ]
    );
}

#[test]
fn a_comment_start_inside_a_string_does_not_start_a_comment() {
    let c = rules("C/C++");
    assert_eq!(
        pieces(&c, "s = \"a // b\"; // c"),
        [
            p("s = ", K::Text),
            p("\"a // b\"", DQ),
            p("; ", K::Text),
            p("// c", COMMENT),
        ]
    );
    assert_eq!(
        pieces(&c, "s = '/*'; x"),
        [p("s = ", K::Text), p("'/*'", SQ), p("; x", K::Text)]
    );
}

#[test]
fn a_string_start_inside_a_comment_does_not_start_a_string() {
    let c = rules("C/C++");
    assert_eq!(pieces(&c, "// say \"hi"), [p("// say \"hi", COMMENT)]);
    assert_eq!(
        pieces(&c, "a /* it's */ b"),
        [p("a ", K::Text), p("/* it's */", COMMENT), p(" b", K::Text)]
    );
    // The same state for the next line: the string did not open.
    assert_eq!(
        end_state(&c, "/* it's */ b", &ColorState::Normal),
        ColorState::Normal
    );
}

#[test]
fn escapes_in_strings() {
    let c = rules("C/C++");
    assert_eq!(
        pieces(&c, r#""a\"b" x"#),
        [p(r#""a\"b""#, DQ), p(" x", K::Text)]
    );
    assert_eq!(
        pieces(&c, r#""a\\" x"#),
        [p(r#""a\\""#, DQ), p(" x", K::Text)]
    );
    assert_eq!(pieces(&c, r"'\'' x"), [p(r"'\''", SQ), p(" x", K::Text)]);
    // PL/SQL: a doubled quote, not a backslash.
    let sql = rules("PL/SQL");
    assert_eq!(
        pieces(&sql, "'it''s' x"),
        [p("'it''s'", SQ), p(" x", K::Text)]
    );
    assert_eq!(pieces(&sql, r"'a\' x"), [p(r"'a\'", SQ), p(" x", K::Text)]);
}

#[test]
fn a_string_without_the_closing_quote_goes_on_to_the_next_line_only_with_an_escape() {
    // Sakura's C type has "strings in one line only" on; with it off the string goes on.
    let c = type_rules("C/C++", |t| t.string_line_only = false);
    let open = ColorState::Quote {
        double: true,
        kind: QuoteKind::Plain,
    };
    assert_eq!(end_state(&c, "x = \"abc", &ColorState::Normal), open);
    assert_eq!(end_state(&c, "x = \"abc\\", &ColorState::Normal), open);
    // The next line is a string up to the closing quote.
    assert_eq!(
        pieces_from(&c, "rest\" + 1", &open),
        [p("rest\"", DQ), p(" + ", K::Text), p("1", K::Number)]
    );
    assert_eq!(end_state(&c, "rest\" + 1", &open), ColorState::Normal);
    assert_eq!(end_state(&c, "still open", &open), open);
}

#[test]
fn strings_in_one_line_only() {
    // `m_bStringLineOnly` (on in the C type): without a closing quote the quote is not a string...
    let line_only = rules("C/C++");
    assert_eq!(pieces(&line_only, "x = \"abc"), [p("x = \"abc", K::Text)]);
    assert_eq!(
        end_state(&line_only, "x = \"abc", &ColorState::Normal),
        ColorState::Normal
    );
    // ...unless the backslash goes on to the next line,
    assert_eq!(
        end_state(&line_only, "x = \"abc\\", &ColorState::Normal),
        ColorState::Quote {
            double: true,
            kind: QuoteKind::Plain
        }
    );
    // ...and with `m_bStringEndLine` it is colored to the end of the line.
    let to_end = type_rules("C/C++", |t| {
        t.string_line_only = true;
        t.string_end_line = true;
    });
    assert_eq!(
        pieces(&to_end, "x = \"abc"),
        [p("x = ", K::Text), p("\"abc", DQ)]
    );
    assert_eq!(
        end_state(&to_end, "x = \"abc", &ColorState::Normal),
        ColorState::Normal
    );
}

#[test]
fn block_comments_across_lines() {
    let c = rules("C/C++");
    let inside = ColorState::Block(0);
    assert_eq!(
        end_state(&c, "int a; /* begin", &ColorState::Normal),
        inside
    );
    assert_eq!(
        pieces(&c, "int a; /* begin"),
        [
            p("int", K::Keyword(0)),
            p(" a; ", K::Text),
            p("/* begin", COMMENT)
        ]
    );
    assert_eq!(
        pieces_from(&c, "middle // x", &inside),
        [p("middle // x", COMMENT)]
    );
    assert_eq!(end_state(&c, "middle // x", &inside), inside);
    assert_eq!(
        pieces_from(&c, "end */ int b;", &inside),
        [
            p("end */", COMMENT),
            p(" ", K::Text),
            p("int", K::Keyword(0)),
            p(" b;", K::Text)
        ]
    );
    assert_eq!(end_state(&c, "end */ int b;", &inside), ColorState::Normal);
    // The end delimiter is not looked for inside the start: `/*/` does not close itself.
    assert_eq!(end_state(&c, "/*/", &ColorState::Normal), inside);
    // A comment closed and another opened on the same line.
    assert_eq!(end_state(&c, "*/ x /* y", &inside), inside);
}

#[test]
fn the_second_block_comment_is_if_0() {
    let c = rules("C/C++");
    let inside = ColorState::Block(1);
    assert_eq!(end_state(&c, "#if 0", &ColorState::Normal), inside);
    assert_eq!(pieces(&c, "#if 0"), [p("#if 0", COMMENT)]);
    assert_eq!(
        pieces_from(&c, "int x; // y", &inside),
        [p("int x; // y", COMMENT)]
    );
    assert_eq!(
        pieces_from(&c, "#endif int", &inside),
        [
            p("#endif", COMMENT),
            p(" ", K::Text),
            p("int", K::Keyword(0))
        ]
    );
    assert_eq!(end_state(&c, "#endif", &inside), ColorState::Normal);
}

#[test]
fn comment_delimiters_ignore_ascii_case() {
    let pascal = rules("Pascal");
    assert_eq!(
        pieces(&pascal, "{ x } y"),
        [p("{ x }", COMMENT), p(" y", K::Text)]
    );
    let dos = rules("MS-DOSバッチファイル");
    assert_eq!(pieces(&dos, "rem hello"), [p("rem hello", COMMENT)]);
    assert_eq!(pieces(&dos, "REM hello"), [p("REM hello", COMMENT)]);
    assert_eq!(pieces(&dos, "remark"), [p("remark", K::Text)]);
}

#[test]
fn a_line_comment_that_must_start_in_a_column() {
    // COBOL: `*` in the seventh column.
    let cobol = rules("COBOL");
    assert_eq!(
        pieces(&cobol, "      * note"),
        [p("      ", K::Text), p("* note", COMMENT)]
    );
    assert_eq!(pieces(&cobol, "     * note"), [p("     * note", K::Text)]);
    // The column counts UTF-16 units: a character outside the BMP takes two.
    assert_eq!(
        pieces(&cobol, "😀    * note"),
        [p("😀    ", K::Text), p("* note", COMMENT)]
    );
    assert_eq!(pieces(&cobol, "😀   * note"), [p("😀   * note", K::Text)]);
}

#[test]
fn keywords_are_whole_words() {
    let c = rules("C/C++");
    assert_eq!(
        pieces(&c, "int integer x_int int_x int"),
        [
            p("int", K::Keyword(0)),
            p(" integer x_int int_x ", K::Text),
            p("int", K::Keyword(0)),
        ]
    );
    assert_eq!(
        pieces(&c, "if(x)"),
        [p("if", K::Keyword(0)), p("(x)", K::Text)]
    );
    // `#include` is a keyword with its `#`.
    assert_eq!(
        pieces(&c, "#include <a.h>"),
        [p("#include", K::Keyword(0)), p(" <a.h>", K::Text)]
    );
    // Japanese text next to a keyword is another word.
    assert_eq!(
        pieces(&c, "if日本"),
        [p("if", K::Keyword(0)), p("日本", K::Text)]
    );
}

#[test]
fn keyword_sets_can_ignore_case() {
    let sql = rules("PL/SQL");
    assert_eq!(
        pieces(&sql, "SELECT select Select selectx"),
        [
            p("SELECT", K::Keyword(0)),
            p(" ", K::Text),
            p("select", K::Keyword(0)),
            p(" ", K::Text),
            p("Select", K::Keyword(0)),
            p(" selectx", K::Text),
        ]
    );
    // C is case sensitive.
    let c = rules("C/C++");
    assert_eq!(pieces(&c, "INT Int"), [p("INT Int", K::Text)]);
}

#[test]
fn a_keyword_is_the_longest_that_matches() {
    // Two sets in one slot would be needed to tell; one set with `if`, `int` and `interface`.
    let java = rules("Java");
    assert_eq!(
        pieces(&java, "interface int i"),
        [
            p("interface", K::Keyword(0)),
            p(" ", K::Text),
            p("int", K::Keyword(0)),
            p(" i", K::Text)
        ]
    );
}

#[test]
fn the_first_slot_with_a_keyword_colors_it() {
    let perl = rules("Perl");
    // `my` is in the first set, `$ARGV` in the second: the `$` goes with the name.
    assert_eq!(
        pieces(&perl, "my $ARGV"),
        [
            p("my", K::Keyword(0)),
            p(" ", K::Text),
            p("$ARGV", K::Keyword(1))
        ]
    );
    // A keyword made of symbols: `$&` is longer than `$`.
    assert_eq!(
        pieces(&perl, "$& x"),
        [p("$&", K::Keyword(1)), p(" x", K::Text)]
    );
}

#[test]
fn numbers() {
    let c = rules("C/C++");
    assert_eq!(
        pieces(&c, "a = 0x1F + 1.5e3 - 1_000;"),
        [
            p("a = ", K::Text),
            p("0x1F", K::Number),
            p(" + ", K::Text),
            p("1.5e3", K::Number),
            p(" - ", K::Text),
            p("1", K::Number),
            p("_000;", K::Text),
        ]
    );
    // A number inside a name is not a number.
    assert_eq!(pieces(&c, "x1 y_2"), [p("x1 y_2", K::Text)]);
    // A sign makes part of the number when nothing is before it.
    assert_eq!(
        pieces(&c, "x = -1;"),
        [p("x = ", K::Text), p("-1", K::Number), p(";", K::Text)]
    );
    // A sign after a name is not part of a number (a name is a "keyword head" for numbers), but
    // the digit after the sign starts one.
    assert_eq!(pieces(&c, "x-1"), [p("x-", K::Text), p("1", K::Number)]);
}

#[test]
fn numbers_are_off_when_their_color_is_off() {
    let no_numbers = type_rules("C/C++", |t| t.colors.get_mut("NUM").unwrap().disp = false);
    assert_eq!(pieces(&no_numbers, "1 + 2"), [p("1 + 2", K::Text)]);
}

#[test]
fn urls_and_mail_addresses() {
    let c = rules("C/C++");
    assert_eq!(
        pieces(&c, "// see http://example.com/a?b=1."),
        [p("// see http://example.com/a?b=1.", COMMENT)]
    );
    assert_eq!(
        pieces(&c, "x http://example.com/a?b=1"),
        [p("x ", K::Text), p("http://example.com/a?b=1", K::Url)]
    );
    // At the end of the line.
    assert_eq!(
        pieces(&c, "x http://example.com"),
        [p("x ", K::Text), p("http://example.com", K::Url)]
    );
    assert_eq!(
        pieces(&c, "mail me@example.jp now"),
        [
            p("mail ", K::Text),
            p("me@example.jp", K::Url),
            p(" now", K::Text)
        ]
    );
    // A URL needs a boundary before it (the `//` that follows is a comment in the C type).
    assert_eq!(
        pieces(&c, "xhttp://a.b"),
        [p("xhttp:", K::Text), p("//a.b", COMMENT)]
    );
    assert_eq!(
        pieces(&text_rules(), "xhttp://a.b"),
        [p("xhttp://a.b", K::Text)]
    );
    // The scheme alone is nothing.
    assert_eq!(
        pieces(&text_rules(), "http:// x"),
        [p("http:// x", K::Text)]
    );
}

#[test]
fn a_url_in_a_string_is_part_of_the_string() {
    let c = rules("C/C++");
    assert_eq!(pieces(&c, "\"http://a.b/c\""), [p("\"http://a.b/c\"", DQ)]);
}

#[test]
fn python_triple_quotes() {
    let py = rules("Python");
    let open = ColorState::Quote {
        double: true,
        kind: QuoteKind::Triple,
    };
    assert_eq!(
        pieces(&py, "x = \"\"\"doc\"\"\" + 1"),
        [
            p("x = ", K::Text),
            p("\"\"\"doc\"\"\"", DQ),
            p(" + ", K::Text),
            p("1", K::Number)
        ]
    );
    assert_eq!(end_state(&py, "\"\"\"doc", &ColorState::Normal), open);
    assert_eq!(
        pieces_from(&py, "more \"quoted\" text\"\"\" x", &open),
        [p("more \"quoted\" text\"\"\"", DQ), p(" x", K::Text)]
    );
    assert_eq!(
        end_state(&py, "more \"quoted\" text\"\"\" x", &open),
        ColorState::Normal
    );
    assert_eq!(end_state(&py, "no end yet", &open), open);
    // A single `"` is an ordinary string.
    assert_eq!(pieces(&py, "'a'"), [p("'a'", SQ)]);
    // A backslash escapes in triple quotes too.
    assert_eq!(
        pieces_from(&py, "a\\\"\"\" b\"\"\"", &open),
        [p("a\\\"\"\" b\"\"\"", DQ)]
    );
    // `#` starts a comment.
    assert_eq!(pieces(&py, "x # y"), [p("x ", K::Text), p("# y", COMMENT)]);
}

#[test]
fn cpp_raw_strings() {
    let c = rules("C/C++");
    let raw = |tag: &str| ColorState::Quote {
        double: true,
        kind: QuoteKind::Raw(tag.to_owned()),
    };
    assert_eq!(
        pieces(&c, "R\"x(a \" b)x\" c"),
        [p("R", K::Text), p("\"x(a \" b)x\"", DQ), p(" c", K::Text)]
    );
    assert_eq!(
        end_state(&c, "auto s = R\"(first", &ColorState::Normal),
        raw(")\"")
    );
    assert_eq!(
        end_state(&c, "u8R\"sep(first", &ColorState::Normal),
        raw(")sep\"")
    );
    assert_eq!(
        pieces_from(&c, "second)\" + 1", &raw(")\"")),
        [p("second)\"", DQ), p(" + ", K::Text), p("1", K::Number)]
    );
    assert_eq!(
        end_state(&c, "just text)x\"", &raw(")sep\"")),
        raw(")sep\"")
    );
    // `R"` after a name is not a raw string, so it is an ordinary string, which the C type
    // ("strings in one line only") does not color without the closing quote.
    assert_eq!(
        end_state(&c, "xR\"(first", &ColorState::Normal),
        ColorState::Normal
    );
    let open_strings = type_rules("C/C++", |t| t.string_line_only = false);
    assert_eq!(
        end_state(&open_strings, "xR\"(first", &ColorState::Normal),
        ColorState::Quote {
            double: true,
            kind: QuoteKind::Plain
        }
    );
}

#[test]
fn csharp_verbatim_strings() {
    let cs = type_rules("C/C++", |t| t.string_type = string_type::CSHARP);
    assert_eq!(
        pieces(&cs, "x = @\"a\\\"\"b\" y"),
        [p("x = @", K::Text), p("\"a\\\"\"b\"", DQ), p(" y", K::Text)]
    );
    let open = ColorState::Quote {
        double: true,
        kind: QuoteKind::Verbatim,
    };
    assert_eq!(end_state(&cs, "x = @\"first", &ColorState::Normal), open);
    // A backslash does not escape in a verbatim string.
    assert_eq!(
        pieces_from(&cs, "a\\\" b", &open),
        [p("a\\\"", DQ), p(" b", K::Text)]
    );
    // An ordinary string still escapes with a backslash, and a backslash before the line ending
    // does not go on to the next line (C# does not track it).
    assert_eq!(pieces(&cs, "\"a\\\"b\""), [p("\"a\\\"b\"", DQ)]);
}

#[test]
fn html_attribute_values_need_an_equal_sign() {
    let html = type_rules("HTML", |t| t.colors.get_mut("KW1").unwrap().disp = false);
    assert_eq!(
        pieces(&html, "<a href=\"x\" id = 'y'>\"z\""),
        [
            p("<a href=", K::Text),
            p("\"x\"", DQ),
            p(" id = ", K::Text),
            p("'y'", SQ),
            p(">\"z\"", K::Text),
        ]
    );
    assert_eq!(
        pieces(&html, "a <!-- b --> c"),
        [p("a ", K::Text), p("<!-- b -->", COMMENT), p(" c", K::Text)]
    );
}

#[test]
fn php_heredocs() {
    let php = type_rules("C/C++", |t| {
        t.heredoc_type = 0;
        t.colors.get_mut("HDC").unwrap().disp = true;
    });
    let inside = ColorState::Heredoc("EOT".to_owned());
    assert_eq!(end_state(&php, "$a = <<<EOT", &ColorState::Normal), inside);
    assert_eq!(
        pieces(&php, "$a = <<<EOT"),
        [p("$a = ", K::Text), p("<<<EOT", K::Heredoc)]
    );
    assert_eq!(
        end_state(&php, "$a = <<<'EOT'", &ColorState::Normal),
        inside
    );
    assert_eq!(
        end_state(&php, "$a = <<<\"EOT\"", &ColorState::Normal),
        inside
    );
    assert_eq!(end_state(&php, "$a = <<< EOT", &ColorState::Normal), inside);
    // The id has to be the last thing on the line.
    assert_eq!(
        end_state(&php, "$a = <<<EOT x", &ColorState::Normal),
        ColorState::Normal
    );
    assert_eq!(
        end_state(&php, "$a = <<<'EOT", &ColorState::Normal),
        ColorState::Normal
    );
    // The lines of the here-document, up to the line with the id.
    assert_eq!(
        pieces_from(&php, "text // not \"a comment\" 1", &inside),
        [p("text // not \"a comment\" 1", K::Heredoc)]
    );
    assert_eq!(end_state(&php, "text", &inside), inside);
    assert_eq!(end_state(&php, "EOTX", &inside), inside);
    assert_eq!(end_state(&php, " EOT", &inside), inside);
    assert_eq!(end_state(&php, "EOT", &inside), ColorState::Normal);
    assert_eq!(pieces_from(&php, "EOT", &inside), [p("EOT", K::Heredoc)]);
    assert_eq!(
        pieces_from(&php, "EOT;", &inside),
        [p("EOT", K::Heredoc), p(";", K::Text)]
    );
    assert_eq!(end_state(&php, "EOT;", &inside), ColorState::Normal);
    // `ID;` has to be the whole line.
    assert_eq!(end_state(&php, "EOT; echo 1;", &inside), inside);
    // Off by default (`HDC` is not shown).
    let off = rules("C/C++");
    assert_eq!(
        end_state(&off, "$a = <<<EOT", &ColorState::Normal),
        ColorState::Normal
    );
}

#[test]
fn a_state_of_a_strategy_that_is_off_is_not_resumed() {
    let c = type_rules("C/C++", |t| t.colors.get_mut("CMT").unwrap().disp = false);
    assert_eq!(
        pieces_from(&c, "text", &ColorState::Block(0)),
        [p("text", K::Text)]
    );
    assert_eq!(
        end_state(&c, "text", &ColorState::Block(0)),
        ColorState::Normal
    );
}

#[test]
fn the_color_of_the_line_ending() {
    let c = rules("C/C++");
    // Inside a block comment the line ending is a comment's; after a line comment it is not.
    let runs = line_runs(&c, "/* x", Some(Eol::CrLf), &ColorState::Normal, 4);
    assert_eq!(runs.last().unwrap().kind, COMMENT);
    assert_eq!(runs.last().unwrap().end, 6);
    let runs = line_runs(&c, "// x", Some(Eol::CrLf), &ColorState::Normal, 4);
    assert_eq!(runs.len(), 2);
    assert_eq!((runs[0].end, runs[0].kind), (4, COMMENT));
    assert_eq!(runs[1].kind, K::Text);
}

#[test]
fn runs_are_cut_at_the_row_end() {
    let c = rules("C/C++");
    let runs = line_runs(&c, "int abc // x", Some(Eol::Lf), &ColorState::Normal, 6);
    assert_eq!(runs.last().unwrap().end, 6);
    assert_eq!(runs[0].kind, K::Keyword(0));
}

#[test]
fn rules_that_color_nothing_make_one_text_run() {
    let none = ColorRules::none();
    assert_eq!(
        pieces(&none, "int x = 1; // c"),
        [p("int x = 1; // c", K::Text)]
    );
    assert_eq!(
        end_state(&none, "/* x", &ColorState::Normal),
        ColorState::Normal
    );
}

/// A C file, as one would find it: the states are carried from line to line the way the layout
/// does, and the lines are colored in the state the line before left.
const SAMPLE_C: &str = r##"/*
 * sample.c -- a file with all the usual things in it
 * see http://example.com/docs?id=1 for more.
 */
#include <stdio.h>
#define LIMIT 0x1F /* hex */

#if 0
int disabled = "not a string;
#endif

static const char *names[] = { "a\"b", 'c', "multi \
line" };

int main(int argc, char **argv)
{
	int i, total = 0; // counts
	for (i = 0; i < LIMIT; i++) {
		total += i * 2.5e1f; /* a /* b */ total -= 1;
	}
	printf("%d items, \\ done\n", total);
	return 0;
}
"##;

#[test]
fn a_real_c_file() {
    let c = rules("C/C++");
    let mut state = ColorState::Normal;
    let mut colored: Vec<Vec<(String, ColorKind)>> = Vec::new();
    let mut states = Vec::new();
    for line in SAMPLE_C.lines() {
        states.push(state.clone());
        colored.push(pieces_from(&c, line, &state));
        state = end_state(&c, line, &state);
    }
    assert_eq!(state, ColorState::Normal);
    let at = |n: usize| &colored[n - 1];

    // The block comment at the top, over four lines, with a URL in it that stays a comment.
    for n in 1..=4 {
        assert_eq!(at(n).len(), 1, "line {n}");
        assert_eq!(at(n)[0].1, COMMENT, "line {n}");
    }
    assert_eq!(states[1], ColorState::Block(0));
    assert_eq!(states[4], ColorState::Normal);

    assert_eq!(at(5)[0], p("#include", K::Keyword(0)));
    assert_eq!(
        at(6),
        &[
            p("#define", K::Keyword(0)),
            p(" LIMIT ", K::Text),
            p("0x1F", K::Number),
            p(" ", K::Text),
            p("/* hex */", COMMENT),
        ]
    );

    // `#if 0` ... `#endif` is the second block comment: the quote inside it is nothing.
    assert_eq!(at(8), &[p("#if 0", COMMENT)]);
    assert_eq!(states[8], ColorState::Block(1));
    assert_eq!(at(9), &[p("int disabled = \"not a string;", COMMENT)]);
    assert_eq!(at(10), &[p("#endif", COMMENT)]);
    assert_eq!(states[9], ColorState::Block(1));
    assert_eq!(states[10], ColorState::Normal);

    // A string with an escaped quote, a character, and a string that goes on over the line end.
    assert_eq!(
        at(12),
        &[
            p("static", K::Keyword(0)),
            p(" ", K::Text),
            p("const", K::Keyword(0)),
            p(" ", K::Text),
            p("char", K::Keyword(0)),
            p(" *names[] = { ", K::Text),
            p("\"a\\\"b\"", DQ),
            p(", ", K::Text),
            p("'c'", SQ),
            p(", ", K::Text),
            p("\"multi \\", DQ),
        ]
    );
    assert_eq!(
        states[12],
        ColorState::Quote {
            double: true,
            kind: QuoteKind::Plain
        }
    );
    assert_eq!(at(13), &[p("line\"", DQ), p(" };", K::Text)]);

    // Code, with a line comment and numbers.
    assert_eq!(at(17)[1], p("int", K::Keyword(0)));
    assert_eq!(*at(17).last().unwrap(), p("// counts", COMMENT));
    let numbers: Vec<&str> = at(18)
        .iter()
        .filter(|(_, kind)| *kind == K::Number)
        .map(|(t, _)| t.as_str())
        .collect();
    assert_eq!(numbers, ["0"]);
    // `/* a /* b */`: the comment ends at the first `*/`, the rest is code.
    let comment_line = at(19);
    assert!(
        comment_line.contains(&p("/* a /* b */", COMMENT)),
        "{comment_line:?}"
    );
    assert!(comment_line.contains(&p("1", K::Number)));
    assert!(comment_line.contains(&p("2.5e1f", K::Number)));
    // A format string with escapes is one string.
    assert_eq!(
        at(21),
        &[
            p("\tprintf(", K::Text),
            p("\"%d items, \\\\ done\\n\"", DQ),
            p(", total);", K::Text),
        ]
    );
    assert_eq!(at(22)[1], p("return", K::Keyword(0)));
}

#[test]
fn crlf_and_other_line_endings() {
    let c = rules("C/C++");
    for eol in [Eol::CrLf, Eol::Lf, Eol::Cr, Eol::Nel, Eol::Ls, Eol::Ps] {
        assert_eq!(
            line_end_state(&c, "x // y", Some(eol), &ColorState::Normal),
            ColorState::Normal,
            "{eol:?}"
        );
        assert_eq!(
            line_end_state(&c, "x /* y", Some(eol), &ColorState::Normal),
            ColorState::Block(0),
            "{eol:?}"
        );
        assert_eq!(
            line_end_state(&c, "x \"y\\", Some(eol), &ColorState::Normal),
            ColorState::Quote {
                double: true,
                kind: QuoteKind::Plain
            },
            "{eol:?}"
        );
    }
}
