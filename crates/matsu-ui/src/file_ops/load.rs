//! Opening files: reading on a background task, into this window or a new one, and reopening.

use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use gpui::{AsyncWindowContext, PromptLevel, WeakEntity};
use matsu_core::buffer::Pos;
use matsu_core::encoding::Encoding;
use matsu_core::file::{FileError, TextFile};

use super::ask::{confirm, document_title, open_error_message, reload_message, tell};
use super::windows::{self, LoadRequest};
use crate::document::SharedDocument;
use crate::file_dialog::{self, Ask};
use crate::history::view::{code_change_message, encoding_of, remembered};
use crate::open::{HugeOpen, open_text_file_with};
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

/// Reads `path` on a background task. A big file shows its progress in the view. A huge file in an
/// encoding that has to be read whole is read after the user agreed (ADR-0020 9.9); when the user
/// did not, the result is [`FileError::Cancelled`], which is not told.
async fn read_file(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    path: &Path,
    encoding: Option<Encoding>,
) -> Result<TextFile, FileError> {
    match read_file_asking(view, cx, path, encoding, true).await {
        Err(FileError::WholeReadNeeded {
            len,
            encoding: found,
        }) => {
            let (message, detail) = whole_read_message(path, len, found);
            if confirm(cx, &message, Some(&detail), ["読み込む(Y)", "キャンセル"]).await {
                read_file_asking(view, cx, path, encoding, false).await
            } else {
                Err(FileError::Cancelled)
            }
        }
        other => other,
    }
}

/// The question before a huge file in an encoding that cannot be read by ranges is read whole
/// (ADR-0020 9.9; the words are the editor's own, Sakura has none): what it costs.
pub(crate) fn whole_read_message(path: &Path, len: u64, encoding: Encoding) -> (String, String) {
    let mb = len.div_ceil(1024 * 1024);
    (
        format!(
            "'{}'\nは {mb} MB の {} のファイルです。",
            path.display(),
            encoding.short_name()
        ),
        "この文字コードは範囲ごとに読めないので、全体を読み込んで変換します。\
         時間がかかり、ファイルの大きさの 2〜3 倍のメモリを使います。読み込みますか？"
            .to_owned(),
    )
}

