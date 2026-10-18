//! Tests of the names (with a fixed time), the generations, and the questions of a save.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use chrono::NaiveDate;
use gpui::{AppContext as _, TestAppContext, VisualTestContext};
use ume_commands::command::Arg;
use ume_commands::sakura::FUNCTIONS;

use super::*;
use crate::document::SharedDocument;
use crate::view::EditorView;

fn time(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(y, mo, d)
        .unwrap()
        .and_hms_opt(h, mi, s)
        .unwrap()
}

fn times() -> Times {
    Times {
        now: time(2026, 10, 18, 9, 5, 7),
        last_write: time(2025, 1, 2, 3, 4, 5),
    }
}

fn path_of(prefs: &BackupPrefs, target: &str) -> String {
    backup_path(prefs, Path::new(target), None, times())
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
}

fn prefs(kind: i32) -> BackupPrefs {
    BackupPrefs {
        enabled: true,
        confirm: false,
        kind,
        ..BackupPrefs::default()
    }
}

// ---- the names ----

#[test]
fn kind_1_and_5_make_bak() {
    // `case 1` and `case 5` of `FormatBackUpPath`
    assert_eq!(path_of(&prefs(1), "/d/a.txt"), "/d/a.bak");
    assert_eq!(path_of(&prefs(5), "/d/a.txt"), "/d/a.txt.bak");
    assert_eq!(path_of(&prefs(1), "/d/日本語.txt"), "/d/日本語.bak");
    assert_eq!(path_of(&prefs(1), "/d/a.b.txt"), "/d/a.b.bak");
    assert_eq!(path_of(&prefs(1), "/d/noext"), "/d/noext.bak");
    assert_eq!(path_of(&prefs(1), "/d/.bashrc"), "/d/.bashrc.bak");
}

#[test]
fn kind_2_adds_the_time_of_the_save() {
    let mut p = prefs(2);
    p.date_bits = YEAR | MONTH | DAY;
    assert_eq!(path_of(&p, "/d/a.txt"), "/d/a_20261018.txt");
    p.date_bits = YEAR | MONTH | DAY | HOUR | MIN | SEC;
    assert_eq!(path_of(&p, "/d/a.txt"), "/d/a_20261018090507.txt");
    p.date_bits = HOUR | MIN;
    assert_eq!(path_of(&p, "/d/メモ.md"), "/d/メモ_0905.md");
    p.date_bits = 0;
    assert_eq!(path_of(&p, "/d/a.txt"), "/d/a_.txt");
}

#[test]
fn kind_4_adds_the_time_of_the_last_save() {
    let mut p = prefs(4);
    p.date_bits = YEAR | MONTH | DAY | HOUR | MIN | SEC;
    assert_eq!(path_of(&p, "/d/a.txt"), "/d/a_20250102030405.txt");
}

#[test]
fn kind_3_and_6_make_a_number() {
    let mut p = prefs(3);
    assert_eq!(path_of(&p, "/d/a.txt"), "/d/a.b00");
    p.kind = 6;
    assert_eq!(path_of(&p, "/d/a.txt"), "/d/a.txt.b00");
    p.ext_char = 'k';
    assert_eq!(path_of(&p, "/d/a.txt"), "/d/a.txt.k00");
}

#[test]
fn the_folder_replaces_the_folder_of_the_file() {
    let p = prefs(1);
    let folder = std::env::temp_dir().join("ume-backup-folder");
    let path = backup_path(&p, Path::new("/d/a.txt"), Some(&folder), times()).unwrap();
    assert_eq!(path, folder.join("a.bak"));
}

#[test]
fn the_detailed_format_has_the_time_the_folders_and_the_extension() {
    let mut p = prefs(1);
    p.advanced = true;
    p.advanced_format = "bak/$1/$0_%Y%m%d-%H%M%S.*".to_owned();
    assert_eq!(
        path_of(&p, "/work/proj/a.txt"),
        "/work/proj/bak/proj/a_20261018-090507.txt"
    );
    // `$2` is two folders up; there is no `$9`; `$` with a letter stays; `?` becomes `_`.
    p.advanced_format = "$2-$9-$x-a?b-%y-%%.*".to_owned();
    assert_eq!(
        path_of(&p, "/work/proj/a.txt"),
        "/work/proj/work--$x-a_b-26-%.txt"
    );
    // The time of the file.
    p.advanced_time = 4;
    p.advanced_format = "%Y%m%d".to_owned();
    assert_eq!(path_of(&p, "/work/a.txt"), "/work/20250102");
    // A trailing `$` and `%` stay or go as in Sakura.
    p.advanced_format = "x$".to_owned();
    assert_eq!(path_of(&p, "/work/a.txt"), "/work/x$");
    p.advanced_format = "x%".to_owned();
    assert_eq!(path_of(&p, "/work/a.txt"), "/work/x");
    // A file without an extension: `*` is empty.
    p.advanced_format = "$0.*.bak".to_owned();
    assert_eq!(path_of(&p, "/work/Makefile"), "/work/Makefile..bak");
}

