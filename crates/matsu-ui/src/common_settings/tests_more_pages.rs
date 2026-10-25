//! Tests of the pages ファイル, バックアップ, 書式, 検索, ファイル名表示 and ステータスバー, in GPUI's
//! headless test windows, like the tests of the first four pages (`tests`): opening at the page,
//! the keys OK writes, the rest of the file, キャンセル, and what the values do in the windows.

use gpui::{AnyWindowHandle, Entity, TestAppContext};
use matsu_commands::command::Arg;
use matsu_commands::format_options::{self, FormatOptions};

use super::model::controls;
use super::tests::{
    Ini, RestoreEditOptions, cancel, check, choose, code_of, ok, run, setup, type_in, visual,
    with_dialog,
};
use super::{Page, current_dialog};
use crate::file_ops::windows::workspace_of;
use crate::view::EditorView;

/// Puts the formats of the commands back when the test ends: they are shared by the whole process.
struct RestoreFormatOptions(FormatOptions);

impl RestoreFormatOptions {
    fn new() -> Self {
        Self(format_options::get())
    }
}

impl Drop for RestoreFormatOptions {
    fn drop(&mut self) {
        format_options::set(self.0.clone());
    }
}

fn view_of(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<EditorView> {
    cx.update(|app| workspace_of(app, handle))
        .unwrap()
        .read_with(cx, |ws, _| ws.editor().clone())
}

/// Runs a function of Sakura with arguments in the window, as a macro does.
fn run_with(cx: &mut TestAppContext, handle: AnyWindowHandle, function: &str, args: &[Arg]) {
    let code = code_of(function);
    let view = view_of(cx, handle);
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, args, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

fn first_line(cx: &mut TestAppContext, handle: AnyWindowHandle) -> String {
    view_of(cx, handle).read_with(cx, |view, app| {
        view.document()
            .read(app)
            .doc()
            .line(0)
            .ok()
            .flatten()
            .map(|l| l.text)
            .unwrap_or_default()
    })
}

fn value(ini: &Ini, section: &str, key: &str) -> Option<String> {
    ini.profile().get(section, key).map(str::to_owned)
}

/// The text of a file, for the save flow.
fn write_file(ini: &Ini, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = ini.dir.0.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

// ---- the pages ----

#[test]
fn the_pages_have_every_control_that_sakura_has() {
    // The check boxes, radio buttons, edit boxes, drop-down lists and buttons of the resources
    // (`IDD_PROP_FILE`, `IDD_PROP_BACKUP`, `IDD_PROP_FORMAT`, `IDD_PROP_GREP`, `IDD_PROP_FNAME`,
    // `IDD_PROP_STATUSBAR`) that this dialog draws; the spin buttons are part of their numbers,
    // the boxes 「表示例」 and the version of the regular expression library are not drawn; the
    // list of `IDD_PROP_FNAME` with its two boxes and eight buttons is one control.
    let count = |page: Page| controls(&page.items().unwrap()).len();
    // 18 are Sakura's; the two about the copies of unsaved documents are MATSU Editor's (ADR-0022),
    // and the size from which a file is opened by ranges (ADR-0020).
    assert_eq!(count(Page::File), 18 + 3);
    assert_eq!(count(Page::Backup), 23);
    assert_eq!(count(Page::Format), 8);
    assert_eq!(count(Page::Search), 9);
    assert_eq!(count(Page::FileName), 3);
    assert_eq!(count(Page::StatusBar), 7);
}

#[test]
fn the_keys_are_sakuras_and_in_the_sections_sakura_writes() {
    let all: Vec<_> = [
        Page::File,
        Page::Backup,
        Page::Format,
        Page::Search,
        Page::FileName,
        Page::StatusBar,
    ]
    .iter()
    .flat_map(|p| controls(&p.items().unwrap()))
    .collect();
    let has = |section: &str, key: &str| all.iter().any(|c| c.section == section && c.key == key);
    // `ShareData_IO_Common`
    for key in [
        "nFileShareMode",
        "bCheckFileTimeStamp",
        "nAutoloadDelay",
        "bUneditableIfUnwritable",
        "bEnableUnmodifiedOverwrite",
        "bNoFilterSaveNew",
        "bNoFilterSaveFile",
        "bDropFileAndClose",
        "nDropFileNumMax",
        "m_bRestoreCurPosition",
        "m_bRestoreBookmarks",
        "bAutoMIMEdecode",
        "bQueryIfCodeChange",
        "bAlertIfFileNotExist",
        "bAlertIfLargeFile",
        "nAlertFileSize",
        "bBackUp",
        "bBackUpDialog",
        "bBackUpFolder",
        "bBackUpFolderRM",
        "szBackUpFolder",
        "nBackUpType",
        "bBackUpType2_Opt1",
        "bBackUpType2_Opt2",
        "bBackUpType2_Opt3",
        "bBackUpType2_Opt4",
        "bBackUpDustBox",
        "bBackUpPathAdvanced",
        "szBackUpPathAdvanced",
        "szMidashiKigou",
        "szInyouKigou",
        "nDateFormatType",
        "szDateFormat",
        "nTimeFormatType",
        "szTimeFormat",
        "bCaretTextForSearch",
        "m_bInheritKeyOtherView",
        "szRegexpLib",
        "bGrepExitConfirm",
        "bGrepRealTime",
        "bGTJW_RETURN",
        "bGTJW_LDBLCLK",
        "nTagJumpMode",
        "nTagJumpModeKeyword",
    ] {
        assert!(has("Common", key), "[Common] {key}");
    }
    // Not Sakura's (ADR-0020, chapter 7).
    assert!(has("Common", "nHugeFileSizeMB"));
    // `ShareData_IO_Nickname`, `ShareData_IO_Statusbar`
    for key in ["bShortPath", "nShortPathMaxWidth"] {
        assert!(has("Nickname", key), "[Nickname] {key}");
    }
    for key in [
        "DispUnicodeInSjis",
        "DispUnicodeInJis",
        "DispUnicodeInEuc",
        "DispUtf8Codepoint",
        "DispSurrogatePairCodepoint",
        "DispSelectCountByByte",
        "DispColByChar",
    ] {
        assert!(has("Statusbar", key), "[Statusbar] {key}");
    }
}

#[gpui::test]
fn each_function_opens_its_page(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    for (function, page) in [
        ("F_OPTION_FILE", Page::File),
        ("F_OPTION_BACKUP", Page::Backup),
        ("F_OPTION_FORMAT", Page::Format),
        ("F_OPTION_GREP", Page::Search),
        ("F_OPTION_FNAME", Page::FileName),
        ("F_OPTION_STATUSBAR", Page::StatusBar),
    ] {
        run(cx, handle, function);
        assert_eq!(with_dialog(cx, |d, _, _| d.page()), page, "{function}");
        assert!(
            !with_dialog(cx, |d, _, _| d.page_controls()).is_empty(),
            "{function}"
        );
        cancel(cx);
    }
}

// ---- ファイル ----

#[gpui::test]
fn ok_on_the_file_page_locks_the_open_files_in_the_new_mode(cx: &mut TestAppContext) {
    use matsu_platform::file_lock::{FileLock, ShareMode};
    let (ini, handle) = setup(cx, "[Common]\r\nnFileShareMode=0\r\n");
    let _restore = RestoreEditOptions::new();
    let path = write_file(&ini, "locked.txt", b"abc");
    let view = view_of(cx, handle);
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| {
                view.load_path_here(path.clone(), None, window, cx)
            })
        })
        .unwrap();
    cx.run_until_parked();
    let held = |cx: &mut TestAppContext| {
        view.read_with(cx, |view, app| view.document().read(app).held_lock_mode())
    };
    assert_eq!(held(cx), None);
    assert!(FileLock::acquire(&path, ShareMode::DenyReadWrite).is_ok());

    run(cx, handle, "F_OPTION_FILE");
    choose(cx, "file_share_mode", 2);
    ok(cx);
    assert_eq!(held(cx), Some(ShareMode::DenyReadWrite));
    assert!(FileLock::acquire(&path, ShareMode::DenyWrite).is_err());

    run(cx, handle, "F_OPTION_FILE");
    choose(cx, "file_share_mode", 1);
    ok(cx);
    assert_eq!(held(cx), Some(ShareMode::DenyWrite));

    run(cx, handle, "F_OPTION_FILE");
    choose(cx, "file_share_mode", 0);
    ok(cx);
    assert_eq!(held(cx), None);
    assert!(FileLock::acquire(&path, ShareMode::DenyReadWrite).is_ok());
}

