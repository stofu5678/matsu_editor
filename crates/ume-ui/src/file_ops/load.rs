//! Opening files: reading on a background task, into this window or a new one, and reopening.

use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use gpui::{AsyncWindowContext, PathPromptOptions, PromptLevel, WeakEntity};
use ume_core::buffer::Pos;
use ume_core::encoding::Encoding;
use ume_core::file::{FileError, TextFile};

use super::ask::{confirm, document_title, open_error_message, reload_message, tell};
use super::windows::{self, LoadRequest, open_editor_window};
use crate::document::SharedDocument;
use crate::open::open_text_file;
use crate::view::EditorView;

/// Files at least this big show their progress in the status bar. Smaller ones are read before
/// the user could see anything.
const SHOW_PROGRESS_FROM: u64 = 4 * 1024 * 1024;

/// How far the background task has read; the window reads it to show progress.
#[derive(Default)]
struct Progress {
    done: AtomicU64,
    total: AtomicU64,
    finished: AtomicBool,
}

/// The status bar text while a file is read, such as `a.txt を読み込み中 25% (3 / 12 MB)`.
pub(crate) fn progress_text(path: &Path, done: u64, total: u64) -> String {
    const MB: u64 = 1024 * 1024;
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let percent = (done * 100).checked_div(total).unwrap_or(0);
    format!(
        "{name} を読み込み中 {percent}% ({} / {} MB)",
        done / MB,
        total / MB
    )
}

/// Reads `path` on a background task. A big file shows its progress in the view.
async fn read_file(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    path: &Path,
    encoding: Option<Encoding>,
) -> Result<TextFile, FileError> {
    let size = std::fs::metadata(path).map_or(0, |m| m.len());
    let progress = Arc::new(Progress::default());
    let shared = progress.clone();
    let target = path.to_owned();
    let task = cx.background_executor().spawn(async move {
        let result = open_text_file(&target, encoding, &mut |done, total| {
            shared.done.store(done, Ordering::Relaxed);
            shared.total.store(total, Ordering::Relaxed);
            ControlFlow::Continue(())
        });
        shared.finished.store(true, Ordering::Release);
        result
    });
    if size >= SHOW_PROGRESS_FROM {
        let timer = cx.background_executor().clone();
        while !progress.finished.load(Ordering::Acquire) {
            timer.timer(Duration::from_millis(100)).await;
            let text = progress_text(
                path,
                progress.done.load(Ordering::Relaxed),
                progress.total.load(Ordering::Relaxed).max(size),
            );
            view.update(cx, |view, cx| {
                view.notice = Some(text);
                cx.notify();
            })
            .ok();
        }
    }
    task.await
}

/// Reads `path` and makes it the document of this window. The caret goes to `restore` if given
/// (reopening keeps it). On a failure the window is as it was and the error is shown.
pub(crate) async fn load_here(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    path: &Path,
    encoding: Option<Encoding>,
    restore: Option<Pos>,
) -> bool {
    match read_file(view, cx, path, encoding).await {
        Ok(file) => {
            let shared = SharedDocument::from_text_file(file);
            view.update(cx, |view, cx| view.load_document(shared, restore, cx))
                .is_ok()
        }
        Err(error) => {
            view.update(cx, |view, cx| {
                view.notice = None;
                cx.notify();
            })
            .ok();
            let (message, detail) = open_error_message(path, &error);
            tell(cx, PromptLevel::Critical, &message, Some(&detail)).await;
            false
        }
    }
}

/// Opens files the way Sakura's open does (`CLoadAgent::OnCheckLoad`): a file that is open in a
/// window brings that window to the front; the first of the others goes into this window if it
/// is untitled and unmodified, and every other file gets a new window.
pub(crate) async fn open_paths(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    paths: Vec<PathBuf>,
    encoding: Option<Encoding>,
) {
    let mut here = view
        .read_with(cx, |view, app| view.accepts_load(app))
        .unwrap_or(false);
    for path in paths {
        let existing = cx
            .update(|_, app| windows::find_open(app, &path).map(|(h, _)| h))
            .ok()
            .flatten();
        if let Some(handle) = existing {
            cx.update(|_, app| windows::activate(app, handle)).ok();
            continue;
        }
        if here {
            here = false;
            load_here(view, cx, &path, encoding, None).await;
        } else {
            let request = LoadRequest { path, encoding };
            cx.update(|_, app| {
                open_editor_window(app, SharedDocument::empty(), Some(request)).ok();
            })
            .ok();
        }
    }
}

/// Shows the open dialog and returns the files chosen, or `None` if the user cancelled.
pub(crate) async fn choose_files(cx: &mut AsyncWindowContext) -> Option<Vec<PathBuf>> {
    let receiver = cx
        .update(|_, app| {
            app.prompt_for_paths(PathPromptOptions {
                files: true,
                directories: false,
                multiple: true,
                prompt: None,
            })
        })
        .ok()?;
    let paths = receiver.await.ok()?.ok()??;
    (!paths.is_empty()).then_some(paths)
}

/// `F_FILE_REOPEN` and `F_FILE_REOPEN_*` (`CViewCommander::Command_FILE_REOPEN`,
/// `CDocFileOperation::ReloadCurrentFile`): reads the file again in `encoding` (the current one
/// if `None`). A modified document asks first unless `no_confirm`. A document without a file only
/// takes the encoding. The caret stays where it was.
pub(crate) async fn reopen(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    encoding: Option<Encoding>,
    no_confirm: bool,
) -> bool {
    let Ok((path, modified, current, caret)) = view.read_with(cx, |view, app| {
        let shared = view.document().read(app);
        (
            shared.path().map(Path::to_owned),
            shared.model().is_modified(),
            shared.file().encoding,
            view.cursor().caret(),
        )
    }) else {
        return false;
    };
    let encoding = encoding.unwrap_or(current);
    let Some(path) = path.filter(|p| p.is_file()) else {
        // Sakura only sets the encoding when there is no file (`ReloadCurrentFile`).
        view.update(cx, |view, cx| {
            view.document().update(cx, |shared, cx| {
                shared.set_format(encoding, encoding.bom_default_on());
                cx.notify();
            });
            cx.notify();
        })
        .ok();
        return true;
    };
    if !no_confirm && modified {
        let title = document_title(Some(&path));
        if !confirm(
            cx,
            &reload_message(&title),
            None,
            ["再読み込みする", "キャンセル"],
        )
        .await
        {
            return false;
        }
    }
    load_here(view, cx, &path, Some(encoding), Some(caret)).await
}
