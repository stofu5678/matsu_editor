//! The coloring of UME Editor's own document types (ADR-0017) on small samples: comments, strings, keywords
//! and numbers of each type, and the two files with which the user found the gaps (a JavaScript file and a
//! Python file).

use ume_coloring::{ColorKind as K, ColorRules, ColorState, line_runs};
use ume_config::settings::DocumentTypes;
use ume_core::buffer::Eol;

/// The rules of the type a file called `file_name` has.
fn rules_for(file_name: &str) -> ColorRules {
    let types = DocumentTypes::builtin();
    let index = types.type_index_for_file_name(file_name);
    ColorRules::from_type(types.get(index).unwrap(), types.keywords())
}

fn pieces_from(rules: &ColorRules, text: &str, state: &ColorState) -> Vec<(String, K)> {
    line_runs(rules, text, Some(Eol::Lf), state, text.len())
        .iter()
        .filter_map(|r| {
            let end = r.end.min(text.len());
            (r.start < end).then(|| (text[r.start..end].to_owned(), r.kind))
        })
        .collect()
}

/// The pieces of one line, from the start of a file.
fn pieces(file_name: &str, text: &str) -> Vec<(String, K)> {
    pieces_from(&rules_for(file_name), text, &ColorState::Normal)
}

fn p(text: &str, kind: K) -> (String, K) {
    (text.to_owned(), kind)
}

/// Has the piece `text` this color somewhere in the line?
fn has(pieces: &[(String, K)], text: &str, kind: K) -> bool {
    pieces.iter().any(|(t, k)| t == text && *k == kind)
}

#[test]
fn a_javascript_file_gets_colors() {
    // The sample of the user: `const`, a call with a string, an arrow function.
    let rules = rules_for("app.js");
    let kw = K::Keyword(0);
    assert_eq!(
        pieces_from(
            &rules,
            "const el = document.getElementById('comment');",
            &ColorState::Normal
        ),
        [
            p("const", kw),
            p(" el = document.getElementById(", K::Text),
            p("'comment'", K::SingleQuote),
            p(");", K::Text),
        ]
    );
    let arrow = pieces_from(
        &rules,
        "const f = (a, b) => a + b * 2; // add",
        &ColorState::Normal,
    );
    assert!(has(&arrow, "const", kw));
    assert!(has(&arrow, "2", K::Number));
    assert!(has(&arrow, "// add", K::Comment));
    // `return` and `function` are keywords, and a longer identifier is not.
    let f = pieces_from(
        &rules,
        "function g() { return constant; }",
        &ColorState::Normal,
    );
    assert!(has(&f, "function", kw));
    assert!(has(&f, "return", kw));
    assert!(
        !f.iter()
            .any(|(t, k)| t.starts_with("const") && matches!(k, K::Keyword(_)))
    );
}

#[test]
fn a_python_file_colors_print_and_the_shebang() {
    // The sample of the user: a `#!` line, `message='Hello'`, `print(message)`.
    let shebang = pieces("hello.py", "#!/usr/bin/env python3");
    assert_eq!(shebang, [p("#!/usr/bin/env python3", K::Comment)]);
    assert_eq!(
        pieces("hello.py", "message='Hello'"),
        [p("message=", K::Text), p("'Hello'", K::SingleQuote)]
    );
    // `print` is in the second set (the color KW2); `def` is in the first (KW1).
    assert_eq!(
        pieces("hello.py", "print(message)"),
        [p("print", K::Keyword(1)), p("(message)", K::Text)]
    );
    let def = pieces("hello.py", "def f(): return len(x)");
    assert!(has(&def, "def", K::Keyword(0)));
    assert!(has(&def, "return", K::Keyword(0)));
    assert!(has(&def, "len", K::Keyword(1)));
}

#[test]
fn typescript_has_the_javascript_words_and_its_own() {
    let line = pieces("a.ts", "export interface A { readonly x: number } // t");
    for word in ["export", "interface", "readonly", "number"] {
        assert!(has(&line, word, K::Keyword(0)), "{word}");
    }
    assert!(has(&line, "// t", K::Comment));
    assert!(has(
        &pieces("a.tsx", "const a = \"x\";"),
        "\"x\"",
        K::DoubleQuote
    ));
}