async fn read_file_asking(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    path: &Path,
    encoding: Option<Encoding>,
    ask: bool,
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
    // A big file is opened by ranges (ADR-0020, chapter 7).
    let huge = cx
        .update(|_, app| {
            let prefs = super::prefs::prefs(app);
            HugeOpen {
                from: prefs.huge_file_len(),
                scan: prefs.huge_scan,
                ask_before_whole_read: ask,
            }
        })
        .ok();
    let task = cx.background_executor().spawn(async move {
        let result = open_text_file_with(&target, encoding, &defaults, huge, &mut |done, total| {
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
        // The user said no to the question before a huge file is read whole in that encoding:
        // nothing is opened.
        Err(FileError::Cancelled) => Err(FileError::Cancelled),
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
    cx.update(|_, app| windows::begin_pending(app, handle, view, reading))
        .ok();
    // The warnings of `CLoadAgent::OnCheckLoad`: a file that is not there, a file that is big. The
    // file is pending already, so that it cannot be opened in a second window meanwhile.
    if !crate::file_watch::open_checks::allow_open(cx, path).await {
        cx.update(|_, app| {
            super::view_mode_request::take(app, path);
            windows::finish_pending(app, view)
        })
        .ok();
        super::start_options::after_load(view, cx, path, false);
        return false;
    }
    let previous = if restore.is_none() {
        cx.update(|_, app| remembered(app, path)).ok().flatten()
    } else {
        None
    };
    // The time stamp the text will belong to: taken before the read, so that a write that happens
    // meanwhile shows up as a change at the next check.
    let stamp = crate::file_watch::state::modified_time(path);
    // Reading the file again (reopen, reload): its own lock would refuse the read, so it is let
    // go of until the file is read (`CLoadAgent::OnCheckLoad`).
    let unlocked = super::lock::release_for_reading(view, cx, path);
    let mut result = read_file(view, cx, path, encoding).await;
    if encoding.is_none()
        && let Some(old) = previous.as_ref().and_then(encoding_of)
    {
        result = match result {
            Ok(file) => settle_encoding(view, cx, path, file, old).await,
            error => error,
        };
    }
    match result {
        Ok(file) => {
            cx.update(|_, app| windows::finish_pending(app, view)).ok();
            let mut shared = SharedDocument::from_text_file(file);
            shared.disk_mut().stamp = stamp;
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
                // `-TYPE=` decides the type before the macro of オープン後 runs.
                if let Some(ext) = super::start_options::expected_type(cx, path) {
                    view.apply_launch_type(&ext, cx);
                }
                let history = previous
                    .clone()
                    .map(|entry| (entry, crate::history::options(cx)));
                // The place and the bookmarks of a file scanned in the background may be past
                // what is scanned yet: they are restored, and the history written, when the scan
                // is done.
                if !view.defer_restore(None, history.clone(), cx) {
                    if let Some((entry, options)) = &history {
                        view.restore_from_history(entry, *options, cx);
                    }
                    view.record_history(cx);
                }
                true
            });
            let replaced = replaced.unwrap_or(false);
            if replaced {
                // `CDocLocker::OnAfterLoad` locked the new document.
                super::lock::report_failure(view, cx).await;
                // A file that is scanned in the background (ADR-0020): the macro of オープン後 and
                // the options of the launch run when the scan is done (a place in the file may be
                // past what is scanned). The macro that opened the file waits for the scan too.
                let scanning = view
                    .read_with(cx, |view, app| view.document().read(app).is_scanning())
                    .unwrap_or(false);
                if scanning {
                    if let Some(entity) = view.upgrade() {
                        cx.update(|_, app| {
                            super::start_options::after_load_when_scanned(&entity, path, app)
                        })
                        .ok();
                    }
                    return true;
                }
                // The file is open: the macro of オープン後 (`FileLoad`).
                crate::macros::run_opened(view, cx).await;
            } else if unlocked {
                super::lock::relock_quietly(view, cx);
            }
            // the options of a launch (`-X=` `-Y=` `-TYPE=` `-M=`...)
            super::start_options::after_load(view, cx, path, replaced);
            replaced
        }
        Err(error) => {
            // a request for view mode is for this read only
            cx.update(|_, app| super::view_mode_request::take(app, path))
                .ok();
            if unlocked {
                super::lock::relock_quietly(view, cx);
            }
            view.update(cx, |view, cx| {
                view.notice = None;
                cx.notify();
            })
            .ok();
            // The user's own "no" to the question before a whole read is not an error.
            if !matches!(error, FileError::Cancelled) {
                let (message, detail) = open_error_message(path, &error);
                tell(cx, PromptLevel::Critical, &message, Some(&detail)).await;
            }
            // Pending until the error is read: a macro that opened the file waits for the box.
            cx.update(|_, app| windows::finish_pending(app, view)).ok();
            super::start_options::after_load(view, cx, path, false);
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
    let reading = cx
        .update(|_, app| windows::is_pending(app, view))
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

/// What the settings say about dropped files (`[Common]` of the settings file).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DropSettings {
    /// 「一度にドロップ可能なファイル数」 (`nDropFileNumMax`).
    pub(crate) max_files: usize,
    /// 「ファイルをドロップしたときは閉じて開く」 (`bDropFileAndClose`).
    pub(crate) close_and_open: bool,
}

impl DropSettings {
    /// Sakura's default (`CShareData::InitShareData`): 8 files, no close and open.
    pub(crate) const DEFAULT: Self = Self {
        max_files: 8,
        close_and_open: false,
    };

    /// Reads the settings at the time of the drop. The limit is held to 1..=99 as the common
    /// settings dialog does (`CPropComFile::GetData`, `sakura_core/prop/CPropComFile.cpp`).
    pub(crate) fn read(cx: &gpui::App) -> Self {
        let Some(profile) = cx
            .try_global::<crate::settings::UserSettings>()
            .and_then(|user| user.profile())
        else {
            return Self::DEFAULT;
        };
        let max_files = profile
            .get_i32("Common", "nDropFileNumMax")
            .map_or(Self::DEFAULT.max_files, |n| n.clamp(1, 99) as usize);
        Self {
            max_files,
            close_and_open: profile
                .get_bool("Common", "bDropFileAndClose")
                .unwrap_or(false),
        }
    }
}

/// Files dropped on the window (`CEditWnd::OnDropFiles`, `sakura_core/window/CEditWnd.cpp`): with
/// `close_and_open` only the first file is taken, then only the first `max_files`. A file that is
/// open already comes to the front; otherwise it goes into this window if that accepts a load,
/// else (`close_and_open`) replaces the document after the question about its changes, else it
/// gets a new tab or window like the files of the open dialog.
pub(crate) async fn drop_files(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    mut paths: Vec<PathBuf>,
    settings: DropSettings,
) {
    if settings.close_and_open {
        paths.truncate(1);
    }
    paths.truncate(settings.max_files);
    if !settings.close_and_open {
        open_paths(view, cx, paths, None).await;
        return;
    }
    let Some(path) = paths.pop() else {
        return;
    };
    let existing = cx
        .update(|_, app| windows::find_open(app, &path))
        .ok()
        .flatten();
    if let Some((handle, found)) = existing {
        cx.update(|_, app| crate::tabs::flows::reveal(app, handle, found))
            .ok();
        return;
    }
    let reading = cx
        .update(|_, app| windows::is_pending(app, view))
        .unwrap_or(true);
    let accepts = view
        .read_with(cx, |view, app| view.accepts_load(app))
        .unwrap_or(false);
    // `FileCloseOpen`: the question about the changes first, then the file replaces the document.
    if reading || (!accepts && !super::close::confirm_discard(view, cx).await) {
        return;
    }
    load_here(view, cx, &path, None, None).await;
}

/// Where the open dialog is told to start by a macro (`FileOpen`'s fourth argument): the folder
/// and the file name. See [`matsu_config::dialog_dir::split_default_name`].
pub(crate) type GivenStart = (Option<PathBuf>, Option<String>);

/// Shows the open dialog and returns the files chosen, or `None` if the user cancelled. It starts
/// in the folder of the file of this window, else where 「ファイルダイアログの初期位置」 says
/// (`GetDlgInitialDir`), unless `given` says where.
pub(crate) async fn choose_files(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    given: Option<GivenStart>,
) -> Option<Vec<PathBuf>> {
    let document = view
        .read_with(cx, |view, app| {
            view.document().read(app).path().map(Path::to_owned)
        })
        .ok()
        .flatten();
    let answer = cx
        .update(|window, app| {
            let (start, name) = given
                .unwrap_or_else(|| (file_dialog::start_folder(app, document.as_deref()), None));
            file_dialog::ask(
                window,
                app,
                Ask::open(true).start(start).name(name).recent(app),
            )
        })
        .ok()?;
    answer.await.ok().flatten()
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
        let title = document_title(Some(&path), None);
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
