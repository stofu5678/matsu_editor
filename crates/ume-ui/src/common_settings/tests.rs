//! Tests of the common settings in GPUI's headless test windows: the pages, the keys, OK and
//! キャンセル with a real settings file in a private temporary folder, and what OK does to the
//! windows that are open.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use gpui::{AnyWindowHandle, Context, Entity, TestAppContext, VisualTestContext, Window};
use ume_commands::edit_options::{self, EditOptions};
use ume_commands::sakura::FUNCTIONS;
use ume_config::Profile;
use ume_config::history::{FileEntry, HistoryStore};

use super::dialog::CommonSettingsDialog;
use super::model::{Kind, controls};
use super::{Page, all_controls, current_dialog, dialog_window};
use crate::document::SharedDocument;
use crate::file_ops::windows::{editors, open_editor_window, workspaces};
use crate::history::{HistoryOptions, HistoryState};
use crate::settings::UserSettings;
use crate::tabs::settings::{DispTabClose, TabOptions};

/// A folder that is removed at the end of the test.
pub(super) struct TempDir(pub(super) PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("ume-ui-common-settings-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Puts the options of the commands back when the test ends: they are shared by the whole process.
pub(super) struct RestoreEditOptions(EditOptions);

impl RestoreEditOptions {
    pub(super) fn new() -> Self {
        Self(edit_options::get())
    }
}

impl Drop for RestoreEditOptions {
    fn drop(&mut self) {
        edit_options::set(self.0);
    }
}

pub(super) fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

/// The options of the commands are one value for the whole process, and OK and starting the
/// application set them: the tests that do so take turns.
pub(super) fn serial() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// The settings file of a test, and the application as `app::run` starts it.
pub(super) struct Ini {
    pub(super) dir: TempDir,
    pub(super) path: PathBuf,
    _serial: std::sync::MutexGuard<'static, ()>,
}

impl Ini {
    pub(super) fn bytes(&self) -> Vec<u8> {
        std::fs::read(&self.path).unwrap()
    }

    pub(super) fn profile(&self) -> Profile {
        Profile::load(&self.path).unwrap()
    }

    pub(super) fn get(&self, key: &str) -> Option<String> {
        self.profile().get("Common", key).map(str::to_owned)
    }
}

pub(super) fn with_bom(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

/// A settings file with `text`, read as `app::run` reads it, and one editor window.
pub(super) fn setup(cx: &mut TestAppContext, text: &str) -> (Ini, AnyWindowHandle) {
    let serial = serial();
    cx.update(gpui_component::init);
    let dir = TempDir::new();
    let path = dir.0.join("ume.ini");
    std::fs::write(&path, with_bom(text)).unwrap();
    cx.update(|cx| {
        let user = UserSettings::load_from(std::slice::from_ref(&path));
        cx.set_global(HistoryState::on_disk(
            HistoryStore::new(path.clone(), false),
            HistoryOptions::default(),
        ));
        crate::tabs::settings::install(cx, &user);
        super::install(cx, &user);
        cx.set_global(user);
    });
    let handle = open_window(cx);
    (
        Ini {
            dir,
            path,
            _serial: serial,
        },
        handle,
    )
}

pub(super) fn open_window(cx: &mut TestAppContext) -> AnyWindowHandle {
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    cx.run_until_parked();
    handle
}

/// Runs a function of Sakura in the window, as a key does.
pub(super) fn run(cx: &mut TestAppContext, handle: AnyWindowHandle, function: &str) {
    let code = code_of(function);
    let view = cx
        .update(|app| crate::file_ops::windows::workspace_of(app, handle))
        .unwrap()
        .read_with(cx, |ws, _| ws.editor().clone());
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, &[], window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

pub(super) fn dialog(cx: &mut TestAppContext) -> Entity<CommonSettingsDialog> {
    cx.update(|app| current_dialog(app))
        .expect("no dialog is open")
}

pub(super) fn is_open(cx: &mut TestAppContext) -> bool {
    cx.update(|app| current_dialog(app)).is_some()
}

/// Does something to the dialog and its window, as a click or a key does.
pub(super) fn with_dialog<R>(
    cx: &mut TestAppContext,
    f: impl FnOnce(&mut CommonSettingsDialog, &mut Window, &mut Context<CommonSettingsDialog>) -> R,
) -> R {
    let dialog = dialog(cx);
    let handle = cx.update(|app| dialog_window(app)).unwrap();
    let result = handle
        .update(cx, |_, window, app| {
            dialog.update(app, |dialog, dcx| f(dialog, window, dcx))
        })
        .unwrap();
    cx.run_until_parked();
    result
}

pub(super) fn ok(cx: &mut TestAppContext) {
    with_dialog(cx, |d, window, dcx| d.ok(window, dcx));
    assert!(!is_open(cx), "OK closes the dialog");
}

pub(super) fn cancel(cx: &mut TestAppContext) {
    with_dialog(cx, |d, window, dcx| d.cancel(window, dcx));
    assert!(!is_open(cx), "キャンセル closes the dialog");
}

pub(super) fn check(cx: &mut TestAppContext, id: &'static str, on: bool) {
    with_dialog(cx, |d, _, dcx| d.set_bool(id, on, dcx));
}

pub(super) fn choose(cx: &mut TestAppContext, id: &'static str, value: i32) {
    with_dialog(cx, |d, _, dcx| d.set_int(id, value, dcx));
}

pub(super) fn type_in(cx: &mut TestAppContext, id: &'static str, text: &str) {
    with_dialog(cx, |d, window, dcx| d.set_input(id, text, window, dcx));
}

pub(super) fn visual(cx: &mut TestAppContext, handle: AnyWindowHandle) -> &mut VisualTestContext {
    VisualTestContext::from_window(handle, cx).into_mut()
}

// ---- the pages ----

#[test]
fn the_pages_are_in_sakuras_order() {
    let titles: Vec<&str> = Page::ALL.iter().map(|p| p.title()).collect();
    // `ComPropSheetInfoList` in `CPropCommon.cpp`.
    assert_eq!(
        titles,
        [
            "全般",
            "ウィンドウ",
            "メインメニュー",
            "ツールバー",
            "タブバー",
            "ステータスバー",
            "編集",
            "ファイル",
            "ファイル名表示",
            "バックアップ",
            "書式",
            "検索",
            "キー割り当て",
            "カスタムメニュー",
            "強調キーワード",
            "支援",
            "マクロ",
            "プラグイン",
        ]
    );
    // The pages that are built; the others are added one by one, each by its own module.
    for page in [
        Page::General,
        Page::Window,
        Page::Toolbar,
        Page::Tab,
        Page::StatusBar,
        Page::Edit,
        Page::File,
        Page::FileName,
        Page::Backup,
        Page::Format,
        Page::Search,
    ] {
        assert!(page.items().is_some(), "{}", page.title());
    }
}

/// The pages that keep their own state (the kind chosen, the row selected) in controls with no key
/// of their own: the id of such a control starts with the page's prefix. The prefixes of the pages
/// that are not built yet are here too, so that each page can be built without changing this list.
const KEYLESS_PREFIXES: [&str; 5] = ["toolbar_", "keybind_", "custmenu_", "keyword_", "macro_"];

#[test]
fn every_control_has_its_own_name_and_sakuras_key() {
    let all = all_controls();
    let mut seen = HashSet::new();
    for control in &all {
        let is_group_member = matches!(control.kind, Kind::Radio { .. });
        if !is_group_member {
            assert!(seen.insert(control.id), "{} is there twice", control.id);
        }
        // 「元の拡張子を保存」 is part of `nBackUpType` (the kinds 5 and 6).
        let part_of_another_key = control.id == "backup_retain_ext";
        if !matches!(
            control.kind,
            Kind::Button { .. } | Kind::Numbers { .. } | Kind::Table { .. }
        ) && control.live
            && !part_of_another_key
            && !KEYLESS_PREFIXES.iter().any(|p| control.id.starts_with(p))
        {
            assert!(!control.key.is_empty(), "{} has no key", control.id);
        }
    }
    // The radio buttons of a group have their own values and one key.
    for page in Page::ALL.iter().filter(|p| p.items().is_some()) {
        let list = controls(&page.items().unwrap());
        for control in &list {
            if let Kind::Radio { value } = control.kind {
                let same = list.iter().filter(|c| {
                    c.id == control.id && matches!(c.kind, Kind::Radio { value: v } if v == value)
                });
                assert_eq!(same.count(), 1, "{} {value}", control.id);
            }
        }
    }
    // Every key of the pages in Sakura's `ShareData_IO_Common`.
    let keys: HashSet<&str> = all.iter().map(|c| c.key).collect();
    for key in [
        "bStopsBothEndsWhenSearchWord",
        "nMRUArrNum_MAX",
        "nOPENFOLDERArrNum_MAX",
        "bDispSTATUSBAR",
        "bSplitterWndVScroll",
        "bDispTabWnd",
        "bDispTabWndMultiWin",
        "bTab_RetainEmptyWin",
        "bTab_CloseOneWin",
        "bNewWindow",
        "bSameTabWidth",
        "bDispTabClose",
        "bChgWndByWheel",
        "eOpenDialogDir",
        "szOpenDialogSelDir",
        "bEnableNoSelectCopy",
        "bEnableLineModePaste",
        "bConvertEOLPaste",
    ] {
        assert!(keys.contains(key), "{key}");
    }
}

#[test]
fn the_pages_have_every_control_that_sakura_has() {
    // The number of check boxes, radio buttons, edit boxes, drop-down lists and buttons of the
    // resources (`IDD_PROP_GENERAL`, `IDD_PROP_WIN`, `IDD_PROP_TAB`, `IDD_PROP_EDIT`) that this
    // dialog draws; the ruler's spin buttons, the hot key and the font label are part of their
    // neighbours.
    let count = |page: Page| controls(&page.items().unwrap()).len();
    assert_eq!(count(Page::General), 20);
    assert_eq!(count(Page::Window), 18);
    assert_eq!(count(Page::Tab), 14);
    assert_eq!(count(Page::Edit), 19);
}

// ---- opening ----

#[gpui::test]
fn each_function_opens_its_page(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    for (function, page) in [
        ("F_OPTION_GENERAL", Page::General),
        ("F_OPTION_WINDOW", Page::Window),
        ("F_OPTION_TAB", Page::Tab),
        ("F_OPTION_EDIT", Page::Edit),
    ] {
        run(cx, handle, function);
        assert_eq!(with_dialog(cx, |d, _, _| d.page()), page, "{function}");
        cancel(cx);
    }
}

#[gpui::test]
fn f_option_opens_the_page_used_last(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION");
    assert_eq!(with_dialog(cx, |d, _, _| d.page()), Page::General);
    with_dialog(cx, |d, _, dcx| d.select_page(Page::Tab, dcx));
    cancel(cx);
    run(cx, handle, "F_OPTION");
    assert_eq!(with_dialog(cx, |d, _, _| d.page()), Page::Tab);
}

#[gpui::test]
fn a_second_function_brings_the_open_dialog_to_its_page(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION_WINDOW");
    let first = dialog(cx);
    run(cx, handle, "F_OPTION_EDIT");
    assert_eq!(first, dialog(cx));
    assert_eq!(with_dialog(cx, |d, _, _| d.page()), Page::Edit);
    assert_eq!(
        cx.update(|app| app.windows().len()),
        2,
        "the editor window and one dialog"
    );
}

#[gpui::test]
fn a_page_that_is_not_there_yet_says_so(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION");
    with_dialog(cx, |d, _, dcx| d.select_page(Page::Plugin, dcx));
    assert!(with_dialog(cx, |d, _, _| d.page_controls()).is_empty());
    // It still draws, and OK from there writes the other pages as they were.
    cx.run_until_parked();
    ok(cx);
}

// ---- OK ----

const INI: &str = "[Other]\r\nx=1\r\n\
[Types(0)]\r\nnInts=0,10240,0,2,-1,-1,0,1,1,0,0,0\r\n\
[Common]\r\nbDispSTATUSBAR=1\r\n";

#[gpui::test]
fn ok_writes_sakuras_keys_and_keeps_the_rest_of_the_file(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    let _restore = RestoreEditOptions::new();
    let before = ini.profile();
    run(cx, handle, "F_OPTION_GENERAL");
    check(cx, "stops_both_ends_paragraph", true);
    type_in(cx, "max_mru_file", "20");
    with_dialog(cx, |d, _, dcx| d.select_page(Page::Window, dcx));
    check(cx, "disp_status_bar", false);
    check(cx, "splitter_h_scroll", false);
    with_dialog(cx, |d, _, dcx| d.select_page(Page::Edit, dcx));
    check(cx, "convert_eol_paste", true);
    ok(cx);

    let after = ini.profile();
    assert_eq!(
        after.get("Common", "bStopsBothEndsWhenSearchParagraph"),
        Some("1")
    );
    assert_eq!(
        after.get("Common", "bStopsBothEndsWhenSearchWord"),
        Some("0")
    );
    assert_eq!(after.get("Common", "nMRUArrNum_MAX"), Some("20"));
    assert_eq!(after.get("Common", "nOPENFOLDERArrNum_MAX"), Some("15"));
    assert_eq!(after.get("Common", "bDispSTATUSBAR"), Some("0"));
    assert_eq!(after.get("Common", "bSplitterWndHScroll"), Some("0"));
    assert_eq!(after.get("Common", "bSplitterWndVScroll"), Some("1"));
    assert_eq!(after.get("Common", "bConvertEOLPaste"), Some("1"));
    assert_eq!(after.get("Common", "eOpenDialogDir"), Some("0"));
    // The other sections are as they were.
    for section in ["Other", "Types(0)"] {
        let old: Vec<_> = before.entries(section).collect();
        let new: Vec<_> = after.entries(section).collect();
        assert_eq!(old, new, "{section}");
    }
    assert!(ini.bytes().starts_with(&[0xEF, 0xBB, 0xBF]));
    // The commands take the options at once.
    assert!(edit_options::get().stops_both_ends_paragraph);
    assert!(edit_options::get().convert_eol_paste);
    assert!(!edit_options::get().stops_both_ends_word);
}

#[gpui::test]
fn ok_changes_every_open_window_at_once(cx: &mut TestAppContext) {
    let (ini, first) = setup(cx, INI);
    let second = open_window(cx);
    let shown = |cx: &mut TestAppContext| -> Vec<bool> {
        cx.update(editors)
            .into_iter()
            .map(|(_, view)| view.read_with(cx, |v, _| v.status_bar_shown()))
            .collect()
    };
    assert_eq!(shown(cx), [true, true]);
    run(cx, first, "F_OPTION_WINDOW");
    check(cx, "disp_status_bar", false);
    ok(cx);
    assert_eq!(shown(cx), [false, false], "both windows");
    assert_eq!(ini.get("bDispSTATUSBAR").as_deref(), Some("0"));
    // The dialog started from the second window shows what the windows have now, and a window
    // opened later starts with the file.
    run(cx, second, "F_OPTION_WINDOW");
    check(cx, "disp_status_bar", true);
    ok(cx);
    assert_eq!(shown(cx), [true, true]);
    open_window(cx);
    assert_eq!(shown(cx), [true, true, true]);
}

#[gpui::test]
fn the_panes_of_every_tab_take_the_status_bar(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "[Common]\r\nbDispTabWnd=1\r\n");
    run(cx, handle, "F_FILENEW");
    run(cx, handle, "F_SPLIT_V");
    run(cx, handle, "F_OPTION_WINDOW");
    check(cx, "disp_status_bar", false);
    ok(cx);
    let views = cx.update(|app| {
        workspaces(app)
            .into_iter()
            .flat_map(|(_, ws)| ws.read(app).all_views())
            .collect::<Vec<_>>()
    });
    assert!(views.len() >= 3, "{} views", views.len());
    for view in views {
        assert!(!view.read_with(cx, |v, _| v.status_bar_shown()));
    }
}

#[gpui::test]
fn the_synchronized_scrolling_follows_the_check_boxes(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    assert!(cx.update(|app| super::splitter_sync(app)).vertical);
    run(cx, handle, "F_OPTION_WINDOW");
    check(cx, "splitter_v_scroll", false);
    ok(cx);
    let now = cx.update(|app| super::splitter_sync(app));
    assert!(!now.vertical && now.horizontal);
}

#[gpui::test]
fn ok_shows_and_hides_the_toolbar_and_the_function_keys_in_every_window(cx: &mut TestAppContext) {
    use crate::bars::{BarSettings, FuncKeyPlace, current as bars};
    let (ini, first) = setup(cx, INI);
    let second = open_window(cx);
    assert_eq!(cx.update(|app| bars(app)), BarSettings::default());
    run(cx, first, "F_OPTION_WINDOW");
    check(cx, "disp_toolbar", false);
    check(cx, "disp_funckey_wnd", true);
    choose(cx, "funckey_place", 1);
    type_in(cx, "funckey_group_num", "6");
    ok(cx);
    assert_eq!(ini.get("bDispTOOLBAR").as_deref(), Some("0"));
    assert_eq!(ini.get("bDispFUNCKEYWND").as_deref(), Some("1"));
    assert_eq!(ini.get("nFUNCKEYWND_Place").as_deref(), Some("1"));
    assert_eq!(ini.get("nFUNCKEYWND_GroupNum").as_deref(), Some("6"));
    let now = cx.update(|app| bars(app));
    assert_eq!(
        (
            now.toolbar,
            now.funckeys,
            now.funckey_place,
            now.funckey_group_num
        ),
        (false, true, FuncKeyPlace::Bottom, 6)
    );
    for handle in [first, second] {
        let ws = cx
            .update(|app| crate::file_ops::windows::workspace_of(app, handle))
            .unwrap();
        assert!(ws.update(cx, |ws, cx| ws.render_toolbar(false, cx).is_none()));
        let bottom = handle
            .update(cx, |_, window, app| {
                ws.update(app, |ws, cx| {
                    ws.render_funckeys(FuncKeyPlace::Bottom, window, cx)
                        .is_some()
                })
            })
            .unwrap();
        assert!(bottom, "both windows");
    }
    // The dialog starts from what the windows have now.
    run(cx, second, "F_OPTION_WINDOW");
    assert!(!with_dialog(cx, |d, _, _| d.values().bool("disp_toolbar")));
    assert!(with_dialog(cx, |d, _, _| d
        .values()
        .bool("disp_funckey_wnd")));
    cancel(cx);
}

#[gpui::test]
fn cancel_changes_nothing(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    let bytes = ini.bytes();
    let options = edit_options::get();
    run(cx, handle, "F_OPTION_WINDOW");
    check(cx, "disp_status_bar", false);
    with_dialog(cx, |d, _, dcx| d.select_page(Page::General, dcx));
    type_in(cx, "max_mru_file", "3");
    check(cx, "stops_both_ends_word", true);
    cancel(cx);
    assert_eq!(ini.bytes(), bytes);
    assert_eq!(edit_options::get(), options);
    let view = cx.update(editors).remove(0).1;
    assert!(view.read_with(cx, |v, _| v.status_bar_shown()));
    // The next dialog starts from the file again.
    run(cx, handle, "F_OPTION_GENERAL");
    assert_eq!(
        with_dialog(cx, |d, _, _| d.values().int("max_mru_file")),
        15
    );
    cancel(cx);
}

#[gpui::test]
fn a_number_is_put_into_its_range_not_refused(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    for (text, written) in [
        ("99", "36"),
        ("-5", "0"),
        ("abc", "0"),
        ("", "0"),
        (" 7 ", "7"),
    ] {
        run(cx, handle, "F_OPTION_GENERAL");
        type_in(cx, "max_mru_file", text);
        type_in(cx, "max_mru_folder", text);
        ok(cx);
        assert_eq!(
            ini.get("nMRUArrNum_MAX").as_deref(),
            Some(written),
            "{text:?}"
        );
        assert_eq!(
            ini.get("nOPENFOLDERArrNum_MAX").as_deref(),
            Some(written),
            "{text:?}"
        );
    }
}

#[gpui::test]
fn a_number_in_the_file_that_is_too_large_is_put_into_range_by_ok(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "[Common]\r\nnMRUArrNum_MAX=500\r\n");
    run(cx, handle, "F_OPTION_GENERAL");
    ok(cx);
    assert_eq!(ini.get("nMRUArrNum_MAX").as_deref(), Some("36"));
}

#[gpui::test]
fn what_is_not_available_is_shown_but_never_written(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "[Common]\r\nbIsFreeCursorMode=1\r\nnCaretType=1\r\n");
    run(cx, handle, "F_OPTION_GENERAL");
    // The value of the file is shown, and a click does nothing.
    assert!(with_dialog(cx, |d, _, _| d.values().bool("free_caret")));
    check(cx, "free_caret", false);
    assert!(with_dialog(cx, |d, _, _| d.values().bool("free_caret")));
    choose(cx, "caret_type", 0);
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int("caret_type")), 1);
    assert!(!with_dialog(cx, |d, w, dcx| d.use_access_key('f', w, dcx)));
    ok(cx);
    assert_eq!(ini.get("bIsFreeCursorMode").as_deref(), Some("1"));
    assert_eq!(ini.get("nCaretType").as_deref(), Some("1"));
    assert_eq!(ini.get("bTaskTrayUse"), None);
    assert_eq!(ini.get("bVistaStyleFileDialog"), None);
}

