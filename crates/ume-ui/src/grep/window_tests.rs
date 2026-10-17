//! Tests of Grep in GPUI's headless test window: the dialog, the window of the result, and the
//! search that runs on a background task. The files are real files in a temporary folder.

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use ume_commands::command::Arg;
use ume_commands::sakura::FUNCTIONS;
use ume_core::buffer::{Document, Eol, Pos};
use ume_core::model::DocumentModel;

use super::settings::GrepSettings;
use crate::document::SharedDocument;
use crate::file_ops::windows::editors;
use crate::view::EditorView;

struct Fixture {
    view: Entity<EditorView>,
}

fn setup(cx: &mut TestAppContext) -> (Fixture, &mut VisualTestContext) {
    setup_with(cx, "")
}

fn setup_with<'a>(cx: &'a mut TestAppContext, text: &str) -> (Fixture, &'a mut VisualTestContext) {
    cx.update(gpui_component::init);
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let shared = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    // The search runs on another task and its pieces come to the window at any moment.
    vcx.executor().allow_parking();
    (Fixture { view }, vcx)
}

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

fn text_of(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> String {
    view.read_with(cx, |view, app| {
        let doc = view.document().read(app).doc();
        (0..doc.line_count())
            .map(|i| {
                let line = doc.line(i).unwrap().unwrap();
                format!(
                    "{}{}",
                    line.text,
                    line.eol.map_or("", ume_core::model::eol_text)
                )
            })
            .collect()
    })
}

impl Fixture {
    fn run(&self, cx: &mut VisualTestContext, function: &str, args: &[Arg]) {
        let code = code_of(function);
        self.view.update_in(cx, |view, window, cx| {
            view.run_function(code, args, window, cx)
        });
        settle(cx);
    }

    fn running(&self, cx: &mut VisualTestContext) -> bool {
        self.view.read_with(cx, |v, _| v.is_grep_running())
    }

    fn title(&self, cx: &mut VisualTestContext) -> String {
        self.view.read_with(cx, |v, app| v.title(app))
    }

    fn modified(&self, cx: &mut VisualTestContext) -> bool {
        self.view
            .read_with(cx, |v, app| v.document().read(app).model().is_modified())
    }

    fn panel(&self, cx: &mut VisualTestContext) -> Entity<super::panel::GrepPanel> {
        self.view
            .read_with(cx, |v, _| v.grep.panel.clone())
            .expect("the dialog is open")
    }

    fn panel_open(&self, cx: &mut VisualTestContext) -> bool {
        self.view.read_with(cx, |v, _| v.grep_panel_open())
    }
}

/// Lets the search and the window talk until nothing is left to do.
fn settle(cx: &mut VisualTestContext) {
    for _ in 0..50 {
        cx.run_until_parked();
    }
}

/// The editor windows other than `not`.
fn other_views(cx: &mut VisualTestContext, not: &Entity<EditorView>) -> Vec<Entity<EditorView>> {
    let all = cx.update(|_, app| editors(app));
    all.into_iter()
        .map(|(_, view)| view)
        .filter(|view| view != not)
        .collect()
}

struct Project {
    dir: tempfile::TempDir,
}

impl Project {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "alpha\nbeta hit\ngamma\n").unwrap();
        std::fs::write(dir.path().join("b.txt"), "no match\nhit again\n").unwrap();
        Self { dir }
    }

    fn root(&self) -> String {
        self.dir.path().to_string_lossy().into_owned()
    }

    fn settings(&self, key: &str) -> GrepSettings {
        GrepSettings {
            key: key.to_owned(),
            folder: self.root(),
            ..GrepSettings::default()
        }
    }

    fn file(&self, name: &str) -> String {
        self.dir.path().join(name).to_string_lossy().into_owned()
    }
}

fn answer(cx: &mut VisualTestContext, button: &str) {
    assert!(cx.has_pending_prompt(), "no prompt to answer {button:?}");
    cx.simulate_prompt_answer(button);
    cx.run_until_parked();
}

