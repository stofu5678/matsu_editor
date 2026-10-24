//! Tests of Sakura's command-line options at work, through `open_from_outside_with` (the entry of
//! the first launch and of a launch that was handed over). The files are real files in a private
//! temporary folder.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use gpui::{Entity, TestAppContext};
use matsu_core::buffer::Pos;
use matsu_core::encoding::Encoding;
use matsu_platform::instance::Request;

use super::*;
use crate::file_ops::windows::{editors, find_open, open_from_outside_with};
use crate::launch::OpenRequest;

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("matsu-ui-launch-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn request(dir: &TempDir, args: &[&str]) -> OpenRequest {
    OpenRequest::from_launch(&Request {
        cwd: dir.0.clone(),
        args: args.iter().map(OsString::from).collect(),
    })
}

/// A launch with these arguments, as the first launch or a handed-over one does it.
fn launch(cx: &mut TestAppContext, dir: &TempDir, args: &[&str]) {
    let request = request(dir, args);
    let options = LaunchOptions::from_request(&request);
    cx.update(|cx| open_from_outside_with(cx, request.paths, &options))
        .unwrap();
    for _ in 0..50 {
        cx.run_until_parked();
    }
}

fn view_of(cx: &mut TestAppContext, path: &Path) -> Entity<EditorView> {
    cx.update(|cx| find_open(cx, path)).expect("open").1
}

fn caret(cx: &mut TestAppContext, view: &Entity<EditorView>) -> Pos {
    view.read_with(cx, |v, _| v.cursor.caret())
}

fn type_of_ext(cx: &mut TestAppContext, ext: &str) -> usize {
    cx.update(|app| {
        crate::doc_types::with_user(app, |user| user.document_types().type_index_for_ext(ext))
    })
}

fn init(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
}

#[gpui::test]
fn the_caret_goes_to_the_line_and_column_after_the_load(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    // "é" is 2 bytes and 1 UTF-16 unit; the file is CR LF
    let path = dir.write("a.txt", "abc\r\nd\u{e9}f日本\r\nxyz".as_bytes());
    launch(cx, &dir, &["-X=4", "-Y=2", "a.txt"]);
    let view = view_of(cx, &path);
    // column 4 counts from 1: 3 UTF-16 units, "dé" and "f" = 4 bytes
    assert_eq!(caret(cx, &view), Pos::new(1, 4));
}

#[gpui::test]
fn the_column_stops_at_the_end_of_the_line(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc\r\ndef\r\n");
    launch(cx, &dir, &["-X=99", "-Y=1", "a.txt"]);
    let view = view_of(cx, &path);
    assert_eq!(caret(cx, &view), Pos::new(0, 3), "not after the CR LF");
}

#[gpui::test]
fn a_line_past_the_end_is_the_end_of_the_text(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc\ndef");
    launch(cx, &dir, &["-Y=50", "a.txt"]);
    let view = view_of(cx, &path);
    assert_eq!(caret(cx, &view), Pos::new(1, 3));
}

#[gpui::test]
fn a_column_alone_moves_in_the_first_line_and_a_line_alone_to_column_one(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    let a = dir.write("a.txt", b"abcdef\nxyz\n");
    let b = dir.write("b.txt", b"abcdef\nxyz\n");
    launch(cx, &dir, &["-X=3", "a.txt"]);
    launch(cx, &dir, &["-Y=2", "b.txt"]);
    let (va, vb) = (view_of(cx, &a), view_of(cx, &b));
    assert_eq!(caret(cx, &va), Pos::new(0, 2));
    assert_eq!(caret(cx, &vb), Pos::new(1, 0));
}

#[gpui::test]
fn without_position_options_the_caret_stays(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc\ndef\n");
    // 0 and below are no position
    launch(cx, &dir, &["-X=0", "-Y=0", "a.txt"]);
    let view = view_of(cx, &path);
    assert_eq!(caret(cx, &view), Pos::new(0, 0));
}

