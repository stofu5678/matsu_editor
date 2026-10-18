//! The automatic save (`CAutoSaveAgent::CheckAutoSave`, `sakura_core/agent/CAutoSaveAgent.cpp`).
//!
//! When the interval has passed, every window saves its documents like 上書き保存 does, if the
//! document is modified and has a name. A document without a name is not saved (Sakura would
//! open the dialog; it does not). Unmodified documents are left alone even when 「無変更でも上書き
//! する」 is on.

use gpui::{App, AsyncWindowContext, Entity, WeakEntity};

use super::state::modified_time;
use crate::document::SharedDocument;
use crate::file_ops::prefs::{FilePrefs, prefs};
use crate::file_ops::save::{SaveRequest, save};
use crate::view::EditorView;

/// Whether the document is to be saved now, and if so marks it as pausing: the check for changes
/// of its file waits until the save is done (`OnBeforeSave` / `OnAfterSave` of
/// `CAutoReloadAgent`), and a second automatic save does not start while the first one asks a
/// question.
///
/// A document waits while its file is being looked at (the question may reload the file), and
/// while the file has been changed by another program and the window has not asked yet: the save
/// would overwrite what the other program wrote. The next interval tries again.
pub(crate) fn begin(shared: &mut SharedDocument, prefs: &FilePrefs) -> bool {
    let Some(path) = shared.path() else {
        return false;
    };
    let disk = shared.disk();
    if !shared.model().is_modified() || disk.pausing {
        return false;
    }
    let watched = prefs.check_file_time_stamp
        && prefs.file_share_mode == 0
        && disk.mode != super::state::WatchMode::None;
    if watched
        && let (Some(stamp), Some(now)) = (disk.stamp, modified_time(path))
        && stamp != now
    {
        return false;
    }
    shared.disk_mut().pausing = true;
    true
}

/// [`begin`] on the document of `view`.
pub(crate) fn begin_in(view: &Entity<EditorView>, cx: &mut App) -> bool {
    let prefs = prefs(cx);
    let document = view.read(cx).document().clone();
    document.update(cx, |shared, _| begin(shared, &prefs))
}

/// Saves the document that [`begin`] marked, then lets the watching go on.
pub(crate) async fn run(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    save(view, cx, SaveRequest::quiet()).await;
    view.update(cx, |view, cx| {
        view.document()
            .update(cx, |shared, _| shared.disk_mut().pausing = false)
    })
    .ok();
}