fn fill_and_ok(
    cx: &mut VisualTestContext,
    panel: &Entity<super::panel::GrepPanel>,
    key: &str,
    folder: &str,
) {
    panel.update_in(cx, |panel, window, cx| {
        panel.key.update(cx, |s, cx| s.set_value(key, window, cx));
        panel
            .folder
            .update(cx, |s, cx| s.set_value(folder, window, cx));
        panel.ok(window, cx);
    });
    settle(cx);
}

#[gpui::test]
fn the_macro_form_writes_the_result_to_a_new_window(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    f.run(cx, "F_GREP", &p.settings("hit").macro_args());
    let windows = other_views(cx, &f.view);
    assert_eq!(windows.len(), 1, "the result has its own window");
    let text = text_of(&windows[0], cx);
    assert!(text.starts_with("\r\n□検索条件  \"hit\"\r\n"), "{text}");
    assert!(
        text.ends_with(&format!(
            "{}(2,6)  [UTF-8]: beta hit\r\n{}(2,1)  [UTF-8]: hit again\r\n2 個が検索されました。\r\n",
            p.file("a.txt"),
            p.file("b.txt")
        )),
        "{text}"
    );
    // the window you ran it in is as it was
    assert_eq!(text_of(&f.view, cx), "");
    assert!(!f.view.read_with(cx, |v, _| v.is_grep_window()));
}

#[gpui::test]
fn a_grep_window_is_titled_with_the_key_and_is_not_modified(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    f.run(cx, "F_GREP", &p.settings("hit").macro_args());
    let result = other_views(cx, &f.view).remove(0);
    let (title, grep, running, modified, caret, accepts, eol) = result.read_with(cx, |v, app| {
        (
            v.title(app),
            v.is_grep_window(),
            v.is_grep_running(),
            v.document().read(app).model().is_modified(),
            v.cursor().caret(),
            v.accepts_load(app),
            v.document().read(app).new_line(),
        )
    });
    assert_eq!(title, "【Grep】hit - UME Editor");
    assert!(grep && !running && !modified && !accepts);
    assert_eq!(caret, Pos::new(0, 0));
    // the line ending of the result is CR LF
    assert_eq!(eol, Eol::CrLf);
}

#[gpui::test]
fn the_title_cuts_a_long_key(_cx: &mut TestAppContext) {
    assert_eq!(super::window::title_key("short"), "short");
    let long = "あ".repeat(40);
    assert_eq!(
        super::window::title_key(&long),
        format!("{}...", "あ".repeat(32))
    );
    assert_eq!(super::window::title_key(&"a".repeat(32)), "a".repeat(32));
}

#[gpui::test]
fn the_dialog_starts_with_the_word_at_the_caret_and_the_defaults(cx: &mut TestAppContext) {
    let (f, cx) = setup_with(cx, "needle here\n");
    f.run(cx, "F_GREP_DIALOG", &[]);
    assert!(f.panel_open(cx));
    let panel = f.panel(cx);
    let (key, file, exclude_file, exclude_folder) = panel.read_with(cx, |panel, app| {
        (
            panel.key.read(app).value().to_string(),
            panel.file.read(app).value().to_string(),
            panel.exclude_file.read(app).value().to_string(),
            panel.exclude_folder.read(app).value().to_string(),
        )
    });
    assert_eq!(key, "needle");
    assert_eq!(file, "*.*");
    assert!(exclude_file.starts_with("*.msi;*.exe"));
    assert_eq!(exclude_folder, ".git;.svn;.vs");
}

#[gpui::test]
fn the_dialog_runs_the_search_in_an_untitled_window(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    f.run(cx, "F_GREP_DIALOG", &[]);
    let panel = f.panel(cx);
    fill_and_ok(cx, &panel, "hit", &p.root());
    assert!(!f.panel_open(cx));
    // this window was untitled and unchanged: the result is here, no new window
    assert!(other_views(cx, &f.view).is_empty());
    let text = text_of(&f.view, cx);
    assert!(text.contains("beta hit"), "{text}");
    assert_eq!(f.title(cx), "【Grep】hit - UME Editor");
    assert!(!f.modified(cx));
    assert!(!f.running(cx));
}

