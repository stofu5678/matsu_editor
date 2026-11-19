//! The slow parts of a save on another thread: the check of the line endings, the check of the
//! characters the encoding cannot write, and the write itself (ADR-0020, chapter 4).
//!
//! Sakura does all of it on the UI thread with a progress bar (`CSaveAgent::OnSave`,
//! `CWriteManager`, `CVisualProgress`). MATSU measured a save of 100 MB at 1 to 1.5 seconds
//! (3 seconds with a line ending conversion) and the check before it at 0.3 to 0.4 seconds, so a
//! document of [`BACKGROUND_FROM`] or more is worked on from a copy of the text
//! ([`SaveSnapshot`], cheap: the pieces are shared) on the background executor. The window shows
//! the progress in the status bar, and Esc stops the job (`FileError::Cancelled`: the file that is
//! replaced is as it was).
//!
//! Who owns what:
//!
//! * [`SaveJob`] is the owner of a running job and lives in the document it saves
//!   (`SharedDocument::begin_saving`). Dropping it stops the thread at its next report of
//!   progress, which removes the temporary file; the thread asks before it replaces the file. The
//!   document drops it when the document is replaced (a close that keeps an empty window, a
//!   close-and-open: `replace_with` makes a new `SharedDocument` over the old one) and when the
//!   document itself goes, so a document that was closed without saving does not get its file
//!   written afterwards. The flow that started the job ends it ([`SharedDocument::end_saving`]).
//! * While the job lives the document is "saving" and edits are refused, because the file gets the
//!   text the job started with.
//! * Smaller documents run the same work on the UI thread at once, as before.

use std::ops::ControlFlow;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use gpui::{App, AsyncApp, AsyncWindowContext, ForegroundExecutor, WeakEntity};
use matsu_core::file::{FileError, SaveSnapshot};

use crate::document::SharedDocument;
use crate::view::EditorView;

/// The size of the text (`Document::raw_len`) from which the work goes to another thread. Below it
/// the work takes a few tens of milliseconds at most.
pub(crate) const BACKGROUND_FROM: usize = 8 * 1024 * 1024;

/// A different [`BACKGROUND_FROM`], for tests that cannot make a document that big.
#[derive(Clone, Copy)]
pub(crate) struct BackgroundFrom(pub usize);

impl gpui::Global for BackgroundFrom {}

/// What the thread and the window share of a running job.
#[derive(Default)]
pub(crate) struct JobState {
    cancel: AtomicBool,
    done: AtomicU64,
    total: AtomicU64,
    /// The size of the text, known from the start: the total to show while the work has not
    /// reported one that means something (a check reports the amount so far as its total).
    size: AtomicU64,
    finished: AtomicBool,
    /// The cancel flag of the whole save flow, if the job belongs to one: it stops the job too.
    flow: Option<Arc<AtomicBool>>,
}

impl JobState {
    /// Asks the job to stop (Esc).
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// The progress callback of `matsu_core::file`: records how far the work is and stops it when
    /// asked.
    /// Whether the save flow that the job belongs to was asked to stop.
    fn flow_cancelled(&self) -> bool {
        self.flow
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
    }

