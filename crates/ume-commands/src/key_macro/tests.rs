//! Tests of the key macro file format and of recording and playing.
//!
//! The format comes from `sakura_core/macro/CMacro.cpp` (`CMacro::Save`) and
//! `sakura_core/macro/CKeyMacroMgr.cpp` (`LoadKeyMacro`); the cases below follow those two
//! functions line by line.

use ume_core::buffer::Document;
use ume_core::cursor::Cursor;
use ume_core::model::DocumentModel;

use super::*;
use crate::layout::PlainLayout;
use crate::testing::render;

fn text(s: &str) -> Arg {
    Arg::Text(s.to_owned())
}

fn load(source: &str) -> KeyMacro {
    KeyMacro::from_text(source).unwrap_or_else(|e| panic!("{e}"))
}

fn error_of(source: &str) -> ParseError {
    KeyMacro::from_text(source).unwrap_err()
}

fn code(name: &str) -> u16 {
    macro_names::by_name(name).unwrap().code
}

fn step(name: &str, args: &[Arg]) -> Step {
    Step {
        code: code(name),
        args: args.to_vec(),
    }
}

// ---- saving ----

#[test]
fn saves_one_call_per_line_with_the_label() {
    let mut m = KeyMacro::new();
    m.push(code("Char"), &[Arg::Char('a')]).unwrap();
    m.push(code("Down"), &[]).unwrap();
    m.push(code("MoveCursor"), &[Arg::Int(3), Arg::Int(-1)])
        .unwrap();
    // CMacro::Save: "Name(args);\t// label\r\n" after the header comment. The label is the
    // string table text of the function (none for the macro-only MoveCursor).
    assert_eq!(
        m.to_text(),
        "//キーボードマクロのファイル\r\n\
         Char(97);\t// 文字入力\r\n\
         Down();\t// カーソル下移動\r\n\
         MoveCursor(3, -1);\t// \r\n"
    );
}

#[test]
fn saved_text_escapes_like_sakura() {
    let mut m = KeyMacro::new();
    m.push(
        code("InsText"),
        &[text(
            "a\\b'c\r\n\td\u{0}\u{1}\u{1f}\u{7f}\u{85}\u{2028}\u{2029}é😀",
        )],
    )
    .unwrap();
    // CMacro::Save replaces \ ' CR LF TAB with a backslash form, and NUL, the other C0
    // controls, DEL, NEL, LS and PS with \uXXXX (lower case hex).
    let line = m.to_text().lines().nth(1).unwrap().to_owned();
    assert_eq!(
        line,
        "InsText('a\\\\b\\'c\\r\\n\\td\\u0000\\u0001\\u001f\\u007f\\u0085\\u2028\\u2029é😀');\t// テキストを貼り付け"
    );
}

#[test]
fn the_file_is_utf8_with_a_bom() {
    let bytes = KeyMacro::new().to_bytes();
    assert_eq!(&bytes[..3], [0xEF, 0xBB, 0xBF]);
    assert_eq!(KeyMacro::from_bytes(&bytes).unwrap(), KeyMacro::new());
    // A file without a BOM is read as well.
    assert_eq!(KeyMacro::from_bytes(b"Down();").unwrap().len(), 1);
    assert!(matches!(
        KeyMacro::from_bytes(&[0xFF, 0xFE, 0x41, 0x00]),
        Err(LoadError::NotUtf8)
    ));
}

#[test]
fn a_saved_macro_loads_back() {
    let mut m = KeyMacro::new();
    for (name, args) in [
        ("Char", vec![Arg::Int(0x65E5)]),
        ("InsText", vec![text("x\\y'z\"\r\n\t\u{0}\u{7f}\u{2028}😀")]),
        ("Down", vec![]),
        ("MoveCursor", vec![Arg::Int(1), Arg::Int(2), Arg::Int(-3)]),
        ("FileOpen", vec![text("C:\\a b\\c.txt"), Arg::Int(0)]),
    ] {
        m.push(code(name), &args).unwrap();
    }
    assert_eq!(KeyMacro::from_text(&m.to_text()).unwrap(), m);
    assert_eq!(KeyMacro::from_bytes(&m.to_bytes()).unwrap(), m);
}