#[test]
fn the_date_format_is_sakuras() {
    let t = time(2026, 3, 4, 5, 6, 7);
    assert_eq!(
        format_date_time("%Y/%m/%d %H:%M:%S %y", &t),
        "2026/03/04 05:06:07 26"
    );
    assert_eq!(format_date_time("%q%%", &t), "q%");
}

// ---- the files ----

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ume-ui-backup-{}-{n}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, text).unwrap();
        path
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn text(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

#[test]
fn generations_move_up_and_the_oldest_go() {
    let dir = TempDir::new();
    let target = dir.write("a.txt", "v1");
    let mut p = prefs(3);
    p.generations = 3;
    let dest = backup_path(&p, &target, None, times()).unwrap();

    for version in ["v1", "v2", "v3", "v4", "v5"] {
        fs::write(&target, version).unwrap();
        make_backup(&p, &target, &dest, false).unwrap();
    }
    // The newest is .b00; three generations are kept.
    assert_eq!(dir.names(), ["a.b00", "a.b01", "a.b02", "a.txt"]);
    assert_eq!(text(&dir.path("a.b00")), "v5");
    assert_eq!(text(&dir.path("a.b01")), "v4");
    assert_eq!(text(&dir.path("a.b02")), "v3");
}

#[test]
fn one_generation_keeps_only_the_last() {
    let dir = TempDir::new();
    let target = dir.write("a.txt", "v1");
    let mut p = prefs(3);
    p.generations = 1;
    let dest = backup_path(&p, &target, None, times()).unwrap();
    make_backup(&p, &target, &dest, false).unwrap();
    fs::write(&target, "v2").unwrap();
    make_backup(&p, &target, &dest, false).unwrap();
    assert_eq!(dir.names(), ["a.b00", "a.txt"]);
    assert_eq!(text(&dir.path("a.b00")), "v2");
}

#[test]
fn a_backup_makes_the_folder_one_level_deep() {
    let dir = TempDir::new();
    let target = dir.write("a.txt", "abc");
    let p = prefs(1);
    let folder = dir.path("bak");
    let dest = backup_path(&p, &target, Some(&folder), times()).unwrap();
    make_backup(&p, &target, &dest, false).unwrap();
    assert_eq!(text(&folder.join("a.bak")), "abc");
    // Two levels are not made (`CreateDirectory`).
    let deep = dir.path("x").join("y");
    let dest = backup_path(&p, &target, Some(&deep), times()).unwrap();
    assert!(matches!(
        make_backup(&p, &target, &dest, false),
        Err(Failure::Copy(_))
    ));
}

#[test]
fn a_backup_that_is_the_file_itself_is_refused_and_the_file_stays() {
    let dir = TempDir::new();
    // Saving `a.bak` with the kind 1, and `a.b00` with the kind 3.
    for (name, kind) in [("a.bak", 1), ("a.b00", 3)] {
        let target = dir.write(name, "keep");
        let p = prefs(kind);
        let dest = backup_path(&p, &target, None, times()).unwrap();
        assert_eq!(dest, target);
        assert!(matches!(
            make_backup(&p, &target, &dest, false),
            Err(Failure::Copy(_))
        ));
        assert_eq!(text(&target), "keep");
    }
    assert_eq!(dir.names(), ["a.b00", "a.bak"]);
}

#[test]
fn a_file_named_like_a_generation_is_not_rotated_away() {
    let dir = TempDir::new();
    let mut p = prefs(3);
    p.generations = 1;
    // Saving `a.b01`: the destination `a.b00` is not the file, but the rotation would move it.
    let target = dir.write("a.b01", "source");
    let other = dir.write("a.b00", "other");
    let dest = backup_path(&p, &target, None, times()).unwrap();
    assert_eq!(dest, other);
    assert!(matches!(
        make_backup(&p, &target, &dest, false),
        Err(Failure::Copy(_))
    ));
    assert_eq!(text(&target), "source");
    assert_eq!(text(&other), "other");
    assert_eq!(dir.names(), ["a.b00", "a.b01"]);
}

#[test]
fn a_backup_that_is_a_hard_link_to_the_file_does_not_empty_it() {
    let dir = TempDir::new();
    let target = dir.write("a.txt", "precious");
    let p = prefs(1);
    // Another name of the same file: refused, nothing changes.
    let link = dir.path("a.bak");
    fs::hard_link(&target, &link).unwrap();
    let dest = backup_path(&p, &target, None, times()).unwrap();
    assert_eq!(dest, link);
    assert!(matches!(
        make_backup(&p, &target, &dest, false),
        Err(Failure::Copy(_))
    ));
    assert_eq!(text(&target), "precious");
    assert_eq!(text(&link), "precious");

    // A hard link to the file under the name of an old backup: the name is replaced, the
    // contents shared with the file are not written to.
    let other = dir.write("b.txt", "old");
    let old_backup = dir.path("b.bak");
    fs::hard_link(&other, &old_backup).unwrap();
    let dest = backup_path(&p, &other, None, times()).unwrap();
    assert_eq!(dest, old_backup);
    // The file is saved as `b.txt`; its backup `b.bak` shares the data with it.
    assert!(matches!(
        make_backup(&p, &other, &dest, false),
        Err(Failure::Copy(_))
    ));
    assert_eq!(text(&other), "old");
}

#[test]
fn the_copy_replaces_the_name_of_a_backup_and_leaves_no_temporary_file() {
    let dir = TempDir::new();
    let target = dir.write("a.txt", "new");
    // An old backup that another name shares: its data stays as it was.
    let old = dir.write("a.bak", "old");
    let keeper = dir.path("keeper");
    fs::hard_link(&old, &keeper).unwrap();
    let p = prefs(1);
    let dest = backup_path(&p, &target, None, times()).unwrap();
    make_backup(&p, &target, &dest, false).unwrap();
    assert_eq!(text(&dest), "new");
    assert_eq!(text(&keeper), "old");
    assert_eq!(dir.names(), ["a.bak", "a.txt", "keeper"]);
}

#[test]
fn the_detailed_format_that_names_the_file_itself_is_refused() {
    let dir = TempDir::new();
    let target = dir.write("a.txt", "keep");
    let mut p = prefs(3);
    p.advanced = true;
    // `$0.*` is `a.txt`: the destination is the file, and does not end in a generation number.
    p.advanced_format = "$0.*".to_owned();
    let dest = backup_path(&p, &target, None, times()).unwrap();
    assert_eq!(dest, target);
    assert!(matches!(
        make_backup(&p, &target, &dest, false),
        Err(Failure::Copy(_))
    ));
    assert_eq!(text(&target), "keep");
    assert_eq!(dir.names(), ["a.txt"]);
}

#[test]
fn a_failed_copy_changes_no_generation() {
    let dir = TempDir::new();
    let mut p = prefs(3);
    p.generations = 2;
    let b00 = dir.write("a.b00", "one");
    let b01 = dir.write("a.b01", "two");
    // The file to back up is gone: the copy fails before the generations are touched.
    let target = dir.path("a.txt");
    let dest = backup_path(&p, &target, None, times()).unwrap();
    assert!(matches!(
        make_backup(&p, &target, &dest, false),
        Err(Failure::Copy(_))
    ));
    assert_eq!(text(&b00), "one");
    assert_eq!(text(&b01), "two");
    assert_eq!(dir.names(), ["a.b00", "a.b01"]);
}

#[test]
fn a_name_near_the_limit_still_gets_its_backup() {
    let dir = TempDir::new();
    // 250 bytes: the backup `.bak` is shorter than the file name's limit of 255.
    let name = format!("{}.txt", "あ".repeat(80));
    assert_eq!(name.len(), 244);
    let target = dir.write(&name, "abc");
    let p = prefs(1);
    let dest = backup_path(&p, &target, None, times()).unwrap();
    make_backup(&p, &target, &dest, false).unwrap();
    assert_eq!(text(&dest), "abc");
    assert_eq!(dir.names().len(), 2, "{:?}", dir.names());
}

#[test]
fn the_temporary_file_is_made_exclusively() {
    let dir = TempDir::new();
    let target = dir.write("a.txt", "abc");
    let dest = dir.path("a.bak");
    let first = copy_to_temporary(&target, &dest).unwrap();
    let second = copy_to_temporary(&target, &dest).unwrap();
    assert_ne!(first, second);
    assert_eq!(text(&first), "abc");
    assert!(first.file_name().unwrap().to_string_lossy().len() < 60);
}

#[test]
fn a_folder_that_is_a_file_fails_the_copy() {
    let dir = TempDir::new();
    let target = dir.write("a.txt", "abc");
    let blocker = dir.write("blocker", "x");
    let p = prefs(1);
    let dest = backup_path(&p, &target, Some(&blocker), times()).unwrap();
    assert!(matches!(
        make_backup(&p, &target, &dest, false),
        Err(Failure::Copy(_))
    ));
}

#[test]
fn prepare_decides_before_asking() {
    let dir = TempDir::new();
    let target = dir.write("a.txt", "abc");
    let now = times().now;
    let local = |_: &Path| true;
    let remote = |_: &Path| false;
    let none = Path::new("");
    let folder = dir.path("bak");

    // Off, or a new file: nothing.
    let mut p = prefs(1);
    p.enabled = false;
    assert_eq!(prepare(&p, &target, none, &local, now), Prepared::Skip);
    p.enabled = true;
    assert_eq!(
        prepare(&p, &dir.path("new.txt"), none, &local, now),
        Prepared::Skip
    );
    assert_eq!(
        prepare(&p, &target, none, &local, now),
        Prepared::Make {
            dest: dir.path("a.bak"),
            to_trash: false
        }
    );

    // The recycle bin only on a local drive.
    p.dust_box = true;
    assert!(matches!(
        prepare(&p, &target, none, &local, now),
        Prepared::Make { to_trash: true, .. }
    ));
    assert!(matches!(
        prepare(&p, &target, none, &remote, now),
        Prepared::Make {
            to_trash: false,
            ..
        }
    ));

    // The folder; with "removable only" it is used for a file that is not on a local drive.
    p.dust_box = false;
    p.use_folder = true;
    assert_eq!(
        prepare(&p, &target, &folder, &local, now),
        Prepared::Make {
            dest: folder.join("a.bak"),
            to_trash: false
        }
    );
    p.folder_removable_only = true;
    assert_eq!(
        prepare(&p, &target, &folder, &local, now),
        Prepared::Make {
            dest: dir.path("a.bak"),
            to_trash: false
        }
    );
    assert_eq!(
        prepare(&p, &target, &folder, &remote, now),
        Prepared::Make {
            dest: folder.join("a.bak"),
            to_trash: false
        }
    );

    // A read-only file is not backed up (`_waccess`).
    let mut permissions = fs::metadata(&target).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&target, permissions.clone()).unwrap();
    assert_eq!(prepare(&p, &target, none, &local, now), Prepared::Skip);
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    fs::set_permissions(&target, permissions).unwrap();
}

