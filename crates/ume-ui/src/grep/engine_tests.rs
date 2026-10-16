//! Tests of the search: real files in temporary folders, and the text the sink gets compared with
//! Sakura's form line by line (`GrepMessageFormat.cpp`, `CGrepAgent.cpp`).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use ume_core::encoding::Encoding;
use ume_core::tag_jump::find_target;

use super::engine::{GrepError, GrepInfo, Outcome, Sink, run};
use super::enumerate::SEP;
use super::format::{Layout, LineType, OutputStyle};
use super::settings::GrepSettings;
use crate::search::state::SearchOptions;

/// Collects what the search writes; may stop it after some files.
struct Collect<'a> {
    text: String,
    files: Vec<String>,
    stop: Option<(usize, &'a AtomicBool)>,
}

impl Sink for Collect<'_> {
    fn write(&mut self, text: &str) {
        self.text.push_str(text);
    }

    fn progress(&mut self, file: &str, _hits: usize) {
        self.files.push(file.to_owned());
        if let Some((after, flag)) = self.stop
            && self.files.len() >= after
        {
            flag.store(true, Ordering::Relaxed);
        }
    }
}

struct Project {
    dir: tempfile::TempDir,
}

impl Project {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    fn write(&self, name: &str, bytes: &[u8]) -> &Self {
        let path = self.dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
        self
    }

    fn root(&self) -> String {
        self.dir.path().to_string_lossy().into_owned()
    }

    /// The full path the result shows for `name` (written with `/`).
    fn path(&self, name: &str) -> String {
        let mut path = self.root();
        for part in name.split('/') {
            path.push(SEP);
            path.push_str(part);
        }
        path
    }

    fn info(&self, key: &str) -> GrepInfo {
        GrepInfo {
            key: key.to_owned(),
            file: "*.*".to_owned(),
            folder: self.root(),
            options: SearchOptions::default(),
            sub_folder: true,
            charset: None,
            layout: Layout::default(),
            file_only: false,
            except_file_regex: false,
        }
    }
}

fn grep(info: &GrepInfo) -> (String, Outcome) {
    let cancel = AtomicBool::new(false);
    let mut sink = Collect {
        text: String::new(),
        files: Vec::new(),
        stop: None,
    };
    let outcome = run(info, &cancel, &mut sink).expect("the search starts");
    (sink.text, outcome)
}

/// The text after the heading: the lines of the hits and the count.
fn body(text: &str) -> &str {
    let (_, rest) = text.split_once("\r\n\r\n\r\n").expect("a heading");
    rest
}

#[test]
fn a_hit_line_has_the_path_the_position_and_the_text() {
    let p = Project::new();
    p.write("a.txt", b"first\r\nsecond line\r\nthird\r\n");
    let (text, outcome) = grep(&p.info("line"));
    assert_eq!(
        outcome,
        Outcome {
            hits: 1,
            stopped: false
        }
    );
    assert_eq!(
        body(&text),
        format!(
            "{}(2,8)  [UTF-8]: second line\r\n1 個が検索されました。\r\n",
            p.path("a.txt")
        )
    );
}

#[test]
fn the_heading_comes_first() {
    let p = Project::new();
    p.write("a.txt", b"x\n");
    let (text, _) = grep(&p.info("x"));
    let expected = format!(
        "\r\n□検索条件  \"x\"\r\n\
         検索対象       *.*\r\n\
         フォルダー       {}\r\n\
         除外ファイル   \r\n\
         除外フォルダー   \r\n\
         \x20   (サブフォルダーも検索)\r\n\
         \x20   (英大文字小文字を区別しない)\r\n\
         \x20   (文字コードセットの自動判別)\r\n\
         \x20   (一致した行を出力)\r\n\
         \r\n\r\n",
        p.root()
    );
    assert!(text.starts_with(&expected), "{text}");
}

