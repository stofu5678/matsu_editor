//! Tests of where the dialogs start, in GPUI's headless test windows. The dialog of the OS is
//! not shown there; what would have been asked of it is recorded by [`record`], and GPUI's
//! dialog (the fallback) is answered by the test.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;
use matsu_config::Profile;
use matsu_platform::dialog::{Chosen, Extra, Kind, RecentKind, Request};
use tempfile::TempDir;

use crate::document::SharedDocument;
use crate::file_ops::prefs::FilePrefs;
use crate::history::{HistoryOptions, HistoryState};
use crate::menu::{Entry, MenuAction};
use crate::settings::UserSettings;
use crate::view::{EditorView, OpenRecentFolder};

thread_local! {
    static ASKED: RefCell<Vec<Request>> = const { RefCell::new(Vec::new()) };
}

/// Called by [`super::ask`] with what the dialog of the OS would be asked.
pub(super) fn record(request: &Request) {
    ASKED.with(|asked| asked.borrow_mut().push(request.clone()));
}

type Prepared = Box<dyn Fn(&Request) -> Option<Option<Chosen>>>;

thread_local! {
    static PREPARED: RefCell<Option<Prepared>> = const { RefCell::new(None) };
}

/// Makes the dialog of the OS answer in this test thread: `answer` gets what is asked and gives
/// the answer (`Some(None)` is a cancel), or `None` to leave the dialog unavailable. Until
/// [`forget_answer`].
pub(crate) fn answer_with(answer: impl Fn(&Request) -> Option<Option<Chosen>> + 'static) {
    PREPARED.with(|prepared| *prepared.borrow_mut() = Some(Box::new(answer)));
}

pub(crate) fn forget_answer() {
    PREPARED.with(|prepared| *prepared.borrow_mut() = None);
}

/// Called by [`super::native`].
pub(super) fn prepared_answer(request: &Request) -> Option<Option<Chosen>> {
    PREPARED.with(|prepared| {
        prepared
            .borrow()
            .as_ref()
            .and_then(|answer| answer(request))
    })
}

/// The dialogs asked for since the last call.
pub(crate) fn asked() -> Vec<Request> {
    ASKED.with(|asked| std::mem::take(&mut *asked.borrow_mut()))
}

/// `[Common]` with the setting of the start folder.
fn settings(mode: i32, selected: &Path) -> UserSettings {
    let text = format!(
        "[Common]\neOpenDialogDir={mode}\nszOpenDialogSelDir={}\n",
        selected.display()
    );
    UserSettings::from_profile(Profile::from_bytes(text.as_bytes()), None)
}

struct Fixture {
    view: Entity<EditorView>,
}

fn setup(cx: &mut TestAppContext, user: UserSettings) -> (Fixture, &mut VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        cx.set_global(user);
        cx.set_global(HistoryState::in_memory(HistoryOptions::default()));
    });
    asked();
    let shared = cx.new(|_| SharedDocument::empty());
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    (Fixture { view }, vcx)
}

impl Fixture {
    fn run(&self, cx: &mut VisualTestContext, function: &str, args: &[Arg]) {
        let code = FUNCTIONS.iter().find(|f| f.name == function).unwrap().code;
        self.view.update_in(cx, |view, window, cx| {
            view.run_function(code, args, window, cx)
        });
        cx.run_until_parked();
    }

    /// Opens `path` in this window and leaves the window with a file.
    fn open(&self, cx: &mut VisualTestContext, path: &Path) {
        self.run(cx, "F_FILEOPEN", &[Arg::Text(path.display().to_string())]);
        assert!(
            self.view
                .read_with(cx, |view, app| view.document().read(app).path().is_some())
        );
        asked();
    }
}

fn only(mut requests: Vec<Request>) -> Request {
    assert_eq!(requests.len(), 1, "{requests:?}");
    requests.pop().unwrap()
}

#[gpui::test]
fn current_starts_where_the_process_is(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let (f, cx) = setup(cx, settings(0, dir.path()));
    f.run(cx, "F_FILEOPEN", &[]);
    let request = only(asked());
    assert_eq!(request.kind, Kind::OpenFile { multiple: true });
    assert_eq!(request.start, std::env::current_dir().ok());
    // GPUI's dialog is the fallback, and answers
    assert!(cx.did_prompt_for_paths());
}

#[gpui::test]
fn selected_starts_in_the_given_folder(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let (f, cx) = setup(cx, settings(2, dir.path()));
    f.run(cx, "F_FILEOPEN", &[]);
    assert_eq!(only(asked()).start.as_deref(), Some(dir.path()));
}

