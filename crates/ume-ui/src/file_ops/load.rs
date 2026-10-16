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
use super::windows::{self, LoadRequest};
use crate::document::SharedDocument;
use crate::history::view::{code_change_message, encoding_of, remembered};
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
    // The settings of the type of this file name: the encoding and the line ending to fall back on.
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
    let defaults = cx
        .update(|_, app| crate::settings::new_document_defaults(app, name.as_deref()))
        .unwrap_or_default();
    let task = cx.background_executor().spawn(async move {
        let result = open_text_file(&target, encoding, &defaults, &mut |done, total| {
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

/// The encoding a file is opened in when the history knows the file and the user did not say
/// (`CMruListener::OnBeforeLoad`): if the detected encoding is not the one used last time, the user
/// is asked (`bQueryIfCodeChange`) or the one used last time is taken. A file that is not there is
/// a new file in the encoding of last time.
async fn settle_encoding(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    path: &Path,
    file: TextFile,
    old: Encoding,
) -> Result<TextFile, FileError> {
    if old == file.encoding {
        return Ok(file);
    }
    let use_old = if path.exists() {
        let ask = cx
            .update(|_, app| crate::history::options(app).query_if_code_change)
            .unwrap_or(false);
        !ask || confirm(
            cx,
            &code_change_message(path, file.encoding, old),
            None,
            ["はい(Y)", "いいえ(N)"],
        )
        .await
    } else {
        true
    };
    if !use_old {
        return Ok(file);
    }
    // If the file cannot be read in that encoding, the detected one stays.
    match read_file(view, cx, path, Some(old)).await {
        Ok(again) => Ok(again),
        Err(_) => Ok(file),
    }
}

/// Reads `path` and makes it the document of this window. The caret goes to `restore` if given
/// (reopening keeps it). On a failure the window is as it was and the error is shown.
///
/// A file the history knows is opened in the encoding of last time (see `settle_encoding`), and
/// its caret, view and bookmarks are restored (`CMruListener`) unless `restore` says where the
/// caret goes; the file goes to the front of the history.
pub(crate) async fn load_here(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    path: &Path,
    encoding: Option<Encoding>,
    restore: Option<Pos>,
) -> bool {
    // The version of the document when the read starts. The window stays usable while a big file
    // is read; if the document changed meanwhile (typing, or another load), the result is
    // dropped instead of replacing what the user did.
    let started = view
        .read_with(cx, |view, app| view.document().read(app).version())
        .ok();
    let handle = cx.window_handle();
    let reading = path.to_owned();
    cx.update(|_, app| windows::begin_pending(app, handle, reading))
        .ok();
    let previous = if restore.is_none() {
        cx.update(|_, app| remembered(app, path)).ok().flatten()
    } else {
        None
    };
    let mut result = read_file(view, cx, path, encoding).await;
    if encoding.is_none()
        && let Some(old) = previous.as_ref().and_then(encoding_of)
    {
        result = match result {
            Ok(file) => settle_encoding(view, cx, path, file, old).await,
            error => error,
        };
    }
    cx.update(|_, app| windows::finish_pending(app, handle))
        .ok();
    match result {
        Ok(file) => {
            let mut shared = SharedDocument::from_text_file(file);
            // `FileOpen(file, code, 1)` of a macro asked for view mode.
            if cx
                .update(|_, app| super::view_mode_request::take(app, path))
                .unwrap_or(false)
            {
                shared.keep_view_mode(true);
            }
            let replaced = view.update(cx, |view, cx| {
                if Some(view.document().read(cx).version()) != started {
                    view.notice = Some(format!(
                        "読み込み中に文書が変更されたので、{} は開かなかった",
                        path.display()
                    ));
                    cx.notify();
                    return false;
                }
                view.load_document(shared, restore, cx);
                if let Some(entry) = &previous {
                    let options = crate::history::options(cx);
                    view.restore_from_history(entry, options, cx);
                }
                view.record_history(cx);
                true
            });
            replaced.unwrap_or(false)
        }
        Err(error) => {
            // a request for view mode is for this read only
            cx.update(|_, app| super::view_mode_request::take(app, path))
                .ok();
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
    // A window that is still reading a file is not free for another one: both reads would
    // target the same document, and the slower one would be dropped.
    let handle = cx.window_handle();
    let reading = cx
        .update(|_, app| windows::has_pending(app, handle))
        .unwrap_or(true);
    let mut here = !reading
        && view
            .read_with(cx, |view, app| view.accepts_load(app))
            .unwrap_or(false);
    for path in paths {
        let existing = cx
            .update(|_, app| windows::find_open(app, &path))
            .ok()
            .flatten();
        if let Some((handle, found)) = existing {
            cx.update(|_, app| crate::tabs::flows::reveal(app, handle, found))
                .ok();
            continue;
        }
        if here {
            here = false;
            load_here(view, cx, &path, encoding, None).await;
        } else {
            let request = LoadRequest { path, encoding };
            // A new tab of this window in tab mode, else a new window (`OpenNewEditor`).
            cx.update(|window, app| {
                let from = window.window_handle();
                crate::tabs::flows::open_document(
                    app,
                    Some(window),
                    from,
                    false,
                    SharedDocument::empty(),
                    Some(request),
                )
                .ok();
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