#[test]
fn an_empty_result_has_only_the_count() {
    let p = Project::new();
    p.write("a.txt", b"nothing here\n");
    let (text, outcome) = grep(&p.info("absent"));
    assert_eq!(outcome.hits, 0);
    assert_eq!(body(&text), "0 個が検索されました。\r\n");
    // and a folder without files
    let empty = Project::new();
    let (text, _) = grep(&empty.info("x"));
    assert_eq!(body(&text), "0 個が検索されました。\r\n");
}

#[test]
fn files_are_searched_in_name_order_and_folders_after_the_files() {
    let p = Project::new();
    p.write("b.txt", b"hit\n")
        .write("a.txt", b"hit\n")
        .write("sub/c.txt", b"hit\n")
        .write("Z.txt", b"hit\n");
    let (text, outcome) = grep(&p.info("hit"));
    assert_eq!(outcome.hits, 4);
    let lines: Vec<&str> = body(&text).lines().collect();
    assert_eq!(
        lines,
        vec![
            format!("{}(1,1)  [UTF-8]: hit", p.path("a.txt")),
            format!("{}(1,1)  [UTF-8]: hit", p.path("b.txt")),
            format!("{}(1,1)  [UTF-8]: hit", p.path("Z.txt")),
            format!("{}(1,1)  [UTF-8]: hit", p.path("sub/c.txt")),
            "4 個が検索されました。".to_owned(),
        ]
    );
}

#[test]
fn the_subfolders_are_searched_only_when_asked() {
    let p = Project::new();
    p.write("a.txt", b"hit\n").write("sub/deep/b.txt", b"hit\n");
    let mut info = p.info("hit");
    info.sub_folder = false;
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 1);
    assert!(text.contains("    (サブフォルダーを検索しない)\r\n"));
    assert!(!text.contains("deep"));
    info.sub_folder = true;
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 2);
    assert!(text.contains(&p.path("sub/deep/b.txt")));
}

#[test]
fn the_file_patterns_choose_the_files() {
    let p = Project::new();
    p.write("a.txt", b"hit\n")
        .write("b.md", b"hit\n")
        .write("議事録.txt", b"hit\n")
        .write("議事録.bak", b"hit\n")
        .write("sub/c.txt", b"hit\n")
        .write(".git/config", b"hit\n")
        .write("tool.exe", b"hit\n");
    let mut info = p.info("hit");
    info.file = "*.txt".to_owned();
    let (text, _) = grep(&info);
    let shown: Vec<&str> = body(&text).lines().collect();
    assert_eq!(shown.len(), 4, "{text}");
    assert!(shown[0].starts_with(&p.path("a.txt")));
    assert!(shown[1].starts_with(&p.path("議事録.txt")));
    assert!(shown[2].starts_with(&p.path("sub/c.txt")));
    // the dialog's exclusions
    let mut settings = GrepSettings {
        folder: p.root(),
        key: "hit".to_owned(),
        file: "*.*".to_owned(),
        ..GrepSettings::default()
    };
    let info = settings.check().unwrap();
    let (text, outcome) = grep(&info);
    // .git is out, tool.exe is out
    assert_eq!(outcome.hits, 5, "{text}");
    assert!(!text.contains("config"));
    assert!(!text.contains("tool.exe(") && !text.contains("tool.exe\r\n"));
    // several patterns and an exclusion of the file
    let mut info = p.info("hit");
    info.file = "議事*.* a.txt;!*.bak".to_owned();
    let (text, _) = grep(&info);
    assert_eq!(body(&text).lines().count(), 3, "{text}");
    assert!(text.contains("除外ファイル   *.bak\r\n"));
    assert!(text.contains("検索対象       議事*.*;a.txt\r\n"));
}