#[gpui::test]
fn the_file_page_has_the_two_checkboxes_of_the_copies_of_unsaved_documents(
    cx: &mut TestAppContext,
) {
    use crate::unsaved::prefs::{KeepPrefs, prefs};
    // The size limit is in the file only, and the dialog keeps it.
    let (ini, handle) = setup(cx, "[MATSU]\r\nKeepUnsavedMaxMB=12\r\n");
    let _restore = RestoreEditOptions::new();
    assert_eq!(
        cx.update(|app| prefs(app)),
        KeepPrefs {
            keep: true,
            no_ask: false,
            max_mb: 12
        },
        "the defaults and the limit of the file are in the windows from the start"
    );
    run(cx, handle, "F_OPTION_FILE");
    assert!(with_dialog(cx, |d, _, _| d.values().bool("keep_unsaved")));
    assert!(!with_dialog(cx, |d, _, _| d
        .values()
        .bool("keep_unsaved_no_ask")));
    check(cx, "keep_unsaved_no_ask", true);
    ok(cx);
    assert_eq!(value(&ini, "MATSU", "KeepUnsaved").as_deref(), Some("1"));
    assert_eq!(
        value(&ini, "MATSU", "KeepUnsavedNoAsk").as_deref(),
        Some("1")
    );
    assert_eq!(
        value(&ini, "MATSU", "KeepUnsavedMaxMB").as_deref(),
        Some("12"),
        "the limit is not touched"
    );
    assert_eq!(
        cx.update(|app| prefs(app)),
        KeepPrefs {
            keep: true,
            no_ask: true,
            max_mb: 12
        }
    );

    // Turning it off asks what to do with the copies (「残す」 leaves them).
    run(cx, handle, "F_OPTION_FILE");
    check(cx, "keep_unsaved", false);
    ok(cx);
    assert!(!cx.update(|app| prefs(app)).keep);
    assert_eq!(value(&ini, "MATSU", "KeepUnsaved").as_deref(), Some("0"));
    let window = visual(cx, handle);
    assert!(window.has_pending_prompt(), "the question about the copies");
    window.simulate_prompt_answer("残す");
    window.run_until_parked();
}