#[gpui::test]
fn a_file_that_cannot_be_written_still_changes_the_session(cx: &mut TestAppContext) {
    let _serial = serial();
    let _restore = RestoreEditOptions::new();
    cx.update(gpui_component::init);
    cx.update(|cx| cx.set_global(HistoryState::in_memory(HistoryOptions::default())));
    let handle = open_window(cx);
    run(cx, handle, "F_OPTION_GENERAL");
    check(cx, "stops_both_ends_word", true);
    ok(cx);
    assert!(edit_options::get().stops_both_ends_word);
}

// ---- the tab bar ----

#[gpui::test]
fn the_tab_bar_and_the_group_follow_the_page(cx: &mut TestAppContext) {
    let (ini, first) = setup(cx, "");
    let second = open_window(cx);
    assert_eq!(workspaces_count(cx), 2);
    run(cx, first, "F_OPTION_TAB");
    // The bar is off: what depends on it is greyed out and cannot be changed.
    assert!(with_dialog(cx, |d, _, _| d.values().bool("tab_grouped")));
    check(cx, "tab_grouped", false);
    assert!(with_dialog(cx, |d, _, _| d.values().bool("tab_grouped")));
    assert!(!with_dialog(cx, |d, w, dcx| d.use_access_key('b', w, dcx)));
    check(cx, "tab_bar", true);
    check(cx, "tab_open_new_win", true);
    ok(cx);

    let settings = cx.update(|app| crate::tabs::settings::current(app));
    assert!(settings.show_bar && settings.grouped && settings.new_window);
    assert_eq!(ini.get("bDispTabWnd").as_deref(), Some("1"));
    assert_eq!(ini.get("bDispTabWndMultiWin").as_deref(), Some("0"));
    assert_eq!(ini.get("bNewWindow").as_deref(), Some("1"));
    // The windows gathered in the one the dialog came from (`ResetGroupId`).
    assert_eq!(workspaces_count(cx), 1);
    assert!(!cx.update(|app| first.update(app, |_, _, _| ()).is_err()));
    assert!(cx.update(|app| second.update(app, |_, _, _| ()).is_err()));
    assert_eq!(tab_count(cx, first), 2);

    // Not grouping spreads them again.
    run(cx, first, "F_OPTION_TAB");
    assert!(with_dialog(cx, |d, _, _| d.values().bool("tab_bar")));
    check(cx, "tab_grouped", false);
    ok(cx);
    assert_eq!(workspaces_count(cx), 2);
    assert_eq!(ini.get("bDispTabWndMultiWin").as_deref(), Some("1"));
    assert!(!cx.update(|app| crate::tabs::settings::current(app)).bound());
}

