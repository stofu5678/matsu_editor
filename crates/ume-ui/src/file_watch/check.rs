//! The check for a file that another program changed (`CAutoReloadAgent::CheckFileTimeStamp`,
//! `sakura_core/agent/CAutoReloadAgent.cpp`).
//!
//! The window calls it every three seconds while it is the active one, and when it becomes
//! active (`CEditWnd::OnEditTimer`, `WM_SETFOCUS` in `sakura_core/window/CEditWnd.cpp`). It
//! compares the write time of the file with the one the document remembers. A different one
//! (older too: `CompareFileTime` is `!=`) is a change; what follows depends on the mode
//! ([`WatchMode`]): ask, say it in the status bar, or reload without asking while the document is
//! unmodified.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use gpui::{AsyncWindowContext, PromptLevel, WeakEntity};

use super::state::{WatchMode, modified_time};
use crate::document::SharedDocument;
use crate::file_ops::ask::document_title;
use crate::file_ops::load::reopen;
use crate::file_ops::prefs::{FilePrefs, prefs};
use crate::view::EditorView;

/// What a change of the file leads to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    /// Say it in the status bar.
    Notify,
    /// Read the file again without asking.
    Reload,
    /// Ask.
    Ask,
}

/// `switch( m_eWatchUpdate )` of `CheckFileTimeStamp`. A mode that does not watch never gets here.
fn action_for(mode: WatchMode, modified: bool) -> Action {
    match mode {
        WatchMode::Notify => Action::Notify,
        // `WU_AUTOLOAD` falls through to the question when the document is modified.
        WatchMode::AutoLoad if !modified => Action::Reload,
        _ => Action::Ask,
    }
}

/// The answers of the question (`IDD_FILEUPDATEQUERY`), in the order of its buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reply {
    Reload,
    Close,
    AutoLoad,
    NotifyOnly,
    NoSupervision,
}

const LABELS: [&str; 5] = [
    "再読込(R)",
    "閉じる(C)",
    "以後未編集で再読込(L)",
    "以後通知メッセージのみ(M)",
    "以後更新を監視しない(N)",
];

const REPLIES: [Reply; 5] = [
    Reply::Reload,
    Reply::Close,
    Reply::AutoLoad,
    Reply::NotifyOnly,
    Reply::NoSupervision,
];

/// The text of the question: the file, then `IDC_FILEUPDATEMSG` and `STR_ERR_DLGUPQRY1` (the
/// document is modified) or `STR_ERR_DLGUPQRY2` (`CDlgFileUpdateQuery::OnInitDialog`).
pub(crate) fn question(path: &Path, modified: bool) -> (String, String) {
    let message = format!(
        "{}\nこのファイルは外部のエディタ等で変更されています。",
        document_title(Some(path))
    );
    let detail = if modified {
        "再ロードを行うと変更が失われますがよろしいですか?"
    } else {
        "再ロードしますか?"
    };
    (message, detail.to_owned())
}

/// The status bar text (the `STR_AUTORELOAD_` string of `sakura_rc.rc`) of 「以後通知メッセージのみ」.
pub(crate) fn notify_text(time: SystemTime) -> String {
    let local = chrono::DateTime::<chrono::Local>::from(time);
    format!("★ファイル更新 {}", local.format("%H:%M:%S"))
}

/// `_ToDoChecking` and the delay of `CheckFileTimeStamp`, for the document of the window: whether
/// to look at the file now. If so, the document is marked as pausing, so that no second check
/// starts while this one waits for the disk or the user.
pub(crate) fn begin(
    shared: &mut SharedDocument,
    prefs: &FilePrefs,
) -> Option<(PathBuf, SystemTime)> {
    let disk = shared.disk_mut();
    // 未編集で再ロード時の遅延: the checks between two automatic reloads.
    if disk.mode == WatchMode::AutoLoad {
        disk.delay_count += 1;
        if disk.delay_count < prefs.autoload_delay {
            return None;
        }
        disk.delay_count = 0;
    }
    if disk.pausing
        || !prefs.check_file_time_stamp
        || disk.mode == WatchMode::None
        || prefs.file_share_mode != 0
    {
        return None;
    }
    let stamp = disk.stamp?;
    let path = shared.path()?.to_owned();
    shared.disk_mut().pausing = true;
    Some((path, stamp))
}

/// [`begin`] on the document of `view`.
pub(crate) fn begin_in(
    view: &gpui::Entity<EditorView>,
    cx: &mut gpui::App,
) -> Option<(PathBuf, SystemTime)> {
    let prefs = prefs(cx);
    let document = view.read(cx).document().clone();
    document.update(cx, |shared, _| begin(shared, &prefs))
}

