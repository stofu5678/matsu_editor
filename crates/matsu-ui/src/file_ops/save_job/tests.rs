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
    view.read_with(cx, |view, app| view.document().read(app).disk().is_paused())
}

/// Runs `f` on the document of `view`.
fn on_document<R>(
    view: &Entity<EditorView>,
    cx: &mut VisualTestContext,
    f: impl FnOnce(&mut SharedDocument) -> R,
) -> R {
    view.update(cx, |view, cx| {
        view.document().update(cx, |shared, _| f(shared))
    })
}

/// Whether a check for changes by other programs could begin now (and if so, it holds its own
/// pause, as the real one does).
fn check_begins(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> bool {
    let prefs = FilePrefs::default();
    on_document(view, cx, |shared| {
        crate::file_watch::check::begin(shared, &prefs).is_some()
    })
}

/// The watching for changes by other programs is paused from the start of the write until the new
/// time stamp is recorded, so that our own rename is not taken for an outside change.
#[gpui::test]
fn the_watching_is_paused_while_the_worker_writes(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, 0);
    open(&view, cx, &path);
    assert!(
        check_begins(&view, cx),
        "a check begins while nothing writes"
    );
    on_document(&view, cx, |shared| shared.disk_mut().resume());
    on_document(&view, cx, |shared| shared.pause_watching_for_write());
    assert!(pausing(&view, cx));
    assert!(
        !check_begins(&view, cx),
        "no check begins while the worker writes"
    );
    on_document(&view, cx, |shared| shared.resume_watching_after_write());
    assert!(!pausing(&view, cx));
    // the whole save leaves the watching as it found it, with the new time stamp
    cx.simulate_input("x");
    run(&view, cx, "F_FILESAVE");
    assert!(!pausing(&view, cx));
    let stamp = view.read_with(cx, |view, app| view.document().read(app).disk().stamp);
    assert_eq!(stamp, crate::file_watch::state::modified_time(&path));
}

/// Each pauser lifts only its own hold: a check that ends during the write does not lift the
/// pause of the save, and a save that ends does not lift the pause of a check that is still going.
#[gpui::test]
fn each_pauser_lifts_only_its_own_pause(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, 0);
    open(&view, cx, &path);
    // the check began first, the save second; the check ends first
    assert!(check_begins(&view, cx));
    on_document(&view, cx, |shared| shared.pause_watching_for_write());
    on_document(&view, cx, |shared| shared.disk_mut().resume());
    assert!(pausing(&view, cx), "the save still holds the watching");
    assert!(!check_begins(&view, cx));
    on_document(&view, cx, |shared| shared.resume_watching_after_write());
    assert!(!pausing(&view, cx));
    // the save began first, the check second; the save ends first
    on_document(&view, cx, |shared| shared.pause_watching_for_write());
    on_document(&view, cx, |shared| shared.disk_mut().pause());
    on_document(&view, cx, |shared| shared.resume_watching_after_write());
    assert!(pausing(&view, cx), "the check still holds the watching");
    on_document(&view, cx, |shared| shared.disk_mut().resume());
    assert!(!pausing(&view, cx));
    // a lift without a hold does nothing wrong
    on_document(&view, cx, |shared| shared.disk_mut().resume());
    assert!(!pausing(&view, cx));
}

/// A pause that someone else owns (a check that asks a question) is left as it was.
#[gpui::test]
fn a_pause_of_someone_else_stays_after_the_save(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, 0);
    open(&view, cx, &path);
    on_document(&view, cx, |shared| shared.disk_mut().pause());
    cx.simulate_input("x");
    run(&view, cx, "F_FILESAVE");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "xabc");
    assert!(pausing(&view, cx));
    on_document(&view, cx, |shared| shared.disk_mut().resume());
    assert!(!pausing(&view, cx));
}

