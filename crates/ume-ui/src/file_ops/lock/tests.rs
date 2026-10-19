//! Tests of the lock of the open file: the state of a document, and the flows (open, save, backup,
//! reload, close, view mode) in GPUI's headless test window with real files.
//!
//! The "other program" is a second [`FileLock`] taken by the test: on every OS it conflicts with
//! the lock of the document the way the OS defines it (the sharing violation on Windows, `flock`
//! on macOS and Linux).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use ume_commands::sakura::FUNCTIONS;
use ume_platform::file_lock::{FileLock, ShareMode};

use super::*;
use crate::document::SharedDocument;
use crate::file_ops::backup::BackupPrefs;
use crate::file_ops::prefs::FilePrefs;

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ume-ui-lock-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn setup(cx: &mut TestAppContext, share_mode: i32) -> (Entity<EditorView>, &mut VisualTestContext) {
    cx.update(gpui_component::init);
    cx.update(|app| {
        app.set_global(FilePrefs {
            file_share_mode: share_mode,
            ..FilePrefs::default()
        })
    });
    let shared = cx.new(|_| SharedDocument::empty());
    cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    })
}

fn open(view: &Entity<EditorView>, cx: &mut VisualTestContext, path: &Path) {
    let path = path.to_owned();
    view.update_in(cx, |view, window, cx| {
        view.load_path_here(path, None, window, cx)
    });
    cx.run_until_parked();
}

fn run(view: &Entity<EditorView>, cx: &mut VisualTestContext, function: &str) {
    let code = FUNCTIONS.iter().find(|f| f.name == function).unwrap().code;
    view.update_in(cx, |view, window, cx| {
        view.run_function(code, &[], window, cx)
    });
    cx.run_until_parked();
}

fn held(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> Option<ShareMode> {
    view.read_with(cx, |view, app| view.document().read(app).held_lock_mode())
}

fn text(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> String {
    view.read_with(cx, |view, app| {
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

/// Lets go of the lock, so that the test can read the file (Windows refuses a read while the
/// lock denies it).
fn unlock(view: &Entity<EditorView>, cx: &mut VisualTestContext) {
    view.update(cx, |view, cx| {
        view.document().update(cx, |shared, _| shared.unlock_file())
    });
}

/// Whether another program could lock the file in both modes.
fn free(path: &Path) -> bool {
    FileLock::acquire(path, ShareMode::DenyReadWrite).is_ok()
}

#[test]
fn the_setting_numbers_are_sakuras_modes() {
    assert_eq!(share_mode(0), None);
    assert_eq!(share_mode(1), Some(ShareMode::DenyWrite));
    assert_eq!(share_mode(2), Some(ShareMode::DenyReadWrite));
    assert_eq!(share_mode(3), None);
}

#[test]
fn the_failure_message_is_sakuras() {
    let failure = LockFailure {
        path: PathBuf::from("/p/日本語.txt"),
        mode: ShareMode::DenyWrite,
    };
    assert_eq!(
        failure.message(),
        "/p/日本語.txt\nを上書きを禁止するでロックできませんでした。\n現在このファイルに対する排他制御は無効となります。"
    );
    let failure = LockFailure {
        mode: ShareMode::DenyReadWrite,
        ..failure
    };
    assert!(failure.message().contains("を読み書きを禁止するでロック"));
}

#[test]
fn the_file_property_tells_the_lock() {
    assert!(property_line(None).contains("ロックしていません"));
    assert!(property_line(Some(ShareMode::DenyWrite)).contains("上書き禁止モード"));
    assert!(property_line(Some(ShareMode::DenyReadWrite)).contains("読み書き禁止モード"));
}

#[test]
fn nothing_is_locked_without_a_file_a_mode_or_a_file_that_exists() {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let mut state = FileLockState::default();
    state.lock(None, 1);
    assert!(!state.is_locked());
    state.lock(Some(&path), 0);
    assert!(!state.is_locked());
    state.lock(Some(&dir.0.join("missing.txt")), 2);
    assert!(!state.is_locked());
    // None of these is a failure to tell.
    assert_eq!(state.take_failure(), None);
    // The setting is remembered even when nothing was locked.
    assert_eq!(state.mode(), 2);
    state.lock(Some(&path), 2);
    assert_eq!(state.held_mode(), Some(ShareMode::DenyReadWrite));
    state.release();
    assert!(!state.is_locked());
    assert!(free(&path));
}

#[gpui::test]
fn the_mode_that_denies_writing_locks_the_file_that_is_opened(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, 1);
    open(&view, cx, &path);
    assert_eq!(text(&view, cx), "abc");
    assert_eq!(held(&view, cx), Some(ShareMode::DenyWrite));
    // Another program can lock to read, not to write.
    assert!(FileLock::acquire(&path, ShareMode::DenyWrite).is_ok());
    assert!(FileLock::acquire(&path, ShareMode::DenyReadWrite).is_err());
}

#[gpui::test]
fn the_mode_that_denies_reading_locks_the_file_that_is_opened(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, 2);
    open(&view, cx, &path);
    assert_eq!(held(&view, cx), Some(ShareMode::DenyReadWrite));
    assert!(FileLock::acquire(&path, ShareMode::DenyWrite).is_err());
    assert!(FileLock::acquire(&path, ShareMode::DenyReadWrite).is_err());
}

#[gpui::test]
fn without_exclusive_control_the_file_is_not_locked(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, 0);
    open(&view, cx, &path);
    assert_eq!(held(&view, cx), None);
    assert!(free(&path));
}

fn save_while_locked(cx: &mut TestAppContext, number: i32, mode: ShareMode) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, number);
    open(&view, cx, &path);
    cx.simulate_input("x");
    run(&view, cx, "F_FILESAVE");
    assert!(!cx.has_pending_prompt());
    assert_eq!(held(&view, cx), Some(mode), "locked again after the save");
    assert!(FileLock::acquire(&path, ShareMode::DenyReadWrite).is_err());
    unlock(&view, cx);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "xabc");
}