#[gpui::test]
fn the_tab_mode_is_not_changed_while_a_file_is_being_read(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    let view = editors_of(cx).remove(0);
    cx.update(|app| {
        crate::file_ops::windows::begin_pending(
            app,
            handle,
            &view.downgrade(),
            "/work/slow.txt".into(),
        )
    });
    let bytes = ini.bytes();
    run(cx, handle, "F_OPTION_TAB");
    check(cx, "tab_bar", true);
    with_dialog(cx, |d, window, dcx| d.ok(window, dcx));
    // The question is answered, the dialog stays and nothing is written.
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
    assert!(is_open(cx));
    assert_eq!(ini.bytes(), bytes);
    assert!(
        !cx.update(|app| crate::tabs::settings::current(app))
            .show_bar
    );
    cancel(cx);
}

fn editors_of(cx: &mut TestAppContext) -> Vec<Entity<crate::view::EditorView>> {
    cx.update(editors).into_iter().map(|(_, v)| v).collect()
}

fn workspaces_count(cx: &mut TestAppContext) -> usize {
    cx.update(workspaces).len()
}

fn tab_count(cx: &mut TestAppContext, handle: AnyWindowHandle) -> usize {
    cx.update(|app| crate::file_ops::windows::workspace_of(app, handle))
        .unwrap()
        .read_with(cx, |ws, _| ws.tab_count())
}

