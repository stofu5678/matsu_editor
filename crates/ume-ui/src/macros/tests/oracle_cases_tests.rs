//! The key macros of the Sakura oracle (`crates/ume/tests/sakura_oracle/cases/*.mac` on the
//! branch of the oracle test) run in GPUI's headless window. The oracle runs the real Sakura
//! Editor v2.4.2 and UME on the same `.mac` text and compares the saved bytes; these tests run
//! the same text through UME's macro runner and check the bytes that Sakura saved.
//!
//! The macro is plain text as in the oracle: `@OUT@` is the file to save to, and the macro
//! ends with `ExitAll();`.

use std::path::Path;

use gpui::{TestAppContext, VisualTestContext};
use ume_commands::command::Arg;
use ume_commands::sakura::FUNCTIONS;

use super::{F_EXECEXTMACRO, setup, text_arg, write};

const SJIS: i64 = 0;
const UTF16LE: i64 = 3;
const UTF8: i64 = 4;

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

/// What an oracle case reads: the input file (opened with `code`, as `-CODE=` does) and the
/// macro text.
struct Case<'a> {
    input: &'a [u8],
    code: i64,
    mac: &'a str,
}

impl Case<'_> {
    /// Runs the case and returns the saved file. Panics when the macro did not save one.
    fn run(&self, cx: &mut TestAppContext) -> Vec<u8> {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.txt");
        std::fs::write(&input, self.input).unwrap();
        let out = dir.path().join("out.txt");
        let mac = self.mac.replace("@OUT@", &forward_slashes(&out));
        let mac = write(dir.path(), "case.mac", &mac);
        cx.update(gpui_component::init);
        let (f, cx) = setup(cx, "");
        f.run(
            cx,
            code_of("F_FILEOPEN"),
            &[text_arg(&input), Arg::Int(self.code)],
        );
        settle(cx);
        f.run(cx, F_EXECEXTMACRO, &[text_arg(&mac)]);
        settle(cx);
        assert!(
            out.is_file(),
            "the macro did not save: {}\n{}\nasking: {:?}",
            self.mac,
            f.output(cx),
            cx.pending_prompt()
        );
        std::fs::read(&out).unwrap()
    }
}

/// A path in a macro string: forward slashes, as the oracle writes it.
fn forward_slashes(path: &Path) -> String {
    path.to_str().unwrap().replace('\\', "/")
}

/// Lets the flows (a file that is read, a save, a wait of the macro) run to their end.
fn settle(cx: &mut VisualTestContext) {
    for _ in 0..50 {
        cx.run_until_parked();
    }
}

const NIHONGO_SJIS: &[u8] = b"\x93\xFA\x96\x7B\x8C\xEA";
const NIHONGO_EUC: &[u8] = b"\xC6\xFC\xCB\xDC\xB8\xEC";
const NIHONGO_UTF8: &[u8] = "日本語".as_bytes();

fn save_as(code: u8, eol: u8) -> String {
    format!("FileSaveAs('@OUT@', {code}, {eol});\nExitAll();\n")
}

fn concat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

// ---- 1. FileSaveAs takes its code argument ----

#[gpui::test]
fn save_sjis_to_utf8(cx: &mut TestAppContext) {
    let input = concat(&[NIHONGO_SJIS, b"\r\nabc"]);
    let out = Case {
        input: &input,
        code: SJIS,
        mac: &save_as(4, 0),
    }
    .run(cx);
    assert_eq!(out, concat(&[NIHONGO_UTF8, b"\r\nabc"]));
}

#[gpui::test]
fn save_sjis_to_euc(cx: &mut TestAppContext) {
    let input = concat(&[NIHONGO_SJIS, b"\r\nabc"]);
    let out = Case {
        input: &input,
        code: SJIS,
        mac: &save_as(2, 0),
    }
    .run(cx);
    assert_eq!(out, concat(&[NIHONGO_EUC, b"\r\nabc"]));
}

#[gpui::test]
fn save_sjis_to_jis(cx: &mut TestAppContext) {
    let input = concat(&[NIHONGO_SJIS, b"\r\nabc"]);
    let out = Case {
        input: &input,
        code: SJIS,
        mac: &save_as(1, 0),
    }
    .run(cx);
    assert_eq!(out, b"\x1b$BF|K\\8l\x1b(B\r\nabc");
}