// ---- the save ----

const YES_BUTTON: &str = "はい(Y)";
const NO_BUTTON: &str = "いいえ(N)";
const CANCEL_BUTTON: &str = "キャンセル";

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

fn setup(
    cx: &mut TestAppContext,
    settings: BackupPrefs,
) -> (gpui::Entity<EditorView>, &mut VisualTestContext) {
    cx.update(gpui_component::init);
    cx.update(|app| app.set_global(settings));
    let shared = cx.new(|_| SharedDocument::empty());
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    (view, vcx)
}

fn run(view: &gpui::Entity<EditorView>, cx: &mut VisualTestContext, function: &str, args: &[Arg]) {
    let code = code_of(function);
    view.update_in(cx, |view, window, cx| {
        view.run_function(code, args, window, cx)
    });
    cx.run_until_parked();
}

fn open(view: &gpui::Entity<EditorView>, cx: &mut VisualTestContext, path: &Path) {
    // 4 is UTF-8.
    run(
        view,
        cx,
        "F_FILEOPEN",
        &[Arg::Text(path.display().to_string()), Arg::Int(4)],
    );
}

fn answer(cx: &mut VisualTestContext, button: &str) {
    assert!(cx.has_pending_prompt(), "no prompt to answer {button:?}");
    cx.simulate_prompt_answer(button);
    cx.run_until_parked();
}