#[gpui::test]
fn a_second_grep_goes_to_the_same_grep_window_below_the_first(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    f.view.update_in(cx, |view, window, cx| {
        view.start_grep(p.settings("hit").info(), false, window, cx);
    });
    settle(cx);
    let first = text_of(&f.view, cx);
    assert!(first.contains("2 個が検索されました。"));
    f.view.update_in(cx, |view, window, cx| {
        view.start_grep(p.settings("alpha").info(), false, window, cx);
    });
    settle(cx);
    let both = text_of(&f.view, cx);
    assert!(both.starts_with(&first), "{both}");
    assert!(both.contains("□検索条件  \"alpha\""));
    assert!(both.ends_with("1 個が検索されました。\r\n"));
    assert!(other_views(cx, &f.view).is_empty(), "no new window");
    assert_eq!(f.title(cx), "【Grep】alpha - UME Editor");
    // the caret is where the second result begins
    let caret = f.view.read_with(cx, |v, _| v.cursor().caret());
    assert_eq!(caret.col, 0);
    assert!(caret.line > 5);
}

#[gpui::test]
fn a_window_with_changes_gets_a_new_window_for_the_result(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    cx.simulate_input("my notes");
    assert!(f.modified(cx));
    f.view.update_in(cx, |view, window, cx| {
        view.start_grep(p.settings("hit").info(), false, window, cx);
    });
    settle(cx);
    assert_eq!(other_views(cx, &f.view).len(), 1);
    // and the notes were not touched
    assert_eq!(text_of(&f.view, cx), "my notes");
}

#[gpui::test]
fn a_wrong_condition_is_told_in_a_box_and_nothing_starts(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    f.run(cx, "F_GREP_DIALOG", &[]);
    let panel = f.panel(cx);
    // no folder
    fill_and_ok(cx, &panel, "x", "");
    answer(cx, "OK");
    assert!(f.panel_open(cx));
    assert!(!f.view.read_with(cx, |v, _| v.is_grep_window()));
    // a folder that is not there
    fill_and_ok(cx, &panel, "x", "/no/such/folder/here");
    answer(cx, "OK");
    // an excluded pattern that is wrong
    let p = Project::new();
    panel.update(cx, |panel, _| {
        panel.settings.except_file_regex = true;
    });
    panel.update_in(cx, |panel, window, cx| {
        panel
            .exclude_file
            .update(cx, |s, cx| s.set_value("(", window, cx));
    });
    fill_and_ok(cx, &panel, "x", &p.root());
    let (_, detail) = cx.pending_prompt().unwrap();
    assert!(
        detail.contains("除外ファイルの正規表現が正しくありません"),
        "{detail}"
    );
    answer(cx, "OK");
    assert!(f.panel_open(cx), "the dialog stays open for the fix");
    panel.update(cx, |panel, _| {
        panel.settings.except_file_regex = false;
    });
    // a regular expression that does not compile
    let p = Project::new();
    panel.update(cx, |panel, _| panel.settings.options.regex = true);
    fill_and_ok(cx, &panel, "(", &p.root());
    let (title, detail) = cx.pending_prompt().unwrap();
    assert_eq!(title, "正規表現エラー");
    assert!(
        detail.contains("正規表現の構文に誤りがあります"),
        "{detail}"
    );
    answer(cx, "OK");
    assert!(f.panel_open(cx));
    assert_eq!(text_of(&f.view, cx), "");
}

#[gpui::test]
fn the_macro_form_with_a_wrong_folder_is_told_here_and_opens_no_window(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let s = GrepSettings {
        key: "x".into(),
        folder: "/no/such/folder/here".into(),
        ..GrepSettings::default()
    };
    f.run(cx, "F_GREP", &s.macro_args());
    answer(cx, "OK");
    assert!(other_views(cx, &f.view).is_empty());
    // and a macro without a folder is an error of the macro
    f.run(
        cx,
        "F_GREP",
        &[Arg::Text("x".into()), Arg::Text("*".into())],
    );
    let notice = f.view.read_with(cx, |v, _| v.notice.clone());
    assert!(notice.unwrap().contains("検索先フォルダー"));
}