/// Sakura saves UTF-16 without a BOM here: the BOM follows the default of the encoding the
/// document had (Shift_JIS: none).
#[gpui::test]
fn save_sjis_to_utf16_without_a_bom(cx: &mut TestAppContext) {
    let input = concat(&[NIHONGO_SJIS, b"\r\n"]);
    let out = Case {
        input: &input,
        code: SJIS,
        mac: &save_as(3, 0),
    }
    .run(cx);
    assert_eq!(out, b"\xe5\x65\x2c\x67\x9e\x8a\r\x00\n\x00");
    let out = Case {
        input: &input,
        code: SJIS,
        mac: &save_as(6, 0),
    }
    .run(cx);
    assert_eq!(out, b"\x65\xe5\x67\x2c\x8a\x9e\x00\r\x00\n");
}

/// Sakura gives this UTF-8 file a BOM: the document was UTF-16, whose default is a BOM.
#[gpui::test]
fn save_utf16le_to_utf8_with_a_bom(cx: &mut TestAppContext) {
    let input = b"\xff\xfe\xe5\x65\x2c\x67\x9e\x8a\r\x00\n\x00a\x00";
    let out = Case {
        input,
        code: UTF16LE,
        mac: &save_as(4, 0),
    }
    .run(cx);
    assert_eq!(out, concat(&[b"\xEF\xBB\xBF", NIHONGO_UTF8, b"\r\na"]));
}

#[gpui::test]
fn save_utf8_to_sjis(cx: &mut TestAppContext) {
    let input = concat(&[NIHONGO_UTF8, b"\r\nabc"]);
    let out = Case {
        input: &input,
        code: UTF8,
        mac: &save_as(0, 0),
    }
    .run(cx);
    assert_eq!(out, concat(&[NIHONGO_SJIS, b"\r\nabc"]));
}

#[gpui::test]
fn save_utf8_to_euc_with_lf(cx: &mut TestAppContext) {
    let input = concat(&[NIHONGO_UTF8, b"\r\nabc"]);
    let out = Case {
        input: &input,
        code: UTF8,
        mac: &save_as(2, 2),
    }
    .run(cx);
    assert_eq!(out, concat(&[NIHONGO_EUC, b"\nabc"]));
}

#[gpui::test]
fn save_utf8_with_a_bom_to_sjis_drops_the_bom(cx: &mut TestAppContext) {
    let input = concat(&[b"\xEF\xBB\xBF", NIHONGO_UTF8, b"\nabc"]);
    let out = Case {
        input: &input,
        code: UTF8,
        mac: &save_as(0, 0),
    }
    .run(cx);
    assert_eq!(out, concat(&[NIHONGO_SJIS, b"\nabc"]));
}

/// Characters the code cannot hold are saved as `?` without a question (the oracle cases
/// `saveloss-*`; the window's save asks).
#[gpui::test]
fn save_unencodable_characters_as_question_marks_without_asking(cx: &mut TestAppContext) {
    let input = concat(&[NIHONGO_UTF8, b"\r\nabc"]);
    let out = Case {
        input: &input,
        code: UTF8,
        mac: &save_as(8, 0),
    }
    .run(cx);
    assert_eq!(out, b"???\r\nabc");
}

/// UTF-32 (code pages 12000 and 12001) has no BOM by default and writes a supplementary
/// character as one 4-byte unit (`S_UnicodeToUTF32LE`).
#[gpui::test]
fn save_utf8_to_utf32le_and_be(cx: &mut TestAppContext) {
    let input = "a\u{3042}\u{1F600}\r\n".as_bytes();
    let le = Case {
        input,
        code: UTF8,
        mac: "FileSaveAs('@OUT@', 12000, 0);\nExitAll();\n",
    }
    .run(cx);
    assert_eq!(le, b"a\0\0\0\x42\x30\0\0\x00\xF6\x01\0\r\0\0\0\n\0\0\0");
    let be = Case {
        input,
        code: UTF8,
        mac: "FileSaveAs('@OUT@', 12001, 0);\nExitAll();\n",
    }
    .run(cx);
    assert_eq!(be, b"\0\0\0a\0\0\x30\x42\0\x01\xF6\x00\0\0\0\r\0\0\0\n");
}