#[gpui::test]
fn a_save_makes_the_backup_without_asking(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("日本語.txt", "old");
    let (view, cx) = setup(cx, prefs(1));
    open(&view, cx, &path);
    cx.simulate_input("new ");
    run(&view, cx, "F_FILESAVE", &[]);
    assert!(!cx.has_pending_prompt());
    assert_eq!(text(&path), "new old");
    assert_eq!(text(&dir.path("日本語.bak")), "old");
}

#[gpui::test]
fn it_does_nothing_with_the_backup_off(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "old");
    let mut p = prefs(1);
    p.enabled = false;
    let (view, cx) = setup(cx, p);
    open(&view, cx, &path);
    cx.simulate_input("new ");
    run(&view, cx, "F_FILESAVE", &[]);
    assert_eq!(dir.names(), ["a.txt"]);
}

#[gpui::test]
fn the_question_yes_no_and_cancel(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "old");
    let mut p = prefs(1);
    p.confirm = true;
    let (view, cx) = setup(cx, p);
    open(&view, cx, &path);
    cx.simulate_input("1 ");

    // Cancel: the save stops.
    run(&view, cx, "F_FILESAVE", &[]);
    let (message, detail) = cx.pending_prompt().unwrap();
    assert!(
        message.contains("バックアップファイルを作成します"),
        "{message}"
    );
    assert!(detail.contains("a.bak"), "{detail}");
    answer(cx, CANCEL_BUTTON);
    assert_eq!(text(&path), "old");
    assert_eq!(dir.names(), ["a.txt"]);

    // No: saved without the backup.
    run(&view, cx, "F_FILESAVE", &[]);
    answer(cx, NO_BUTTON);
    assert_eq!(text(&path), "1 old");
    assert_eq!(dir.names(), ["a.txt"]);

    // Yes: both.
    cx.simulate_input("2 ");
    run(&view, cx, "F_FILESAVE", &[]);
    answer(cx, YES_BUTTON);
    assert_eq!(text(&path), "1 2 old");
    assert_eq!(text(&dir.path("a.bak")), "1 old");
}