#[gpui::test]
fn the_file_page_writes_sakuras_keys_and_keeps_the_rest_of_the_file(cx: &mut TestAppContext) {
    let (ini, handle) = setup(
        cx,
        "[Other]\r\nx=1\r\n[Common]\r\nbBackUpType2_Opt1=120\r\nnFileShareMode=2\r\n",
    );
    let _restore = RestoreEditOptions::new();
    let before = ini.profile();
    run(cx, handle, "F_OPTION_FILE");
    // What the file holds is shown, also where MATSU Editor does nothing with it.
    assert_eq!(
        with_dialog(cx, |d, _, _| d.values().int("file_share_mode")),
        2
    );
    assert!(with_dialog(cx, |d, _, _| d.values().bool("auto_save")));
    assert_eq!(
        with_dialog(cx, |d, _, _| d.values().int("drop_file_num_max")),
        8
    );
    // The number of files is edited only while 「閉じて開く」 is off.
    type_in(cx, "drop_file_num_max", "200");
    check(cx, "drop_file_and_close", true);
    type_in(cx, "drop_file_num_max", "3");
    assert_eq!(
        with_dialog(cx, |d, _, dcx| d.input_text("drop_file_num_max", dcx)).as_deref(),
        Some("200"),
        "the box was not edited after 「閉じて開く」"
    );
    check(cx, "restore_cur_position", false);
    check(cx, "enable_unmodified_overwrite", true);
    check(cx, "auto_mime_decode", true);
    check(cx, "no_filter_save_new", false);
    // A box that is not available does nothing.
    choose(cx, "file_share_mode", 0);
    ok(cx);

    assert_eq!(
        value(&ini, "Common", "bDropFileAndClose").as_deref(),
        Some("1")
    );
    assert_eq!(
        value(&ini, "Common", "nDropFileNumMax").as_deref(),
        Some("99"),
        "a number goes into its range"
    );
    assert_eq!(
        value(&ini, "Common", "m_bRestoreCurPosition").as_deref(),
        Some("0")
    );
    assert_eq!(
        value(&ini, "Common", "m_bRestoreBookmarks").as_deref(),
        Some("1")
    );
    assert_eq!(
        value(&ini, "Common", "bQueryIfCodeChange").as_deref(),
        Some("1")
    );
    assert_eq!(
        value(&ini, "Common", "bEnableUnmodifiedOverwrite").as_deref(),
        Some("1")
    );
    assert_eq!(
        value(&ini, "Common", "nFileShareMode").as_deref(),
        Some("0")
    );
    // The boxes that are not available leave the file as it was.
    assert_eq!(
        value(&ini, "Common", "bBackUpType2_Opt1").as_deref(),
        Some("120")
    );
    assert_eq!(
        value(&ini, "Common", "bAutoMIMEdecode").as_deref(),
        Some("1")
    );
    assert_eq!(
        value(&ini, "Common", "bNoFilterSaveNew").as_deref(),
        Some("0")
    );
    let after = ini.profile();
    let old: Vec<_> = before.entries("Other").collect();
    let new: Vec<_> = after.entries("Other").collect();
    assert_eq!(old, new);
}