#[gpui::test]
fn the_view_goes_to_the_row_when_both_are_given(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    let text: String = (0..300).map(|i| format!("line {i}\n")).collect();
    let path = dir.write("long.txt", text.as_bytes());
    launch(cx, &dir, &["-VX=1", "-VY=101", "long.txt"]);
    let view = view_of(cx, &path);
    assert_eq!(view.read_with(cx, |v, _| v.top_row()), 100);
    // only one of them: the view stays
    let other = dir.write("other.txt", text.as_bytes());
    launch(cx, &dir, &["-VY=101", "other.txt"]);
    let view = view_of(cx, &other);
    assert_eq!(view.read_with(cx, |v, _| v.top_row()), 0);
}

#[gpui::test]
fn view_mode_opens_the_file_read_only(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    let a = dir.write("a.txt", b"abc");
    let b = dir.write("b.txt", b"abc");
    launch(cx, &dir, &["-R", "a.txt", "b.txt"]);
    for path in [&a, &b] {
        let view = view_of(cx, path);
        assert!(view.read_with(cx, |v, app| v.document().read(app).view_mode()));
    }
    let c = dir.write("c.txt", b"abc");
    launch(cx, &dir, &["c.txt"]);
    let view = view_of(cx, &c);
    assert!(!view.read_with(cx, |v, app| v.document().read(app).view_mode()));
}

#[gpui::test]
fn view_mode_without_a_file_makes_an_empty_read_only_window(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    launch(cx, &dir, &["-R"]);
    let views = cx.update(editors);
    assert_eq!(views.len(), 1);
    assert!(views[0].1.read_with(cx, |v, app| {
        let shared = v.document().read(app);
        shared.view_mode() && shared.path().is_none()
    }));
}

#[gpui::test]
fn the_encoding_of_code_reads_the_file(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    // 0xEF is "ï" in Latin1 (8) and nothing valid in UTF-8
    let path = dir.write("a.txt", b"na\xefve\n");
    launch(cx, &dir, &["-CODE=8", "a.txt"]);
    let view = view_of(cx, &path);
    let (encoding, line) = view.read_with(cx, |v, app| {
        let shared = v.document().read(app);
        (
            shared.file().encoding,
            shared.doc().line(0).unwrap().unwrap().text,
        )
    });
    assert_eq!(encoding, Encoding::Latin1);
    assert_eq!(line, "na\u{ef}ve");
    // 99 is the detection, and a number that is no encoding is too
    let utf8 = dir.write("b.txt", "日本語\n".as_bytes());
    launch(cx, &dir, &["-CODE=99", "b.txt"]);
    let view = view_of(cx, &utf8);
    assert_eq!(
        view.read_with(cx, |v, app| v.document().read(app).file().encoding),
        Encoding::Utf8
    );
}

#[gpui::test]
fn the_type_of_the_extension_is_chosen_and_kept(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    let path = dir.write("notes.txt", b"int main() {}\n");
    let wanted = type_of_ext(cx, "cpp");
    assert_ne!(wanted, 0, "the built-in types know cpp");
    launch(cx, &dir, &["-TYPE=cpp", "notes.txt"]);
    let view = view_of(cx, &path);
    let (index, locked) = view.read_with(cx, |v, app| {
        (
            v.view_settings().type_index,
            v.document().read(app).type_locked(),
        )
    });
    assert_eq!(index, wanted);
    assert!(locked);
}

#[gpui::test]
fn the_type_option_is_for_the_first_file_only(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    let a = dir.write("a.txt", b"x\n");
    let b = dir.write("b.txt", b"x\n");
    launch(cx, &dir, &["-TYPE=cpp", "-X=2", "a.txt", "b.txt"]);
    let (va, vb) = (view_of(cx, &a), view_of(cx, &b));
    assert!(va.read_with(cx, |v, app| v.document().read(app).type_locked()));
    assert!(!vb.read_with(cx, |v, app| v.document().read(app).type_locked()));
    // the caret is for both
    assert_eq!(caret(cx, &va), Pos::new(0, 1));
    assert_eq!(caret(cx, &vb), Pos::new(0, 1));
}

