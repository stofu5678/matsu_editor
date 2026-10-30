//! Tests of the TeX, Erlang and rule file analyzers.
//!
//! Every expected row is `line:depth:info:name`; the expectations follow the Sakura functions
//! that are ported (`MakeTopicList_tex`, `MakeFuncList_Erlang`, `MakeFuncList_RuleFile`).

use matsu_core::encoding::Encoding;

use crate::erlang::make_func_list_erlang;
use crate::rule_file::{RuleFileOutline, make_func_list_rule_file};
use crate::tex::make_topic_list_tex;
use crate::{FuncInfoArr, OutlineKind, Request, VecSource, analyze};

fn dump(arr: &FuncInfoArr) -> String {
    arr.items()
        .iter()
        .map(|i| format!("{}:{}:{}:{}", i.line, i.depth, i.info, i.name))
        .collect::<Vec<_>>()
        .join("\n")
}

fn lines(s: &[&str]) -> String {
    s.iter().map(|l| format!("{l}\n")).collect()
}

fn tex(text: &str) -> String {
    dump(&make_topic_list_tex(&VecSource::new(text)))
}

fn erlang(text: &str) -> String {
    dump(&make_func_list_erlang(&VecSource::new(text)))
}

/// A rule file in UTF-8 with a BOM.
fn utf8_bom(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

fn shift_jis(text: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    Encoding::ShiftJis
        .codec()
        .encode(text, &mut bytes)
        .expect("every character is in Shift_JIS");
    bytes
}

fn rule_outline(rules: &[u8], text: &str) -> RuleFileOutline {
    make_func_list_rule_file(&VecSource::new(text), Some(rules))
}

fn rule(rules: &str, text: &str) -> String {
    dump(&rule_outline(&utf8_bom(rules), text).items)
}

// ---- TeX (MakeTopicList_tex) ----

#[test]
fn tex_sections_are_numbered_and_nested() {
    let text = lines(&[
        "\\documentclass{article}",
        "\\begin{document}",
        "\\section{はじめに}",
        "text \\label{sec:intro}",
        "\\subsection{背景}",
        "\\subsection*{注記}",
        "\\section{方法}",
        "\\subsubsection{詳細}",
        "\\end{document}",
    ]);
    // a tag with `*` has no number but counts; a skipped level shows as "0."
    assert_eq!(
        tex(&text),
        "3:0:0:1. はじめに\n5:1:0:1.1. 背景\n6:1:0:注記\n7:0:0:2. 方法\n8:1:0:2.0.1. 詳細"
    );
}

#[test]
fn tex_chapter_is_the_top_and_its_sections_go_under_it() {
    let text = lines(&["\\chapter{序}", "\\section{a}", "\\chapter{b}"]);
    assert_eq!(tex(&text), "1:0:0:1. 序\n2:1:0:1.1. a\n3:0:0:2. b");
}

#[test]
fn tex_comments_and_things_that_are_not_topics_are_skipped() {
    // a comment ends the line; `\label` and the like are not topics
    assert_eq!(tex("text % \\section{x}\n"), "");
    assert_eq!(tex("\\label{a}\\ref{b}\n"), "");
    // the title needs its closing brace on the line, and is not empty
    assert_eq!(tex("\\section{abc\n"), "");
    assert_eq!(tex("\\section{}\n"), "");
    // `\%` is a tag of its own up to the brace, so the section after it is lost (as in Sakura)
    assert_eq!(tex("\\% \\section{y}\n"), "");
}

#[test]
fn tex_two_tags_on_a_line() {
    assert_eq!(tex("\\section{A}\\section{B}\n"), "1:0:0:1. A\n1:0:0:2. B");
}

#[test]
fn tex_slide_of_prosper_takes_the_second_braces_as_the_title() {
    let text = lines(&[
        "\\begin{slide}{スライド1}",
        "\\begin{slide}{x}{}",
        "\\begin{itemize}",
    ]);
    // the braces after the first pair are not read; the third is not a slide
    assert_eq!(tex(&text), "1:0:0:1. スライド1\n2:0:0:2. x");
}

#[test]
fn tex_title_is_cut_to_the_buffer() {
    let long = "あ".repeat(300);
    let out = make_topic_list_tex(&VecSource::new(&format!("\\section{{{long}}}\n")));
    // "1. " and 252 characters: 255 in all
    assert_eq!(out.items()[0].name.chars().count(), 255);
}

#[test]
fn tex_kind_runs_through_analyze() {
    let src = VecSource::new("\\section{a}\n");
    let analysis = analyze(&src, &Request::new(OutlineKind::Tex));
    assert_eq!(dump(&analysis.items), "1:0:0:1. a");
    assert_eq!(analysis.kind.title(), "TeX");
}

// ---- Erlang (MakeFuncList_Erlang) ----

#[test]
fn erlang_functions_with_clauses_and_arities() {
    let text = lines(&[
        "-module(m).",
        "-export([f/0]).",
        "",
        "f() -> ok.",
        "",
        "g(A, B) ->",
        "    A + B;",
        "g(A, _) when A > 0 ->",
        "    A.",
        "",
        "h(X,",
        "  Y,",
        "  Z)",
        "  -> X.",
        "",
        "'quoted atom'(A) -> A.",
        "i({A, B}, [C|D], \"str, with comma\", 'x') -> ok.",
        "j() % comment",
        "  -> ok.",
    ]);
    // every clause is listed; the line is the one of the name; the arity counts the arguments
    // at the top level only, and a quoted text and a comment are skipped
    assert_eq!(
        erlang(&text),
        "4:0:0:f/0\n6:0:0:g/2\n8:0:0:g/2\n11:0:0:h/3\n16:0:0:quoted atom/1\n17:0:0:i/4\n18:0:0:j/0"
    );
}

#[test]
fn erlang_things_that_are_not_functions() {
    // a call without a body, a statement end in the arguments, a semicolon, a capital letter
    assert_eq!(erlang("foo(1).\n"), "");
    assert_eq!(erlang("f(A.) -> ok.\n"), "");
    assert_eq!(erlang("f(A; B) -> ok.\n"), "");
    assert_eq!(erlang("X = f(1) -> ok.\n"), "");
    assert_eq!(erlang("  f() -> ok.\n"), "");
    // an atom that is not closed
    assert_eq!(erlang("'f() -> ok.\n"), "");
}

#[test]
fn erlang_a_decimal_point_and_many_arguments() {
    assert_eq!(erlang("f(1.5) -> ok.\n"), "1:0:0:f/1");
    assert_eq!(erlang("f(a,b,c,d,e,f,g,h,i,j,k,l) -> ok.\n"), "1:0:0:f/12");
    // brackets of the other kinds do not end the arguments
    assert_eq!(erlang("f([A], {B, C}, #{k => v}) -> ok.\n"), "1:0:0:f/3");
}

#[test]
fn erlang_a_name_that_does_not_fit_is_cut() {
    let name = "a".repeat(70);
    let out = make_func_list_erlang(&VecSource::new(&format!("{name}() -> ok.\n")));
    // Sakura's buffer holds 63 characters; the arity does not fit after them
    assert_eq!(out.items()[0].name, "a".repeat(63));
}

#[test]
fn erlang_kind_runs_through_analyze() {
    let src = VecSource::new("f(X) -> X.\n");
    let analysis = analyze(&src, &Request::new(OutlineKind::Erlang));
    assert_eq!(dump(&analysis.items), "1:0:0:f/1");
    assert_eq!(analysis.kind.title(), "Erlang 関数一覧");
}

// ---- the rule file (ReadRuleFile, MakeFuncList_RuleFile) ----

#[test]
fn rule_file_plain_rules_title_and_levels() {
    let rules = lines(&[
        ";Title=メモの構成",
        "; a comment",
        "■ /// Big",
        "● /// Mid,Lv=1",
    ]);
    let doc = lines(&["■ 第一章", "本文", "● 節A", "  ● 節B", "■ 第二章"]);
    let out = rule_outline(&utf8_bom(&rules), &doc);
    // the blanks at the start of a line are skipped; Mid goes under Big (a greater level), and
    // a group seen again puts the item at its level
    assert_eq!(
        dump(&out.items),
        "1:0:0:■ 第一章\n3:1:0:● 節A\n4:1:0:● 節B\n5:0:0:■ 第二章"
    );
    assert_eq!(out.title.as_deref(), Some("メモの構成"));
    assert_eq!(out.error, None);
}

#[test]
fn rule_file_levels_nest_by_the_numbers() {
    let rules = lines(&["#  /// A", "##  /// B,Lv=1", "###  /// C,Lv=2"]);
    let doc = lines(&["# a", "## b", "### c", "## d", "# e"]);
    assert_eq!(
        rule(&rules, &doc),
        "1:0:0:# a\n2:1:0:## b\n3:2:0:### c\n4:1:0:## d\n5:0:0:# e"
    );
}

#[test]
fn rule_file_several_keys_a_root_and_except() {
    // an empty first key is the root; its name is the group up to the comma
    let rules = lines(&[" /// 全体,Lv=0", "a,b /// Item", "// /// Except"]);
    let doc = lines(&["a1", "b2", "c3", "// a", "  a4"]);
    assert_eq!(
        rule(&rules, &doc),
        "1:0:65536:全体\n1:1:0:a1\n2:1:0:b2\n5:1:0:a4"
    );
}

#[test]
fn rule_file_blank_lines_and_a_full_width_blank() {
    let rules = lines(&["# /// H"]);
    let doc = "　\t \n\u{3000}# x\n";
    assert_eq!(rule(&rules, doc), "2:0:0:# x");
}

#[test]
fn rule_file_comment_character_can_change() {
    let rules = lines(&[";CommentChar=#", "# comment /// x", ";a /// Semi"]);
    // ';' is a plain character now and '#' starts a comment
    assert_eq!(rule(&rules, ";a z\n# comment\n"), "1:0:0:;a z");
    // an empty value means no comment character at all
    let rules = lines(&[";CommentChar=", ";a /// Semi"]);
    assert_eq!(rule(&rules, ";a z\n"), "1:0:0:;a z");
}

#[test]
fn rule_file_directives_ignore_case_and_need_the_whole_text() {
    // `;title=` is read; `;Mode=Regexx` is not a mode
    let rules = lines(&[";title=x", ";Mode=Regexx", "a /// G"]);
    let out = rule_outline(&utf8_bom(&rules), "abc\n");
    assert_eq!(out.title.as_deref(), Some("x"));
    assert_eq!(dump(&out.items), "1:0:0:abc");
}

#[test]
fn rule_file_regular_expression_mode() {
    let rules = lines(&[
        ";Mode=Regex",
        "^#+\\s /// Head",
        "^\\s*function\\s+(\\w+) /// Func,Lv=1",
    ]);
    let doc = lines(&["# Title", "  function foo() {", "x"]);
    // the line is not trimmed in this mode
    assert_eq!(
        rule(&rules, &doc),
        "1:0:0:# Title\n2:1:0:  function foo() {"
    );
}

#[test]
fn rule_file_regex_option_i_and_the_dollar() {
    let rules = lines(&[";Mode=Regex", "^abc$ /// X"]);
    assert_eq!(rule(&rules, "ABC\nabc\n"), "2:0:0:abc");
    let rules = lines(&[";Mode=Regex", ";RegexOption=i", "^abc$ /// X"]);
    assert_eq!(rule(&rules, "ABC\nabc\nabcd\n"), "1:0:0:ABC\n2:0:0:abc");
}

#[test]
fn rule_file_regex_option_x_and_a() {
    // `x`: a blank in the pattern is ignored
    let rules = lines(&[";Mode=Regex", ";RegexOption=x", "^a b$ /// X"]);
    assert_eq!(rule(&rules, "ab\na b\n"), "1:0:0:ab");
    // `a`: \d is ASCII only
    let rules = lines(&[";Mode=Regex", ";RegexOption=a", "^\\d$ /// X"]);
    assert_eq!(rule(&rules, "１\n1\n"), "2:0:0:1");
    let rules = lines(&[";Mode=Regex", "^\\d$ /// X"]);
    assert_eq!(rule(&rules, "１\n1\n"), "1:0:0:１\n2:0:0:1");
}

#[test]
fn rule_file_regex_replace_mode_shows_the_replaced_text() {
    let rules = lines(&[";Mode=RegexReplace", "^(\\w+): /// [$1] /// Label"]);
    // the text from where the match starts, as long as the replacement of the match
    assert_eq!(rule(&rules, "name: value\n"), "1:0:0:[name]");
    let rules = lines(&[";Mode=RegexReplace", "\\d+ /// <$&> /// N"]);
    assert_eq!(rule(&rules, "ABC123DEF"), "1:0:0:<123>");
    // a line without a match is not listed
    assert_eq!(rule(&rules, "ABC\n"), "");
}

#[test]
fn rule_file_regex_replace_global() {
    // with `g` all the matches are replaced, and the length is worked out from the whole line
    // (`nTextLen = GetStringLen() - nLineLen + nMatchLen`)
    let rules = lines(&[";Mode=RegexReplace", ";RegexOption=g", "\\d /// # /// N"]);
    assert_eq!(rule(&rules, "a1b2c3"), "1:0:0:#");
}

#[test]
fn rule_file_regex_root_item() {
    let rules = lines(&[";Mode=Regex", " /// Root", "^# /// H"]);
    assert_eq!(rule(&rules, "# a\n"), "1:0:65536:Root\n1:1:0:# a");
}

#[test]
fn rule_file_bad_regular_expression_stops_with_a_message() {
    let rules = lines(&[";Title=T", ";Mode=Regex", "^a /// A", "(unclosed /// Bad"]);
    let out = rule_outline(&utf8_bom(&rules), "abc\n");
    assert!(out.items.is_empty());
    assert_eq!(out.title.as_deref(), Some("T"));
    let message = out.error.expect("a message");
    assert!(
        message.starts_with("正規表現ルール\n(unclosed\nでエラーが発生しました。\n"),
        "{message}"
    );
    // a replacement is part of the message
    let rules = lines(&[";Mode=RegexReplace", "(a /// r /// G"]);
    let out = rule_outline(&utf8_bom(&rules), "a\n");
    assert!(
        out.error
            .expect("a message")
            .starts_with("正規表現ルール\n(a\nr\nでエラーが発生しました。\n")
    );
}

#[test]
fn rule_file_missing_or_without_rules_gives_an_empty_list() {
    // a file that cannot be opened: no items, no title, no message
    let out = make_func_list_rule_file(&VecSource::new("a\n"), None);
    assert_eq!(out, RuleFileOutline::default());
    // a file without rules: the title is not used either (`nCount < 1` returns first)
    let out = rule_outline(&utf8_bom(";Title=T\n; nothing\n"), "a\n");
    assert_eq!(out, RuleFileOutline::default());
    let out = rule_outline(&[], "a\n");
    assert_eq!(out, RuleFileOutline::default());
}

#[test]
fn rule_file_shift_jis_and_utf8_with_bom() {
    let rules = lines(&["■ /// 見出し", "● /// 小見出し,Lv=1"]);
    let doc = lines(&["■ あ", "● い"]);
    let expected = "1:0:0:■ あ\n2:1:0:● い";
    // a BOM means UTF-8; the file without one is Shift_JIS
    assert_eq!(dump(&rule_outline(&utf8_bom(&rules), &doc).items), expected);
    assert_eq!(
        dump(&rule_outline(&shift_jis(&rules), &doc).items),
        expected
    );
    // UTF-8 without a BOM is read as Shift_JIS, so the keys are not the text
    assert_eq!(dump(&rule_outline(rules.as_bytes(), &doc).items), "");
}

#[test]
fn rule_file_line_endings_of_the_file() {
    let rules = "■ /// Big\r\n● /// Mid,Lv=1\r● /// Mid,Lv=1\n";
    assert_eq!(rule(rules, "■ a\n● b\n"), "1:0:0:■ a\n2:1:0:● b");
}

#[test]
fn rule_file_runs_through_analyze() {
    let bytes = utf8_bom(";Title=X\n# /// H\n");
    let src = VecSource::new("# a\n");
    let mut request = Request::new(OutlineKind::RuleFile);
    request.rule_file = Some(&bytes);
    let analysis = analyze(&src, &request);
    assert_eq!(dump(&analysis.items), "1:0:0:# a");
    assert_eq!(analysis.title.as_deref(), Some("X"));
    assert_eq!(analysis.error, None);
    // the other kinds have neither
    let other = analyze(&src, &Request::new(OutlineKind::Text));
    assert_eq!((other.title, other.error), (None, None));
}