#[gpui::test]
fn the_search_can_be_stopped_from_the_window(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    f.view.update_in(cx, |view, window, cx| {
        view.start_grep(p.settings("hit").info(), false, window, cx);
        // stopped before the search got going
        view.stop_grep(cx);
    });
    assert!(f.running(cx));
    settle(cx);
    assert!(!f.running(cx));
    let text = text_of(&f.view, cx);
    assert!(
        text.ends_with("中断しました。\r\n0 個が検索されました。\r\n"),
        "{text}"
    );
    assert!(!f.modified(cx));
}

#[gpui::test]
fn a_grep_window_cannot_be_closed_while_it_searches_and_never_asks_to_save(
    cx: &mut TestAppContext,
) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    // A search that does not end: closing the file does nothing (`OnBeforeClose`).
    f.view.update(cx, |view, cx| view.pretend_grep_running(cx));
    assert!(f.running(cx));
    f.run(cx, "F_FILECLOSE", &[]);
    assert!(!cx.has_pending_prompt());
    let notice = f.view.read_with(cx, |v, _| v.notice.clone());
    assert_eq!(notice.as_deref(), Some("Grepの処理中です。"));
    f.view.update(cx, |view, cx| view.stop_pretending_grep(cx));
    // a real search, and the same
    f.view.update_in(cx, |view, window, cx| {
        view.start_grep(p.settings("hit").info(), false, window, cx);
    });
    settle(cx);
    assert!(!f.running(cx));
    // edited after the search: the file is closed without a question
    cx.simulate_input("memo");
    assert!(f.modified(cx));
    f.run(cx, "F_FILECLOSE", &[]);
    assert!(!cx.has_pending_prompt(), "no question about saving");
    assert_eq!(text_of(&f.view, cx), "");
    // the window is an ordinary one again: its notes are asked about
    assert!(!f.view.read_with(cx, |v, _| v.is_grep_window()));
    assert_eq!(f.title(cx), "(無題) - UME Editor");
    cx.simulate_input("notes");
    f.run(cx, "F_FILECLOSE", &[]);
    assert!(cx.has_pending_prompt(), "the notes are asked about");
}

#[gpui::test]
fn a_grep_window_asks_before_it_closes_when_the_setting_says_so(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    f.view.update_in(cx, |view, window, cx| {
        view.start_grep(p.settings("hit").info(), false, window, cx);
    });
    settle(cx);
    // 「Grepモードで保存確認する」 (`m_bGrepExitConfirm`): the question is asked also when nothing
    // was edited after the search (`CEditDoc::OnFileClose`), with the title of the Grep window.
    cx.update(|_, app| {
        app.set_global(crate::search::prefs::SearchPrefs {
            grep_exit_confirm: true,
            ..Default::default()
        })
    });
    assert!(!f.modified(cx));
    f.run(cx, "F_FILECLOSE", &[]);
    assert!(cx.has_pending_prompt(), "the question is asked");
    let (message, _) = cx.pending_prompt().unwrap();
    let title = f.view.read_with(cx, |v, _| v.grep_title().unwrap());
    assert!(message.contains(&title), "{message}");
    cx.simulate_prompt_answer("キャンセル");
    cx.run_until_parked();
    assert!(
        f.view.read_with(cx, |v, _| v.is_grep_window()),
        "still open"
    );
}

#[gpui::test]
fn the_dialog_remembers_the_values_for_the_next_time(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    f.run(cx, "F_GREP_DIALOG", &[]);
    let panel = f.panel(cx);
    panel.update(cx, |panel, _| {
        panel.settings.sub_folder = false;
        panel.settings.options.match_case = true;
    });
    fill_and_ok(cx, &panel, "hit", &p.root());
    // a window of its own: the dialog starts with them
    let (g, cx) = setup(cx);
    g.run(cx, "F_GREP_DIALOG", &[]);
    let panel = g.panel(cx);
    let (key, folder, sub, case) = panel.read_with(cx, |panel, app| {
        (
            panel.key.read(app).value().to_string(),
            panel.folder.read(app).value().to_string(),
            panel.settings.sub_folder,
            panel.settings.options.match_case,
        )
    });
    assert_eq!(key, "hit");
    assert_eq!(folder, p.root());
    assert!(!sub && case);
    // cancel closes it
    panel.update_in(cx, |panel, window, cx| panel.cancel(window, cx));
    assert!(!g.panel_open(cx));
}