#[gpui::test]
fn an_empty_window_takes_the_type_and_the_encoding(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    launch(cx, &dir, &["-TYPE=cpp", "-CODE=3"]);
    let views = cx.update(editors);
    assert_eq!(views.len(), 1);
    let wanted = type_of_ext(cx, "cpp");
    let (index, locked, encoding, eol) = views[0].1.read_with(cx, |v, app| {
        let shared = v.document().read(app);
        (
            v.view_settings().type_index,
            shared.type_locked(),
            shared.file().encoding,
            shared.model().new_line(),
        )
    });
    assert_eq!(index, wanted);
    assert!(locked);
    assert_eq!(encoding, Encoding::Utf16Le);
    assert_eq!(
        eol,
        Eol::CrLf,
        "an encoding that is not the default of the type"
    );
}

#[gpui::test]
fn a_handed_over_launch_brings_its_options(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    let first = dir.write("a.txt", b"one\ntwo\n");
    launch(cx, &dir, &["a.txt"]);
    // the second launch is read by the process that receives it, with its working folder
    let second = dir.write("b.txt", b"alpha\nbeta\n");
    launch(cx, &dir, &["-R", "-Y=2", "-X=3", "b.txt"]);
    let view = view_of(cx, &second);
    assert_eq!(caret(cx, &view), Pos::new(1, 2));
    assert!(view.read_with(cx, |v, app| v.document().read(app).view_mode()));
    // the first window is as it was
    let view = view_of(cx, &first);
    assert!(!view.read_with(cx, |v, app| v.document().read(app).view_mode()));
}

#[gpui::test]
fn a_file_that_is_open_already_gets_the_caret_and_no_second_window(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"one\ntwo\nthree\n");
    launch(cx, &dir, &["a.txt"]);
    launch(cx, &dir, &["-Y=3", "a.txt"]);
    assert_eq!(cx.windows().len(), 1);
    let view = view_of(cx, &path);
    // the column stays (0), the line is 3
    assert_eq!(caret(cx, &view), Pos::new(2, 0));
    // with a column too
    launch(cx, &dir, &["-X=3", "-Y=2", "a.txt"]);
    assert_eq!(caret(cx, &view), Pos::new(1, 2));
    assert_eq!(cx.windows().len(), 1);
    // without -Y nothing moves
    launch(cx, &dir, &["-X=1", "a.txt"]);
    assert_eq!(caret(cx, &view), Pos::new(1, 2));
}

#[gpui::test]
fn the_window_gets_the_size_and_the_place(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    launch(cx, &dir, &["-SX=801", "-SY=601", "-WX=40", "-WY=50"]);
    let handle = cx.windows()[0];
    let bounds = handle.update(cx, |_, window, _| window.bounds()).unwrap();
    assert_eq!(bounds.size.width, px(800.));
    assert_eq!(bounds.size.height, px(600.));
    assert_eq!(bounds.origin.x, px(40.));
    assert_eq!(bounds.origin.y, px(50.));
    // the next window is the default again
    launch(cx, &dir, &[]);
    let second = cx.windows()[1];
    let bounds = second.update(cx, |_, window, _| window.bounds()).unwrap();
    assert_eq!(bounds.size.width, px(1000.));
}

#[test]
fn the_placement_of_a_size_of_nothing_is_the_default() {
    let default = size(px(1000.), px(700.));
    let p = Placement {
        width: Some(0),
        height: Some(-3),
        ..Placement::default()
    };
    assert_eq!(p.size_or(default), default);
    let p = Placement {
        width: Some(500),
        ..Placement::default()
    };
    assert_eq!(p.size_or(default), size(px(500.), px(700.)));
    let mut bounds = Bounds::new(point(px(1.), px(2.)), default);
    Placement {
        y: Some(-9),
        ..Placement::default()
    }
    .place(&mut bounds);
    assert_eq!(bounds.origin, point(px(1.), px(-9.)));
}

// ---- Grep ----

