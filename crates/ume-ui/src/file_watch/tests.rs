//! Tests of the watching of files, the automatic save and the warnings before a file is opened,
//! in GPUI's headless test window with real files in a private temporary folder.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime};

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use ume_core::file::{SaveOptions, no_progress};

use super::check::check;
use super::state::{WatchMode, modified_time};
use crate::document::SharedDocument;
use crate::file_ops::prefs::FilePrefs;
use crate::view::EditorView;

const RELOAD: &str = "再読込(R)";
const CLOSE: &str = "閉じる(C)";
const AUTO_LOAD: &str = "以後未編集で再読込(L)";
const NOTIFY_ONLY: &str = "以後通知メッセージのみ(M)";
const NO_SUPERVISION: &str = "以後更新を監視しない(N)";

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ume-ui-watch-{}-{n}", std::process::id()));
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

/// Writes `text` to the file and gives it a write time that differs from the old one (two writes
/// in the same instant would look the same).
fn change(path: &Path, text: &str) {
    let before = modified_time(path).unwrap_or(SystemTime::UNIX_EPOCH);
    std::fs::write(path, text).unwrap();
    let file = std::fs::File::options().write(true).open(path).unwrap();
    file.set_modified(before + Duration::from_secs(10)).unwrap();
}

fn setup(cx: &mut TestAppContext) -> (Entity<EditorView>, &mut VisualTestContext) {
    cx.update(gpui_component::init);
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

/// One look at the file, as the timer of the window does it.
fn look(view: &Entity<EditorView>, cx: &mut VisualTestContext) {
    view.update_in(cx, |_, window, cx| {
        crate::file_ops::start(window, cx, async move |view, cx| check(&view, cx).await)
    });
    cx.run_until_parked();
}

/// One automatic save of the window's document, as the timer does it.
async fn save_if_needed(view: &gpui::WeakEntity<EditorView>, cx: &mut gpui::AsyncWindowContext) {
    let prefs = cx
        .update(|_, app| crate::file_ops::prefs::prefs(app))
        .unwrap();
    let begun = view
        .update(cx, |view, cx| {
            view.document()
                .update(cx, |shared, _| super::auto_save::begin(shared, &prefs))
        })
        .unwrap();
    if begun {
        super::auto_save::run(view, cx).await;
    }
}

fn text(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> String {
    view.read_with(cx, |view, app| {
        let doc = view.document().read(app).doc();
        doc.line(0)
            .ok()
            .flatten()
            .map(|l| l.text)
            .unwrap_or_default()
    })
}

fn mode(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> WatchMode {
    view.read_with(cx, |view, app| view.document().read(app).disk().mode)
}

fn answer(cx: &mut VisualTestContext, button: &str) {
    assert!(cx.has_pending_prompt(), "no prompt to answer {button:?}");
    cx.simulate_prompt_answer(button);
    cx.run_until_parked();
}

fn type_text(_view: &Entity<EditorView>, cx: &mut VisualTestContext, s: &str) {
    cx.simulate_input(s);
}

#[gpui::test]
fn a_file_that_nobody_changed_is_left_alone(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "one\n");
    let (view, cx) = setup(cx);
    open(&view, cx, &path);
    look(&view, cx);
    assert!(!cx.has_pending_prompt());
    assert_eq!(text(&view, cx), "one");
}

#[gpui::test]
fn a_changed_file_asks_and_reloads(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "one\n");
    let (view, cx) = setup(cx);
    open(&view, cx, &path);
    change(&path, "two\n");
    look(&view, cx);
    let (message, detail) = cx.pending_prompt().expect("the question");
    assert!(message.contains("a.txt"));
    assert_eq!(detail, "再ロードしますか?");
    answer(cx, RELOAD);
    assert_eq!(text(&view, cx), "two");
    // the reloaded file is the new stamp: no second question
    look(&view, cx);
    assert!(!cx.has_pending_prompt());
    assert_eq!(mode(&view, cx), WatchMode::Query);
}

#[gpui::test]
fn closing_the_question_keeps_the_text_and_asks_no_more_for_this_change(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "one\n");
    let (view, cx) = setup(cx);
    open(&view, cx, &path);
    change(&path, "two\n");
    look(&view, cx);
    answer(cx, CLOSE);
    assert_eq!(text(&view, cx), "one");
    look(&view, cx);
    assert!(!cx.has_pending_prompt(), "the new stamp was taken");
    change(&path, "three\n");
    look(&view, cx);
    assert!(cx.has_pending_prompt(), "a later change asks again");
}

#[gpui::test]
fn a_modified_document_warns_that_the_changes_are_lost(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "one\n");
    let (view, cx) = setup(cx);
    open(&view, cx, &path);
    type_text(&view, cx, "x");
    change(&path, "two\n");
    look(&view, cx);
    let (_, detail) = cx.pending_prompt().unwrap();
    assert!(detail.contains("変更が失われます"));
    answer(cx, RELOAD);
    assert_eq!(text(&view, cx), "two");
}