#[gpui::test]
fn the_dialog_starts_from_the_tab_mode_the_windows_have(cx: &mut TestAppContext) {
    // F_SHOWTAB changes the mode of the windows; the dialog must show that, not the file as the
    // application read it.
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_SHOWTAB");
    run(cx, handle, "F_OPTION_TAB");
    assert!(with_dialog(cx, |d, _, _| d.values().bool("tab_bar")));
    cancel(cx);
}

#[gpui::test]
fn the_options_of_the_tabs_are_written_and_taken(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "[Common]\r\nbDispTabWnd=1\r\n");
    assert_eq!(
        cx.update(|app| crate::tabs::settings::options(app)),
        TabOptions::default()
    );
    run(cx, handle, "F_OPTION_TAB");
    check(cx, "tab_retain_empty", false);
    check(cx, "tab_close_one_win", true);
    check(cx, "tab_same_width", true);
    choose(cx, "tab_disp_close", 2);
    check(cx, "tab_change_by_wheel", true);
    ok(cx);
    assert_eq!(
        cx.update(|app| crate::tabs::settings::options(app)),
        TabOptions {
            retain_empty: false,
            close_one_win: true,
            same_width: true,
            disp_close: DispTabClose::Auto,
            change_by_wheel: true,
            ..TabOptions::default()
        }
    );
    for (key, value) in [
        ("bTab_RetainEmptyWin", "0"),
        ("bTab_CloseOneWin", "1"),
        ("bSameTabWidth", "1"),
        ("bDispTabClose", "2"),
        ("bChgWndByWheel", "1"),
    ] {
        assert_eq!(ini.get(key).as_deref(), Some(value), "{key}");
    }
    // Not available yet: not written.
    assert_eq!(ini.get("bDispTabIcon"), None);
}