#[test]
fn a_folder_excluded_by_name_is_not_entered() {
    let p = Project::new();
    p.write("src/a.rs", b"hit\n")
        .write("target/b.rs", b"hit\n")
        .write("src/target/c.rs", b"hit\n");
    let mut info = p.info("hit");
    info.file = "*.rs;#target".to_owned();
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 1, "{text}");
    assert!(text.contains(&p.path("src/a.rs")));
}

#[test]
fn excluded_files_as_regular_expressions() {
    let p = Project::new();
    p.write("a.rs", b"hit\n").write("gen/b.rs", b"hit\n");
    let mut info = p.info("hit");
    info.file = "*.rs;!gen".to_owned();
    info.except_file_regex = true;
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 1, "{text}");
    assert!(text.contains("    (除外ファイルは正規表現)\r\n"));
    assert!(text.contains("除外ファイル   gen\r\n"));
    // an expression that does not compile stops the search before it writes anything
    info.file = "*.rs;!(".to_owned();
    let cancel = AtomicBool::new(false);
    let mut sink = Collect {
        text: String::new(),
        files: Vec::new(),
        stop: None,
    };
    let Err(GrepError::ExceptRegex { pattern, .. }) = run(&info, &cancel, &mut sink) else {
        panic!("the expression is wrong");
    };
    assert_eq!(pattern, "(");
    assert_eq!(sink.text, "");
}

#[test]
fn the_conditions_that_are_wrong_write_nothing() {
    let p = Project::new();
    let cancel = AtomicBool::new(false);
    let mut sink = Collect {
        text: String::new(),
        files: Vec::new(),
        stop: None,
    };
    let mut info = p.info("x");
    info.folder = format!("{}{SEP}nowhere", p.root());
    assert!(matches!(
        run(&info, &cancel, &mut sink),
        Err(GrepError::FolderMissing)
    ));
    let mut info = p.info("x");
    info.file = r"*\a.txt".to_owned();
    assert!(matches!(
        run(&info, &cancel, &mut sink),
        Err(GrepError::Keys(_))
    ));
    let mut info = p.info("(");
    info.options.regex = true;
    let e = run(&info, &cancel, &mut sink).unwrap_err();
    assert!(matches!(e, GrepError::Pattern(_)));
    assert!(e.message().contains("正規表現の構文に誤りがあります"));
    assert_eq!(sink.text, "");
}

#[test]
fn the_encodings_of_the_files_are_detected_and_named() {
    let p = Project::new();
    // "日本語を検索" in Shift_JIS
    let sjis: &[u8] = b"\x93\xFA\x96\x7B\x8C\xEA\x82\xF0\x8C\x9F\x8D\xF5\r\n";
    p.write("a_sjis.txt", sjis);
    p.write("b_utf8.txt", "日本語を検索\r\n".as_bytes());
    let mut utf16: Vec<u8> = vec![0xFF, 0xFE];
    for unit in "日本語を検索\r\n".encode_utf16() {
        utf16.extend_from_slice(&unit.to_le_bytes());
    }
    p.write("c_utf16.txt", &utf16);
    p.write(
        "d_utf8bom.txt",
        b"\xEF\xBB\xBF\xE6\x97\xA5\xE6\x9C\xAC\xE8\xAA\x9E\r\n",
    );
    let (text, outcome) = grep(&p.info("日本語"));
    assert_eq!(outcome.hits, 4, "{text}");
    let lines: Vec<&str> = body(&text).lines().collect();
    assert_eq!(
        lines[0],
        format!("{}(1,1)  [SJIS]: 日本語を検索", p.path("a_sjis.txt"))
    );
    assert_eq!(
        lines[1],
        format!("{}(1,1)  [UTF-8]: 日本語を検索", p.path("b_utf8.txt"))
    );
    assert_eq!(
        lines[2],
        format!("{}(1,1)  [UTF-16]: 日本語を検索", p.path("c_utf16.txt"))
    );
    assert_eq!(
        lines[3],
        format!("{}(1,1)  [UTF-8]: 日本語", p.path("d_utf8bom.txt"))
    );
}