/// The encoding the file was opened with changes nothing, the BOM included.
#[gpui::test]
fn save_in_the_same_encoding_keeps_the_bom(cx: &mut TestAppContext) {
    let input = concat(&[b"\xEF\xBB\xBF", NIHONGO_UTF8, b"\r\n"]);
    let out = Case {
        input: &input,
        code: UTF8,
        mac: &save_as(4, 0),
    }
    .run(cx);
    assert_eq!(out, input);
}

// ---- 2. search and replace calls finish ----

/// `inputs/ascii.words.lf`.
const WORDS: &[u8] = b"foo Foo FOO foobar barfoo\nfoo foo\nthe foo.\n";

fn on_words(cx: &mut TestAppContext, mac: &str) -> String {
    let out = Case {
        input: WORDS,
        code: SJIS,
        mac: &format!("{mac}FileSaveAs('@OUT@', 0, 0);\nExitAll();\n"),
    }
    .run(cx);
    String::from_utf8(out).unwrap()
}

/// Checks the cases of the oracle that run on `WORDS`: the macro (without the save) and the
/// text it leaves. The options are Sakura's numbers: 1 whole word, 2 match case, 32 search again
/// from the top, 128 the selection, 256 insert before, 512 insert after.
fn check_words(cx: &mut TestAppContext, cases: &[(&str, &str)]) {
    for (mac, expected) in cases {
        assert_eq!(&on_words(cx, mac), expected, "{mac}");
    }
}

#[gpui::test]
fn replace_all_replaces_every_match(cx: &mut TestAppContext) {
    check_words(
        cx,
        &[
            // replace-all
            (
                "ReplaceAll('foo', 'bar', 0);\n",
                "bar bar bar barbar barbar\nbar bar\nthe bar.\n",
            ),
            // replace-all-case-sensitive
            (
                "ReplaceAll('foo', 'bar', 2);\n",
                "bar Foo FOO barbar barbar\nbar bar\nthe bar.\n",
            ),
            // replace-all-word-only
            (
                "ReplaceAll('foo', 'bar', 1);\n",
                "bar bar bar foobar barfoo\nbar bar\nthe bar.\n",
            ),
            // replace-all-longer-text
            (
                "ReplaceAll('foo', 'quux!', 0);\n",
                "quux! quux! quux! quux!bar barquux!\nquux! quux!\nthe quux!.\n",
            ),
            // replace-all-with-empty
            ("ReplaceAll('foo', '', 0);\n", "   bar bar\n \nthe .\n"),
            // replace-all-with-tab
            (
                "ReplaceAll('foo', '\\t', 0);\n",
                "\t \t \t \tbar bar\t\n\t \t\nthe \t.\n",
            ),
            // replace-all-with-newline
            (
                "ReplaceAll('foo', 'a\\nb', 0);\n",
                "a\nb a\nb a\nb a\nbbar bara\nb\na\nb a\nb\nthe a\nb.\n",
            ),
            // replace-all-no-match
            (
                "ReplaceAll('zzz', 'bar', 0);\n",
                "foo Foo FOO foobar barfoo\nfoo foo\nthe foo.\n",
            ),
            // replace-all-insert-before, replace-all-append-after
            (
                "ReplaceAll('foo', 'X', 256);\n",
                "Xfoo XFoo XFOO Xfoobar barXfoo\nXfoo Xfoo\nthe Xfoo.\n",
            ),
            (
                "ReplaceAll('foo', 'X', 512);\n",
                "fooX FooX FOOX fooXbar barfooX\nfooX fooX\nthe fooX.\n",
            ),
            // replace-all-selected-area
            (
                "Down();\nDown_Sel();\nReplaceAll('foo', 'bar', 128);\n",
                "foo Foo FOO foobar barfoo\nbar bar\nthe foo.\n",
            ),
            // replace-all-undo
            (
                "ReplaceAll('foo', 'bar', 0);\nUndo();\n",
                "foo Foo FOO foobar barfoo\nfoo foo\nthe foo.\n",
            ),
            // replace-all-from-middle
            (
                "Down();\nReplaceAll('foo', 'bar', 0);\n",
                "bar bar bar barbar barbar\nbar bar\nthe bar.\n",
            ),
        ],
    );
}

