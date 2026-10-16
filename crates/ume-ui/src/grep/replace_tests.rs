//! Tests of Grep 置換: real files in temporary folders, what is written to them and the text of
//! the result (`CGrepAgent::DoGrepReplaceFile`, `sakura_core/agent/CGrepAgent.cpp`).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use ume_core::encoding::Encoding;
use ume_core::tag_jump::find_target;

use super::engine::{GrepError, GrepInfo, Outcome, ReplaceInfo, Sink, run};
use super::enumerate::SEP;
use super::format::{Layout, LineType};
use crate::search::state::SearchOptions;

/// Collects what the search writes; may stop it after some files.
struct Collect<'a> {
    text: String,
    files: usize,
    stop: Option<(usize, &'a AtomicBool)>,
}

impl Sink for Collect<'_> {
    fn write(&mut self, text: &str) {
        self.text.push_str(text);
    }

    fn progress(&mut self, _file: &str, _hits: usize) {
        self.files += 1;
        if let Some((after, flag)) = self.stop
            && self.files >= after
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

    fn read(&self, name: &str) -> Vec<u8> {
        std::fs::read(self.dir.path().join(name)).unwrap()
    }

    fn exists(&self, name: &str) -> bool {
        self.dir.path().join(name).exists()
    }

    fn root(&self) -> String {
        self.dir.path().to_string_lossy().into_owned()
    }

    fn path(&self, name: &str) -> String {
        let mut path = self.root();
        for part in name.split('/') {
            path.push(SEP);
            path.push_str(part);
        }
        path
    }

    /// A Grep 置換 of `key` with `new`, with a backup (Sakura's default).
    fn info(&self, key: &str, new: &str) -> GrepInfo {
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
            replace: Some(ReplaceInfo {
                text: new.to_owned(),
                paste: false,
                backup: true,
            }),
        }
    }
}

fn regex() -> SearchOptions {
    SearchOptions {
        regex: true,
        ..SearchOptions::default()
    }
}