fn active_editor(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
) -> Entity<crate::view::EditorView> {
    cx.update(|app| crate::file_ops::windows::workspace_of(app, handle))
        .unwrap()
        .read_with(cx, |ws, _| ws.editor().clone())
}

#[gpui::test]
fn the_sort_and_the_caption_of_the_tabs_are_live_and_taken_at_once(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "[Common]\r\nbDispTabWnd=1\r\nbTab_ListFull=1\r\n");
    let view = active_editor(cx, handle);
    let caption = |cx: &mut TestAppContext| view.read_with(cx, crate::app::Workspace::caption_of);
    assert_eq!(caption(cx), "(無題)");
    assert!(cx.update(|app| crate::tabs::settings::options(app).sort_list));
    run(cx, handle, "F_OPTION_TAB");
    assert!(with_dialog(cx, |d, _, _| d
        .page_controls()
        .iter()
        .filter(|c| c.id == "tab_sort_list" || c.id == "tab_caption")
        .all(|c| c.live)));
    check(cx, "tab_sort_list", false);
    type_in(cx, "tab_caption", "<$f>${U?*$}");
    ok(cx);
    assert_eq!(ini.get("bSortTabList").as_deref(), Some("0"));
    assert_eq!(ini.get("szTabWndCaption").as_deref(), Some("<$f>${U?*$}"));
    // The sorting is taken, the choice of the list the dialog does not have is kept.
    let options = cx.update(|app| crate::tabs::settings::options(app));
    assert!(!options.sort_list);
    assert!(options.list_full);
    assert_eq!(caption(cx), "<(無題)>");
}