#[gpui::test]
fn selected_folder_that_is_not_there_leaves_the_choice_to_the_os(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let (f, cx) = setup(cx, settings(2, &dir.path().join("gone")));
    f.run(cx, "F_FILEOPEN", &[]);
    assert_eq!(only(asked()).start, None);
}

#[gpui::test]
fn recent_starts_in_the_newest_recent_folder_that_exists(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();
    let (f, cx) = setup(cx, settings(1, dir.path()));
    cx.update(|_, app| {
        crate::history::update(app, |history| {
            history.add_folder(&other.path().to_string_lossy());
            history.add_folder(&dir.path().join("gone").to_string_lossy());
        });
    });
    f.run(cx, "F_FILEOPEN", &[]);
    // the newest folder is gone; the next one is the start
    assert_eq!(only(asked()).start.as_deref(), Some(other.path()));
}

#[gpui::test]
fn a_document_with_a_file_starts_in_the_folder_of_the_file(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let selected = TempDir::new().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "abc").unwrap();
    let (f, cx) = setup(cx, settings(2, selected.path()));
    f.open(cx, &file);
    f.run(cx, "F_FILEOPEN", &[]);
    assert_eq!(only(asked()).start.as_deref(), Some(dir.path()));
}

#[gpui::test]
fn save_as_starts_in_the_folder_of_the_file_and_offers_its_name(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("メモ.txt");
    std::fs::write(&file, "abc").unwrap();
    let (f, cx) = setup(cx, settings(0, dir.path()));
    f.open(cx, &file);
    f.run(cx, "F_FILESAVEAS_DIALOG", &[]);
    let request = only(asked());
    assert_eq!(request.kind, Kind::SaveFile);
    assert_eq!(request.start.as_deref(), Some(dir.path()));
    assert_eq!(request.name.as_deref(), Some("メモ.txt"));
    // GPUI's save dialog got the folder too
    assert!(cx.did_prompt_for_new_path());
}

#[gpui::test]
fn save_of_a_document_without_a_file_starts_where_the_setting_says(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let (f, cx) = setup(cx, settings(2, dir.path()));
    cx.simulate_input("abc");
    f.run(cx, "F_FILESAVE", &[]);
    let request = only(asked());
    assert_eq!(request.kind, Kind::SaveFile);
    assert_eq!(request.start.as_deref(), Some(dir.path()));
    // Sakura suggests 無題 with the date and the time, and the dialog adds .txt
    let name = request.name.unwrap();
    let stamp = name.strip_prefix("無題_").expect(&name);
    assert_eq!(stamp.len(), "20261019_120000".len(), "{name}");
    assert!(
        stamp.chars().all(|c| c.is_ascii_digit() || c == '_'),
        "{name}"
    );
    assert_eq!(request.default_extension.as_deref(), Some("txt"));
}

/// The patterns of the entry called `name` in the file types of the request.
fn patterns(request: &Request, name: &str) -> Vec<String> {
    request
        .filters
        .iter()
        .find(|filter| filter.name == name)
        .unwrap_or_else(|| panic!("{name} in {:?}", request.filters))
        .patterns
        .clone()
}

fn set_filter_options(cx: &mut VisualTestContext, new: bool, file: bool) {
    cx.update(|_, app| {
        app.set_global(FilePrefs {
            no_filter_save_new: new,
            no_filter_save_file: file,
            ..FilePrefs::default()
        });
    });
}

#[gpui::test]
fn the_save_dialog_of_a_new_document_has_sakuras_three_file_types(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let (f, cx) = setup(cx, settings(2, dir.path()));
    cx.simulate_input("abc");
    f.run(cx, "F_FILESAVE", &[]);
    let request = only(asked());
    let names: Vec<_> = request.filters.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        ["ユーザー指定", "テキストファイル", "すべてのファイル"]
    );
    // the options are on by default: ;*.* after *.txt
    assert_eq!(patterns(&request, "ユーザー指定"), ["*.txt", "*.*"]);
    assert_eq!(patterns(&request, "テキストファイル"), ["*.txt"]);
    assert_eq!(patterns(&request, "すべてのファイル"), ["*.*"]);
    assert_eq!(request.selected_filter, 0);
}

#[gpui::test]
fn the_option_for_a_new_document_decides_whether_all_files_follow(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let (f, cx) = setup(cx, settings(2, dir.path()));
    cx.simulate_input("abc");
    // off for new documents, on for the others
    set_filter_options(cx, false, true);
    f.run(cx, "F_FILESAVE", &[]);
    assert_eq!(patterns(&only(asked()), "ユーザー指定"), ["*.txt"]);
}

