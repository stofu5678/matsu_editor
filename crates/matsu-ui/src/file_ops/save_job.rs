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
//! * [`SaveJob`] is the owner of a running job and lives in the view that started it
//!   (`EditorView::save_job`). Dropping it (the job ends, or the tab or the window is closed, which
//!   drops the view) stops the thread at its next report of progress, which removes the temporary
//!   file; the thread asks before it replaces the file, so a document that was closed without
//!   saving does not get its file written afterwards.
//! * The document holds only a `Weak` of the job's state ([`JobState`]): while the job lives the
//!   document is "saving" and edits are refused, because the file gets the text the job started
//!   with. A document that is not saving never looks at the state.
//! * Smaller documents run the same work on the UI thread at once, as before.

use std::ops::ControlFlow;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;

use gpui::{AsyncWindowContext, WeakEntity};
use matsu_core::file::{FileError, SaveSnapshot};

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
    fn state(&self) -> Arc<JobState> {
        self.0.clone()
    }

    /// What the document keeps to know it is saving.
    pub fn watch(&self) -> Weak<JobState> {
        Arc::downgrade(&self.0)
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
    let prepared = view.update(cx, |view, cx| {
        view.document().update(cx, |shared, _| {
            if shared.is_saving() {
                return Err(FileError::Cancelled);
            }
            let snapshot = shared.save_snapshot()?;
            let big = shared.raw_len() >= from;
            if big {
                shared.begin_saving(job.watch());
            }
            Ok((snapshot, big, shared.version()))
        })
    });
    let (snapshot, big, version) = match prepared {
        Ok(prepared) => prepared?,
        Err(_) => return Err(FileError::Cancelled),
    };
    if !big {
        return work(&snapshot, &mut |_, _| ControlFlow::Continue(()));
    }
    let state = job.state();
    // The view owns the job from here: when the view is gone the job is dropped and the thread
    // stops. (The state is shared with this flow for the progress.)
    if view
        .update(cx, |view, _| view.save_job = Some(job))
        .is_err()
    {
        return Err(FileError::Cancelled);
    }
    let task = cx.background_executor().spawn({
        let state = state.clone();
        async move {
            let result = work(&snapshot, &mut |done, total| state.report(done, total));
            state.finished.store(true, Ordering::Release);
            result
        }
    });
    let timer = cx.background_executor().clone();
    while !state.finished.load(Ordering::Acquire) {
        timer.timer(Duration::from_millis(100)).await;
        let text = progress_text(
            verb,
            path,
            state.done.load(Ordering::Relaxed),
            state.total.load(Ordering::Relaxed),
        );
        let shown = view.update(cx, |view, cx| {
            view.notice = Some(text);
            cx.notify();
        });
        if shown.is_err() {
            // The view is gone: its job was dropped with it, which asked the thread to stop.
            state.cancel();
            return Err(FileError::Cancelled);
        }
    }
    let result = task.await;
    // The edits were refused meanwhile; if the text changed anyway (a way in that does not ask), the
    // file does not have it, and whoever goes on must not take the document for what was written.
    let Ok(changed) = view.read_with(cx, |view, app| {
        view.document().read(app).version() != version
    }) else {
        return Err(FileError::Cancelled);
    };
    view.update(cx, |view, cx| {
        // The job is over: the view lets go of it, and the document is not saving any more.
        view.save_job = None;
        view.notice = None;
        cx.notify();
    })
    .ok();
    match result {
        Ok(_) if changed => Err(FileError::Io(std::io::Error::other(
            "the text changed while the file was written",
        ))),
        other => other,
    }
}

#[cfg(test)]
mod tests;