#[gpui::test]
fn a_caption_format_in_the_file_is_in_use_from_the_start(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "[Common]\r\nbDispTabWnd=1\r\nszTabWndCaption=[$f]\r\n");
    let view = active_editor(cx, handle);
    let caption = view.read_with(cx, crate::app::Workspace::caption_of);
    assert_eq!(caption, "[(無題)]");
}

#[gpui::test]
fn the_last_file_closes_into_an_untitled_document_when_the_option_says_so(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "[Common]\r\nbDispTabWnd=1\r\n");
    // Sakura's default: the window stays (`bTab_RetainEmptyWin`).
    run(cx, handle, "F_WINCLOSE");
    assert_eq!(workspaces_count(cx), 1);
    assert_eq!(tab_count(cx, handle), 1);
    run(cx, handle, "F_OPTION_TAB");
    check(cx, "tab_retain_empty", false);
    ok(cx);
    run(cx, handle, "F_WINCLOSE");
    assert_eq!(workspaces_count(cx), 0);
}

#[gpui::test]
fn the_close_button_of_the_window_closes_one_file_when_the_option_says_so(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "[Common]\r\nbDispTabWnd=1\r\nbTab_CloseOneWin=1\r\n");
    run(cx, handle, "F_FILENEW");
    assert_eq!(tab_count(cx, handle), 2);
    // The close button of the window: one tab goes, the window stays.
    assert!(!visual(cx, handle).simulate_close());
    cx.run_until_parked();
    assert_eq!(tab_count(cx, handle), 1);
    assert_eq!(workspaces_count(cx), 1);
}

#[gpui::test]
fn the_close_button_of_the_window_closes_the_group_by_default(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "[Common]\r\nbDispTabWnd=1\r\n");
    run(cx, handle, "F_FILENEW");
    assert_eq!(tab_count(cx, handle), 2);
    assert!(visual(cx, handle).simulate_close());
}

// ---- 編集 ----

#[gpui::test]
fn the_start_folder_of_the_file_dialogs_follows_the_page(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    let folder = ini.dir.0.join("start");
    std::fs::create_dir_all(&folder).unwrap();
    let start =
        |cx: &mut TestAppContext| cx.update(|app| crate::file_dialog::start_folder(app, None));
    // The current folder of the process, Sakura's default.
    assert_eq!(start(cx), std::env::current_dir().ok());
    run(cx, handle, "F_OPTION_EDIT");
    // The box of the given folder can be used only when that choice is made.
    type_in(cx, "open_dialog_sel_dir", "ignored");
    assert_eq!(
        with_dialog(cx, |d, _, dcx| d.input_text("open_dialog_sel_dir", dcx)).as_deref(),
        Some("%Personal%\\")
    );
    choose(cx, "open_dialog_dir", 2);
    type_in(cx, "open_dialog_sel_dir", &folder.to_string_lossy());
    ok(cx);
    assert_eq!(ini.get("eOpenDialogDir").as_deref(), Some("2"));
    assert_eq!(
        ini.get("szOpenDialogSelDir").as_deref(),
        Some(folder.to_string_lossy().as_ref())
    );
    assert_eq!(start(cx), Some(folder.clone()));

    // The recent folders.
    run(cx, handle, "F_OPTION_EDIT");
    choose(cx, "open_dialog_dir", 1);
    ok(cx);
    assert_eq!(ini.get("eOpenDialogDir").as_deref(), Some("1"));
}