#[test]
fn a_fixed_encoding_reads_every_file_in_it_and_the_lines_have_no_name() {
    let p = Project::new();
    p.write("a.txt", b"\x93\xFA\x96\x7B\x8C\xEA\r\n");
    let mut info = p.info("日本語");
    info.charset = Some(Encoding::ShiftJis);
    let (text, _) = grep(&info);
    assert!(text.contains("    (文字コードセット：SJIS)\r\n"));
    assert_eq!(
        body(&text),
        format!(
            "{}(1,1): 日本語\r\n1 個が検索されました。\r\n",
            p.path("a.txt")
        )
    );
    // the same file read as UTF-8 does not have the word
    info.charset = Some(Encoding::Utf8);
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 0, "{text}");
}

#[test]
fn line_endings_do_not_matter_and_the_last_line_needs_none() {
    let p = Project::new();
    p.write("crlf.txt", b"a\r\nhit 2\r\nc\r\n")
        .write("lf.txt", b"a\nhit 2\nc\n")
        .write("cr.txt", b"a\rhit 2\rc\r")
        .write("mixed.txt", b"a\r\nb\nhit 3\rc")
        .write("last.txt", b"a\nb\nhit 3");
    let (text, outcome) = grep(&p.info("hit"));
    assert_eq!(outcome.hits, 5, "{text}");
    let lines: Vec<String> = body(&text).lines().map(str::to_owned).collect();
    let numbers: Vec<&str> = lines
        .iter()
        .take(5)
        .map(|l| l.split_once('(').unwrap().1.split_once(')').unwrap().0)
        .collect();
    // cr, crlf, last, lf, mixed (name order)
    assert_eq!(numbers, vec!["2,1", "2,1", "3,1", "2,1", "3,1"]);
    // the text of a line never has the break
    assert!(lines[0].ends_with(": hit 2"));
    assert!(lines[2].ends_with(": hit 3"));
}

#[test]
fn the_column_counts_utf16_units() {
    let p = Project::new();
    // 𠮷 is two UTF-16 units, あ one
    p.write("a.txt", "𠮷あ𠮷xyz\n".as_bytes());
    let (text, _) = grep(&p.info("xyz"));
    assert!(body(&text).contains("(1,6)  [UTF-8]: 𠮷あ𠮷xyz"), "{text}");
}

#[test]
fn case_words_and_regular_expressions() {
    let p = Project::new();
    p.write("a.txt", b"Foo foobar FOO\nbar\n");
    // ignoring the case, every part of the line is one hit line
    let mut info = p.info("foo");
    info.layout.line_type = LineType::MatchPart;
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 3, "{text}");
    let lines: Vec<&str> = body(&text).lines().collect();
    assert!(lines[0].ends_with("(1,1)  [UTF-8]: Foo"));
    assert!(lines[1].ends_with("(1,5)  [UTF-8]: foo"));
    assert!(lines[2].ends_with("(1,12)  [UTF-8]: FOO"));
    // with the case
    info.options.match_case = true;
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 1, "{text}");
    assert!(text.contains("    (英大文字小文字を区別する)\r\n"));
    assert!(text.contains("(1,5)  [UTF-8]: foo"));
    // whole words
    info.options = SearchOptions {
        whole_word: true,
        ..SearchOptions::default()
    };
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 2, "{text}");
    assert!(text.contains("    (単語単位で探す)\r\n"));
    // regular expression, and the word option is ignored
    info.options = SearchOptions {
        regex: true,
        whole_word: true,
        ..SearchOptions::default()
    };
    info.key = r"(f)o+\w*".to_owned();
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 3, "{text}");
    assert!(text.contains("    (正規表現:Onigmo)\r\n"));
    info.key = r"^bar$".to_owned();
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 1, "{text}");
    assert!(text.contains("(2,1)  [UTF-8]: bar\r\n"));
}