#[gpui::test]
fn the_options_of_the_history_reach_the_windows(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    let options = |cx: &mut TestAppContext| cx.update(|app| crate::history::options(app));
    assert!(options(cx).restore_cur_position && options(cx).restore_bookmarks);
    run(cx, handle, "F_OPTION_FILE");
    check(cx, "restore_cur_position", false);
    check(cx, "restore_bookmarks", false);
    check(cx, "query_if_code_change", false);
    cancel(cx);
    assert!(
        options(cx).restore_cur_position,
        "キャンセル changes nothing"
    );
    run(cx, handle, "F_OPTION_FILE");
    check(cx, "restore_cur_position", false);
    check(cx, "restore_bookmarks", false);
    check(cx, "query_if_code_change", false);
    ok(cx);
    let now = options(cx);
    assert!(!now.restore_cur_position && !now.restore_bookmarks && !now.query_if_code_change);
}

#[gpui::test]
fn the_file_page_starts_the_history_options_from_the_file(cx: &mut TestAppContext) {
    let (_ini, _handle) = setup(
        cx,
        "[Common]\r\nm_bRestoreCurPosition=0\r\nbQueryIfCodeChange=0\r\n",
    );
    let now = cx.update(|app| crate::history::options(app));
    assert!(!now.restore_cur_position && now.restore_bookmarks && !now.query_if_code_change);
}

#[gpui::test]
fn an_unmodified_document_is_saved_when_the_setting_says_so(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    let path = write_file(&ini, "a.txt", b"abc");
    run_with(
        cx,
        handle,
        "F_FILEOPEN",
        &[Arg::Text(path.display().to_string())],
    );
    // Sakura's default: nothing is written, and a change behind the editor's back stays.
    std::fs::write(&path, b"changed").unwrap();
    run(cx, handle, "F_FILESAVE");
    assert_eq!(std::fs::read(&path).unwrap(), b"changed");
    run(cx, handle, "F_OPTION_FILE");
    check(cx, "enable_unmodified_overwrite", true);
    ok(cx);
    run(cx, handle, "F_FILESAVE");
    assert_eq!(std::fs::read(&path).unwrap(), b"abc");
    // And off again.
    run(cx, handle, "F_OPTION_FILE");
    check(cx, "enable_unmodified_overwrite", false);
    ok(cx);
    std::fs::write(&path, b"changed").unwrap();
    run(cx, handle, "F_FILESAVE");
    assert_eq!(std::fs::read(&path).unwrap(), b"changed");
}

#[gpui::test]
fn the_options_of_the_watching_the_warnings_and_the_automatic_save_reach_the_windows(
    cx: &mut TestAppContext,
) {
    let (_ini, handle) = setup(
        cx,
        "[Common]\r\nbCheckFileTimeStamp=0\r\nnAutoloadDelay=4\r\nbAlertIfLargeFile=1\r\n\
         nAlertFileSize=20\r\nbBackUpType2_Opt1=120\r\nbBackUpType2_Opt3=9\r\n\
         bAutoMIMEdecode=1\r\nbNoFilterSaveFile=0\r\nnHugeFileSizeMB=512\r\n",
    );
    let started = cx.update(|app| crate::file_ops::prefs::prefs(app));
    assert!(started.auto_mime_decode);
    assert_eq!(started.huge_file_size_mb, 512);
    assert!(started.no_filter_save_new && !started.no_filter_save_file);
    assert!(!started.check_file_time_stamp);
    assert_eq!(started.autoload_delay, 4);
    assert!(started.alert_if_large_file && started.alert_file_size == 20);
    assert!(started.auto_save && started.auto_save_interval == 9);
    run(cx, handle, "F_OPTION_FILE");
    check(cx, "check_file_time_stamp", true);
    check(cx, "uneditable_if_unwritable", false);
    check(cx, "auto_save", false);
    ok(cx);
    let now = cx.update(|app| crate::file_ops::prefs::prefs(app));
    assert!(now.check_file_time_stamp && !now.uneditable_if_unwritable && !now.auto_save);
    assert_eq!(now.autoload_delay, 4, "the numbers stay");
    assert!(
        now.auto_mime_decode && !now.no_filter_save_file,
        "so do the others"
    );
}