#[gpui::test]
fn f_grep_without_arguments_uses_what_the_dialog_left(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    f.run(cx, "F_GREP_DIALOG", &[]);
    let panel = f.panel(cx);
    fill_and_ok(cx, &panel, "alpha", &p.root());
    let first = text_of(&f.view, cx);
    assert!(first.contains("□検索条件  \"alpha\""));
    // the same Grep again, in the Grep window
    f.run(cx, "F_GREP", &[]);
    let again = text_of(&f.view, cx);
    assert!(again.starts_with(&first));
    assert!(again.len() > first.len());
}

#[gpui::test]
fn a_grep_is_recorded_in_a_key_macro_with_its_arguments(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rec.mac");
    let (f, cx) = setup(cx);
    let p = Project::new();
    f.view
        .update(cx, |view, cx| view.toggle_key_macro_recording(cx));
    f.run(cx, "F_GREP_DIALOG", &[]);
    let panel = f.panel(cx);
    fill_and_ok(cx, &panel, "hit", &p.root());
    f.view
        .update(cx, |view, cx| view.toggle_key_macro_recording(cx));
    f.view
        .read_with(cx, |view, _| view.save_key_macro_to(&path))
        .unwrap();
    let saved = String::from_utf8_lossy(&std::fs::read(&path).unwrap()).into_owned();
    let flags = 0x01 | 0x10 | 0x20 | (99 << 8);
    let files =
        "*.*;#.git;#.svn;#.vs;!*.msi;!*.exe;!*.obj;!*.pdb;!*.ilk;!*.res;!*.pch;!*.iobj;!*.ipdb";
    assert!(
        saved.contains(&format!(
            "Grep('hit', '{files}', '{}', {flags}, 99);",
            // the macro file escapes a backslash
            p.root().replace('\\', "\\\\")
        )),
        "{saved}"
    );
}

#[gpui::test]
fn a_window_in_view_mode_gets_a_new_window_for_the_result(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    f.view.update(cx, |view, cx| {
        view.document()
            .clone()
            .update(cx, |shared, cx| shared.set_view_mode(true, cx));
    });
    f.view.update_in(cx, |view, window, cx| {
        view.start_grep(p.settings("hit").info(), false, window, cx);
    });
    settle(cx);
    assert_eq!(other_views(cx, &f.view).len(), 1);
    assert_eq!(text_of(&f.view, cx), "");
}

#[gpui::test]
fn only_one_of_the_grep_and_the_search_dialogs_is_open(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    f.run(cx, "F_GREP_DIALOG", &[]);
    assert!(f.panel_open(cx));
    f.run(cx, "F_SEARCH_DIALOG", &[]);
    assert!(!f.panel_open(cx), "the search dialog replaced it");
    assert!(
        f.view
            .read_with(cx, |v, app| v.search_panel_mode(app).is_some())
    );
    f.run(cx, "F_GREP_DIALOG", &[]);
    assert!(f.panel_open(cx));
    assert!(
        f.view
            .read_with(cx, |v, app| v.search_panel_mode(app).is_none())
    );
}

fn replace_args(p: &Project, key: &str, new: &str, flags: i64) -> Vec<Arg> {
    vec![
        Arg::Text(key.to_owned()),
        Arg::Text(new.to_owned()),
        Arg::Text("*.txt".to_owned()),
        Arg::Text(p.root()),
        Arg::Int(flags),
        Arg::Int(4),
    ]
}

fn read(p: &Project, name: &str) -> String {
    std::fs::read_to_string(p.file(name)).unwrap()
}