fn grep(info: &GrepInfo) -> (String, Outcome) {
    let cancel = AtomicBool::new(false);
    let mut sink = Collect {
        text: String::new(),
        files: 0,
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

fn utf16le(text: &str, bom: bool) -> Vec<u8> {
    let mut bytes = if bom { vec![0xFF, 0xFE] } else { Vec::new() };
    bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    bytes
}

#[test]
fn a_plain_replacement_changes_every_match_and_keeps_the_line_breaks() {
    let p = Project::new();
    p.write("a.txt", "abc abc\r\nxyz\r\nabc\nlast abc".as_bytes());
    let (text, outcome) = grep(&p.info("abc", "ふー"));
    assert_eq!(
        outcome,
        Outcome {
            hits: 4,
            stopped: false
        }
    );
    assert_eq!(
        p.read("a.txt"),
        "ふー ふー\r\nxyz\r\nふー\nlast ふー".as_bytes(),
        "the breaks of every line stay, and the last line has none"
    );
    // the result shows the line as it was, one line for each line with a hit
    assert_eq!(
        body(&text),
        format!(
            "{f}(1,1)  [UTF-8]: abc abc\r\n{f}(3,1)  [UTF-8]: abc\r\n{f}(4,6)  [UTF-8]: last abc\r\n4 個を置換しました。\r\n",
            f = p.path("a.txt")
        )
    );
    // the heading says what it is replaced with
    assert!(text.contains("□検索条件  \"abc\"\r\n置換後         \"ふー\"\r\n検索対象"));
}

#[test]
fn a_file_without_a_hit_is_left_alone_and_nothing_is_kept() {
    let p = Project::new();
    p.write("a.txt", b"nothing\n");
    let before = std::fs::metadata(p.path("a.txt"))
        .unwrap()
        .modified()
        .unwrap();
    let (text, outcome) = grep(&p.info("abc", "x"));
    assert_eq!(outcome.hits, 0);
    assert_eq!(body(&text), "0 個を置換しました。\r\n");
    assert_eq!(p.read("a.txt"), b"nothing\n");
    assert!(!p.exists("a.txt.skrold"));
    let after = std::fs::metadata(p.path("a.txt"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(before, after, "the file was not written");
}

#[test]
fn the_empty_pattern_is_refused() {
    let p = Project::new();
    p.write("a.txt", b"x\n");
    let cancel = AtomicBool::new(false);
    let mut sink = Collect {
        text: String::new(),
        files: 0,
        stop: None,
    };
    let result = run(&p.info("", "x"), &cancel, &mut sink);
    assert!(matches!(result, Err(GrepError::NoPattern)));
    assert_eq!(sink.text, "", "nothing is written");
    assert_eq!(p.read("a.txt"), b"x\n");
}

#[test]
fn a_regular_expression_replaces_with_groups() {
    let p = Project::new();
    p.write("a.txt", "taro@example, jiro@sample\r\nnone\r\n".as_bytes());
    let mut info = p.info(r"(\w+)@(\w+)", "$2:$1");
    info.options = regex();
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 2, "{text}");
    assert_eq!(
        p.read("a.txt"),
        "example:taro, sample:jiro\r\nnone\r\n".as_bytes()
    );
    // only the first match of a line is shown, as the line
    assert_eq!(
        body(&text),
        format!(
            "{}(1,1)  [UTF-8]: taro@example, jiro@sample\r\n2 個を置換しました。\r\n",
            p.path("a.txt")
        )
    );
}

#[test]
fn a_regular_expression_works_on_japanese_text_and_case_conversion() {
    let p = Project::new();
    p.write("a.txt", "あいう abc\nえお def\n".as_bytes());
    let mut info = p.info(r"([ぁ-ん]+) (\w+)", r"\U$2\E-$1");
    info.options = regex();
    let (_, outcome) = grep(&info);
    assert_eq!(outcome.hits, 2);
    assert_eq!(p.read("a.txt"), "ABC-あいう\nDEF-えお\n".as_bytes());
}

#[test]
fn a_regular_expression_with_a_wrong_replacement_does_not_start() {
    let p = Project::new();
    p.write("a.txt", b"abc\n");
    let mut info = p.info("abc", "x\\");
    info.options = regex();
    let cancel = AtomicBool::new(false);
    let mut sink = Collect {
        text: String::new(),
        files: 0,
        stop: None,
    };
    let result = run(&info, &cancel, &mut sink);
    assert!(matches!(result, Err(GrepError::Replacement(_))));
    assert_eq!(p.read("a.txt"), b"abc\n");
}

#[test]
fn a_paste_puts_the_text_in_as_it_is_even_for_a_regular_expression() {
    let p = Project::new();
    p.write("a.txt", b"one two\n");
    let mut info = p.info(r"\w+", "$1\\n");
    info.options = regex();
    info.replace.as_mut().unwrap().paste = true;
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 2);
    assert_eq!(p.read("a.txt"), b"$1\\n $1\\n\n");
    assert!(text.contains("置換後         (クリップボードから貼り付け)\r\n検索対象"));
    assert!(!text.contains("$1"), "{text}");
}

#[test]
fn whole_words_only_are_replaced() {
    let p = Project::new();
    p.write("a.txt", b"cat concat cat.\n");
    let mut info = p.info("cat", "dog");
    info.options.whole_word = true;
    let (_, outcome) = grep(&info);
    assert_eq!(outcome.hits, 2);
    assert_eq!(p.read("a.txt"), b"dog concat dog.\n");
}

#[test]
fn case_is_ignored_unless_asked() {
    let p = Project::new();
    p.write("a.txt", b"Abc aBC abc\n");
    let (_, outcome) = grep(&p.info("abc", "-"));
    assert_eq!(outcome.hits, 3);
    assert_eq!(p.read("a.txt"), b"- - -\n");
    p.write("a.txt", b"Abc aBC abc\n");
    let mut info = p.info("abc", "-");
    info.options.match_case = true;
    let (_, outcome) = grep(&info);
    assert_eq!(outcome.hits, 1);
    assert_eq!(p.read("a.txt"), b"Abc aBC -\n");
}

#[test]
fn the_new_text_is_not_searched_again() {
    let p = Project::new();
    p.write("a.txt", b"aa\n");
    let (_, outcome) = grep(&p.info("a", "aa"));
    assert_eq!(outcome.hits, 2);
    assert_eq!(p.read("a.txt"), b"aaaa\n");
}

#[test]
fn a_shift_jis_file_keeps_its_encoding_and_line_breaks() {
    let p = Project::new();
    // 日本語 / 日本語 as Shift_JIS, with CR LF and LF
    let sjis: &[u8] = &[
        0x93, 0xFA, 0x96, 0x7B, 0x8C, 0xEA, 0x0D, 0x0A, 0x93, 0xFA, 0x96, 0x7B, 0x8C, 0xEA, 0x0A,
    ];
    p.write("a.txt", sjis);
    let mut info = p.info("本", "木");
    info.charset = Some(Encoding::ShiftJis);
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 2, "{text}");
    // 木 is 0x96D8
    assert_eq!(
        p.read("a.txt"),
        [
            0x93, 0xFA, 0x96, 0xD8, 0x8C, 0xEA, 0x0D, 0x0A, 0x93, 0xFA, 0x96, 0xD8, 0x8C, 0xEA,
            0x0A
        ]
    );
    // the backup is the file as it was
    assert_eq!(p.read("a.txt.skrold"), sjis);
}

#[test]
fn a_shift_jis_file_is_detected_and_written_back_in_shift_jis() {
    let p = Project::new();
    let line = "これは日本語の文章です。検索と置換を試します。\r\n";
    let sjis = encoding_to_sjis(line);
    let mut bytes = Vec::new();
    for _ in 0..5 {
        bytes.extend_from_slice(&sjis);
    }
    p.write("a.txt", &bytes);
    let (text, outcome) = grep(&p.info("置換", "交換"));
    assert_eq!(outcome.hits, 5, "{text}");
    assert!(text.contains("[SJIS]"), "{text}");
    let changed = encoding_to_sjis(&line.replace("置換", "交換"));
    let mut expected = Vec::new();
    for _ in 0..5 {
        expected.extend_from_slice(&changed);
    }
    assert_eq!(p.read("a.txt"), expected);
}

/// Shift_JIS bytes of Japanese text, from the codec of the project.
fn encoding_to_sjis(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    Encoding::ShiftJis
        .codec()
        .encode(text, &mut out)
        .expect("the text is in Shift_JIS");
    out
}

#[test]
fn a_utf16_file_keeps_its_bom_and_encoding() {
    let p = Project::new();
    p.write("a.txt", &utf16le("abc\r\nそのabc\r\n", true));
    let (text, outcome) = grep(&p.info("abc", "あ"));
    assert_eq!(outcome.hits, 2, "{text}");
    assert_eq!(p.read("a.txt"), utf16le("あ\r\nそのあ\r\n", true));
    assert_eq!(p.read("a.txt.skrold"), utf16le("abc\r\nそのabc\r\n", true));
    assert!(text.contains("[UTF-16]"), "{text}");
}

#[test]
fn a_utf8_bom_is_kept() {
    let p = Project::new();
    p.write("a.txt", b"\xEF\xBB\xBFabc\r\n");
    let (_, outcome) = grep(&p.info("abc", "d"));
    assert_eq!(outcome.hits, 1);
    assert_eq!(p.read("a.txt"), b"\xEF\xBB\xBFd\r\n");
}

#[test]
fn the_lines_that_are_not_changed_keep_their_bytes() {
    let p = Project::new();
    // an invalid byte in a line that is not touched
    p.write("a.txt", b"bad \xFF byte\r\nabc\r\n");
    let (_, outcome) = grep(&p.info("abc", "d"));
    assert_eq!(outcome.hits, 1);
    assert_eq!(p.read("a.txt"), b"bad \xFF byte\r\nd\r\n");
}

#[test]
fn the_backup_is_kept_as_skrold_and_an_old_one_is_replaced() {
    let p = Project::new();
    p.write("a.txt", b"abc\n").write("a.txt.skrold", b"older\n");
    let (_, outcome) = grep(&p.info("abc", "d"));
    assert_eq!(outcome.hits, 1);
    assert_eq!(p.read("a.txt"), b"d\n");
    assert_eq!(p.read("a.txt.skrold"), b"abc\n");
}

#[test]
fn no_backup_is_made_when_it_is_not_asked() {
    let p = Project::new();
    p.write("a.txt", b"abc\n");
    let mut info = p.info("abc", "d");
    info.replace.as_mut().unwrap().backup = false;
    let (_, outcome) = grep(&info);
    assert_eq!(outcome.hits, 1);
    assert_eq!(p.read("a.txt"), b"d\n");
    assert!(!p.exists("a.txt.skrold"));
}

#[test]
fn only_the_first_hit_of_a_file_is_shown_with_the_option_but_all_are_replaced() {
    let p = Project::new();
    p.write("a.txt", b"abc\nabc abc\nabc\n");
    let mut info = p.info("abc", "d");
    info.file_only = true;
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 4);
    assert_eq!(p.read("a.txt"), b"d\nd d\nd\n");
    assert_eq!(
        body(&text),
        format!(
            "{}(1,1)  [UTF-8]: abc\r\n4 個を置換しました。\r\n",
            p.path("a.txt")
        )
    );
}

