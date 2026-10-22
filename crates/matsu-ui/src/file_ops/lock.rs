//! Keeping the open file locked against other programs (`CDocFileOperation::DoFileLock`,
//! `DoFileUnlock`, `sakura_core/doc/CDocFileOperation.cpp`; `CFile::FileLock`,
//! `sakura_core/io/CFile.cpp`).
//!
//! The document holds the lock ([`FileLockState`]) while it is open, with 排他制御 of the common
//! settings (`FilePrefs::file_share_mode`: 1 denies writing, 2 denies reading and writing).
//! The lock is taken when a file is loaded (`CDocLocker::OnAfterLoad`) and released and taken again
//! around the moments Sakura does it:
//!
//! * a save (`CDocLocker::OnBeforeSave`, `OnAfterSave`), because the save replaces the file;
//! * the backup copy (`CBackupAgent::OnPreBeforeSave`);
//! * a reload of the same file (`CLoadAgent::OnCheckLoad`);
//! * view mode on and off (`Command_VIEWMODE`) and a change of the mode in the common settings
//!   (`CEditDoc::OnChangeSetting`): release, check again whether the file can be written, lock;
//! * closing: the lock goes with the document.
//!
//! The OS side is `matsu_platform::file_lock`: Windows enforces the lock against every program;
//! macOS and Linux have only advisory locks, which other programs ignore unless they lock too.
//!
//! When the lock cannot be taken the document goes on without it and says so
//! (`STR_FILE_LOCK_ERR`), except where Sakura passes `bMsg = false` (putting the lock back after
//! a refused load).

use std::path::{Path, PathBuf};

use gpui::{App, AsyncWindowContext, Context, PromptLevel, WeakEntity, Window};
use matsu_platform::file_lock::{FileLock, ShareMode};

use super::ask::tell;
use crate::view::EditorView;

/// `SHAREMODE_NOT_EXCLUSIVE`, `SHAREMODE_DENY_WRITE`, `SHAREMODE_DENY_READWRITE` as the setting
/// `nFileShareMode` has them. Any other number locks nothing here (Sakura would open the file with
/// full sharing, which locks nothing either).
pub(crate) fn share_mode(number: i32) -> Option<ShareMode> {
    match number {
        1 => Some(ShareMode::DenyWrite),
        2 => Some(ShareMode::DenyReadWrite),
        _ => None,
    }
}

/// A lock that could not be taken.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LockFailure {
    pub path: PathBuf,
    pub mode: ShareMode,
}

impl LockFailure {
    /// `STR_FILE_LOCK_ERR` with `STR_EXCLU_DENY_READWRITE` or `STR_EXCLU_DENY_WRITE`.
    pub fn message(&self) -> String {
        let mode = match self.mode {
            ShareMode::DenyReadWrite => "読み書きを禁止する",
            ShareMode::DenyWrite => "上書きを禁止する",
        };
        format!(
            "{}\nを{mode}でロックできませんでした。\n現在このファイルに対する排他制御は無効となります。",
            self.path.display()
        )
    }
}

/// `STR_DLGFLPROP_W_LOCK`, `STR_DLGFLPROP_RW_LOCK` and `STR_DLGFLPROP_NOT_LOCK`, for the file
/// property dialog (`CDlgProperty::SetData`).
pub(crate) fn property_line(lock: Option<ShareMode>) -> &'static str {
    match lock {
        Some(ShareMode::DenyWrite) => {
            "あなたはこのファイルを、他プロセスからの上書き禁止モードでロックしています。\n"
        }
        Some(ShareMode::DenyReadWrite) => {
            "あなたはこのファイルを、他プロセスからの読み書き禁止モードでロックしています。\n"
        }
        None => "あなたはこのファイルをロックしていません。\n",
    }
}

/// What a document knows of the lock of its file.
#[derive(Default)]
pub(crate) struct FileLockState {
    held: Option<FileLock>,
    /// The setting the document last locked with (`CDocFile::GetShareMode`):
    /// `CEditDoc::OnChangeSetting` compares it with the setting now.
    mode: i32,
    /// A failure that was not shown yet.
    failure: Option<LockFailure>,
}

impl FileLockState {
    /// `FileUnlock`.
    pub fn release(&mut self) {
        self.held = None;
    }