fn grep_project(dir: &TempDir) -> PathBuf {
    let src = dir.0.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("a.txt"), "hello world\nneedle here\n").unwrap();
    std::fs::write(src.join("b.rs"), "fn needle() {}\n").unwrap();
    src
}

fn all_text(view: &Entity<EditorView>, cx: &mut TestAppContext) -> String {
    view.read_with(cx, |v, app| {
        let doc = v.document().read(app).doc();
        (0..doc.line_count())
            .map(|i| doc.line(i).unwrap().unwrap().text + "\n")
            .collect()
    })
}

#[gpui::test]
fn grep_mode_runs_the_search_in_a_new_window(cx: &mut TestAppContext) {
    init(cx);
    cx.executor().allow_parking();
    let dir = TempDir::new();
    grep_project(&dir);
    launch(
        cx,
        &dir,
        &[
            "-GREPMODE",
            "-GKEY=needle",
            "-GFILE=*.txt",
            "-GFOLDER=src",
            "-GOPT=S",
        ],
    );
    for _ in 0..200 {
        cx.run_until_parked();
    }
    let views = cx.update(editors);
    assert_eq!(views.len(), 1);
    let view = &views[0].1;
    assert!(view.read_with(cx, |v, _| v.is_grep_window()));
    let text = all_text(view, cx);
    assert!(text.contains("a.txt(2,1): needle"), "{text}");
    assert!(!text.contains("b.rs"), "{text}");
    assert!(text.contains("1 個"), "{text}");
}

#[gpui::test]
fn grep_mode_does_not_open_the_files(cx: &mut TestAppContext) {
    init(cx);
    cx.executor().allow_parking();
    let dir = TempDir::new();
    let src = grep_project(&dir);
    launch(
        cx,
        &dir,
        &[
            "-GREPMODE",
            "-GKEY=zzz",
            "-GFILE=*.*",
            "-GFOLDER=src",
            "src/a.txt",
        ],
    );
    assert_eq!(cx.windows().len(), 1);
    assert!(cx.update(|cx| find_open(cx, &src.join("a.txt"))).is_none());
}

#[gpui::test]
fn grep_options_decide_the_hits(cx: &mut TestAppContext) {
    init(cx);
    cx.executor().allow_parking();
    let dir = TempDir::new();
    grep_project(&dir);
    // L: the case matters, so "NEEDLE" finds nothing; without it, it finds both files
    launch(
        cx,
        &dir,
        &[
            "-GREPMODE",
            "-GKEY=NEEDLE",
            "-GFILE=*.*",
            "-GFOLDER=src",
            "-GOPT=SL",
        ],
    );
    for _ in 0..200 {
        cx.run_until_parked();
    }
    let text = all_text(&cx.update(editors)[0].1, cx);
    assert!(text.contains("0 個"), "{text}");
    launch(
        cx,
        &dir,
        &[
            "-GREPMODE",
            "-GKEY=NEEDLE",
            "-GFILE=*.*",
            "-GFOLDER=src",
            "-GOPT=S",
        ],
    );
    for _ in 0..200 {
        cx.run_until_parked();
    }
    let views = cx.update(editors);
    let text = all_text(&views[1].1, cx);
    assert!(text.contains("2 個"), "{text}");
}

#[gpui::test]
fn a_wrong_grep_folder_is_told_and_nothing_runs(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    launch(
        cx,
        &dir,
        &["-GREPMODE", "-GKEY=x", "-GFILE=*.*", "-GFOLDER=nowhere"],
    );
    assert_eq!(cx.windows().len(), 1);
    assert!(cx.has_pending_prompt());
}