    fn report(&self, done: u64, total: u64) -> ControlFlow<()> {
        self.done.store(done, Ordering::Relaxed);
        self.total.store(total, Ordering::Relaxed);
        if self.cancel.load(Ordering::Relaxed) || self.flow_cancelled() {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

/// The owner of a running job. See the module documentation.
pub(crate) struct SaveJob(Arc<JobState>);

impl SaveJob {
    #[cfg(test)]
    pub fn new() -> Self {
        Self(Arc::default())
    }

    /// A job of a save flow whose cancel flag is `flow`.
    pub fn with_flow(flow: Option<Arc<AtomicBool>>) -> Self {
        Self(Arc::new(JobState {
            flow,
            ..JobState::default()
        }))
    }

    /// The state the thread and the window share.
    pub fn state(&self) -> Arc<JobState> {
        self.0.clone()
    }
}

impl Drop for SaveJob {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

/// The status bar text while the work runs, such as `a.txt を保存中 25% (3 / 12 MB) Esc で中止`.
/// Without a total (not known yet) there is no percentage and no size: 100% is never said before
/// anything is done, and a part is never more than the whole.
pub(crate) fn progress_text(verb: &str, path: &Path, done: u64, total: u64) -> String {
    const MB: u64 = 1024 * 1024;
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    if total == 0 {
        return format!("{name} を{verb}中 Esc で中止");
    }
    let done = done.min(total);
    let percent = (done * 100) / total;
    format!(
        "{name} を{verb}中 {percent}% ({} / {} MB) Esc で中止",
        done / MB,
        total / MB
    )
}

/// What a piece of work gets: the copy of the text and how to report progress.
pub(crate) type Progress<'a> = &'a mut dyn FnMut(u64, u64) -> ControlFlow<()>;

/// The owner of every temporary change that a save makes to its document, so that each is undone
/// on every path out of the flow: the job that runs on another thread, the pause of the watching
/// for changes by other programs, the lock of the file that was let go of for the write. The write
/// is done, it failed, the user declined a question, the window is gone (a tab joined into another
/// window removes the window that started the save while the document lives on).
///
/// The normal end is [`end_now`](Self::end_now), with an `App` at hand. A guard that is dropped
/// without it does the same later, in a task of the foreground executor, with the `App` of the
/// application and not a window: an update through the window of a flow that is gone fails, and
/// `Drop` itself may run inside an update, where the `App` is borrowed already. Both go through
/// the document by a weak handle and its generation: a document that was replaced is not touched.
pub(crate) struct JobEnd {
    document: WeakEntity<SharedDocument>,
    generation: u64,
    app: AsyncApp,
    foreground: ForegroundExecutor,
    /// The job of the work that runs on another thread, while it runs.
    job: Option<Arc<JobState>>,
    /// The save holds a pause of the watching ([`SharedDocument::pause_watching_for_write`]).
    paused: bool,
    /// The save let go of the lock of the file for the write.
    released_lock: bool,
    /// The cancel flag of the save flow the guard belongs to.
    flow: Option<Arc<AtomicBool>>,
}

impl JobEnd {
    pub fn new(app: &App, document: WeakEntity<SharedDocument>, generation: u64) -> Self {
        Self {
            document,
            generation,
            app: app.to_async(),
            foreground: app.foreground_executor().clone(),
            job: None,
            paused: false,
            released_lock: false,
            flow: None,
        }
    }

    /// The generation of the document this guard was made for.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// A guard for the document of `view` as it is now. `None` if the view or its window is gone.
    pub fn for_view(
        view: &WeakEntity<EditorView>,
        cx: &mut AsyncWindowContext,
        flow: &Arc<AtomicBool>,
    ) -> Option<Self> {
        let (document, generation) = view
            .read_with(cx, |view, app| {
                let document = view.document();
                (document.downgrade(), document.read(app).generation())
            })
            .ok()?;
        let mut hold = cx
            .update(|_, app| Self::new(app, document, generation))
            .ok()?;
        hold.flow = Some(flow.clone());
        Some(hold)
    }

    /// Whether the save flow was asked to stop (Esc), in any phase.
    pub fn flow_cancelled(&self) -> bool {
        self.flow
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
    }

    /// The save let go of the lock of the file and holds a pause of the watching, as the write
    /// needs: [`SharedDocument::unlock_file`] and [`SharedDocument::pause_watching_for_write`].
    pub fn took_for_write(&mut self) {
        self.released_lock = true;
        self.paused = true;
    }

    /// The end of the job that ran on another thread: the document is not saving any more and a
    /// lock that was asked for meanwhile is taken with the settings of now. The rest (the pause,
    /// the lock for the write) waits for [`end_now`](Self::end_now), because the finish of the save
    /// needs them.
    pub fn end_job_now(&mut self, app: &mut App) {
        if let Some(state) = self.job.take() {
            undo(
                app,
                &self.document,
                self.generation,
                Some(&state),
                false,
                false,
            );
        }
    }

    /// The end, now: everything the save changed is undone, and nothing is left for `Drop`.
    pub fn end_now(&mut self, app: &mut App) {
        let job = self.job.take();
        let (paused, released) = (
            std::mem::take(&mut self.paused),
            std::mem::take(&mut self.released_lock),
        );
        undo(
            app,
            &self.document,
            self.generation,
            job.as_ref(),
            paused,
            released,
        );
    }
}

impl Drop for JobEnd {
    fn drop(&mut self) {
        // Nothing is left to undo after `end_now`, and nothing was taken by a check that ended.
        if self.job.is_none() && !self.paused && !self.released_lock {
            return;
        }
        let (document, generation, job) = (self.document.clone(), self.generation, self.job.take());
        let (paused, released) = (self.paused, self.released_lock);
        let app = self.app.clone();
        // Detached: nothing could own it (the flow it belonged to is gone), and it ends at once.
        self.foreground
            .spawn(async move {
                app.update(|app| undo(app, &document, generation, job.as_ref(), paused, released));
            })
            .detach();
    }
}

/// Undoes what a save changed on the document, if it is still the document that was saved.
fn undo(
    app: &mut App,
    document: &WeakEntity<SharedDocument>,
    generation: u64,
    job: Option<&Arc<JobState>>,
    paused: bool,
    released_lock: bool,
) {
    // The job is stopped as well, if it still runs (the flow was dropped, not finished).
    if let Some(state) = job {
        state.cancel();
    }
    let mode = super::prefs::prefs(app).file_share_mode;
    let Some(document) = document.upgrade() else {
        return;
    };
    document.update(app, |shared, _| {
        // A document that was replaced has its own lock, watching and job.
        if shared.generation() != generation {
            return;
        }
        if let Some(state) = job
            && shared.end_saving(state)
        {
            shared.take_deferred_lock(mode);
        }
        if paused {
            shared.resume_watching_after_write();
        }
        // The lock that was let go of for the write comes back with the setting of now, unless the
        // finish of the save took it already.
        if released_lock && !shared.is_saving() && !shared.is_file_locked() {
            shared.lock_file(mode);
        }
    });
}

/// Runs `work` on a copy of the text of the document of `view`. A big document (see
/// [`BACKGROUND_FROM`]) is worked on by the background executor while the window shows `verb`
/// ("検査", "保存") and its progress, with the edits refused; a small one is worked on here at once.
///
/// `Err(FileError::Cancelled)` also when the document is saving already (the job of another save).
pub(crate) async fn run<T: Send + 'static>(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    verb: &'static str,
    path: &Path,
    hold: &mut JobEnd,
    work: impl FnOnce(&SaveSnapshot, Progress<'_>) -> Result<T, FileError> + Send + 'static,
) -> Result<T, FileError> {
    let from = cx
        .update(|_, app| {
            app.try_global::<BackgroundFrom>()
                .map_or(BACKGROUND_FROM, |b| b.0)
        })
        .unwrap_or(BACKGROUND_FROM);
    // A cancel that came before this phase (between two phases, or before the first) stops it
    // before it starts.
    if hold.flow_cancelled() {
        return Err(FileError::Cancelled);
    }
    let job = SaveJob::with_flow(hold.flow.clone());
    let state = job.state();
    let prepared = view.update(cx, |view, cx| {
        let document = view.document().clone();
        let prepared = document.update(cx, |shared, _| {
            if shared.is_saving() {
                return Err(FileError::Cancelled);
            }
            let snapshot = shared.save_snapshot()?;
            let size = shared.raw_len();
            let big = size >= from;
            let (version, generation) = (shared.version(), shared.generation());
            if big {
                shared.begin_saving(job);
            }
            Ok((snapshot, big, version, generation, size))
        });
        // Only a weak hold goes across the awaits: the document must go when its last view is
        // closed (that drops the job and raises the flag at once), not when this flow ends.
        (document.downgrade(), prepared)
    });
    let Ok((document, prepared)) = prepared else {
        return Err(FileError::Cancelled);
    };
    let (snapshot, big, version, generation, size) = prepared?;
    if !big {
        let flow = hold.flow.clone();
        return work(&snapshot, &mut |_, _| {
            if flow.as_ref().is_some_and(|f| f.load(Ordering::Relaxed)) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });
    }
    // The guard owns the end of the job from here (the document owns the job itself).
    hold.job = Some(state.clone());
    state.size.store(size as u64, Ordering::Relaxed);
    let task = cx.background_executor().spawn({
        let state = state.clone();
        async move {
            // A panic of the work (a bug) is an error of the save, not a wait that never ends, and
            // the temporary file goes with the unwinding.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                work(&snapshot, &mut |done, total| state.report(done, total))
            }))
            .unwrap_or_else(|_| {
                Err(FileError::Io(std::io::Error::other(
                    "the save stopped by an internal error",
                )))
            });
            state.finished.store(true, Ordering::Release);
            result
        }
    });
    let result = wait_for(
        view,
        cx,
        &document,
        &state,
        task,
        verb,
        path,
        (version, generation),
    )
    .await;
    // The job is over, however it ended (or the document was replaced, which dropped it already).
    // If the window cannot be updated any more the guard does it when it is dropped.
    cx.update(|_, app| hold.end_job_now(app)).ok();
    result
}

/// The waiting of [`run`]: shows the progress until the thread is done, then judges the result.
#[allow(clippy::too_many_arguments)]
async fn wait_for<T: Send + 'static>(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    document: &WeakEntity<SharedDocument>,
    state: &Arc<JobState>,
    task: gpui::Task<Result<T, FileError>>,
    verb: &'static str,
    path: &Path,
    (version, generation): (u64, u64),
) -> Result<T, FileError> {
    let timer = cx.background_executor().clone();
    // The status text this job last wrote, and the view it wrote it in, so that it clears only its
    // own.
    let mut own_notice: Option<(WeakEntity<EditorView>, String)> = None;
    while !state.finished.load(Ordering::Acquire) {
        timer.timer(Duration::from_millis(100)).await;
        let text = progress_text(
            verb,
            path,
            state.done.load(Ordering::Relaxed),
            // What the work says is the total only when it is more than what is done (a check says
            // the amount so far); the size of the text is the better whole.
            state
                .total
                .load(Ordering::Relaxed)
                .max(state.size.load(Ordering::Relaxed)),
        );
        // The wait belongs to the document, not to the view that started the save: each tick finds
        // a view that shows the document (a split pane may have been removed) for the progress.
        // Only a document that is gone or replaced stops the job (a window that is gone cannot go
        // on either).
        let current = cx
            .update(|_, app| super::save::finishing_view(view, document, Some(generation), app))
            .ok()
            .flatten();
        let Some(current) = current else {
            state.cancel();
            return Err(FileError::Cancelled);
        };
        let shown = current.update(cx, |view, cx| {
            view.notice = Some(text.clone());
            cx.notify();
        });
        if shown.is_ok() {
            own_notice = Some((current, text));
        }
    }
    let result = task.await;
    // The document that was saved is the one that is here: it was not replaced (that drops the job
    // and stops the thread, but the write may have been past its last look), and the text did not
    // change either (the edits were refused; if it changed anyway, a way in that does not ask, the
    // file does not have the text, and whoever goes on must not take the document for what was
    // written).
    let same = cx
        .update(|_, app| {
            // A document that is gone is not the one that was saved.
            document.upgrade().map_or((false, false), |document| {
                let shared = document.read(app);
                (
                    shared.generation() == generation,
                    shared.version() == version,
                )
            })
        })
        .unwrap_or((false, false));
    if let Some((shown_in, text)) = own_notice {
        shown_in
            .update(cx, |view, cx| {
                if view.notice.as_ref() == Some(&text) {
                    view.notice = None;
                    cx.notify();
                }
            })
            .ok();
    }
    match (result, same) {
        (Ok(_), (false, _)) => Err(FileError::Cancelled),
        (Ok(_), (true, false)) => Err(FileError::Io(std::io::Error::other(
            "the text changed while the file was written",
        ))),
        (other, _) => other,
    }
}

#[cfg(test)]
mod tests;