#[gpui::test]
fn the_folder_button_puts_the_folder_in_the_box_with_percent_doubled(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    let folder = ini.dir.0.join("100%");
    std::fs::create_dir_all(&folder).unwrap();
    run(cx, handle, "F_OPTION_EDIT");
    choose(cx, "open_dialog_dir", 2);
    with_dialog(cx, |d, window, dcx| {
        d.choose_folder("open_dialog_sel_dir", "t", window, dcx)
    });
    assert!(cx.did_prompt_for_paths());
    let chosen = folder.clone();
    cx.simulate_path_prompt_response(move |_| Some(vec![chosen]));
    cx.run_until_parked();
    let text = with_dialog(cx, |d, _, dcx| d.input_text("open_dialog_sel_dir", dcx)).unwrap();
    assert_eq!(text, folder.to_string_lossy().replace('%', "%%"));
    ok(cx);
    // The file holds the doubled `%`, which the settings expand back to the folder.
    assert_eq!(ini.get("szOpenDialogSelDir"), Some(text));
    assert_eq!(
        cx.update(|app| crate::file_dialog::start_folder(app, None)),
        Some(folder)
    );
}

#[gpui::test]
fn the_edit_options_reach_the_commands(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    let _restore = RestoreEditOptions::new();
    run(cx, handle, "F_OPTION_EDIT");
    assert!(with_dialog(cx, |d, _, _| d
        .values()
        .bool("enable_no_select_copy")));
    assert!(!with_dialog(cx, |d, _, _| d
        .values()
        .bool("convert_eol_paste")));
    check(cx, "convert_eol_paste", true);
    ok(cx);
    let now = edit_options::get();
    assert!(now.convert_eol_paste && now.enable_line_mode_paste && now.enable_no_select_copy);
}

#[gpui::test]
fn the_application_starts_with_the_file(cx: &mut TestAppContext) {
    let (_ini, _handle) = setup(
        cx,
        "[Common]\r\nbStopsBothEndsWhenSearchWord=1\r\nbConvertEOLPaste=1\r\n\
         bSplitterWndHScroll=0\r\nbTab_RetainEmptyWin=0\r\n",
    );
    let _restore = RestoreEditOptions::new();
    let options = edit_options::get();
    assert!(options.stops_both_ends_word && options.convert_eol_paste);
    assert!(!cx.update(|app| super::splitter_sync(app)).horizontal);
    assert!(
        !cx.update(|app| crate::tabs::settings::options(app))
            .retain_empty
    );
}

// ---- the history ----

#[gpui::test]
fn the_buttons_clear_the_history_after_a_question(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    cx.update(|app| {
        crate::history::update(app, |h| {
            h.add_file(FileEntry::new("/a/b.txt"));
            h.add_file(FileEntry::new("/c/d.txt"));
        });
    });
    let lists = |cx: &mut TestAppContext| {
        let h = cx.update(|app| crate::history::current(app)).unwrap();
        (h.files.len(), h.folders.len())
    };
    assert_eq!(lists(cx), (2, 2));
    run(cx, handle, "F_OPTION_GENERAL");
    // キャンセル of the question keeps the history.
    assert!(with_dialog(cx, |d, w, dcx| d.use_access_key('c', w, dcx)));
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("キャンセル");
    cx.run_until_parked();
    assert_eq!(lists(cx), (2, 2));
    // OK clears it at once, before the dialog is closed, and says so.
    assert!(with_dialog(cx, |d, w, dcx| d.use_access_key('c', w, dcx)));
    let (question, _) = cx.pending_prompt().unwrap();
    assert!(question.contains("ファイルの履歴"), "{question}");
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
    assert_eq!(lists(cx), (0, 2));
    let (done, _) = cx.pending_prompt().unwrap();
    assert!(done.contains("削除しました"), "{done}");
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
    // The folders have their own button.
    assert!(with_dialog(cx, |d, w, dcx| d.use_access_key('l', w, dcx)));
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
    assert_eq!(lists(cx), (0, 0));
    cancel(cx);
}

#[gpui::test]
fn the_number_of_files_in_the_history_is_taken_by_the_menu(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    cx.update(|app| {
        crate::history::update(app, |h| {
            for name in ["a", "b", "c", "d"] {
                h.add_file(FileEntry::new(format!("/x/{name}.txt")));
            }
        });
    });
    assert_eq!(
        cx.update(|app| crate::history::current(app))
            .unwrap()
            .files
            .visible()
            .len(),
        4
    );
    run(cx, handle, "F_OPTION_GENERAL");
    type_in(cx, "max_mru_file", "2");
    ok(cx);
    assert_eq!(
        cx.update(|app| crate::history::current(app))
            .unwrap()
            .files
            .visible()
            .len(),
        2
    );
}

// ---- the keys ----