#[gpui::test]
fn the_grep_dialog_opens_filled_in(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    grep_project(&dir);
    launch(
        cx,
        &dir,
        &[
            "-GREPMODE",
            "-GREPDLG",
            "-GKEY=needle",
            "-GFILE=*.txt;!*.bak",
            "-GFOLDER=src",
            "-GOPT=SLR",
        ],
    );
    let views = cx.update(editors);
    assert_eq!(views.len(), 1);
    let view = &views[0].1;
    assert!(view.read_with(cx, |v, _| v.grep_panel_open()));
    assert!(!view.read_with(cx, |v, _| v.is_grep_running()));
    let panel = view.read_with(cx, |v, _| v.grep.panel.clone().unwrap());
    let settings = panel.read_with(cx, |p, _| p.settings.clone());
    assert_eq!(settings.key, "needle");
    assert_eq!(settings.file, "*.txt");
    assert_eq!(settings.exclude_file, "*.bak");
    assert_eq!(
        PathBuf::from(&settings.folder),
        dir.0.join("src"),
        "relative to the folder of the launch"
    );
    assert!(settings.sub_folder && settings.options.match_case && settings.options.regex);
}

#[test]
fn grep_info_follows_the_command_line() {
    let args = crate::command_line::parse(&[
        "-GKEY=a".into(),
        "-GFILE=*.rs;#.git;!*.lock".into(),
        "-GFOLDER=src;/abs;\"with;semi\"".into(),
        "-GOPT=SLRWFBD3N".into(),
        "-GCODE=4".into(),
    ])
    .grep;
    let info = grep_info(&args, Path::new("/work"), None);
    assert_eq!(info.key, "a");
    assert_eq!(info.file, "*.rs;#.git;!*.lock");
    assert_eq!(
        info.folder,
        format!(
            "{};/abs;\"{}\"",
            Path::new("/work").join("src").display(),
            Path::new("/work").join("with;semi").display()
        )
    );
    assert!(info.sub_folder && info.options.match_case && info.options.regex);
    assert!(info.options.whole_word && info.file_only);
    assert_eq!(info.charset, Some(Encoding::Utf8));
    assert_eq!(info.layout.style, OutputStyle::ResultOnly);
    assert_eq!(info.layout.line_type, LineType::NoHitLine);
    assert!(info.layout.base_folder && info.layout.separate_folder);
    assert!(info.replace.is_none());
    // the start of Sakura: Shift_JIS, ordinary form, no sub folder
    let plain = grep_info(&GrepArgs::default(), Path::new(""), None);
    assert_eq!(plain.charset, Some(Encoding::ShiftJis));
    assert_eq!(plain.layout.style, OutputStyle::Normal);
    assert_eq!(plain.layout.line_type, LineType::MatchPart);
    assert!(!plain.sub_folder);
}

#[test]
fn grep_replace_info_takes_the_clipboard_for_c() {
    let args =
        crate::command_line::parse(&["-GKEY=a".into(), "-GREPR=b".into(), "-GOPT=O".into()]).grep;
    let info = grep_info(&args, Path::new(""), None);
    let replace = info.replace.unwrap();
    assert_eq!(
        (replace.text.as_str(), replace.paste, replace.backup),
        ("b", false, true)
    );
    let args =
        crate::command_line::parse(&["-GKEY=a".into(), "-GREPR=b".into(), "-GOPT=C".into()]).grep;
    let replace = grep_info(&args, Path::new(""), Some("clip".into()))
        .replace
        .unwrap();
    assert_eq!(
        (replace.text.as_str(), replace.paste, replace.backup),
        ("clip", true, false)
    );
}

#[test]
fn the_dialog_without_a_folder_starts_in_the_folder_of_the_launch() {
    let args = GrepArgs {
        key: "k".into(),
        ..GrepArgs::default()
    };
    let settings = grep_dialog_settings(&args, Path::new("/work"), GrepSettings::default());
    assert_eq!(PathBuf::from(&settings.folder), PathBuf::from("/work"));
    assert_eq!(settings.key, "k");
    // no exclusions given: what the last Grep left stays
    assert_eq!(settings.exclude_file, GrepSettings::default().exclude_file);
}