// ---- バックアップ ----

#[gpui::test]
fn the_backup_page_shows_the_packed_numbers_and_keeps_the_other_bits(cx: &mut TestAppContext) {
    // `bBackUpType2_Opt1` holds the bits of the date and time (年 32 .. 秒 1) and the bit of the
    // automatic save (64); `_Opt2` the number of generations in the low 16 bits and the first
    // letter of the extension in the next 8.
    let text = "[Common]\r\nbBackUp=1\r\nnBackUpType=3\r\nbBackUpType2_Opt1=35\r\n\
                bBackUpType2_Opt2=6422547\r\nbBackUpType2_Opt3=7\r\nbBackUpType2_Opt4=4\r\n\
                szBackUpFolder=C:\\bak\r\n";
    let (ini, handle) = setup(cx, text);
    run(cx, handle, "F_OPTION_BACKUP");
    let has = |cx: &mut TestAppContext, id: &str| with_dialog(cx, |d, _, _| d.values().bool(id));
    let int = |cx: &mut TestAppContext, id: &str| with_dialog(cx, |d, _, _| d.values().int(id));
    assert!(has(cx, "backup"));
    assert_eq!(int(cx, "backup_type"), 3);
    assert!(has(cx, "backup_year") && !has(cx, "backup_month") && !has(cx, "backup_day"));
    assert!(!has(cx, "backup_hour") && has(cx, "backup_min") && has(cx, "backup_sec"));
    assert_eq!(int(cx, "backup_generations"), 19);
    assert_eq!(int(cx, "backup_advanced_type"), 4);
    assert_eq!(
        with_dialog(cx, |d, _, dcx| d.input_text("backup_folder", dcx)).as_deref(),
        Some("C:\\bak")
    );
    // OK leaves the other bits of the numbers as they are (the automatic save, the letter).
    ok(cx);
    for (key, expected) in [
        ("bBackUp", "1"),
        ("nBackUpType", "3"),
        ("bBackUpType2_Opt1", "35"),
        ("bBackUpType2_Opt2", "6422547"),
        ("bBackUpType2_Opt3", "7"),
        ("bBackUpType2_Opt4", "4"),
        ("szBackUpFolder", "C:\\bak"),
    ] {
        assert_eq!(
            value(&ini, "Common", key).as_deref(),
            Some(expected),
            "{key}"
        );
    }
}

#[gpui::test]
fn the_backup_page_is_live_and_keeps_the_extension_kinds(cx: &mut TestAppContext) {
    let text = "[Common]\r\nbBackUp=1\r\nnBackUpType=6\r\n";
    let (ini, handle) = setup(cx, text);
    run(cx, handle, "F_OPTION_BACKUP");
    let has = |cx: &mut TestAppContext, id: &str| with_dialog(cx, |d, _, _| d.values().bool(id));
    let int = |cx: &mut TestAppContext, id: &str| with_dialog(cx, |d, _, _| d.values().int(id));
    // Kind 6 is the generations with the extension kept.
    assert_eq!(int(cx, "backup_type"), 3);
    assert!(has(cx, "backup_retain_ext"));
    // The kind 1 with the check is 5.
    choose(cx, "backup_type", 1);
    ok(cx);
    assert_eq!(value(&ini, "Common", "nBackUpType").as_deref(), Some("5"));
    // The box of the detailed setting holds the name that the settings make.
    assert_eq!(
        value(&ini, "Common", "szBackUpPathAdvanced").as_deref(),
        Some(".\\$0.*.bak")
    );
    // Off: the controls of the page are not usable and keep their values.
    run(cx, handle, "F_OPTION_BACKUP");
    check(cx, "backup", false);
    choose(cx, "backup_type", 2);
    assert_eq!(int(cx, "backup_type"), 1);
    ok(cx);
    assert_eq!(value(&ini, "Common", "bBackUp").as_deref(), Some("0"));
    assert_eq!(value(&ini, "Common", "nBackUpType").as_deref(), Some("5"));
}