#[test]
fn a_line_is_written_once_for_the_line_type_and_each_part_for_the_part_type() {
    let p = Project::new();
    p.write("a.txt", b"ab ab ab\nzz\n");
    let mut info = p.info("ab");
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 1, "{text}");
    assert!(text.contains("(1,1)  [UTF-8]: ab ab ab\r\n"));
    info.layout.line_type = LineType::MatchPart;
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 3, "{text}");
    assert!(text.contains("    (一致した箇所のみ出力)\r\n"));
    let lines: Vec<&str> = body(&text).lines().collect();
    assert!(lines[0].ends_with("(1,1)  [UTF-8]: ab"));
    assert!(lines[1].ends_with("(1,4)  [UTF-8]: ab"));
    assert!(lines[2].ends_with("(1,7)  [UTF-8]: ab"));
}

#[test]
fn the_lines_without_a_hit() {
    let p = Project::new();
    p.write("a.txt", b"ab\nzz\n\nab\nyy");
    let mut info = p.info("ab");
    info.layout.line_type = LineType::NoHitLine;
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 3, "{text}");
    assert!(text.contains("    (一致しなかった行を出力)\r\n"));
    let lines: Vec<&str> = body(&text).lines().collect();
    assert!(lines[0].ends_with("(2,1)  [UTF-8]: zz"));
    assert!(lines[1].ends_with("(3,1)  [UTF-8]: "));
    assert!(lines[2].ends_with("(5,1)  [UTF-8]: yy"));
}

#[test]
fn a_regular_expression_that_matches_the_empty_text_goes_on() {
    let p = Project::new();
    p.write("a.txt", b"ab\n");
    let mut info = p.info("x*");
    info.options.regex = true;
    info.layout.line_type = LineType::MatchPart;
    let (text, outcome) = grep(&info);
    // before a, before b, before the line break, and at its end
    assert!(outcome.hits >= 2, "{text}");
    assert!(!outcome.stopped);
}

#[test]
fn a_match_with_a_line_break_does_not_add_an_empty_line() {
    let p = Project::new();
    p.write("a.txt", b"one\r\ntwo\r\n");
    let mut info = p.info(r"one\r\n");
    info.options.regex = true;
    info.layout.line_type = LineType::MatchPart;
    info.layout.style = OutputStyle::ResultOnly;
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 1, "{text}");
    assert_eq!(body(&text), "one\r\n1 個が検索されました。\r\n");
}

#[test]
fn the_per_file_style() {
    let p = Project::new();
    p.write("a.txt", b"hit one\nno\nhit two\n")
        .write("b.txt", b"hit three\n");
    let mut info = p.info("hit");
    info.layout.style = OutputStyle::PerFile;
    let (text, _) = grep(&info);
    assert_eq!(
        body(&text),
        format!(
            "■\"{}\"  [UTF-8]\r\n\
             ・(     1,1    ): hit one\r\n\
             ・(     3,1    ): hit two\r\n\
             ■\"{}\"  [UTF-8]\r\n\
             ・(     1,1    ): hit three\r\n\
             3 個が検索されました。\r\n",
            p.path("a.txt"),
            p.path("b.txt")
        )
    );
}

#[test]
fn the_result_only_style() {
    let p = Project::new();
    p.write("a.txt", b"hit one\nno\nhit two\n");
    let mut info = p.info("hit");
    info.layout.style = OutputStyle::ResultOnly;
    let (text, _) = grep(&info);
    assert_eq!(
        body(&text),
        "hit one\r\nhit two\r\n2 個が検索されました。\r\n"
    );
}