#[test]
fn launch_options_split_what_is_for_every_file_and_what_is_for_the_first() {
    let line = crate::command_line::parse(&[
        "-X=2".into(),
        "-TYPE=rs".into(),
        "-M=run.mac".into(),
        "-CODE=4".into(),
    ]);
    let options = LaunchOptions::new(&line, Path::new("/w"));
    let first = options.after_load(true).unwrap();
    assert_eq!(first.doc_type.as_deref(), Some("rs"));
    assert_eq!(
        first.macro_,
        Some(MacroLaunch::File(Path::new("/w").join("run.mac")))
    );
    let later = options.after_load(false).unwrap();
    assert!(later.doc_type.is_none() && later.macro_.is_none());
    assert_eq!(later.file.caret_x, Some(1));
    assert_eq!(options.file.encoding, Some(Encoding::Utf8));
    // nothing to do after the load
    let none = LaunchOptions::new(&crate::command_line::parse(&["-R".into()]), Path::new(""));
    assert!(none.after_load(true).is_none());
    // -MTYPE other than file is text
    let line = crate::command_line::parse(&["-M=x".into(), "-MTYPE=JS".into()]);
    let options = LaunchOptions::new(&line, Path::new("/w"));
    assert_eq!(
        options.after_load(true).unwrap().macro_,
        Some(MacroLaunch::Text {
            language: "JS".into(),
            code: "x".into()
        })
    );
    let line = crate::command_line::parse(&["-M=x".into(), "-MTYPE=File".into()]);
    assert!(matches!(
        LaunchOptions::new(&line, Path::new("")).macro_,
        Some(MacroLaunch::File(_))
    ));
}

#[gpui::test]
fn the_macro_runs_after_the_load(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc\n");
    // a key macro file that types "B"
    dir.write("run.mac", b"Char(66);\n");
    launch(cx, &dir, &["-M=run.mac", "a.txt"]);
    let view = view_of(cx, &path);
    let output = view.read_with(cx, |v, _| v.macro_output_text());
    assert_eq!(all_text(&view, cx), "Babc\n", "{output}");
}

/// `-TYPE=` is decided when the file is read, before the macro of オープン後: the macro sees the
/// type (`SetDocumentTypeWhenCreate`, `sLoadInfo.nType`). Here the macro turns the tabs of the text
/// into spaces by the tab width of the type: 8 for the type of `.txt`, 2 for `cpp`.
#[gpui::test]
fn the_macro_of_opening_sees_the_type_of_the_type_option(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"a\tb\n");
    dir.write("open.mac", b"SelectAll();\nTABToSPACE();\n");
    let ini = format!(
        "[Folders]\r\nszMACROFOLDER={}\r\n[Macro]\r\nName[000]=\r\nFile[000]=open.mac\r\n\
         nMacroOnOpened=0\r\n[Other]\r\nnTypesCount=17\r\n\
         [Types(1)]\r\nnInts=1,120,0,8,-1,-1,0,1,1,0,0,0\r\n\
         [Types(2)]\r\nnInts=2,10240,0,2,0,-1,0,1,1,0,0,0\r\n",
        dir.0.display()
    );
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(ini.as_bytes());
    let profile = matsu_config::Profile::from_bytes(&bytes);
    cx.update(|app| {
        app.set_global(crate::settings::UserSettings::from_profile(profile, None));
    });
    launch(cx, &dir, &["-TYPE=cpp", "a.txt"]);
    let view = view_of(cx, &path);
    let output = view.read_with(cx, |v, _| v.macro_output_text());
    // Width 2: "a", a tab to column 2 is one space. (With the type of the file it would be seven.)
    assert_eq!(all_text(&view, cx), "a b\n", "{output}");
}

#[gpui::test]
fn the_macro_text_of_mtype_is_run(cx: &mut TestAppContext) {
    init(cx);
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"abc\n");
    // `-M=` is the text of a key macro when `-MTYPE=` says `mac` (`CMDLINEOPT_MTYPE`)
    launch(cx, &dir, &["-M=Char(65);", "-MTYPE=mac", "a.txt"]);
    let view = view_of(cx, &path);
    assert_eq!(all_text(&view, cx), "Aabc\n");
}