#[gpui::test]
fn the_save_dialog_of_a_file_lists_the_extensions_of_its_type(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("メモ.txt");
    std::fs::write(&file, "abc").unwrap();
    let (f, cx) = setup(cx, settings(0, dir.path()));
    f.open(cx, &file);
    // the type テキスト lists txt, log, 1st, err and ps; the extension of the file goes first
    f.run(cx, "F_FILESAVEAS_DIALOG", &[]);
    let request = only(asked());
    assert_eq!(
        patterns(&request, "ユーザー指定"),
        ["*.txt", "*.log", "*.1st", "*.err", "*.ps", "*.*"]
    );
    // Sakura gives every save dialog the default extension
    assert_eq!(request.default_extension.as_deref(), Some("txt"));
    // off for the documents that have a file
    set_filter_options(cx, true, false);
    f.run(cx, "F_FILESAVEAS_DIALOG", &[]);
    assert_eq!(
        patterns(&only(asked()), "ユーザー指定"),
        ["*.txt", "*.log", "*.1st", "*.err", "*.ps"]
    );
}

#[gpui::test]
fn the_file_of_another_extension_of_the_basic_type_lists_that_extension(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("memo.zzz");
    std::fs::write(&file, "abc").unwrap();
    let (f, cx) = setup(cx, settings(0, dir.path()));
    f.open(cx, &file);
    set_filter_options(cx, true, false);
    f.run(cx, "F_FILESAVEAS_DIALOG", &[]);
    assert_eq!(patterns(&only(asked()), "ユーザー指定"), ["*.zzz"]);
}

#[gpui::test]
fn the_dialogs_that_are_not_save_dialogs_have_no_file_types(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let (f, cx) = setup(cx, settings(2, dir.path()));
    f.run(cx, "F_FILEOPEN", &[]);
    let request = only(asked());
    assert!(request.filters.is_empty());
    assert_eq!(request.default_extension, None);
}

#[gpui::test]
fn the_fallback_dialog_gets_the_default_extension(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("memo");
    let (f, cx) = setup(cx, settings(2, dir.path()));
    cx.simulate_input("abc");
    f.run(cx, "F_FILESAVE", &[]);
    let chosen = target.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    cx.run_until_parked();
    // the options dialog of the Save As: Enter is OK
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(std::fs::read(dir.path().join("memo.txt")).unwrap(), b"abc");
    assert!(!target.exists());
}

#[gpui::test]
fn the_fallback_dialog_adds_the_extension_of_the_type_of_a_named_document(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("a.c");
    std::fs::write(&file, "abc").unwrap();
    let (f, cx) = setup(cx, settings(0, dir.path()));
    f.open(cx, &file);
    f.run(cx, "F_FILESAVEAS_DIALOG", &[]);
    let chosen = dir.path().join("bar");
    cx.simulate_new_path_selection(move |_| Some(chosen));
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(std::fs::read(dir.path().join("bar.c")).unwrap(), b"abc");
}