#[test]
fn the_base_folder_and_the_folders_are_written_once() {
    let p = Project::new();
    p.write("a.txt", b"hit\n")
        .write("b.txt", b"hit\n")
        .write("sub/c.txt", b"hit\n");
    // ベースフォルダー表示: the files are shown relative to the base
    let mut info = p.info("hit");
    info.layout.base_folder = true;
    let (text, _) = grep(&info);
    assert_eq!(
        body(&text),
        format!(
            "■\"{}\"\r\n\
             ・a.txt(1,1)  [UTF-8]: hit\r\n\
             ・b.txt(1,1)  [UTF-8]: hit\r\n\
             ・sub{SEP}c.txt(1,1)  [UTF-8]: hit\r\n\
             3 個が検索されました。\r\n",
            p.root()
        )
    );
    // フォルダー毎に表示, with the base folder
    info.layout.separate_folder = true;
    let (text, _) = grep(&info);
    assert_eq!(
        body(&text),
        format!(
            "◎\"{}\"\r\n\
             ■\r\n\
             ・a.txt(1,1)  [UTF-8]: hit\r\n\
             ・b.txt(1,1)  [UTF-8]: hit\r\n\
             ■\"sub\"\r\n\
             ・c.txt(1,1)  [UTF-8]: hit\r\n\
             3 個が検索されました。\r\n",
            p.root()
        )
    );
    // フォルダー毎に表示 without the base folder: the folders are whole
    info.layout.base_folder = false;
    let (text, _) = grep(&info);
    assert_eq!(
        body(&text),
        format!(
            "■\"{root}\"\r\n\
             ・a.txt(1,1)  [UTF-8]: hit\r\n\
             ・b.txt(1,1)  [UTF-8]: hit\r\n\
             ■\"{root}{SEP}sub\"\r\n\
             ・c.txt(1,1)  [UTF-8]: hit\r\n\
             3 個が検索されました。\r\n",
            root = p.root()
        )
    );
}

#[test]
fn only_the_first_hit_of_each_file_with_the_file_option() {
    let p = Project::new();
    p.write("a.txt", b"hit\nhit\nhit\n")
        .write("b.txt", b"x\nhit hit\n");
    let mut info = p.info("hit");
    info.file_only = true;
    info.layout.line_type = LineType::MatchPart;
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 2, "{text}");
    assert!(text.contains("    (ファイル毎最初のみ検索)\r\n"));
    let lines: Vec<&str> = body(&text).lines().collect();
    assert!(lines[0].ends_with("(1,1)  [UTF-8]: hit"));
    assert!(lines[1].ends_with("(2,1)  [UTF-8]: hit"));
}

#[test]
fn a_search_of_the_names_lists_the_files_with_their_encodings() {
    let p = Project::new();
    p.write("a.txt", b"\x93\xFA\x96\x7B\x8C\xEA\r\n")
        .write("b.txt", "plain ascii\n".as_bytes())
        .write("sub/c.log", "日本語\n".as_bytes());
    let mut info = p.info("");
    info.file = "*.txt;*.log".to_owned();
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 3, "{text}");
    assert!(text.contains("□検索条件  「ファイル検索」\r\n"));
    assert_eq!(
        body(&text),
        format!(
            "{}  [SJIS]\r\n{}  [UTF-8]\r\n{}  [UTF-8]\r\n3 個が検索されました。\r\n",
            p.path("a.txt"),
            p.path("b.txt"),
            p.path("sub/c.log"),
        )
    );
    // with a fixed encoding the names have no encoding; and in the per-file style they are headings
    info.charset = Some(Encoding::Utf8);
    info.layout.style = OutputStyle::PerFile;
    let (text, _) = grep(&info);
    assert!(text.contains(&format!("■\"{}\"\r\n", p.path("a.txt"))));
    info.layout.style = OutputStyle::ResultOnly;
    let (text, _) = grep(&info);
    assert!(text.contains(&format!("\r\n{}\r\n", p.path("b.txt"))));
}