    /// `FileLock`: releases, then locks `path` in the mode of `number`. No path, a file that is
    /// not there, or the mode 0 locks nothing and is no failure (`_ToDoLock`, `IsFileExist`).
    pub fn lock(&mut self, path: Option<&Path>, number: i32) {
        self.held = None;
        self.mode = number;
        let Some(mode) = share_mode(number) else {
            return;
        };
        let Some(path) = path.filter(|path| path.exists()) else {
            return;
        };
        match FileLock::acquire(path, mode) {
            Ok(lock) => self.held = Some(lock),
            Err(_) => {
                self.failure = Some(LockFailure {
                    path: path.to_owned(),
                    mode,
                });
            }
        }
    }

    /// `CFile::IsFileLocking`.
    pub fn is_locked(&self) -> bool {
        self.held.is_some()
    }

    /// The mode of the lock that is held.
    pub fn held_mode(&self) -> Option<ShareMode> {
        self.held.as_ref().map(FileLock::mode)
    }

    /// The setting number last given to [`lock`](Self::lock).
    pub fn mode(&self) -> i32 {
        self.mode
    }

    /// Forgets a failure without showing it (`bMsg = false`).
    pub fn forget_failure(&mut self) {
        self.failure = None;
    }

    /// The failure to show, once.
    pub fn take_failure(&mut self) -> Option<LockFailure> {
        self.failure.take()
    }
}

/// Shows the failure of the lock of the document of `view`, if there is one (`TopWarningMessage`).
pub(crate) async fn report_failure(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    let failure = view
        .update(cx, |view, cx| {
            view.document()
                .update(cx, |shared, _| shared.take_lock_failure())
        })
        .ok()
        .flatten();
    if let Some(failure) = failure {
        tell(cx, PromptLevel::Warning, &failure.message(), None).await;
    }
}

/// [`report_failure`] for a command that runs inside the window.
pub(crate) fn report_failure_in(window: &mut Window, cx: &mut App, failure: LockFailure) {
    window
        .spawn(cx, async move |cx| {
            tell(cx, PromptLevel::Warning, &failure.message(), None).await;
        })
        .detach();
}

/// [`report_failure`] for a command of the view: a macro that called the command waits until the
/// box is closed (Sakura's boxes are modal).
pub(crate) fn report_failure_in_view(
    failure: &LockFailure,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) {
    let answer = window.prompt(PromptLevel::Warning, &failure.message(), None, &["OK"], cx);
    crate::macros::keep_box(answer, window, cx);
}

/// Releases the lock of the document of `view` if it holds `path`, before the file is read again.
/// Returns whether it did, for [`relock_quietly`].
pub(crate) fn release_for_reading(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    path: &Path,
) -> bool {
    view.update(cx, |view, cx| {
        view.document().update(cx, |shared, _| {
            let held = shared.path() == Some(path) && shared.is_file_locked();
            if held {
                shared.unlock_file();
            }
            held
        })
    })
    .unwrap_or(false)
}

/// Takes the lock back after a read that did not replace the document
/// (`DoFileLock(false)` in `CLoadAgent::OnCheckLoad`).
pub(crate) fn relock_quietly(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    view.update(cx, |view, cx| {
        let mode = super::prefs::prefs(cx).file_share_mode;
        view.document().update(cx, |shared, _| {
            shared.lock_file(mode);
            shared.forget_lock_failure();
        })
    })
    .ok();
}

/// What OK in the common settings does to the open documents (`CEditDoc::OnChangeSetting`): a
/// document whose mode differs releases its lock, checks whether the file can be written, and
/// locks in the new mode.
pub(crate) fn apply_setting(cx: &mut App, number: i32) {
    for (handle, view) in super::windows::editors(cx) {
        let document = view.read(cx).document().clone();
        let failure = document.update(cx, |shared, _| {
            if shared.lock_mode() == number {
                return None;
            }
            shared.relock_checking_writable(number);
            shared.take_lock_failure()
        });
        if let Some(failure) = failure {
            handle
                .update(cx, |_, window, cx| report_failure_in(window, cx, failure))
                .ok();
        }
    }
}

#[cfg(test)]
mod tests;
