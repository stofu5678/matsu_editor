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

/// Gives the document of `view` a job, as the flow of a save does; returns its state.
fn begin(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> std::sync::Arc<JobState> {
    let job = SaveJob::new();
    let state = job.state();
    view.update(cx, |view, cx| {
        view.document()
            .update(cx, |shared, _| shared.begin_saving(job))
    });
    state
}

#[gpui::test]
fn the_document_is_saving_while_its_job_lives_and_refuses_edits(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, usize::MAX);
    open(&view, cx, &path);
    let state = begin(&view, cx);
    assert!(is_saving(&view, cx));
    cx.simulate_input("x");
    let text = view.read_with(cx, |view, app| view.document().read(app).doc().line(0));
    assert_eq!(text.unwrap().unwrap().text, "abc", "the typing was refused");
    // Esc asks the job to stop.
    view.update(cx, |view, cx| view.document().read(cx).cancel_saving());
    assert!(state.report(0, 1).is_break());
    view.update(cx, |view, cx| {
        view.document()
            .update(cx, |shared, _| shared.end_saving(&state))
    });
    assert!(!is_saving(&view, cx), "the flow ended the job");
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

/// A close that keeps an empty window, and a close-and-open, give the view another document
/// (`load_document`, `replace_with`): the job of the old one must stop before it replaces a file
/// that nobody wants written any more.
#[gpui::test]
fn replacing_the_document_stops_the_job_of_the_old_one(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, usize::MAX);
    open(&view, cx, &path);
    let state = begin(&view, cx);
    assert!(state.report(0, 1).is_continue());
    // what the close does with the window it keeps
    view.update(cx, |view, cx| {
        view.load_document(SharedDocument::empty(), None, cx)
    });
    assert!(state.report(0, 1).is_break(), "the thread is told to stop");
    assert!(!is_saving(&view, cx), "the new document is not saving");
    cx.simulate_input("y");
    let text = view.read_with(cx, |view, app| view.document().read(app).doc().line(0));
    assert_eq!(text.unwrap().unwrap().text, "y");
}

/// The flow that began a job does not end the job of a document that was replaced and began
/// another one meanwhile.
#[gpui::test]
fn the_end_of_an_old_flow_leaves_the_job_of_the_new_document_alone(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, usize::MAX);
    open(&view, cx, &path);
    let old = begin(&view, cx);
    view.update(cx, |view, cx| {
        view.load_document(SharedDocument::empty(), None, cx)
    });
    let new = begin(&view, cx);
    view.update(cx, |view, cx| {
        view.document()
            .update(cx, |shared, _| shared.end_saving(&old))
    });
    assert!(is_saving(&view, cx));
    assert!(new.report(0, 1).is_continue());
}

#[gpui::test]
fn a_second_save_while_one_runs_is_refused_before_anything_is_touched(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, usize::MAX);
    open(&view, cx, &path);
    cx.simulate_input("x");
    let state = begin(&view, cx);
    run(&view, cx, "F_FILESAVE");
    assert!(!cx.has_pending_prompt());
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "abc",
        "not written"
    );
    let modified = view.read_with(cx, |view, app| {
        view.document().read(app).model().is_modified()
    });
    assert!(modified);
    view.update(cx, |view, cx| {
        view.document()
            .update(cx, |shared, _| shared.end_saving(&state))
    });
    // and when the first is over, a save works
    run(&view, cx, "F_FILESAVE");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "xabc");
}

/// Two saves start; the first begins writing while the second waits at a question. The second
/// is refused when it gets to the write, and it does not touch the lock of the file (on Windows a
/// lock taken again while the first writes can stop its rename).
#[gpui::test]
fn a_save_that_waited_at_a_question_is_refused_when_another_began_writing(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "a\r\nb\nc\r\n");
    let (view, cx) = setup(cx, usize::MAX);
    cx.update(|_, app| {
        app.set_global(FilePrefs {
            file_share_mode: 1,
            ..FilePrefs::default()
        })
    });
    open(&view, cx, &path);
    let held_before = view.read_with(cx, |view, app| view.document().read(app).held_lock_mode());
    assert!(held_before.is_some(), "the file is locked");
    cx.simulate_input("x");
    let code = FUNCTIONS
        .iter()
        .find(|f| f.name == "F_FILESAVE")
        .unwrap()
        .code;
    view.update_in(cx, |view, window, cx| {
        view.run_function(code, &[], window, cx)
    });
    settle(cx);
    assert!(
        cx.has_pending_prompt(),
        "the question about the mixed line endings"
    );
    // the other save begins writing now
    let state = begin(&view, cx);
    cx.simulate_prompt_answer("そのまま保存(N)");
    settle(cx);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "a\r\nb\nc\r\n",
        "not written"
    );
    let held = view.read_with(cx, |view, app| view.document().read(app).held_lock_mode());
    assert_eq!(held, held_before, "the lock was not touched");
    assert!(
        is_saving(&view, cx),
        "the other save is still the one that runs"
    );
    assert!(state.report(0, 1).is_continue(), "and was not stopped");
}