// ---- 書式 ----

#[gpui::test]
fn the_formats_of_the_date_and_the_time_reach_the_commands(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    let _restore = RestoreFormatOptions::new();
    run(cx, handle, "F_OPTION_FORMAT");
    assert_eq!(
        with_dialog(cx, |d, _, _| d.values().int("date_format_type")),
        0
    );
    // The picture is edited only when 「カスタム」 is chosen.
    type_in(cx, "date_format", "ignored");
    assert_eq!(
        with_dialog(cx, |d, _, dcx| d.input_text("date_format", dcx)).as_deref(),
        Some("yyyy'年'M'月'd'日('dddd')'")
    );
    choose(cx, "date_format_type", 1);
    type_in(cx, "date_format", "yyyy/MM/dd");
    choose(cx, "time_format_type", 1);
    type_in(cx, "time_format", "HH:mm:ss");
    ok(cx);
    assert_eq!(
        value(&ini, "Common", "nDateFormatType").as_deref(),
        Some("1")
    );
    assert_eq!(
        value(&ini, "Common", "szDateFormat").as_deref(),
        Some("yyyy/MM/dd")
    );
    assert_eq!(
        value(&ini, "Common", "nTimeFormatType").as_deref(),
        Some("1")
    );
    assert_eq!(
        value(&ini, "Common", "szTimeFormat").as_deref(),
        Some("HH:mm:ss")
    );

    let is_digits = |text: &str, pattern: &str| {
        text.len() == pattern.len()
            && text.chars().zip(pattern.chars()).all(|(c, p)| match p {
                '9' => c.is_ascii_digit(),
                _ => c == p,
            })
    };
    run(cx, handle, "F_INS_DATE");
    let date = first_line(cx, handle);
    assert!(is_digits(&date, "9999/99/99"), "{date}");
    run(cx, handle, "F_INS_TIME");
    let line = first_line(cx, handle);
    assert!(is_digits(&line[date.len()..], "99:99:99"), "{line}");

    // 「標準」 again: Sakura's default picture, whatever the box holds.
    run(cx, handle, "F_OPTION_FORMAT");
    choose(cx, "date_format_type", 0);
    ok(cx);
    assert_eq!(
        value(&ini, "Common", "nDateFormatType").as_deref(),
        Some("0")
    );
    assert_eq!(
        value(&ini, "Common", "szDateFormat").as_deref(),
        Some("yyyy/MM/dd"),
        "the picture stays in the file"
    );
    assert!(format_options::get().date_picture().starts_with("yyyy'年'"));
}

#[gpui::test]
fn the_formats_come_from_the_file_when_the_application_starts(cx: &mut TestAppContext) {
    let _restore = RestoreFormatOptions::new();
    let (_ini, _handle) = setup(
        cx,
        "[Common]\r\nnDateFormatType=1\r\nszDateFormat=d.M.yy\r\nszInyouKigou=# \r\n",
    );
    assert_eq!(format_options::get().date_picture(), "d.M.yy");
    assert_eq!(
        format_options::get().time_picture(),
        matsu_commands::convert::datetime::DEFAULT_TIME_PICTURE
    );
    let quote = cx.update(|app| app.global::<crate::clipboard::QuoteString>().0.clone());
    assert_eq!(quote, "# ");
}

#[gpui::test]
fn the_quote_of_the_page_is_the_quote_of_the_passage_copy(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    visual(cx, handle).simulate_input("abc");
    visual(cx, handle).simulate_keystrokes("shift-left shift-left shift-left");
    run(cx, handle, "F_COPYLINESASPASSAGE");
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert!(
        copied.as_deref().is_some_and(|t| t.starts_with("> abc")),
        "{copied:?}"
    );
    run(cx, handle, "F_OPTION_FORMAT");
    type_in(cx, "quote_string", "| ");
    ok(cx);
    assert_eq!(value(&ini, "Common", "szInyouKigou").as_deref(), Some("| "));
    run(cx, handle, "F_COPYLINESASPASSAGE");
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert!(
        copied.as_deref().is_some_and(|t| t.starts_with("| abc")),
        "{copied:?}"
    );
}