#[test]
fn saves_and_loads_a_file() {
    let dir = scratch_dir();
    let path = dir.join("a.mac");
    let mut m = KeyMacro::new();
    m.push(code("Char"), &[Arg::Int(65)]).unwrap();
    m.save(&path).unwrap();
    assert_eq!(KeyMacro::load(&path).unwrap(), m);
    assert!(matches!(
        KeyMacro::load(&dir.join("missing.mac")),
        Err(LoadError::Io(_))
    ));
    std::fs::remove_dir_all(dir).unwrap();
}

/// A fresh directory in the system's temporary directory (this crate has no `tempfile`).
fn scratch_dir() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "ume-key-macro-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn push_checks_the_function_and_the_arguments() {
    let mut m = KeyMacro::new();
    assert_eq!(m.push(0, &[]), Err(StepError::NoMacroName(0)));
    assert!(matches!(
        m.push(code("Down"), &[Arg::Int(1), Arg::Int(2)]),
        Err(StepError::TooManyArguments { max: 0, .. })
    ));
    assert!(matches!(
        m.push(code("Char"), &[text("a")]),
        Err(StepError::WrongArgumentType { position: 1, .. })
    ));
    assert!(matches!(
        m.push(code("InsText"), &[Arg::Int(1)]),
        Err(StepError::WrongArgumentType { position: 1, .. })
    ));
    assert!(m.is_empty());
    // The file format holds 32-bit numbers, so a recorded one must be readable again.
    let too_big = Arg::Int(i64::from(i32::MAX) + 1);
    assert!(matches!(
        m.push(code("Char"), &[too_big]),
        Err(StepError::NumberOutOfRange { position: 1, .. })
    ));
    m.push(code("Char"), &[Arg::Int(i64::from(i32::MAX))])
        .unwrap();
    m.push(code("Char"), &[Arg::Int(i64::from(i32::MIN))])
        .unwrap();
    assert_eq!(KeyMacro::from_text(&m.to_text()).unwrap(), m);
}

// ---- loading ----

#[test]
fn loads_calls_and_arguments() {
    let m = load(
        "\u{feff}//キーボードマクロのファイル\r\n\
         \r\n\
         \x20\t// a comment after blanks\r\n\
         \x20 Down();\r\n\
         S_Char( 97 );\t// comment\r\n\
         InsText(\"it's\");\r\n\
         InsText('say \\\"hi\\\" \\'x\\'');\r\n\
         MoveCursor(1,2,3)\r\n\
         Up(",
    );
    assert_eq!(
        m.steps(),
        [
            step("Down", &[]),
            step("Char", &[Arg::Int(97)]),
            step("InsText", &[text("it's")]),
            step("InsText", &[text("say \"hi\" 'x'")]),
            step("MoveCursor", &[Arg::Int(1), Arg::Int(2), Arg::Int(3)]),
            step("Up", &[]),
        ]
    );
}

#[test]
fn a_name_is_everything_before_the_open_parenthesis() {
    // CKeyMacroMgr::LoadKeyMacro copies szLine[nBgn..i], where szLine[i] is the first '(', as
    // the name, so a blank before the parenthesis is part of the name.
    assert_eq!(
        error_of("Down ();").kind,
        ParseErrorKind::UnknownFunction("Down ".into())
    );
    // A line without a parenthesis has an empty name in Sakura.
    assert!(matches!(
        error_of("Down").kind,
        ParseErrorKind::UnknownFunction(_)
    ));
}