#[gpui::test]
fn notify_only_says_it_in_the_status_bar(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "one\n");
    let (view, cx) = setup(cx);
    open(&view, cx, &path);
    change(&path, "two\n");
    look(&view, cx);
    answer(cx, NOTIFY_ONLY);
    assert_eq!(mode(&view, cx), WatchMode::Notify);
    change(&path, "three\n");
    look(&view, cx);
    assert!(!cx.has_pending_prompt());
    let notice = view.read_with(cx, |v, _| v.notice.clone()).unwrap();
    assert!(notice.starts_with("★ファイル更新 "), "{notice}");
    assert_eq!(text(&view, cx), "one", "nothing was reloaded");
}

#[gpui::test]
fn no_supervision_stops_the_watching(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "one\n");
    let (view, cx) = setup(cx);
    open(&view, cx, &path);
    change(&path, "two\n");
    look(&view, cx);
    answer(cx, NO_SUPERVISION);
    change(&path, "three\n");
    look(&view, cx);
    assert!(!cx.has_pending_prompt());
    assert_eq!(text(&view, cx), "one");
}

#[gpui::test]
fn auto_load_reloads_an_unmodified_document_without_asking(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "one\n");
    let (view, cx) = setup(cx);
    open(&view, cx, &path);
    change(&path, "two\n");
    look(&view, cx);
    answer(cx, AUTO_LOAD);
    assert_eq!(text(&view, cx), "two");
    assert_eq!(
        mode(&view, cx),
        WatchMode::AutoLoad,
        "the new document keeps the mode"
    );
    change(&path, "three\n");
    look(&view, cx);
    assert!(!cx.has_pending_prompt());
    assert_eq!(text(&view, cx), "three");
    // a modified document is asked about even in this mode
    type_text(&view, cx, "x");
    change(&path, "four\n");
    look(&view, cx);
    assert!(cx.has_pending_prompt());
    answer(cx, CLOSE);
    assert_eq!(mode(&view, cx), WatchMode::Query);
}

#[gpui::test]
fn the_delay_counts_the_looks_between_automatic_reloads(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "one\n");
    let (view, cx) = setup(cx);
    cx.update(|_, app| {
        app.set_global(FilePrefs {
            autoload_delay: 3,
            ..FilePrefs::default()
        })
    });
    open(&view, cx, &path);
    view.update(cx, |view, cx| {
        view.document().update(cx, |shared, _| {
            shared.disk_mut().mode = WatchMode::AutoLoad;
        })
    });
    change(&path, "two\n");
    look(&view, cx);
    look(&view, cx);
    assert_eq!(text(&view, cx), "one", "two looks of three");
    look(&view, cx);
    assert_eq!(text(&view, cx), "two");
}

#[gpui::test]
fn our_own_save_is_not_a_change(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "one\n");
    let (view, cx) = setup(cx);
    open(&view, cx, &path);
    type_text(&view, cx, "x");
    view.update(cx, |view, cx| {
        view.save_now(
            path.clone(),
            &SaveOptions::keep(view.document().read(cx).file()),
            cx,
        )
        .unwrap();
    });
    look(&view, cx);
    assert!(!cx.has_pending_prompt());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "xone\n");
}

#[gpui::test]
fn the_setting_and_the_lock_turn_the_watching_off(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "one\n");
    let (view, cx) = setup(cx);
    open(&view, cx, &path);
    change(&path, "two\n");
    cx.update(|_, app| {
        app.set_global(FilePrefs {
            check_file_time_stamp: false,
            ..FilePrefs::default()
        })
    });
    look(&view, cx);
    assert!(!cx.has_pending_prompt(), "更新の監視 is off");
    cx.update(|_, app| {
        app.set_global(FilePrefs {
            file_share_mode: 1,
            ..FilePrefs::default()
        })
    });
    look(&view, cx);
    assert!(!cx.has_pending_prompt(), "the file is locked");
    cx.update(|_, app| app.set_global(FilePrefs::default()));
    look(&view, cx);
    assert!(
        cx.has_pending_prompt(),
        "watching again: the change is still there"
    );
}

#[gpui::test]
fn a_document_without_a_file_or_a_missing_file_is_not_watched(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let (view, cx) = setup(cx);
    look(&view, cx);
    assert!(!cx.has_pending_prompt());
    let path = dir.0.join("new.txt");
    open(&view, cx, &path);
    std::fs::write(&path, "made elsewhere").unwrap();
    look(&view, cx);
    assert!(
        !cx.has_pending_prompt(),
        "the stamp of a file that was not there is zero"
    );
}

#[gpui::test]
fn saving_under_another_name_goes_back_to_asking(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "one\n");
    let other = dir.0.join("b.txt");
    let (view, cx) = setup(cx);
    open(&view, cx, &path);
    view.update(cx, |view, cx| {
        view.document().update(cx, |shared, _| {
            shared.disk_mut().mode = WatchMode::None;
        })
    });
    view.update(cx, |view, cx| {
        view.document().update(cx, |shared, _| {
            let options = SaveOptions::keep(shared.file());
            shared
                .save_as(other.clone(), &options, &mut no_progress)
                .unwrap();
        })
    });
    assert_eq!(mode(&view, cx), WatchMode::Query);
    assert!(modified_time(&other).is_some());
}