#[gpui::test]
fn a_new_file_has_no_backup(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.path("new.txt");
    let (view, cx) = setup(cx, prefs(1));
    cx.simulate_input("abc");
    run(
        &view,
        cx,
        "F_FILESAVEAS",
        &[Arg::Text(path.display().to_string())],
    );
    assert_eq!(dir.names(), ["new.txt"]);
}

#[gpui::test]
fn a_backup_that_fails_asks_and_the_save_still_writes(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "old");
    let blocker = dir.write("blocker", "x");
    let mut p = prefs(1);
    p.use_folder = true;
    p.folder = blocker.display().to_string();
    let (view, cx) = setup(cx, p);
    open(&view, cx, &path);
    cx.simulate_input("1 ");

    // No: the save stops.
    run(&view, cx, "F_FILESAVE", &[]);
    let (message, _) = cx.pending_prompt().unwrap();
    assert!(message.contains("バックアップの作成に失敗"), "{message}");
    answer(cx, NO_BUTTON);
    assert_eq!(text(&path), "old");

    // Yes: the file is written without the backup.
    run(&view, cx, "F_FILESAVE", &[]);
    answer(cx, YES_BUTTON);
    assert_eq!(text(&path), "1 old");
    assert_eq!(dir.names(), ["a.txt", "blocker"]);
}

#[gpui::test]
fn the_date_kind_makes_a_name_with_the_date(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "old");
    let mut p = prefs(2);
    p.date_bits = YEAR | MONTH | DAY | HOUR | MIN | SEC;
    let (view, cx) = setup(cx, p);
    open(&view, cx, &path);
    cx.simulate_input("1 ");
    run(&view, cx, "F_FILESAVE", &[]);
    let names = dir.names();
    assert_eq!(names.len(), 2, "{names:?}");
    let backup = names.iter().find(|n| *n != "a.txt").unwrap();
    assert!(
        backup.starts_with("a_") && backup.ends_with(".txt"),
        "{backup}"
    );
    assert_eq!(backup.len(), "a_20261018090507.txt".len());
    assert_eq!(text(&dir.path(backup)), "old");
}
