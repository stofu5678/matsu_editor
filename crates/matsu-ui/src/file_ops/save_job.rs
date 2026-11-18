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
    finished: AtomicBool,
}

impl JobState {
    /// Asks the job to stop (Esc).
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// The progress callback of `matsu_core::file`: records how far the work is and stops it when
    /// asked.
    fn report(&self, done: u64, total: u64) -> ControlFlow<()> {
        self.done.store(done, Ordering::Relaxed);
        self.total.store(total, Ordering::Relaxed);
        if self.cancel.load(Ordering::Relaxed) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

/// The owner of a running job. See the module documentation.
pub(crate) struct SaveJob(Arc<JobState>);

impl SaveJob {
    pub fn new() -> Self {
        Self(Arc::default())
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

/// The status bar text while the work runs, such as `a.txt を保存中 25% (3 / 12 MB) Escで中止`.
pub(crate) fn progress_text(verb: &str, path: &Path, done: u64, total: u64) -> String {
    const MB: u64 = 1024 * 1024;
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let percent = (done * 100).checked_div(total).unwrap_or(0).min(100);
    format!(
        "{name} を{verb}中 {percent}% ({} / {} MB) Esc で中止",
        done / MB,
        total / MB
    )
}

/// What a piece of work gets: the copy of the text and how to report progress.
pub(crate) type Progress<'a> = &'a mut dyn FnMut(u64, u64) -> ControlFlow<()>;

/// Ends the job of a save on its document, on every path out of the flow that started it: the
/// write is done, it failed, the user declined a question, the window is gone (a tab joined into
/// another window removes the window that started the save while the document lives on).
///
/// The normal end is [`end_in`](Self::end_in), with an `App` at hand. A guard that is dropped
/// without it does the same later, in a task of the foreground executor, with the `App` of the
/// application and not a window: an update through the window of a flow that is gone fails, and
/// `Drop` itself may run inside an update, where the `App` is borrowed already.
pub(crate) struct JobEnd {
    document: WeakEntity<SharedDocument>,
    state: Arc<JobState>,
    app: AsyncApp,
    foreground: ForegroundExecutor,
    armed: bool,
}

impl JobEnd {
    pub fn new(app: &App, document: WeakEntity<SharedDocument>, state: Arc<JobState>) -> Self {
        Self {
            document,
            state,
            app: app.to_async(),
            foreground: app.foreground_executor().clone(),
            armed: true,
        }
    }

    /// The end, now: the document is not saving any more and a lock that was asked for meanwhile is
    /// taken with the settings of now.
    pub fn end_in(mut self, app: &mut App) {
        self.armed = false;
        end_job(app, &self.document, &self.state);
    }
}

impl Drop for JobEnd {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let (document, state, app) = (self.document.clone(), self.state.clone(), self.app.clone());
        // Detached: nothing could own it (the flow it belonged to is gone), and it ends at once.
        self.foreground
            .spawn(async move {
                let _ = app.update(|app| end_job(app, &document, &state));
            })
            .detach();
    }
}

fn end_job(app: &mut App, document: &WeakEntity<SharedDocument>, state: &Arc<JobState>) {
    // The job is stopped as well, if it still runs (the flow was dropped, not finished).
    state.cancel();
    let mode = super::prefs::prefs(app).file_share_mode;
    if let Some(document) = document.upgrade() {
        document.update(app, |shared, _| {
            if shared.end_saving(state) {
                shared.take_deferred_lock(mode);
            }
        });
    }
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
    work: impl FnOnce(&SaveSnapshot, Progress<'_>) -> Result<T, FileError> + Send + 'static,
) -> Result<T, FileError> {
    let from = cx
        .update(|_, app| {
            app.try_global::<BackgroundFrom>()
                .map_or(BACKGROUND_FROM, |b| b.0)
        })
        .unwrap_or(BACKGROUND_FROM);
    let job = SaveJob::new();
    let state = job.state();
    let prepared = view.update(cx, |view, cx| {
        let document = view.document().clone();
        let prepared = document.update(cx, |shared, _| {
            if shared.is_saving() {
                return Err(FileError::Cancelled);
            }
            let snapshot = shared.save_snapshot()?;
            let big = shared.raw_len() >= from;
            let (version, generation) = (shared.version(), shared.generation());
            if big {
                shared.begin_saving(job);
            }
            Ok((snapshot, big, version, generation))
        });
        // Only a weak hold goes across the awaits: the document must go when its last view is
        // closed (that drops the job and raises the flag at once), not when this flow ends.
        let weak = document.downgrade();
        // The guard that ends the job on every path (taken in the same update that began it).
        let guard = matches!(prepared, Ok((_, true, _, _)))
            .then(|| JobEnd::new(cx, weak.clone(), state.clone()));
        (weak, prepared, guard)
    });
    let Ok((document, prepared, guard)) = prepared else {
        return Err(FileError::Cancelled);
    };
    let (snapshot, big, version, generation) = prepared?;
    let Some(guard) = guard.filter(|_| big) else {
        return work(&snapshot, &mut |_, _| ControlFlow::Continue(()));
    };
    let task = cx.background_executor().spawn({
        let state = state.clone();
        async move {
            let result = work(&snapshot, &mut |done, total| state.report(done, total));
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
    cx.update(|_, app| guard.end_in(app)).ok();
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
            state.total.load(Ordering::Relaxed),
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