#[gpui::test]
fn the_recent_folders_are_in_the_menu_and_open_the_dialog_there(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let chosen = dir.path().join("b.txt");
    std::fs::write(&chosen, "chosen").unwrap();
    let (f, cx) = setup(cx, settings(0, dir.path()));
    cx.update(|_, app| {
        crate::history::update(app, |history| {
            history.add_folder("/elsewhere/メモ");
            history.add_folder(&dir.path().to_string_lossy());
        });
    });
    // the menu: one item for each folder, newest first, with Sakura's access keys
    let lists =
        crate::history::menu::RecentLists::new(cx.update(|_, app| crate::history::current(app)));
    let menus = crate::menu::main_menu_with(
        &matsu_commands::keymap::KeyMap::default(),
        &|_: u16| true,
        &lists,
    );
    let items: Vec<_> = menus
        .iter()
        .flat_map(|menu| flatten(&menu.entries))
        .filter_map(|entry| match &entry.action {
            MenuAction::RecentFolder(path) => Some((entry.label.clone(), path.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        items,
        [
            (
                format!("1 {}", dir.path().display()),
                dir.path().to_string_lossy().into_owned()
            ),
            ("2 /elsewhere/メモ".to_owned(), "/elsewhere/メモ".to_owned()),
        ]
    );
    // choosing one shows the open dialog in that folder
    cx.dispatch_action(OpenRecentFolder {
        path: dir.path().to_string_lossy().into_owned(),
    });
    cx.run_until_parked();
    let request = only(asked());
    assert_eq!(request.kind, Kind::OpenFile { multiple: true });
    assert_eq!(request.start.as_deref(), Some(dir.path()));
    // and the file chosen is opened
    let file = chosen.clone();
    cx.simulate_path_prompt_response(move |_| Some(vec![file]));
    cx.run_until_parked();
    let text = f.view.read_with(cx, |view, app| {
        view.document()
            .read(app)
            .doc()
            .line(0)
            .unwrap()
            .unwrap()
            .text
    });
    assert_eq!(text, "chosen");
}

fn flatten(entries: &[Entry]) -> Vec<crate::menu::Item> {
    let mut out = Vec::new();
    for entry in entries {
        match entry {
            Entry::Item(item) => out.push(item.clone()),
            Entry::Submenu { entries, .. } => out.extend(flatten(entries)),
            Entry::Separator => {}
        }
    }
    out
}

#[gpui::test]
fn a_recent_folder_that_is_gone_still_shows_the_dialog(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let (f, cx) = setup(cx, settings(0, dir.path()));
    f.view.update_in(cx, |view, window, cx| {
        view.open_recent_folder(
            dir.path().join("gone").to_string_lossy().into_owned(),
            window,
            cx,
        );
    });
    cx.run_until_parked();
    // the OS is not told a start that is not there
    assert_eq!(only(asked()).start, None);
}

#[test]
fn the_start_that_is_not_a_folder_is_dropped_from_the_request() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "").unwrap();
    let ask = super::Ask::open(false).start(Some(file));
    assert_eq!(ask.request().start, None);
    let ask = super::Ask::save()
        .start(Some(PathBuf::from(dir.path())))
        .name(Some("x.txt".to_owned()));
    let request = ask.request();
    assert_eq!(request.start.as_deref(), Some(dir.path()));
    assert_eq!(request.name.as_deref(), Some("x.txt"));
    assert_eq!(request.kind, Kind::SaveFile);
}

// ---- the lists of recent files and folders (`COMBO_MRU`, `COMBO_OPENFOLDER`) ----

fn recent_lists(request: &Request) -> Vec<(RecentKind, Vec<String>)> {
    request
        .extras
        .iter()
        .filter_map(|extra| match extra {
            Extra::Recent { items, kind, .. } => Some((*kind, items.clone())),
            _ => None,
        })
        .collect()
}

#[gpui::test]
fn the_open_dialog_has_the_recent_files_and_folders(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let (f, cx) = setup(cx, settings(0, dir.path()));
    cx.update(|_, app| {
        crate::history::update(app, |history| {
            history.add_file(matsu_config::history::FileEntry::new(
                "/a/one.txt".to_owned(),
            ));
            history.add_file(matsu_config::history::FileEntry::new(
                "/b/メモ.txt".to_owned(),
            ));
            history.add_folder("/a");
        });
    });
    f.run(cx, "F_FILEOPEN", &[]);
    let request = only(asked());
    // newest first, as the menus show them
    assert_eq!(
        recent_lists(&request),
        [
            (
                RecentKind::Files,
                vec!["/b/メモ.txt".to_owned(), "/a/one.txt".to_owned()]
            ),
            // the folders of the files are remembered too
            (
                RecentKind::Folders,
                vec!["/a".to_owned(), "/b/".to_owned(), "/a/".to_owned()]
            ),
        ]
    );
}

#[gpui::test]
fn the_lists_are_there_without_a_history_too(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let (f, cx) = setup(cx, settings(0, dir.path()));
    f.run(cx, "F_FILEOPEN", &[]);
    assert_eq!(
        recent_lists(&only(asked())),
        [(RecentKind::Files, vec![]), (RecentKind::Folders, vec![])]
    );
}

#[gpui::test]
fn the_save_dialog_with_fields_has_them_after_the_fields(cx: &mut TestAppContext) {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "abc").unwrap();
    let (f, cx) = setup(cx, settings(0, dir.path()));
    f.open(cx, &file);
    cx.update(|_, app| crate::history::update(app, |h| h.add_folder("/a")));
    f.run(cx, "F_FILESAVEAS_DIALOG", &[]);
    let request = only(asked());
    assert_eq!(request.kind, Kind::SaveFile);
    let kinds: Vec<_> = request
        .extras
        .iter()
        .map(|e| match e {
            Extra::Combo { .. } => "combo",
            Extra::Check { .. } => "check",
            Extra::Recent { .. } => "recent",
        })
        .collect();
    assert_eq!(kinds, ["combo", "check", "combo", "recent", "recent"]);
}