#[test]
fn several_folders_are_searched_one_after_the_other() {
    let one = Project::new();
    let two = Project::new();
    one.write("a.txt", b"hit\n");
    two.write("b.txt", b"hit\n");
    let mut info = one.info("hit");
    info.folder = format!("{};\"{}\"", one.root(), two.root());
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 2, "{text}");
    assert!(text.contains(&format!(
        "フォルダー       {};{}\r\n",
        one.root(),
        two.root()
    )));
    let lines: Vec<&str> = body(&text).lines().collect();
    assert!(lines[0].starts_with(&one.path("a.txt")));
    assert!(lines[1].starts_with(&two.path("b.txt")));
}

#[test]
fn a_stop_in_the_middle_ends_the_result_with_the_notice_and_the_count() {
    let p = Project::new();
    for name in ["a", "b", "c", "d"] {
        p.write(&format!("{name}.txt"), b"hit\n");
    }
    let info = p.info("hit");
    let cancel = AtomicBool::new(false);
    let mut sink = Collect {
        text: String::new(),
        files: Vec::new(),
        stop: Some((2, &cancel)),
    };
    let outcome = run(&info, &cancel, &mut sink).unwrap();
    // the stop comes while the second file is opened: the first was done, the others never start
    assert_eq!(
        outcome,
        Outcome {
            hits: 1,
            stopped: true
        }
    );
    assert_eq!(sink.files.len(), 2);
    assert_eq!(
        body(&sink.text),
        format!(
            "{}(1,1)  [UTF-8]: hit\r\n中断しました。\r\n1 個が検索されました。\r\n",
            p.path("a.txt"),
        )
    );
}

#[test]
fn a_stop_before_the_start_still_writes_the_heading_and_the_count() {
    let p = Project::new();
    p.write("a.txt", b"hit\n");
    let cancel = AtomicBool::new(true);
    let mut sink = Collect {
        text: String::new(),
        files: Vec::new(),
        stop: None,
    };
    let outcome = run(&p.info("hit"), &cancel, &mut sink).unwrap();
    assert_eq!(
        outcome,
        Outcome {
            hits: 0,
            stopped: true
        }
    );
    assert_eq!(
        body(&sink.text),
        "中断しました。\r\n0 個が検索されました。\r\n"
    );
}

#[test]
fn a_stop_inside_a_big_file_is_noticed() {
    let p = Project::new();
    let mut big = Vec::new();
    for _ in 0..5000 {
        big.extend_from_slice(b"hit hit hit\n");
    }
    p.write("big.txt", &big);
    // stop at the first progress report of the file: the lines are never searched
    let cancel = AtomicBool::new(false);
    let mut sink = Collect {
        text: String::new(),
        files: Vec::new(),
        stop: Some((1, &cancel)),
    };
    let outcome = run(&p.info("hit"), &cancel, &mut sink).unwrap();
    assert!(outcome.stopped);
    assert!(outcome.hits < 5000);
}

#[test]
fn a_file_that_cannot_be_read_is_reported_and_the_search_goes_on() {
    let p = Project::new();
    p.write("a.txt", b"hit\n").write("c.txt", b"hit\n");
    // a link to nothing: listed by the folder, but it cannot be opened
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(p.path("missing"), p.path("b.txt")).unwrap();
        let (text, outcome) = grep(&p.info("hit"));
        // a link to nothing is not a file for the listing, so there is no report; the search
        // goes on with the others
        assert_eq!(outcome.hits, 2, "{text}");
    }
}