#[gpui::test]
fn a_quote_set_by_a_macro_is_shown_and_kept_by_ok(cx: &mut TestAppContext) {
    // `F_SET_QUOTESTRING` changes the quote of the session, not the file.
    let (ini, handle) = setup(cx, "[Common]\r\nszInyouKigou=# \r\n");
    cx.update(|app| app.set_global(crate::clipboard::QuoteString("| ".to_owned())));
    run(cx, handle, "F_OPTION_GENERAL");
    with_dialog(cx, |d, _, dcx| d.select_page(Page::Format, dcx));
    assert_eq!(
        with_dialog(cx, |d, _, dcx| d.input_text("quote_string", dcx)).as_deref(),
        Some("| ")
    );
    ok(cx);
    assert_eq!(value(&ini, "Common", "szInyouKigou").as_deref(), Some("| "));
    let quote = cx.update(|app| app.global::<crate::clipboard::QuoteString>().0.clone());
    assert_eq!(quote, "| ");
}

#[gpui::test]
fn the_heading_marks_of_the_page_reach_the_outline_of_a_text(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    let marks = |cx: &mut TestAppContext| {
        cx.update(|app| app.global::<crate::outline::HeadingMarks>().0.clone())
    };
    assert_eq!(marks(cx), matsu_outline::text::DEFAULT_HEADING_MARKS);
    run(cx, handle, "F_OPTION_FORMAT");
    type_in(cx, "heading_marks", "*#");
    cancel(cx);
    assert_eq!(marks(cx), matsu_outline::text::DEFAULT_HEADING_MARKS);
    run(cx, handle, "F_OPTION_FORMAT");
    type_in(cx, "heading_marks", "*#");
    ok(cx);
    assert_eq!(marks(cx), "*#");
    assert_eq!(
        value(&ini, "Common", "szMidashiKigou").as_deref(),
        Some("*#")
    );
}

// ---- 検索 ----

#[gpui::test]
fn the_word_at_the_caret_is_the_first_search_string_only_when_the_setting_says_so(
    cx: &mut TestAppContext,
) {
    let (ini, handle) = setup(cx, "");
    visual(cx, handle).simulate_input("foo");
    let seed = |cx: &mut TestAppContext| {
        view_of(cx, handle).read_with(cx, |view, app| view.dialog_seed(app))
    };
    assert_eq!(seed(cx).as_deref(), Some("foo"));
    run(cx, handle, "F_OPTION_GREP");
    check(cx, "caret_text_for_search", false);
    ok(cx);
    assert_eq!(
        value(&ini, "Common", "bCaretTextForSearch").as_deref(),
        Some("0")
    );
    assert_eq!(
        seed(cx),
        None,
        "the dialogs keep the search string they had"
    );
    // A selection is always taken.
    visual(cx, handle).simulate_keystrokes("shift-left shift-left");
    assert_eq!(seed(cx).as_deref(), Some("oo"));
}

#[gpui::test]
fn the_modes_of_the_tags_file_search_reach_the_tag_jumps(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "[Common]\r\nnTagJumpMode=0\r\n");
    let modes = |cx: &mut TestAppContext| {
        cx.update(|app| {
            let prefs = crate::tags::prefs(app);
            (prefs.direct_mode, prefs.keyword_mode)
        })
    };
    assert_eq!(modes(cx), (0, 3), "from the file, and Sakura's default");
    run(cx, handle, "F_OPTION_GREP");
    choose(cx, "tag_jump_mode", 1);
    choose(cx, "tag_jump_mode_keyword", 2);
    ok(cx);
    assert_eq!(value(&ini, "Common", "nTagJumpMode").as_deref(), Some("1"));
    assert_eq!(
        value(&ini, "Common", "nTagJumpModeKeyword").as_deref(),
        Some("2")
    );
    assert_eq!(modes(cx), (1, 2));
}