#[gpui::test]
fn replace_replaces_the_selected_match_and_selects_the_next(cx: &mut TestAppContext) {
    check_words(
        cx,
        &[
            // replace-one-after-search
            (
                "SearchNext('foo', 0);\nReplace('foo', 'bar', 0);\n",
                "bar Foo FOO foobar barfoo\nfoo foo\nthe foo.\n",
            ),
            // replace-one-twice
            (
                "SearchNext('foo', 0);\nReplace('foo', 'bar', 0);\nReplace('foo', 'bar', 0);\n",
                "bar bar FOO foobar barfoo\nfoo foo\nthe foo.\n",
            ),
        ],
    );
}

#[gpui::test]
fn search_next_and_prev_select_the_match(cx: &mut TestAppContext) {
    check_words(
        cx,
        &[
            // search-next-delete
            (
                "SearchNext('foo', 0);\nDelete();\n",
                " Foo FOO foobar barfoo\nfoo foo\nthe foo.\n",
            ),
            // search-next-twice
            (
                "SearchNext('foo', 0);\nSearchNext('foo', 0);\nDelete();\n",
                "foo  FOO foobar barfoo\nfoo foo\nthe foo.\n",
            ),
            // search-next-case-insensitive
            (
                "SearchNext('FOO', 0);\nSearchNext('FOO', 0);\nDelete();\n",
                "foo  FOO foobar barfoo\nfoo foo\nthe foo.\n",
            ),
            // search-next-case-sensitive
            (
                "SearchNext('Foo', 2);\nDelete();\n",
                "foo  FOO foobar barfoo\nfoo foo\nthe foo.\n",
            ),
            // search-next-word-only
            (
                "SearchNext('foo', 1);\nSearchNext('foo', 1);\nSearchNext('foo', 1);\nDelete();\n",
                "foo Foo  foobar barfoo\nfoo foo\nthe foo.\n",
            ),
            // search-next-wraps
            (
                "GoFileEnd();\nSearchNext('barfoo', 32);\nDelete();\n",
                "foo Foo FOO foobar \nfoo foo\nthe foo.\n",
            ),
            // search-prev-from-end
            (
                "GoFileEnd();\nSearchPrev('foo', 0);\nDelete();\n",
                "foo Foo FOO foobar barfoo\nfoo foo\nthe .\n",
            ),
        ],
    );
}

#[gpui::test]
fn search_and_replace_find_japanese_text(cx: &mut TestAppContext) {
    let input = "日本語のテキスト\r\nABC abc\r\n".as_bytes();
    // search-next-japanese: the macro writes the text as \uXXXX
    let out = Case {
        input,
        code: UTF8,
        mac: "SearchNext('\\u30c6\\u30ad\\u30b9\\u30c8', 0);\nDelete();\n\
              FileSaveAs('@OUT@', 4, 0);\nExitAll();\n",
    }
    .run(cx);
    assert_eq!(out, "日本語の\r\nABC abc\r\n".as_bytes());
    // replace-all-japanese
    let out = Case {
        input,
        code: UTF8,
        mac: "ReplaceAll('\\u65e5\\u672c', '\\u4e8c\\u672c', 0);\n\
              FileSaveAs('@OUT@', 4, 0);\nExitAll();\n",
    }
    .run(cx);
    assert_eq!(out, "二本語のテキスト\r\nABC abc\r\n".as_bytes());
}

// ---- 3. ChangeTabWidth is a statement of a key macro ----