/// The last view of a document is closed during a save: the document goes, and its job with it,
/// at once (the flow holds the document only weakly), so the thread is told before it writes.
#[gpui::test]
fn dropping_the_last_hold_of_a_document_stops_its_job_at_once(cx: &mut TestAppContext) {
    let shared = cx.new(|_| SharedDocument::empty());
    let job = SaveJob::new();
    let state = job.state();
    shared.update(cx, |shared, _| shared.begin_saving(job));
    let weak = shared.downgrade();
    assert!(state.report(0, 1).is_continue());
    drop(shared);
    // gpui frees a dropped entity when the update that saw the drop ends
    cx.update(|_| {});
    cx.run_until_parked();
    assert!(weak.upgrade().is_none(), "the document is gone");
    assert!(state.report(0, 1).is_break(), "and the thread is told");
}

/// The document is replaced while the thread writes: the old flow does not finish on the
/// replacement (its path, its lock, its notice stay its own).
#[gpui::test]
fn a_flow_of_a_replaced_document_leaves_the_replacement_alone(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, 0);
    open(&view, cx, &path);
    cx.simulate_input("x");
    let code = FUNCTIONS
        .iter()
        .find(|f| f.name == "F_FILESAVE")
        .unwrap()
        .code;
    view.update_in(cx, |view, window, cx| {
        view.run_function(code, &[], window, cx)
    });
    // the flow waits for its thread; the window gets another document meanwhile
    view.update(cx, |view, cx| {
        view.load_document(SharedDocument::empty(), None, cx)
    });
    settle(cx);
    let (path_of_new, notice, saving) = view.read_with(cx, |view, app| {
        let shared = view.document().read(app);
        (
            shared.path().map(PathBuf::from),
            view.notice.clone(),
            shared.is_saving(),
        )
    });
    assert_eq!(
        path_of_new, None,
        "the replacement did not take the path of the file"
    );
    assert!(!notice.is_some_and(|n| n.contains("に保存した")));
    assert!(!saving);
}

fn pausing(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> bool {
    view.read_with(cx, |view, app| view.document().read(app).disk().pausing)
}

/// The watching for changes by other programs is paused from the start of the write until the new
/// time stamp is recorded, so that our own rename is not taken for an outside change.
#[gpui::test]
fn the_watching_is_paused_while_the_worker_writes(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, 0);
    open(&view, cx, &path);
    let prefs = FilePrefs::default();
    let begin = |view: &Entity<EditorView>, cx: &mut VisualTestContext| {
        view.update(cx, |view, cx| {
            view.document().update(cx, |shared, _| {
                let began = crate::file_watch::check::begin(shared, &prefs).is_some();
                // a check that began is over for the test
                shared.disk_mut().pausing = false;
                began
            })
        })
    };
    assert!(begin(&view, cx), "a check begins while nothing writes");
    let was = view.update(cx, |view, cx| {
        view.document()
            .update(cx, |shared, _| shared.pause_watching_for_write())
    });
    assert!(!was);
    assert!(pausing(&view, cx));
    // `begin` clears `pausing` itself in the test helper above: look without it
    let during = view.update(cx, |view, cx| {
        view.document().update(cx, |shared, _| {
            crate::file_watch::check::begin(shared, &prefs).is_some()
        })
    });
    assert!(!during, "no check begins while the worker writes");
    view.update(cx, |view, cx| {
        view.document()
            .update(cx, |shared, _| shared.resume_watching_after_write(was))
    });
    assert!(!pausing(&view, cx));
    assert!(begin(&view, cx), "and one begins again after");
    // the whole save leaves the watching as it found it, with the new time stamp
    cx.simulate_input("x");
    run(&view, cx, "F_FILESAVE");
    assert!(!pausing(&view, cx));
    let stamp = view.read_with(cx, |view, app| view.document().read(app).disk().stamp);
    assert_eq!(stamp, crate::file_watch::state::modified_time(&path));
}

/// A pause that someone else owns (a check that asks a question) is left as it was.
#[gpui::test]
fn a_pause_of_someone_else_stays_after_the_save(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, 0);
    open(&view, cx, &path);
    view.update(cx, |view, cx| {
        view.document()
            .update(cx, |shared, _| shared.disk_mut().pausing = true)
    });
    cx.simulate_input("x");
    run(&view, cx, "F_FILESAVE");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "xabc");
    assert!(pausing(&view, cx));
}

/// The replacement document has a job of its own: the old job does not write over its progress
/// and does not clear it.
#[gpui::test]
fn an_old_job_leaves_the_notice_of_the_replacement_alone(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, 0);
    open(&view, cx, &path);
    cx.simulate_input("x");
    let code = FUNCTIONS
        .iter()
        .find(|f| f.name == "F_FILESAVE")
        .unwrap()
        .code;
    view.update_in(cx, |view, window, cx| {
        view.run_function(code, &[], window, cx)
    });
    cx.run_until_parked();
    view.update(cx, |view, cx| {
        view.load_document(SharedDocument::empty(), None, cx);
        view.notice = Some("the notice of the new document".to_owned());
    });
    settle(cx);
    let notice = view.read_with(cx, |view, _| view.notice.clone());
    assert_eq!(notice.as_deref(), Some("the notice of the new document"));
}