/// The save is finished in the view that started it; when that view is gone (a split that was
/// removed) and the document goes on in another view, in that one; a document that was replaced
/// or is gone is not finished at all.
#[gpui::test]
fn the_finish_of_a_save_goes_to_a_view_of_the_document_that_was_saved(cx: &mut TestAppContext) {
    use crate::file_ops::save::finishing_view;
    cx.update(gpui_component::init);
    cx.update(|app| app.set_global(FilePrefs::default()));
    let shared = cx.new(|_| SharedDocument::empty());
    let first = cx.new({
        let shared = shared.clone();
        move |cx| EditorView::new(shared, cx)
    });
    let second = cx.new({
        let shared = shared.clone();
        move |cx| EditorView::new(shared, cx)
    });
    let generation = Some(cx.read(|app| shared.read(app).generation()));
    let document = shared.downgrade();
    let pick = |cx: &mut TestAppContext, view: &WeakEntity<EditorView>, generation| {
        cx.read(|app| finishing_view(view, &document, generation, app))
    };
    // the view that started the save is there
    assert_eq!(
        pick(cx, &first.downgrade(), generation).map(|v| v.entity_id()),
        Some(first.entity_id())
    );
    // it is gone (the split was removed): the other view of the document finishes
    let gone = first.downgrade();
    drop(first);
    cx.update(|_| {});
    assert_eq!(
        pick(cx, &gone, generation).map(|v| v.entity_id()),
        Some(second.entity_id())
    );
    // the document was replaced meanwhile: nothing is finished
    shared.update(cx, |shared, cx| {
        shared.replace_with(SharedDocument::empty(), cx);
    });
    assert!(pick(cx, &second.downgrade(), generation).is_none());
    // and when every view and the document are gone
    drop(second);
    drop(shared);
    cx.update(|_| {});
    assert!(pick(cx, &gone, generation).is_none());
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

fn held(
    view: &Entity<EditorView>,
    cx: &mut VisualTestContext,
) -> Option<matsu_platform::file_lock::ShareMode> {
    view.read_with(cx, |view, app| view.document().read(app).held_lock_mode())
}

/// Every way to take the lock goes through one gate: while a save writes the file none of them
/// takes it (the writer needs the file free), and the save's own end takes it.
#[gpui::test]
fn no_way_takes_the_lock_while_a_save_writes(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, usize::MAX);
    cx.update(|_, app| {
        app.set_global(FilePrefs {
            file_share_mode: 1,
            ..FilePrefs::default()
        })
    });
    open(&view, cx, &path);
    assert!(held(&view, cx).is_some(), "locked after the load");
    // the flow lets go of the lock and starts its job
    on_document(&view, cx, |shared| shared.unlock_file());
    let state = begin(&view, cx);
    // the setting changes to the mode that denies reading and writing (`lock::apply_setting`)
    on_document(&view, cx, |shared| shared.relock_checking_writable(2));
    assert!(held(&view, cx).is_none(), "not taken for the new setting");
    assert_eq!(
        view.read_with(cx, |view, app| view.document().read(app).lock_mode()),
        2,
        "but remembered as the one the document locks with"
    );
    // view mode on and off (`Command_VIEWMODE`)
    run(&view, cx, "F_VIEWMODE");
    run(&view, cx, "F_VIEWMODE");
    assert!(held(&view, cx).is_none(), "not taken when view mode ends");
    // a lock taken back after a check or a backup
    on_document(&view, cx, |shared| shared.lock_file(1));
    assert!(held(&view, cx).is_none());
    // the save ends and takes the lock (`finish_save_now` does it with the setting of then)
    on_document(&view, cx, |shared| shared.end_saving(&state));
    on_document(&view, cx, |shared| shared.lock_file(2));
    assert!(held(&view, cx).is_some(), "taken when the save is over");
}

/// The wait belongs to the document: the view that started the save is gone (a split pane that
/// was removed) and another view still shows the document; the job goes on and its result is
/// handed on.
#[gpui::test]
fn the_wait_goes_on_when_the_view_that_started_the_save_is_gone(cx: &mut TestAppContext) {
    use std::sync::atomic::AtomicBool;
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, usize::MAX);
    open(&view, cx, &path);
    let shared = view.read_with(cx, |view, _| view.document().clone());
    // another view over the document, which is the one that started the save
    let initiator = cx.update(|_, app| {
        let shared = shared.clone();
        app.new(move |cx| EditorView::new(shared, cx))
    });
    let (version, generation) = cx.read(|app| {
        let doc = shared.read(app);
        (doc.version(), doc.generation())
    });
    let job = SaveJob::new();
    let state: std::sync::Arc<JobState> = job.state();
    let weak_initiator = initiator.downgrade();
    drop(initiator);
    cx.update(|_, _| {});
    // the job's work ends after 350 ms of the clock the test moves
    let task = {
        let state = state.clone();
        let executor = cx.executor();
        let timer = executor.clone();
        executor.spawn(async move {
            timer.timer(Duration::from_millis(350)).await;
            state.finished.store(true, Ordering::Release);
            Ok::<_, FileError>(7)
        })
    };
    let document = shared.downgrade();
    let result = std::rc::Rc::new(std::cell::RefCell::new(None));
    let seen_notice = std::sync::Arc::new(AtomicBool::new(false));
    {
        let result = result.clone();
        let seen = seen_notice.clone();
        let view = view.clone();
        let state = state.clone();
        let path = path.clone();
        view.clone().update_in(cx, move |_, window, cx| {
            window
                .spawn(cx, async move |cx| {
                    let waited = wait_for(
                        &weak_initiator,
                        cx,
                        &document,
                        &state,
                        task,
                        "保存",
                        &path,
                        (version, generation),
                    )
                    .await;
                    seen.store(
                        view.read_with(cx, |v, _| v.notice.is_none()),
                        Ordering::Relaxed,
                    );
                    *result.borrow_mut() = Some(waited);
                })
                .detach();
        });
    }
    settle(cx);
    let waited = result.borrow_mut().take().expect("the wait ended");
    assert!(
        matches!(waited, Ok(7)),
        "the result of the worker is handed on: {waited:?}"
    );
    assert!(
        seen_notice.load(Ordering::Relaxed),
        "the status text of the job is cleared in the view that showed it"
    );
    assert!(state.report(0, 1).is_continue(), "the job was not stopped");
    drop(job);
}