#[test]
fn the_matched_parts_are_shown_one_by_one_for_a_plain_replacement() {
    let p = Project::new();
    p.write("a.txt", b"abc x abc\n");
    let mut info = p.info("abc", "d");
    info.layout.line_type = LineType::MatchPart;
    let (text, _) = grep(&info);
    assert_eq!(
        body(&text),
        format!(
            "{f}(1,1)  [UTF-8]: abc\r\n{f}(1,7)  [UTF-8]: abc\r\n2 個を置換しました。\r\n",
            f = p.path("a.txt")
        )
    );
}

#[test]
fn lines_without_a_hit_cannot_be_asked_for() {
    // `GrepInfo::Normalized`: the lines of the hits are shown instead
    let p = Project::new();
    p.write("a.txt", b"abc\nother\n");
    let mut info = p.info("abc", "d");
    info.layout.line_type = LineType::NoHitLine;
    let (text, outcome) = grep(&info);
    assert_eq!(outcome.hits, 1);
    assert!(text.contains("    (一致した行を出力)\r\n"), "{text}");
    assert_eq!(
        body(&text),
        format!(
            "{}(1,1)  [UTF-8]: abc\r\n1 個を置換しました。\r\n",
            p.path("a.txt")
        )
    );
}

#[cfg(unix)]
#[test]
fn a_read_only_file_is_not_changed_and_is_told() {
    use std::os::unix::fs::PermissionsExt;
    let p = Project::new();
    p.write("a.txt", b"abc\n").write("b.txt", b"abc\n");
    let path = p.dir.path().join("a.txt");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).unwrap();
    let (text, outcome) = grep(&p.info("abc", "d"));
    assert_eq!(p.read("a.txt"), b"abc\n");
    assert!(
        !p.exists("a.txt.skrold"),
        "the backup of a failed file is removed"
    );
    assert_eq!(p.read("b.txt"), b"d\n");
    assert_eq!(outcome.hits, 1, "only the file that was written counts");
    assert!(
        text.contains(&format!(
            "ファイルの削除に失敗しました。[{}]\r\n",
            p.path("a.txt")
        )),
        "{text}"
    );
    // the hit of the file that failed is in the result all the same
    assert!(
        text.contains(&format!("{}(1,1)", p.path("a.txt"))),
        "{text}"
    );
}