#[gpui::test]
fn a_save_works_while_writing_is_denied_and_locks_the_file_again(cx: &mut TestAppContext) {
    save_while_locked(cx, 1, ShareMode::DenyWrite);
}

#[gpui::test]
fn a_save_works_while_reading_is_denied_and_locks_the_file_again(cx: &mut TestAppContext) {
    save_while_locked(cx, 2, ShareMode::DenyReadWrite);
}

#[gpui::test]
fn saving_under_another_name_locks_the_new_file(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let other = dir.0.join("b.txt");
    let (view, cx) = setup(cx, 2);
    open(&view, cx, &path);
    cx.simulate_input("x");
    let target = other.clone();
    view.update_in(cx, |_, window, cx| {
        crate::file_ops::start(window, cx, async move |view, cx| {
            let request = crate::file_ops::save::SaveRequest {
                path: Some(target),
                ..Default::default()
            };
            crate::file_ops::save::save(&view, cx, request).await;
        })
    });
    cx.run_until_parked();
    assert!(FileLock::acquire(&other, ShareMode::DenyReadWrite).is_err());
    // The old file is let go of.
    assert!(free(&path));
    unlock(&view, cx);
    assert_eq!(std::fs::read_to_string(&other).unwrap(), "xabc");
}

#[gpui::test]
fn the_backup_is_made_while_the_file_is_locked(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "old");
    let (view, cx) = setup(cx, 2);
    cx.update(|_, app| {
        app.set_global(BackupPrefs {
            enabled: true,
            confirm: false,
            kind: 1,
            ..BackupPrefs::default()
        })
    });
    open(&view, cx, &path);
    cx.simulate_input("new");
    run(&view, cx, "F_FILESAVE");
    assert!(!cx.has_pending_prompt());
    assert_eq!(std::fs::read_to_string(dir.0.join("a.bak")).unwrap(), "old");
    assert_eq!(held(&view, cx), Some(ShareMode::DenyReadWrite));
    unlock(&view, cx);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "newold");
}