#[gpui::test]
fn the_replace_macro_changes_the_files_and_writes_the_result_to_a_new_window(
    cx: &mut TestAppContext,
) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    // the subfolders and a backup
    f.run(
        cx,
        "F_GREP_REPLACE",
        &replace_args(&p, "hit", "HIT", 0x01 | 0x20 | 0x20_0000),
    );
    assert_eq!(read(&p, "a.txt"), "alpha\nbeta HIT\ngamma\n");
    assert_eq!(read(&p, "b.txt"), "no match\nHIT again\n");
    assert_eq!(read(&p, "a.txt.skrold"), "alpha\nbeta hit\ngamma\n");
    let windows = other_views(cx, &f.view);
    assert_eq!(windows.len(), 1, "the result has its own window");
    let text = text_of(&windows[0], cx);
    assert!(
        text.starts_with("\r\n□検索条件  \"hit\"\r\n置換後         \"HIT\"\r\n"),
        "{text}"
    );
    assert!(
        text.ends_with(&format!(
            "{}(2,6): beta hit\r\n{}(2,1): hit again\r\n2 個を置換しました。\r\n",
            p.file("a.txt"),
            p.file("b.txt")
        )),
        "{text}"
    );
    assert_eq!(
        text_of(&f.view, cx),
        "",
        "the window it ran in is as it was"
    );
    let title = windows[0].read_with(cx, |v, app| v.title(app));
    assert_eq!(title, "【Grep】hit - UME Editor");
}

#[gpui::test]
fn the_replace_macro_without_a_backup_keeps_none(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    f.run(cx, "F_GREP_REPLACE", &replace_args(&p, "hit", "", 0x01));
    assert_eq!(read(&p, "a.txt"), "alpha\nbeta \ngamma\n");
    assert!(!std::path::Path::new(&p.file("a.txt.skrold")).exists());
}

#[gpui::test]
fn the_replace_macro_needs_a_pattern_and_a_new_text(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    for args in [
        replace_args(&p, "", "x", 1),
        replace_args(&p, "hit", "x", 1)[..1].to_vec(),
    ] {
        f.run(cx, "F_GREP_REPLACE", &args);
        // no window, no change
        assert!(other_views(cx, &f.view).is_empty());
        assert_eq!(read(&p, "a.txt"), "alpha\nbeta hit\ngamma\n");
    }
}

#[gpui::test]
fn the_text_of_the_clipboard_replaces_with_the_paste_flag(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("$1 \\n".to_owned()));
    // a regular expression, but the clipboard is put in as it is
    let flags = 0x01 | 0x08 | 0x10_0000;
    f.run(
        cx,
        "F_GREP_REPLACE",
        &replace_args(&p, "h(i)t", "ignored", flags),
    );
    assert_eq!(read(&p, "a.txt"), "alpha\nbeta $1 \\n\ngamma\n");
    let text = text_of(&other_views(cx, &f.view)[0], cx);
    assert!(
        text.contains("置換後         (クリップボードから貼り付け)\r\n検索対象"),
        "{text}"
    );
}

#[gpui::test]
fn an_empty_clipboard_stops_a_paste_with_a_box_and_changes_nothing(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    cx.write_to_clipboard(gpui::ClipboardItem::new_string(String::new()));
    f.run(
        cx,
        "F_GREP_REPLACE",
        &replace_args(&p, "hit", "x", 0x01 | 0x10_0000),
    );
    let (_, detail) = cx.pending_prompt().expect("a box");
    assert_eq!(detail, "クリップボードに有効なデータがありません！");
    answer(cx, "OK");
    assert!(other_views(cx, &f.view).is_empty());
    assert_eq!(read(&p, "a.txt"), "alpha\nbeta hit\ngamma\n");
}