#[gpui::test]
fn a_file_that_cannot_be_written_opens_without_editing(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "one\n");
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&path, permissions.clone()).unwrap();
    let (view, cx) = setup(cx);
    open(&view, cx, &path);
    let title = view.read_with(cx, |v, app| v.title(app));
    assert!(title.contains("(上書き禁止)"), "{title}");
    type_text(&view, cx, "x");
    assert_eq!(text(&view, cx), "one", "editing is refused");
    // with the option off the file is edited (and saving shows the error)
    cx.update(|_, app| {
        app.set_global(FilePrefs {
            uneditable_if_unwritable: false,
            ..FilePrefs::default()
        })
    });
    type_text(&view, cx, "x");
    assert_eq!(text(&view, cx), "xone");
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    std::fs::set_permissions(&path, permissions).unwrap();
}

#[gpui::test]
fn the_warning_for_a_missing_file_is_asked_only_when_chosen(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.0.join("new.txt");
    let (view, cx) = setup(cx);
    open(&view, cx, &path);
    assert!(!cx.has_pending_prompt(), "off by default");
    cx.update(|_, app| {
        app.set_global(FilePrefs {
            alert_if_file_not_exist: true,
            ..FilePrefs::default()
        })
    });
    let other = dir.0.join("other.txt");
    open(&view, cx, &other);
    let (message, _) = cx.pending_prompt().expect("the warning");
    assert!(message.contains("other.txt"));
    assert!(message.contains("存在しません"));
    answer(cx, "OK");
}

#[gpui::test]
fn a_big_file_asks_before_it_is_opened(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("big.txt", &"x".repeat(1 << 20));
    let (view, cx) = setup(cx);
    cx.update(|_, app| {
        app.set_global(FilePrefs {
            alert_if_large_file: true,
            alert_file_size: 1,
            ..FilePrefs::default()
        })
    });
    open(&view, cx, &path);
    let (message, _) = cx.pending_prompt().expect("the question");
    assert!(message.contains("1MB以上"));
    answer(cx, "開かない(N)");
    assert_eq!(text(&view, cx), "", "not opened");
    open(&view, cx, &path);
    answer(cx, "開く(Y)");
    assert_eq!(text(&view, cx).len(), 1 << 20);
    // a small file is not asked about
    let small = dir.write("small.txt", "a\n");
    open(&view, cx, &small);
    assert!(!cx.has_pending_prompt());
    assert_eq!(text(&view, cx), "a");
}

#[gpui::test]
fn the_automatic_save_writes_a_modified_document_with_a_name(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "one\n");
    let (view, cx) = setup(cx);
    let run = |view: &Entity<EditorView>, cx: &mut VisualTestContext| {
        view.update_in(cx, |_, window, cx| {
            crate::file_ops::start(window, cx, async move |view, cx| {
                save_if_needed(&view, cx).await
            })
        });
        cx.run_until_parked();
    };
    // untitled and modified: not saved, and no dialog
    type_text(&view, cx, "u");
    run(&view, cx);
    assert!(!cx.has_pending_prompt());
    assert!(!cx.did_prompt_for_new_path());
    // unmodified: not written
    open(&view, cx, &path);
    std::fs::write(&path, "changed by hand\n").unwrap();
    run(&view, cx);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "changed by hand\n");
    // modified with a name: written
    open(&view, cx, &path);
    type_text(&view, cx, "x");
    run(&view, cx);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "xchanged by hand\n"
    );
    assert!(!view.read_with(cx, |v, app| v.document().read(app).model().is_modified()));
}

#[gpui::test]
fn the_automatic_save_waits_while_the_question_about_a_change_is_open(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "one\n");
    let (view, cx) = setup(cx);
    open(&view, cx, &path);
    type_text(&view, cx, "x");
    change(&path, "two\n");
    look(&view, cx);
    assert!(cx.has_pending_prompt());
    view.update_in(cx, |_, window, cx| {
        crate::file_ops::start(window, cx, async move |view, cx| {
            save_if_needed(&view, cx).await
        })
    });
    cx.run_until_parked();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "two\n",
        "nothing was overwritten"
    );
    answer(cx, RELOAD);
    assert_eq!(text(&view, cx), "two");
}

#[gpui::test]
fn the_automatic_save_leaves_a_file_that_another_program_changed(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "one\n");
    let (view, cx) = setup(cx);
    open(&view, cx, &path);
    type_text(&view, cx, "x");
    // changed, and the window has not looked yet
    change(&path, "two\n");
    view.update_in(cx, |_, window, cx| {
        crate::file_ops::start(window, cx, async move |view, cx| {
            save_if_needed(&view, cx).await
        })
    });
    cx.run_until_parked();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "two\n");
    assert!(!cx.has_pending_prompt());
}