#[gpui::test]
fn the_keys_of_a_property_sheet(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION_GENERAL");
    let window = cx.update(|app| dialog_window(app)).unwrap();
    // Ctrl+Tab and Ctrl+PageDown go on, Ctrl+Shift+Tab and Ctrl+PageUp go back, round the end.
    cx.simulate_keystrokes(window, "ctrl-tab");
    assert_eq!(with_dialog(cx, |d, _, _| d.page()), Page::Window);
    cx.simulate_keystrokes(window, "ctrl-pagedown");
    assert_eq!(with_dialog(cx, |d, _, _| d.page()), Page::MainMenu);
    cx.simulate_keystrokes(window, "ctrl-pageup");
    cx.simulate_keystrokes(window, "ctrl-shift-tab");
    assert_eq!(with_dialog(cx, |d, _, _| d.page()), Page::General);
    cx.simulate_keystrokes(window, "ctrl-shift-tab");
    assert_eq!(with_dialog(cx, |d, _, _| d.page()), Page::Plugin);
    cx.simulate_keystrokes(window, "ctrl-tab");
    assert_eq!(with_dialog(cx, |d, _, _| d.page()), Page::General);

    // The access key of a label: Alt+B is 「単語の両端に止まる(&B)」.
    assert!(!with_dialog(cx, |d, _, _| d
        .values()
        .bool("stops_both_ends_word")));
    cx.simulate_keystrokes(window, "alt-b");
    assert!(with_dialog(cx, |d, _, _| d
        .values()
        .bool("stops_both_ends_word")));
    cx.simulate_keystrokes(window, "alt-b");
    assert!(!with_dialog(cx, |d, _, _| d
        .values()
        .bool("stops_both_ends_word")));

    // Esc is キャンセル.
    cx.simulate_keystrokes(window, "alt-b");
    cx.simulate_keystrokes(window, "escape");
    assert!(!is_open(cx));
    assert_eq!(ini.get("bStopsBothEndsWhenSearchWord"), None);

    // Enter is OK.
    run(cx, handle, "F_OPTION_GENERAL");
    let window = cx.update(|app| dialog_window(app)).unwrap();
    cx.simulate_keystrokes(window, "alt-p");
    cx.simulate_keystrokes(window, "enter");
    assert!(!is_open(cx));
    assert_eq!(
        ini.get("bStopsBothEndsWhenSearchParagraph").as_deref(),
        Some("1")
    );
    edit_options::set(EditOptions::default());
}

#[gpui::test]
fn the_access_keys_choose_radio_buttons_and_fill_boxes(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION_EDIT");
    assert_eq!(
        with_dialog(cx, |d, _, _| d.values().int("open_dialog_dir")),
        0
    );
    assert!(with_dialog(cx, |d, w, dcx| d.use_access_key('m', w, dcx)));
    assert_eq!(
        with_dialog(cx, |d, _, _| d.values().int("open_dialog_dir")),
        1
    );
    assert!(with_dialog(cx, |d, w, dcx| d.use_access_key('F', w, dcx)));
    assert_eq!(
        with_dialog(cx, |d, _, _| d.values().int("open_dialog_dir")),
        2
    );
    assert!(!with_dialog(cx, |d, w, dcx| d.use_access_key('q', w, dcx)));
    cancel(cx);
}

#[gpui::test]
fn alt_x_is_cancel_and_alt_o_is_ok_where_no_label_has_the_letter(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    // The page 「プラグイン」 is not there yet and has no labels.
    run(cx, handle, "F_OPTION");
    with_dialog(cx, |d, _, dcx| d.select_page(Page::Plugin, dcx));
    with_dialog(cx, |d, w, dcx| d.use_access_key('x', w, dcx));
    assert!(!is_open(cx));
    assert_eq!(ini.get("bDispSTATUSBAR"), None);
    run(cx, handle, "F_OPTION");
    with_dialog(cx, |d, w, dcx| d.use_access_key('o', w, dcx));
    assert!(!is_open(cx));
    assert_eq!(ini.get("bDispSTATUSBAR").as_deref(), Some("1"));
}

#[test]
fn text_that_is_not_a_number_is_zero() {
    use super::dialog::parse_number;
    assert_eq!(parse_number("15"), 15);
    assert_eq!(parse_number(" 15 "), 15);
    assert_eq!(parse_number("-3"), 0);
    assert_eq!(parse_number("1e3"), 0);
    assert_eq!(parse_number("４"), 0);
    assert_eq!(parse_number("99999999999"), 0);
    assert_eq!(parse_number(""), 0);
}

#[gpui::test]
fn the_icons_in_menus_follow_the_check_box_in_every_window(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "[Other]\r\nx=1\r\n");
    // Sakura's default is on.
    assert!(cx.update(|app| crate::menu_icons::enabled(app)));
    run(cx, handle, "F_OPTION_WINDOW");
    assert!(with_dialog(cx, |d, _, _| d.values().bool("menu_icon")));
    check(cx, "menu_icon", false);
    ok(cx);
    assert!(!cx.update(|app| crate::menu_icons::enabled(app)));
    assert_eq!(ini.get("bMenuIcon").as_deref(), Some("0"));
    assert_eq!(
        ini.profile().get("Other", "x"),
        Some("1"),
        "the rest of the file stays"
    );
    run(cx, handle, "F_OPTION_WINDOW");
    check(cx, "menu_icon", true);
    ok(cx);
    assert!(cx.update(|app| crate::menu_icons::enabled(app)));
}

#[gpui::test]
fn the_application_starts_with_the_icons_off_when_the_file_says_so(cx: &mut TestAppContext) {
    let (_ini, _handle) = setup(cx, "[Common]\r\nbMenuIcon=0\r\n");
    assert!(!cx.update(|app| crate::menu_icons::enabled(app)));
}