#[test]
fn a_character_the_encoding_cannot_write_leaves_the_file_as_it_was() {
    let p = Project::new();
    let sjis: &[u8] = &[0x93, 0xFA, 0x0D, 0x0A, b'a', b'b', b'c', 0x0D, 0x0A];
    p.write("a.txt", sjis);
    let mut info = p.info("abc", "🙂");
    info.charset = Some(Encoding::ShiftJis);
    let (text, outcome) = grep(&info);
    assert_eq!(p.read("a.txt"), sjis);
    assert!(!p.exists("a.txt.skrold"));
    assert_eq!(outcome.hits, 0);
    assert!(
        text.contains(&format!("書き込めませんでした。 [{}]\r\n", p.path("a.txt"))),
        "{text}"
    );
}

#[test]
fn a_stop_before_a_file_leaves_it_and_the_rest_alone() {
    let p = Project::new();
    p.write("a.txt", b"abc\n").write("b.txt", b"abc\n");
    let cancel = AtomicBool::new(false);
    let mut sink = Collect {
        text: String::new(),
        files: 0,
        stop: Some((2, &cancel)),
    };
    let outcome = run(&p.info("abc", "d"), &cancel, &mut sink).unwrap();
    assert!(outcome.stopped);
    assert_eq!(outcome.hits, 1);
    // the files come in the order of the listing: the first is done, the second is not touched
    let contents = [p.read("a.txt"), p.read("b.txt")];
    assert_eq!(
        contents.iter().filter(|c| c.as_slice() == b"d\n").count(),
        1,
        "{contents:?}"
    );
    assert_eq!(
        contents.iter().filter(|c| c.as_slice() == b"abc\n").count(),
        1,
        "{contents:?}"
    );
    assert!(
        sink.text
            .contains("中断しました。\r\n1 個を置換しました。\r\n")
    );
}