#[test]
fn the_tag_jump_reads_what_grep_writes() {
    let p = Project::new();
    p.write("a.txt", b"first\nsecond hit\n")
        .write("sub/b.txt", "あ𠮷 hit\n".as_bytes());
    let root = PathBuf::from(p.root());
    let is_file = |path: &Path| path.is_file();
    for (style, base, separate) in [
        (OutputStyle::Normal, false, false),
        (OutputStyle::Normal, true, false),
        (OutputStyle::Normal, true, true),
        (OutputStyle::PerFile, false, false),
        (OutputStyle::PerFile, true, true),
    ] {
        let mut info = p.info("hit");
        info.layout.style = style;
        info.layout.base_folder = base;
        info.layout.separate_folder = separate;
        let (text, outcome) = grep(&info);
        assert_eq!(outcome.hits, 2);
        let lines: Vec<String> = text.split("\r\n").map(str::to_owned).collect();
        let at = |i: usize| lines.get(i).cloned();
        // the line of each hit is the one with the text
        for (needle, file, line, column) in [
            ("second hit", "a.txt", 2, 8),
            ("あ𠮷 hit", "sub/b.txt", 1, 5),
        ] {
            let i = lines
                .iter()
                .position(|l| l.ends_with(needle))
                .unwrap_or_else(|| panic!("{needle} in {text}"));
            let target = find_target(&at, i, Some(&root), &is_file).unwrap_or_else(|| {
                panic!("no target for {:?} ({style:?} {base} {separate})", lines[i])
            });
            assert_eq!(
                target.path.canonicalize().unwrap(),
                Path::new(&p.path(file)).canonicalize().unwrap(),
                "{:?}",
                lines[i]
            );
            assert_eq!(target.line, Some(line), "{:?}", lines[i]);
            assert_eq!(target.column, Some(column), "{:?}", lines[i]);
        }
    }
}

#[cfg(unix)]
#[test]
fn a_link_to_a_folder_above_is_not_followed_for_ever() {
    let p = Project::new();
    p.write("a.txt", b"hit\n").write("sub/b.txt", b"hit\n");
    std::os::unix::fs::symlink(p.root(), p.path("sub/loop")).unwrap();
    let (text, outcome) = grep(&p.info("hit"));
    assert_eq!(outcome.hits, 2, "{text}");
}

#[cfg(unix)]
#[test]
fn a_pipe_in_the_folder_is_not_a_file_to_search() {
    let p = Project::new();
    p.write("a.txt", b"hit\n");
    let made = std::process::Command::new("mkfifo")
        .arg(p.path("pipe.txt"))
        .status();
    if !made.is_ok_and(|s| s.success()) {
        eprintln!("SKIP: no mkfifo");
        return;
    }
    // the search of the names reads the start of each file too: it must not wait on the pipe
    for key in ["hit", ""] {
        let (text, outcome) = grep(&p.info(key));
        assert_eq!(outcome.hits, 1, "{text}");
        assert!(!text.contains("pipe.txt"));
    }
}

#[test]
fn many_hits_in_one_long_line_have_their_columns_and_end_quickly() {
    let p = Project::new();
    let mut line = "a".repeat(20_000);
    line.push_str("𠮷a\n");
    p.write("a.txt", line.as_bytes());
    let mut info = p.info("a");
    info.layout.line_type = LineType::MatchPart;
    info.layout.style = OutputStyle::PerFile;
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 20_001, "hits");
    // the last `a` is after 20000 `a`s and one character of two UTF-16 units
    assert!(text.contains("・(     1,20003): a\r\n"), "the last column");
    assert!(text.contains("・(     1,1    ): a\r\n"));
}

#[test]
fn a_stop_is_noticed_inside_a_line_with_many_hits() {
    let p = Project::new();
    p.write("a.txt", "a".repeat(100_000).as_bytes());
    let mut info = p.info("a");
    info.layout.line_type = LineType::MatchPart;
    let cancel = AtomicBool::new(false);
    // stops when the output has some size
    struct StopOnText<'a>(&'a AtomicBool, usize);
    impl Sink for StopOnText<'_> {
        fn write(&mut self, text: &str) {
            self.1 += text.len();
            if self.1 > 2000 {
                self.0.store(true, Ordering::Relaxed);
            }
        }
    }
    let mut sink = StopOnText(&cancel, 0);
    let outcome = run(&info, &cancel, &mut sink).unwrap();
    assert!(outcome.stopped);
    assert!(outcome.hits < 100_000, "{}", outcome.hits);
}