#[gpui::test]
fn change_tab_width_sets_the_width_the_conversion_uses(cx: &mut TestAppContext) {
    // convert-tab-to-space-width8
    let input = b"a\tb\r\n\tc\r\n    d\r\ne  f\r\n        g\r\n";
    let out = Case {
        input,
        code: SJIS,
        mac: "ChangeTabWidth(8);\nSelectAll();\nTABToSPACE();\n\
              FileSaveAs('@OUT@', 0, 0);\nExitAll();\n",
    }
    .run(cx);
    assert_eq!(
        out,
        b"a       b\r\n        c\r\n    d\r\ne  f\r\n        g\r\n"
    );
    // The width 2 gives other spaces, so the width is the one the macro set and not a default.
    let out = Case {
        input: b"a\tb\r\n",
        code: SJIS,
        mac: "ChangeTabWidth(2);\nSelectAll();\nTABToSPACE();\n\
              FileSaveAs('@OUT@', 0, 0);\nExitAll();\n",
    }
    .run(cx);
    assert_eq!(out, b"a b\r\n");
}

#[gpui::test]
fn indent_space_runs_after_change_tab_width(cx: &mut TestAppContext) {
    // indent-space-width2: one space in front of each selected line, whatever the width
    let input = b"  lead\ntrail  \n\tx\n";
    let out = Case {
        input,
        code: SJIS,
        mac: "ChangeTabWidth(2);\nSelectAll();\nIndentSpace();\n\
              FileSaveAs('@OUT@', 0, 0);\nExitAll();\n",
    }
    .run(cx);
    assert_eq!(out, b"   lead\n trail  \n \tx\n");
}

/// The other functions that Sakura's `CMacro::HandleCommand` passes on to `HandleFunction`.
#[gpui::test]
fn the_layout_statements_of_a_key_macro_run(cx: &mut TestAppContext) {
    // ChangeInsertSpace(1): the Tab key (IndentTab) inserts spaces up to the next tab stop.
    let out = Case {
        input: b"",
        code: SJIS,
        mac: "ChangeTabWidth(4);\nChangeInsertSpace(1);\nChar(97);\nIndentTab();\nChar(98);\n\
              FileSaveAs('@OUT@', 0, 0);\nExitAll();\n",
    }
    .run(cx);
    assert_eq!(out, b"a   b");
    // ChangeWrapColm and ChangeWrapColumn are the same function.
    for name in ["ChangeWrapColm", "ChangeWrapColumn"] {
        Case {
            input: b"abcdefghijklmnopqrstuvwxyz",
            code: SJIS,
            mac: &format!("{name}(10);\nFileSaveAs('@OUT@', 0, 0);\nExitAll();\n"),
        }
        .run(cx);
    }
}

// ---- 4. a mixed line ending save keeps each line ending ----

#[gpui::test]
fn save_mixed_eol_keeps_each_line_ending(cx: &mut TestAppContext) {
    // jp.utf8.mixed: CRLF, LF and CR in one file
    let input = "日本語\r\nABC\nｱｲｳ\rＡＢＣ\r\n\tタブ\n".as_bytes();
    let out = Case {
        input,
        code: UTF8,
        mac: &save_as(4, 0),
    }
    .run(cx);
    assert_eq!(out, input);
}

// ---- InsText with a line break ----

#[gpui::test]
fn ins_text_with_a_line_break_finishes(cx: &mut TestAppContext) {
    // ins-text-with-newline
    let out = Case {
        input: b"ABC\r\ndef\r\n",
        code: SJIS,
        mac: "Down();\nInsText('x\\ny');\nFileSaveAs('@OUT@', 0, 0);\nExitAll();\n",
    }
    .run(cx);
    // The inserted text keeps its own line break (LF), so the file ends up with mixed endings,
    // and the save asks nothing (the cause of the hang: the mixed line ending question).
    assert_eq!(out, b"ABC\r\nx\nydef\r\n");
}

// ---- a command that cannot run does not stop the macro (macro-continue-*) ----

const GRID: &[u8] = b"abcdefghij\nABCDEFGHIJ\n0123456789\nshort\n\nlast line here";

fn box_then(command: &str) -> String {
    format!(
        "Right();\nRight();\nBeginBoxSelect();\nRight_BoxSel();\nRight_BoxSel();\nDown_BoxSel();\n\
         {command}\nDelete();\n{}",
        save_as(0, 0)
    )
}