#[gpui::test]
fn the_search_page_leaves_what_is_not_available_alone(cx: &mut TestAppContext) {
    let (ini, handle) = setup(
        cx,
        "[Common]\r\nszRegexpLib=onig.dll\r\nbGrepRealTime=1\r\nbGTJW_RETURN=0\r\n",
    );
    run(cx, handle, "F_OPTION_GREP");
    type_in(cx, "regexp_lib", "other.dll");
    check(cx, "grep_real_time", false);
    check(cx, "gtjw_return", true);
    check(cx, "inherit_key_other_view", false);
    ok(cx);
    assert_eq!(
        value(&ini, "Common", "szRegexpLib").as_deref(),
        Some("onig.dll")
    );
    assert_eq!(value(&ini, "Common", "bGrepRealTime").as_deref(), Some("1"));
    assert_eq!(value(&ini, "Common", "bGTJW_RETURN").as_deref(), Some("0"));
    assert_eq!(value(&ini, "Common", "m_bInheritKeyOtherView"), None);
}

// ---- ファイル名表示 and ステータスバー ----

#[gpui::test]
fn the_pages_of_the_other_sections_write_their_own_sections(cx: &mut TestAppContext) {
    let (ini, handle) = setup(
        cx,
        "[Nickname]\r\nbShortPath=0\r\nnShortPathMaxWidth=60\r\nArrNum=1\r\nFrom00=%DeskTop%\\\r\n\
         To00=D\r\n[Statusbar]\r\nDispUtf8Codepoint=0\r\nDispColByChar=1\r\n",
    );
    run(cx, handle, "F_OPTION_FNAME");
    assert!(!with_dialog(cx, |d, _, _| d.values().bool("short_path")));
    assert_eq!(
        with_dialog(cx, |d, _, _| d.values().int("short_path_max_width")),
        60
    );
    check(cx, "short_path", true);
    type_in(cx, "short_path_max_width", "10");
    with_dialog(cx, |d, _, dcx| d.select_page(Page::StatusBar, dcx));
    // The keys of 「ステータスバー」 are in [Statusbar], where the defaults are Sakura's.
    assert!(!with_dialog(cx, |d, _, _| d
        .values()
        .bool("disp_utf8_codepoint")));
    assert!(with_dialog(cx, |d, _, _| d
        .values()
        .bool("disp_col_by_char")));
    assert!(with_dialog(cx, |d, _, _| d
        .values()
        .bool("disp_sp_codepoint")));
    assert!(!with_dialog(cx, |d, _, _| d
        .values()
        .bool("disp_sel_count_by_byte")));
    check(cx, "disp_col_by_char", false);
    ok(cx);
    assert_eq!(value(&ini, "Nickname", "bShortPath").as_deref(), Some("1"));
    assert_eq!(
        value(&ini, "Nickname", "nShortPathMaxWidth").as_deref(),
        Some("10")
    );
    assert_eq!(
        value(&ini, "Nickname", "From00").as_deref(),
        Some("%DeskTop%\\")
    );
    // The page is live: the box that was turned off is written, the others stay as they were.
    assert_eq!(
        value(&ini, "Statusbar", "DispColByChar").as_deref(),
        Some("0")
    );
    assert_eq!(
        value(&ini, "Statusbar", "DispUtf8Codepoint").as_deref(),
        Some("0")
    );
    assert_eq!(
        value(&ini, "Statusbar", "DispSurrogatePairCodepoint").as_deref(),
        Some("1")
    );
    assert_eq!(value(&ini, "Common", "DispColByChar"), None);
}

#[gpui::test]
fn cancel_writes_nothing_on_the_new_pages(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "[Common]\r\nbDispSTATUSBAR=1\r\n");
    let bytes = ini.bytes();
    run(cx, handle, "F_OPTION_FILE");
    check(cx, "drop_file_and_close", true);
    check(cx, "restore_bookmarks", false);
    cancel(cx);
    run(cx, handle, "F_OPTION_FORMAT");
    type_in(cx, "quote_string", "x");
    cancel(cx);
    run(cx, handle, "F_OPTION_GREP");
    check(cx, "grep_exit_confirm", true);
    cancel(cx);
    assert_eq!(ini.bytes(), bytes);
    assert!(cx.update(|app| current_dialog(app)).is_none());
    // And the windows keep what they had.
    assert!(
        cx.update(|app| crate::history::options(app))
            .restore_bookmarks
    );
    assert!(
        !cx.update(|app| crate::search::prefs::prefs(app))
            .grep_exit_confirm
    );
}