#[gpui::test]
fn the_replace_dialog_has_the_new_text_and_replaces(cx: &mut TestAppContext) {
    let (f, cx) = setup_with(cx, "hit here\n");
    let p = Project::new();
    f.run(cx, "F_GREP_REPLACE_DLG", &[]);
    assert!(f.panel_open(cx));
    let panel = f.panel(cx);
    let (mode, key, backup) = panel.read_with(cx, |panel, app| {
        (
            panel.replace_mode,
            panel.key.read(app).value().to_string(),
            panel.settings.backup,
        )
    });
    assert!(mode);
    assert_eq!(key, "hit", "the word at the caret");
    assert!(backup, "the backup is on at first");
    panel.update_in(cx, |panel, window, cx| {
        panel
            .replacement
            .update(cx, |s, cx| s.set_value("日本語", window, cx));
    });
    fill_and_ok(cx, &panel, "hit", &p.root());
    assert!(!f.panel_open(cx));
    assert_eq!(read(&p, "a.txt"), "alpha\nbeta 日本語\ngamma\n");
    assert_eq!(read(&p, "a.txt.skrold"), "alpha\nbeta hit\ngamma\n");
    // this window has no name and no changes: it takes the result
    let windows = other_views(cx, &f.view);
    let text = text_of(windows.first().unwrap_or(&f.view), cx);
    assert!(text.contains("2 個を置換しました。"), "{text}");
    // the dialog starts again with the values it had
    f.run(cx, "F_GREP_REPLACE_DLG", &[]);
    let panel = f.panel(cx);
    let (replacement, paste) = panel.read_with(cx, |panel, app| {
        (
            panel.replacement.read(app).value().to_string(),
            panel.settings.paste,
        )
    });
    assert_eq!(replacement, "日本語");
    assert!(!paste);
}

#[gpui::test]
fn the_replace_dialog_wants_a_condition(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    f.run(cx, "F_GREP_REPLACE_DLG", &[]);
    let panel = f.panel(cx);
    fill_and_ok(cx, &panel, "", &p.root());
    let (_, detail) = cx.pending_prompt().expect("a box");
    assert_eq!(detail, "置換条件を指定してください。");
    answer(cx, "OK");
    assert!(f.panel_open(cx));
    assert_eq!(read(&p, "a.txt"), "alpha\nbeta hit\ngamma\n");
}

#[gpui::test]
fn the_replace_dialog_unchecks_a_paste_without_a_clipboard(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let p = Project::new();
    cx.write_to_clipboard(gpui::ClipboardItem::new_string(String::new()));
    f.run(cx, "F_GREP_REPLACE_DLG", &[]);
    let panel = f.panel(cx);
    panel.update(cx, |panel, _| panel.settings.paste = true);
    fill_and_ok(cx, &panel, "hit", &p.root());
    let (_, detail) = cx.pending_prompt().expect("a box");
    assert_eq!(detail, "クリップボードに有効なデータがありません！");
    answer(cx, "OK");
    assert!(f.panel_open(cx), "it stays open");
    assert!(!panel.read_with(cx, |panel, _| panel.settings.paste));
    assert_eq!(read(&p, "a.txt"), "alpha\nbeta hit\ngamma\n");
}

#[gpui::test]
fn the_two_dialogs_are_one_panel_in_two_forms(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    f.run(cx, "F_GREP_REPLACE_DLG", &[]);
    assert!(f.panel(cx).read_with(cx, |panel, _| panel.replace_mode));
    f.run(cx, "F_GREP_DIALOG", &[]);
    assert!(!f.panel(cx).read_with(cx, |panel, _| panel.replace_mode));
    f.run(cx, "F_GREP_REPLACE_DLG", &[]);
    assert!(f.panel(cx).read_with(cx, |panel, _| panel.replace_mode));
}

#[gpui::test]
fn a_replacement_is_not_recorded_in_a_key_macro(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rec.mac");
    let (f, cx) = setup(cx);
    let p = Project::new();
    f.view
        .update(cx, |view, cx| view.toggle_key_macro_recording(cx));
    f.run(cx, "F_GREP_REPLACE_DLG", &[]);
    let panel = f.panel(cx);
    fill_and_ok(cx, &panel, "hit", &p.root());
    f.view
        .update(cx, |view, cx| view.toggle_key_macro_recording(cx));
    f.view
        .read_with(cx, |view, _| view.save_key_macro_to(&path))
        .unwrap();
    let saved = String::from_utf8_lossy(&std::fs::read(&path).unwrap()).into_owned();
    // `GrepReplace` is not a function of key macros (`MacroFunction::key_macro`)
    assert!(!saved.contains("Grep"), "{saved}");
}