/// Looks at the file of the window's document and does what Sakura does if another program
/// changed it. Returns at once when there is nothing to do.
#[cfg(test)]
pub(crate) async fn check(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    let started = view
        .update(cx, |view, cx| {
            let prefs = prefs(cx);
            view.document()
                .update(cx, |shared, _| begin(shared, &prefs))
        })
        .ok()
        .flatten();
    if let Some((path, stamp)) = started {
        run(view, cx, path, stamp).await;
    }
}

/// The rest of the check, for a document that [`begin`] marked as pausing.
pub(crate) async fn run(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    path: PathBuf,
    stamp: SystemTime,
) {
    // The pause is lifted through the document, which outlives the pane that started the check.
    let Ok(document) = view.read_with(cx, |view, _| view.document().downgrade()) else {
        return;
    };
    // The disk may be slow (a network drive): ask it off the window's thread.
    let asked = path.clone();
    let now = cx
        .background_executor()
        .spawn(async move { modified_time(&asked) })
        .await;
    let Some(now) = now.filter(|now| *now != stamp) else {
        resume(&document, cx);
        return;
    };
    // The document may have been replaced while the disk was asked: then this change is not
    // the one of the stamp above.
    let settled = view
        .update(cx, |view, cx| {
            view.document().update(cx, |shared, _| {
                let same = shared.disk().stamp == Some(stamp) && shared.path() == Some(&path);
                if same {
                    shared.disk_mut().stamp = Some(now);
                }
                same.then(|| (shared.disk().mode, shared.model().is_modified()))
            })
        })
        .ok()
        .flatten();
    let Some((mode, modified)) = settled else {
        resume(&document, cx);
        return;
    };
    match action_for(mode, modified) {
        Action::Notify => {
            view.update(cx, |view, cx| {
                view.notice = Some(notify_text(now));
                cx.notify();
            })
            .ok();
        }
        Action::Reload => {
            reload(view, cx, WatchMode::AutoLoad).await;
        }
        Action::Ask => {
            let (message, detail) = question(&path, modified);
            let reply = match cx
                .prompt(PromptLevel::Warning, &message, Some(&detail), &LABELS)
                .await
            {
                Ok(index) => REPLIES.get(index).copied().unwrap_or(Reply::Close),
                Err(_) => Reply::Close,
            };
            // The window may have loaded another file while the question was open.
            let same = view
                .read_with(cx, |view, app| {
                    let shared = view.document().read(app);
                    shared.path() == Some(path.as_path()) && shared.disk().stamp == Some(now)
                })
                .unwrap_or(false);
            if !same {
                resume(&document, cx);
                return;
            }
            match reply {
                Reply::Reload => reload(view, cx, WatchMode::Query).await,
                Reply::AutoLoad => reload(view, cx, WatchMode::AutoLoad).await,
                Reply::NotifyOnly => set_mode(view, cx, WatchMode::Notify),
                Reply::NoSupervision => set_mode(view, cx, WatchMode::None),
                Reply::Close => set_mode(view, cx, WatchMode::Query),
            }
        }
    }
    resume(&document, cx);
}

/// `ResumeWatching`.
pub(crate) fn resume(document: &WeakEntity<SharedDocument>, cx: &mut AsyncWindowContext) {
    document
        .update(cx, |shared, _| shared.disk_mut().pausing = false)
        .ok();
}

fn set_mode(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext, mode: WatchMode) {
    view.update(cx, |view, cx| {
        view.document().update(cx, |shared, _| {
            let disk = shared.disk_mut();
            disk.mode = mode;
            disk.delay_count = 0;
        })
    })
    .ok();
}

/// `ReloadCurrentFile( GetCodeSet() )` and then the mode: the new document takes it over.
async fn reload(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext, mode: WatchMode) {
    set_mode(view, cx, mode);
    reopen(view, cx, None, true).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mode_and_the_changes_decide_the_action() {
        assert_eq!(action_for(WatchMode::Query, false), Action::Ask);
        assert_eq!(action_for(WatchMode::Query, true), Action::Ask);
        assert_eq!(action_for(WatchMode::Notify, true), Action::Notify);
        assert_eq!(action_for(WatchMode::AutoLoad, false), Action::Reload);
        assert_eq!(action_for(WatchMode::AutoLoad, true), Action::Ask);
    }

    #[test]
    fn the_question_names_the_file_and_the_loss() {
        let (message, detail) = question(Path::new("a.txt"), true);
        assert!(message.starts_with("a.txt\n"));
        assert!(detail.contains("変更が失われます"));
        let (_, detail) = question(Path::new("a.txt"), false);
        assert_eq!(detail, "再ロードしますか?");
    }

    #[test]
    fn the_notice_has_the_clock_time() {
        let text = notify_text(SystemTime::UNIX_EPOCH);
        assert!(text.starts_with("★ファイル更新 "));
        assert_eq!(text.chars().filter(|c| *c == ':').count(), 2);
    }
}
