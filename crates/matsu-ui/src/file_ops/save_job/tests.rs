//! Tests of the save that runs on another thread (`file_ops::save_job`).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_commands::sakura::FUNCTIONS;

use super::*;
use crate::document::SharedDocument;
use crate::file_ops::prefs::FilePrefs;

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("matsu-ui-job-{}-{n}", std::process::id()));
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

fn setup(cx: &mut TestAppContext, from: usize) -> (Entity<EditorView>, &mut VisualTestContext) {
    cx.update(gpui_component::init);
    cx.update(|app| {
        app.set_global(FilePrefs::default());
        app.set_global(BackgroundFrom(from));
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

/// Lets the flow go on: the progress loop of a job waits for a timer, which the test clock only
/// passes when it is told to. Gives up after a bounded number of rounds.
fn settle(cx: &mut VisualTestContext) {
    for _ in 0..50 {
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(150));
    }
    cx.run_until_parked();
}

fn run(view: &Entity<EditorView>, cx: &mut VisualTestContext, function: &str) {
    let code = FUNCTIONS.iter().find(|f| f.name == function).unwrap().code;
    view.update_in(cx, |view, window, cx| {
        view.run_function(code, &[], window, cx)
    });
    settle(cx);
}

fn is_saving(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> bool {
    view.read_with(cx, |view, app| view.document().read(app).is_saving())
}

#[test]
fn a_job_reports_and_stops_when_asked() {
    let job = SaveJob::new();
    let state = job.0.clone();
    assert!(state.report(3, 10).is_continue());
    assert_eq!(state.done.load(Ordering::Relaxed), 3);
    assert_eq!(state.total.load(Ordering::Relaxed), 10);
    state.cancel();
    assert!(state.report(4, 10).is_break());
}

#[test]
fn dropping_the_owner_of_a_job_stops_it() {
    // The flow ends, or its window closes: the thread must not go on.
    let job = SaveJob::new();
    let state = job.0.clone();
    drop(job);
    assert!(state.report(0, 10).is_break());
}

#[test]
fn the_status_text_names_the_file_and_the_way_to_stop() {
    let text = progress_text("保存", Path::new("/work/a.txt"), 3 << 20, 12 << 20);
    assert_eq!(text, "a.txt を保存中 25% (3 / 12 MB) Esc で中止");
    // an unknown total does not divide by zero
    assert!(progress_text("検査", Path::new("a.txt"), 5, 0).contains("0%"));
}

#[gpui::test]
fn the_document_is_saving_while_its_job_lives_and_refuses_edits(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, usize::MAX);
    open(&view, cx, &path);
    let job = SaveJob::new();
    view.update(cx, |view, cx| {
        view.document()
            .update(cx, |shared, _| shared.begin_saving(job.watch()))
    });
    assert!(is_saving(&view, cx));
    cx.simulate_input("x");
    let text = view.read_with(cx, |view, app| view.document().read(app).doc().line(0));
    assert_eq!(text.unwrap().unwrap().text, "abc", "the typing was refused");
    // Esc asks the job to stop.
    view.update(cx, |view, cx| view.document().read(cx).cancel_saving());
    assert!(job.0.report(0, 1).is_break());
    drop(job);
    assert!(!is_saving(&view, cx), "the job is gone");
    cx.simulate_input("x");
    let text = view.read_with(cx, |view, app| view.document().read(app).doc().line(0));
    assert_eq!(text.unwrap().unwrap().text, "xabc");
}

#[gpui::test]
fn a_save_that_runs_on_another_thread_writes_the_file(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc\r\ndef\r\n");
    // from 0 bytes: every document is worked on by the background executor
    let (view, cx) = setup(cx, 0);
    open(&view, cx, &path);
    cx.simulate_input("x");
    run(&view, cx, "F_FILESAVE");
    assert!(!cx.has_pending_prompt());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "xabc\r\ndef\r\n");
    assert!(!is_saving(&view, cx));
    assert!(
        view.read_with(cx, |view, _| view.save_job.is_none()),
        "the view lets go of the job when it is over"
    );
    let (modified, notice) = view.read_with(cx, |view, app| {
        (
            view.document().read(app).model().is_modified(),
            view.notice.clone(),
        )
    });
    assert!(!modified, "the document is saved");
    assert!(notice.unwrap().contains("に保存した"));
    // typing works again
    cx.simulate_input("y");
    let text = view.read_with(cx, |view, app| view.document().read(app).doc().line(0));
    assert_eq!(text.unwrap().unwrap().text, "xyabc");
    // no temporary file is left in the folder
    let names: Vec<_> = std::fs::read_dir(&dir.0)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["a.txt"]);
}

#[gpui::test]
fn the_check_before_the_save_runs_on_another_thread_too(cx: &mut TestAppContext) {
    // Characters that Shift_JIS cannot write are asked about, and the answer is used.
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, 0);
    open(&view, cx, &path);
    view.update(cx, |view, cx| {
        view.document().update(cx, |shared, _| {
            shared.set_format(matsu_core::encoding::Encoding::ShiftJis, false)
        })
    });
    cx.simulate_input("\u{1f600}");
    run(&view, cx, "F_FILESAVE");
    assert!(cx.has_pending_prompt(), "the question about the character");
    cx.simulate_prompt_answer("中断(N)");
    settle(cx);
    assert!(!is_saving(&view, cx));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "abc");
}

#[gpui::test]
fn the_view_owns_the_job_and_closing_it_stops_the_thread(cx: &mut TestAppContext) {
    // A tab or a window that is closed drops its view, and the view its job: the thread must stop
    // before it replaces the file (the file of a document that was closed without saving).
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, usize::MAX);
    open(&view, cx, &path);
    let job = SaveJob::new();
    let state = job.state();
    view.update(cx, |view, _| view.save_job = Some(job));
    assert!(state.report(0, 1).is_continue());
    // dropping the view's hold on the job is what dropping the view does
    view.update(cx, |view, _| view.save_job = None);
    assert!(state.report(0, 1).is_break());
}

#[gpui::test]
fn a_second_save_while_one_runs_is_refused_before_anything_is_touched(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, usize::MAX);
    open(&view, cx, &path);
    cx.simulate_input("x");
    let job = SaveJob::new();
    view.update(cx, |view, cx| {
        view.document()
            .update(cx, |shared, _| shared.begin_saving(job.watch()))
    });
    run(&view, cx, "F_FILESAVE");
    assert!(!cx.has_pending_prompt());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "abc", "not written");
    let modified = view.read_with(cx, |view, app| view.document().read(app).model().is_modified());
    assert!(modified);
    drop(job);
    // and when the first is over, a save works
    run(&view, cx, "F_FILESAVE");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "xabc");
}