#[test]
fn a_stop_in_a_file_writes_nothing_of_it() {
    let p = Project::new();
    // many lines: the flag is read every 32 lines, and set from the first line of the sink
    let mut content = Vec::new();
    for _ in 0..200 {
        content.extend_from_slice(b"abc\n");
    }
    p.write("a.txt", &content);
    let cancel = AtomicBool::new(false);
    let mut sink = Collect {
        text: String::new(),
        files: 0,
        stop: Some((1, &cancel)),
    };
    let outcome = run(&p.info("abc", "d"), &cancel, &mut sink).unwrap();
    assert!(outcome.stopped);
    assert_eq!(outcome.hits, 0);
    assert_eq!(p.read("a.txt"), content, "the file is as it was");
    assert!(!p.exists("a.txt.skrold"));
    // no temporary file of the save is left
    let names: Vec<_> = std::fs::read_dir(p.dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 1, "{names:?}");
}

#[test]
fn the_tag_jump_reads_what_grep_replace_writes() {
    let p = Project::new();
    p.write("a.txt", b"first\nsecond hit\n");
    let root = PathBuf::from(p.root());
    let is_file = |path: &Path| path.is_file();
    let (text, _) = grep(&p.info("hit", "HIT"));
    let lines: Vec<String> = text.split("\r\n").map(str::to_owned).collect();
    let i = lines
        .iter()
        .position(|l| l.ends_with("second hit"))
        .unwrap_or_else(|| panic!("{text}"));
    let at = |i: usize| lines.get(i).cloned();
    let target = find_target(&at, i, Some(&root), &is_file).expect("a target");
    assert_eq!(target.line, Some(2));
    assert_eq!(target.column, Some(8));
}