#[gpui::test]
fn put_file_to_the_locked_file_works_and_locks_it_again(cx: &mut TestAppContext) {
    use ume_commands::command::Arg;
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, 2);
    open(&view, cx, &path);
    let code = FUNCTIONS
        .iter()
        .find(|f| f.name == "F_PUTFILE")
        .unwrap()
        .code;
    let target = Arg::Text(path.display().to_string());
    cx.simulate_input("x");
    view.update_in(cx, |view, window, cx| {
        view.run_function(code, &[target, Arg::Int(4)], window, cx)
    });
    cx.run_until_parked();
    assert!(!cx.has_pending_prompt());
    assert_eq!(held(&view, cx), Some(ShareMode::DenyReadWrite));
    assert!(!free(&path));
    unlock(&view, cx);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "xabc");
}

#[gpui::test]
fn reading_the_same_file_again_works_and_keeps_the_lock(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, 2);
    open(&view, cx, &path);
    // Another program wrote it meanwhile (a lock that is advisory does not stop that).
    #[cfg(unix)]
    std::fs::write(&path, "changed").unwrap();
    // Read again: the file is read through its own lock.
    open(&view, cx, &path);
    #[cfg(unix)]
    assert_eq!(text(&view, cx), "changed");
    #[cfg(windows)]
    assert_eq!(text(&view, cx), "abc");
    assert_eq!(held(&view, cx), Some(ShareMode::DenyReadWrite));
    assert!(!free(&path));
}

#[gpui::test]
fn closing_the_file_releases_the_lock(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, 2);
    open(&view, cx, &path);
    assert!(!free(&path));
    run(&view, cx, "F_FILECLOSE");
    assert_eq!(held(&view, cx), None);
    assert!(free(&path));
}

#[gpui::test]
fn another_file_in_the_window_releases_the_first_one(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let a = dir.write("a.txt", "abc");
    let b = dir.write("b.txt", "def");
    let (view, cx) = setup(cx, 1);
    open(&view, cx, &a);
    open(&view, cx, &b);
    assert_eq!(text(&view, cx), "def");
    assert!(FileLock::acquire(&a, ShareMode::DenyReadWrite).is_ok());
    assert!(FileLock::acquire(&b, ShareMode::DenyReadWrite).is_err());
}

#[gpui::test]
fn view_mode_releases_the_lock_and_leaving_it_locks_again(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, 2);
    open(&view, cx, &path);
    assert_eq!(held(&view, cx), Some(ShareMode::DenyReadWrite));
    run(&view, cx, "F_VIEWMODE");
    assert_eq!(held(&view, cx), None);
    assert!(free(&path));
    run(&view, cx, "F_VIEWMODE");
    assert_eq!(held(&view, cx), Some(ShareMode::DenyReadWrite));
}

#[gpui::test]
fn a_changed_setting_locks_in_the_new_mode(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, 1);
    open(&view, cx, &path);
    let change = |view: &Entity<EditorView>, cx: &mut VisualTestContext, number| {
        view.update(cx, |view, cx| {
            view.document()
                .update(cx, |shared, _| shared.relock_checking_writable(number))
        })
    };
    change(&view, cx, 2);
    assert_eq!(held(&view, cx), Some(ShareMode::DenyReadWrite));
    change(&view, cx, 0);
    assert_eq!(held(&view, cx), None);
    assert!(free(&path));
    change(&view, cx, 1);
    assert_eq!(held(&view, cx), Some(ShareMode::DenyWrite));
}

#[gpui::test]
fn a_file_that_cannot_be_written_is_not_locked(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&path, permissions.clone()).unwrap();
    let (view, cx) = setup(cx, 2);
    open(&view, cx, &path);
    assert_eq!(held(&view, cx), None);
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    std::fs::set_permissions(&path, permissions).unwrap();
}

/// A lock that another program holds cannot be taken. Only where the other lock does not also
/// stop the reading of the file (macOS and Linux): on Windows the file does not even open.
#[cfg(unix)]
#[gpui::test]
fn a_lock_that_fails_is_told_and_the_file_stays_open_unlocked(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let _other = FileLock::acquire(&path, ShareMode::DenyReadWrite).unwrap();
    let (view, cx) = setup(cx, 1);
    open(&view, cx, &path);
    assert_eq!(text(&view, cx), "abc");
    assert_eq!(held(&view, cx), None);
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
    assert!(!cx.has_pending_prompt());
}