#[test]
fn json_has_strings_numbers_and_literals() {
    let line = pieces(
        "a.json",
        r#"{"name": "a\"b", "n": -12.5, "ok": true, "x": null}"#,
    );
    assert!(has(&line, r#""name""#, K::DoubleQuote));
    assert!(has(&line, r#""a\"b""#, K::DoubleQuote), "an escaped quote");
    assert!(has(&line, "-12.5", K::Number), "{line:?}");
    assert!(has(&line, "true", K::Keyword(0)));
    assert!(has(&line, "null", K::Keyword(0)));
    // A word inside a string is not a keyword.
    let inside = pieces("a.json", r#"{"k": "true"}"#);
    assert!(!has(&inside, "true", K::Keyword(0)));
    // `.jsonc` has comments.
    assert!(has(&pieces("a.jsonc", "{} // c"), "// c", K::Comment));
}

#[test]
fn css_has_block_comments_strings_and_at_rules() {
    let rules = rules_for("a.css");
    let line = pieces_from(
        &rules,
        "@media (min-width: 10px) { a { content: \"x\"; } } /* c */",
        &ColorState::Normal,
    );
    assert!(has(&line, "@media", K::Keyword(0)));
    assert!(has(&line, "\"x\"", K::DoubleQuote));
    assert!(has(&line, "/* c */", K::Comment));
    // `//` is not a comment in CSS.
    let slash = pieces_from(&rules, "a { b: url(//x) }", &ColorState::Normal);
    assert!(!slash.iter().any(|(_, k)| *k == K::Comment));
    // The keywords ignore case.
    assert!(has(
        &pieces("a.css", "color: INHERIT;"),
        "INHERIT",
        K::Keyword(0)
    ));
}

#[test]
fn markdown_has_html_comments_and_plain_quotes() {
    let line = pieces("a.md", "It's a <!-- note --> \"quote\"");
    assert!(has(&line, "<!-- note -->", K::Comment));
    // The quotes of prose are not strings.
    assert!(
        !line
            .iter()
            .any(|(_, k)| matches!(k, K::SingleQuote | K::DoubleQuote))
    );
    // A comment goes over lines.
    let rules = rules_for("a.markdown");
    assert!(
        pieces_from(&rules, "text <!-- a", &ColorState::Normal)
            .iter()
            .any(|(t, k)| t == "<!-- a" && *k == K::Comment)
    );
}

#[test]
fn rust_has_comments_strings_keywords_and_lifetimes_do_not_break_the_line() {
    let line = pieces("main.rs", "let s: &'static str = \"a\\\"b\"; // c");
    assert!(has(&line, "let", K::Keyword(0)));
    assert!(has(&line, "str", K::Keyword(0)));
    assert!(has(&line, "\"a\\\"b\"", K::DoubleQuote));
    assert!(has(&line, "// c", K::Comment));
    // The lifetime `'static` is not the start of a string.
    assert!(!line.iter().any(|(_, k)| *k == K::SingleQuote));
    let f = pieces("main.rs", "pub fn f<'a>(x: &'a u32) -> u32 { x + 0x1F }");
    for word in ["pub", "fn", "u32"] {
        assert!(has(&f, word, K::Keyword(0)), "{word}");
    }
    assert!(has(&f, "0x1F", K::Number));
}

#[test]
fn go_has_keywords_and_predeclared_names() {
    let line = pieces(
        "main.go",
        "func main() { x := make([]int, 3); if x != nil { return } } // c",
    );
    for word in ["func", "make", "int", "if", "nil", "return"] {
        assert!(has(&line, word, K::Keyword(0)), "{word}");
    }
    assert!(has(&line, "3", K::Number));
    assert!(has(&line, "// c", K::Comment));
    assert!(has(&pieces("main.go", "r := 'x'"), "'x'", K::SingleQuote));
}

#[test]
fn ruby_has_hash_comments_block_comments_and_keywords() {
    let line = pieces("a.rb", "puts 'hi' if x # note");
    assert!(has(&line, "puts", K::Keyword(0)));
    assert!(has(&line, "if", K::Keyword(0)));
    assert!(has(&line, "'hi'", K::SingleQuote));
    assert!(has(&line, "# note", K::Comment));
    // `=begin` ... `=end` goes over lines.
    let rules = rules_for("a.rb");
    let begin = pieces_from(&rules, "=begin", &ColorState::Normal);
    assert!(has(&begin, "=begin", K::Comment));
    assert!(has(&pieces("a.rake", "def x; end"), "def", K::Keyword(0)));
}

#[test]
fn shell_has_comments_strings_and_keywords() {
    let line = pieces("run.sh", "if [ -f \"$f\" ]; then echo 'ok'; fi # done");
    for word in ["if", "then", "echo", "fi"] {
        assert!(has(&line, word, K::Keyword(0)), "{word}");
    }
    assert!(has(&line, "\"$f\"", K::DoubleQuote));
    assert!(has(&line, "'ok'", K::SingleQuote));
    assert!(has(&line, "# done", K::Comment));
    assert_eq!(
        pieces("run.bash", "#!/bin/bash"),
        [p("#!/bin/bash", K::Comment)]
    );
}

#[test]
fn yaml_has_comments_double_quotes_and_literals() {
    let line = pieces("a.yml", "enabled: true # on");
    assert!(has(&line, "true", K::Keyword(0)));
    assert!(has(&line, "# on", K::Comment));
    assert!(has(
        &pieces("a.yaml", "name: \"x # y\""),
        "\"x # y\"",
        K::DoubleQuote
    ));
    // An apostrophe in plain text does not start a string.
    let plain = pieces("a.yml", "text: don't stop");
    assert!(!plain.iter().any(|(_, k)| *k == K::SingleQuote));
    assert!(has(&pieces("a.yml", "n: 42"), "42", K::Number));
}