#[test]
fn unescapes_like_sakura() {
    let one = |source: &str| match &load(&format!("InsText('{source}');")).steps()[0].args[..] {
        [Arg::Text(s)] => s.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(one(r"a\\nb"), "a\\nb"); // \\ then n: a backslash and an n
    assert_eq!(one(r"\r\n\t"), "\r\n\t");
    assert_eq!(one(r"\u0041\u3042"), "Aあ");
    assert_eq!(one(r"\u41g"), "Ag"); // one to four hex digits
    assert_eq!(one(r"\uD83D\uDE00"), "😀"); // UTF-16 units
    assert_eq!(one(r"\u"), ""); // no digits: the \u is dropped
    assert_eq!(one(r"\q"), "\\q"); // an unknown escape stays
    assert_eq!(one(r"\\u0041"), "\\u0041"); // an escaped backslash does not start \u
}

#[test]
fn the_rest_of_the_line_after_an_argument_is_skipped() {
    // After an argument Sakura looks for the next ',' or ')' and ignores what is between.
    let m = load("MoveCursor(-1, 22 junk , 3) trailing junk");
    assert_eq!(
        m.steps(),
        [step(
            "MoveCursor",
            &[Arg::Int(-1), Arg::Int(22), Arg::Int(3)]
        )]
    );
}

#[test]
fn grep_takes_its_extra_arguments() {
    // m_pData of Grep says 5 arguments (s_MacroInfoEx_i), GrepReplace 6 (s_MacroInfoEx_ii).
    let m = load("Grep('a','b','c',1,2);\nGrepReplace('a','b','c','d',1,2);");
    assert_eq!(m.steps()[0].args.len(), 5);
    assert_eq!(m.steps()[1].args.len(), 6);
    assert_eq!(
        error_of("Grep('a','b','c',1,2,3);").kind,
        ParseErrorKind::TooManyArguments
    );
}

#[test]
fn reports_the_line_and_the_column() {
    let unknown = error_of("Down();\nNoSuchThing();");
    assert_eq!(
        (unknown.line, unknown.column, &unknown.kind),
        (
            2,
            None,
            &ParseErrorKind::UnknownFunction("NoSuchThing".into())
        )
    );
    // STR_ERR_DLGKEYMACMGR8 names only the line.
    assert!(unknown.to_string().starts_with("line 2: "));

    // Char takes a number (STR_ERR_DLGKEYMACMGR4): the column is where the text starts.
    let wrong = error_of("Char('a');");
    assert_eq!(
        (wrong.line, wrong.column, &wrong.kind),
        (
            1,
            Some(6),
            &ParseErrorKind::WrongArgumentType {
                name: "Char".into(),
                position: 1
            }
        )
    );
    // InsText takes a text (STR_ERR_DLGKEYMACMGR6).
    let wrong = error_of("\n\n  InsText(5);");
    assert_eq!((wrong.line, wrong.column), (3, Some(11)));
    assert!(matches!(
        wrong.kind,
        ParseErrorKind::WrongArgumentType { position: 1, .. }
    ));

    // STR_ERR_DLGKEYMACMGR5: no closing quote.
    for source in ["InsText('abc);", "InsText('abc\\');"] {
        assert_eq!(
            error_of(source).kind,
            ParseErrorKind::UnterminatedText {
                name: "InsText".into(),
                position: 1
            }
        );
    }

    // STR_ERR_DLGKEYMACMGR3: a function takes up to four arguments in a file even where the
    // table declares fewer (the type is VT_EMPTY, which Sakura lets through); five are too many.
    assert!(KeyMacro::from_text("Down(1,2,3,4);").is_ok());
    assert_eq!(
        error_of("Down(1,2,3,4,5);").kind,
        ParseErrorKind::TooManyArguments
    );

    // STR_ERR_DLGKEYMACMGR7: not an argument.
    assert_eq!(error_of("Char(x);").kind, ParseErrorKind::Syntax);
    assert_eq!(error_of("Char(-);").kind, ParseErrorKind::Syntax);
    assert_eq!(
        error_of("Char(99999999999);").kind,
        ParseErrorKind::NumberOutOfRange
    );
    // A line that ends before the `)` is accepted with the arguments read so far.
    assert_eq!(load("Char(").len(), 1);
    assert_eq!(load("Char(97,").steps()[0].args, [Arg::Int(97)]);
}

// ---- recording ----

#[test]
fn records_only_while_recording() {
    let mut r = KeyMacroRecorder::new();
    assert!(!r.record(code("Down"), &[]));
    r.start();
    assert!(r.is_recording());
    assert!(r.record(code("Down"), &[]));
    assert!(r.record(code("Char"), &[Arg::Char('日')]));
    let m = r.stop();
    assert!(!r.is_recording());
    assert_eq!(
        m.steps(),
        [step("Down", &[]), step("Char", &[Arg::Int(0x65E5)])]
    );
    // Stopping again gives an empty macro, and starting drops an old recording.
    assert!(r.stop().is_empty());
    r.start();
    r.record(code("Down"), &[]);
    r.start();
    assert!(r.stop().is_empty());
}

#[test]
fn keeps_only_what_a_key_macro_keeps() {
    // CSMacroMgr::CanFuncIsKeyMacro: F_FILESAVE is commented out and the functions that return
    // a value are not listed; F_DOWN is listed.
    let mut r = KeyMacroRecorder::new();
    r.start();
    assert!(!r.record(code("FileSave"), &[]));
    assert!(!r.record(code("GetFilename"), &[]));
    assert!(!r.record(0, &[]));
    // Arguments that do not fit the function.
    let five = [1, 2, 3, 4, 5].map(Arg::Int);
    assert!(!r.record(code("Down"), &five));
    assert!(!r.record(code("Char"), &[text("a")]));
    assert!(r.record(code("Down"), &[]));
    assert_eq!(r.stop().len(), 1);
}

#[test]
fn a_recording_saves_and_loads() {
    let mut r = KeyMacroRecorder::new();
    r.start();
    for c in "hi".chars() {
        r.record(code("Char"), &[Arg::Char(c)]);
    }
    r.record(code("Left"), &[]);
    let m = r.stop();
    assert_eq!(
        m.to_text(),
        "//キーボードマクロのファイル\r\n\
         Char(104);\t// 文字入力\r\n\
         Char(105);\t// 文字入力\r\n\
         Left();\t// カーソル左移動\r\n"
    );
    assert_eq!(KeyMacro::from_text(&m.to_text()).unwrap(), m);
}

// ---- playing ----

struct Doc {
    model: DocumentModel,
    cursor: Cursor,
    layout: PlainLayout,
}

impl Doc {
    fn new() -> Self {
        Self {
            model: DocumentModel::new(Document::from_utf8(Vec::new()).unwrap()),
            cursor: Cursor::default(),
            layout: PlainLayout::default(),
        }
    }

    fn play(&mut self, m: &KeyMacro) -> Result<(), PlayError> {
        let mut cx = CommandContext::new(&mut self.model, &mut self.cursor, &mut self.layout, None);
        m.play(Registry::global(), &mut cx)
    }

    fn marked(&self) -> String {
        render(self.model.doc(), &self.cursor)
    }
}

#[test]
fn plays_through_the_registry() {
    let mut d = Doc::new();
    d.play(&load("Char(104);\nChar(105);\nLeft();\nDeleteBack();"))
        .unwrap();
    assert_eq!(d.marked(), "|i");
}

#[test]
fn each_call_is_one_undo_step() {
    let mut d = Doc::new();
    d.play(&load("Char(104);\nChar(105);")).unwrap();
    assert_eq!(d.marked(), "hi|");
    d.play(&load("Undo();")).unwrap();
    assert_eq!(d.marked(), "h|");
}

#[test]
fn stops_at_the_first_call_that_fails() {
    // FileSave has no command in this crate, and Char needs an argument.
    let mut d = Doc::new();
    let e = d
        .play(&load("Char(97);\nFileSave();\nChar(98);"))
        .unwrap_err();
    assert_eq!(e.index, 1);
    assert!(matches!(e.error, CommandError::Unknown(_)));
    assert_eq!(d.marked(), "a|");
    let e = d.play(&load("Char();")).unwrap_err();
    assert!(matches!(e.error, CommandError::BadArgs(_)));
    assert!(e.to_string().starts_with("step 1: "));
}

#[test]
fn a_loaded_macro_plays_the_same_as_the_recording() {
    let mut r = KeyMacroRecorder::new();
    r.start();
    for c in "ab日".chars() {
        r.record(code("Char"), &[Arg::Char(c)]);
    }
    let recorded = r.stop();
    let loaded = KeyMacro::from_bytes(&recorded.to_bytes()).unwrap();
    let (mut a, mut b) = (Doc::new(), Doc::new());
    a.play(&recorded).unwrap();
    b.play(&loaded).unwrap();
    assert_eq!(a.marked(), "ab日|");
    assert_eq!(a.marked(), b.marked());
}