/// Sakura returns quietly (`Command_MERGE`, `Command_WordDeleteToStart`, `Command_UNINDENT`: the
/// early `return` or `ErrorBeep()` with a box selected) and the macro goes on to delete the box.
#[gpui::test]
fn a_command_refused_with_a_box_does_not_stop_the_macro(cx: &mut TestAppContext) {
    for command in [
        "Merge();",
        "WordDeleteToStart();",
        "WordDeleteToEnd();",
        "UnindentTab();",
        "UnindentSpace();",
    ] {
        let out = Case {
            input: GRID,
            code: SJIS,
            mac: &box_then(command),
        }
        .run(cx);
        assert_eq!(
            out, b"abefghij\nABEFGHIJ\n0123456789\nshort\n\nlast line here",
            "{command}"
        );
    }
}

/// `CViewCommander::HandleCommand` returns `TRUE` for a function that view mode forbids
/// (`IsFuncEnable`); the macro goes on, and the edits after the second `ViewMode()` work.
#[gpui::test]
fn edits_in_view_mode_do_not_stop_the_macro(cx: &mut TestAppContext) {
    let out = Case {
        input: b"abc\nde\n",
        code: SJIS,
        mac: &format!(
            "ViewMode();\nInsText('X');\nDelete();\nDeleteBack();\nUndo();\nViewMode();\n\
             InsText('Y');\n{}",
            save_as(0, 0)
        ),
    }
    .run(cx);
    assert_eq!(out, b"Yabc\nde\n");
}

// ---- regular expressions match the line ending; a search drops the selection ----

const REGEX_LF: &[u8] = include_bytes!("../../../../ume/tests/sakura_oracle/inputs/ascii.regex.lf");

fn regex_case(cx: &mut TestAppContext, body: &str) -> String {
    let mac = format!("{body}\nFileSaveAs('@OUT@', 0, 0);\nExitAll();\n");
    let out = Case {
        input: REGEX_LF,
        code: SJIS,
        mac: &mac,
    }
    .run(cx);
    String::from_utf8(out).unwrap()
}

/// `regex-newline-lf-in-lf-doc`: Sakura replaced the line break and joined the lines.
#[gpui::test]
fn a_regex_replaces_the_line_break(cx: &mut TestAppContext) {
    let text = regex_case(cx, "ReplaceAll('end\\n', 'END|', 4);");
    assert!(
        text.contains("tel 03-1234-5678 END|user@example.com, x@y.org\n"),
        "{text:?}"
    );
    assert!(text.ends_with("the END|"), "{text:?}");
}

/// `regex-search-next-newline`: Sakura selected `qux 7` and its line break, and Delete joined the
/// lines.
#[gpui::test]
fn a_regex_search_selects_the_line_break(cx: &mut TestAppContext) {
    let text = regex_case(cx, "SearchNext('qux 7\\n', 4);\nDelete();");
    assert!(
        text.starts_with("foo123 bar45\nFoo-Bar baz_aaa bbb aaa\n"),
        "{text:?}"
    );
}

/// `regex-anchor-text-end-lower-z`: `\z` is only at the very end of the text, which is after the
/// last line break here, so Sakura left the text as it was.
#[gpui::test]
fn the_end_of_text_anchor_does_not_match_before_a_line_break(cx: &mut TestAppContext) {
    let text = regex_case(cx, "ReplaceAll('end\\z', 'END', 4);");
    assert_eq!(text.as_bytes(), REGEX_LF);
}

/// `regex-search-next-from-selection`: the search drops the selection first, so a miss leaves
/// nothing selected and Delete deletes nothing.
#[gpui::test]
fn a_search_that_misses_leaves_the_selection_dropped(cx: &mut TestAppContext) {
    let text = regex_case(cx, "SelectAll();\nSearchNext('foo', 4);\nDelete();");
    assert_eq!(text.as_bytes(), REGEX_LF);
}

/// After a miss nothing is selected, so what a macro does next acts at the caret: the first
/// search selected `qux`, the second missed and dropped it, and the text goes after it.
#[gpui::test]
fn a_miss_drops_the_selection_of_the_earlier_hit(cx: &mut TestAppContext) {
    let text = regex_case(
        cx,
        "SearchNext('qux', 4);\nSearchNext('zzz', 4);\nInsText('X');",
    );
    assert!(text.contains("baz_quxX 7\n"), "{text:?}");
}