/// A guard for the job `state` of the document of `view`, as `run` takes it.
fn guard_for(
    view: &Entity<EditorView>,
    state: &std::sync::Arc<JobState>,
    cx: &mut VisualTestContext,
) -> JobEnd {
    let document = view.read_with(cx, |view, _| view.document().downgrade());
    let state = state.clone();
    cx.update(|_, app| JobEnd::new(app, document, state))
}

/// The flow that began a job is gone without ending it (a tab joined into another window removed
/// its window; a question was declined): the guard ends the job on the document by itself, with
/// the `App` and not the window, and the document is editable again.
#[gpui::test]
fn a_guard_that_is_dropped_ends_the_job_and_the_document_is_editable(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, usize::MAX);
    open(&view, cx, &path);
    let state = begin(&view, cx);
    let guard = guard_for(&view, &state, cx);
    assert!(is_saving(&view, cx));
    drop(guard);
    cx.run_until_parked();
    assert!(!is_saving(&view, cx), "the job ended");
    assert!(
        state.report(0, 1).is_break(),
        "and the thread was told to stop"
    );
    cx.simulate_input("x");
    let text = view.read_with(cx, |view, app| view.document().read(app).doc().line(0));
    assert_eq!(
        text.unwrap().unwrap().text,
        "xabc",
        "edits are accepted again"
    );
}

/// A check ends without a write (the user cancels the question that follows) after the sharing
/// setting changed during it: the lock that was deferred is back, with the setting of then.
#[gpui::test]
fn a_lock_deferred_during_a_check_is_taken_when_the_check_ends(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, usize::MAX);
    cx.update(|_, app| {
        app.set_global(FilePrefs {
            file_share_mode: 1,
            ..FilePrefs::default()
        })
    });
    open(&view, cx, &path);
    assert!(held(&view, cx).is_some());
    let state = begin(&view, cx);
    let guard = guard_for(&view, &state, cx);
    // the setting changes to the mode that denies reading and writing during the check
    cx.update(|_, app| {
        app.set_global(FilePrefs {
            file_share_mode: 2,
            ..FilePrefs::default()
        })
    });
    on_document(&view, cx, |shared| shared.relock_checking_writable(2));
    assert!(held(&view, cx).is_none(), "deferred while the check runs");
    // the check ends (declined, cancelled, failed): `finish_save_now` never runs
    cx.update(|_, app| guard.end_in(app));
    assert!(!is_saving(&view, cx));
    assert_eq!(
        held(&view, cx),
        Some(matsu_platform::file_lock::ShareMode::DenyReadWrite),
        "locked again with the setting of that moment"
    );
}

/// The same when the guard is dropped, not ended with an `App` at hand.
#[gpui::test]
fn a_lock_deferred_during_a_save_is_taken_also_when_the_flow_is_gone(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", "abc");
    let (view, cx) = setup(cx, usize::MAX);
    cx.update(|_, app| {
        app.set_global(FilePrefs {
            file_share_mode: 1,
            ..FilePrefs::default()
        })
    });
    open(&view, cx, &path);
    let state = begin(&view, cx);
    let guard = guard_for(&view, &state, cx);
    on_document(&view, cx, |shared| shared.relock_checking_writable(1));
    assert!(held(&view, cx).is_none());
    drop(guard);
    cx.run_until_parked();
    assert_eq!(
        held(&view, cx),
        Some(matsu_platform::file_lock::ShareMode::DenyWrite)
    );
}
