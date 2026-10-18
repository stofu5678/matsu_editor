//! The automatic save (`CAutoSaveAgent::CheckAutoSave`, `sakura_core/agent/CAutoSaveAgent.cpp`).
//!
//! When the interval has passed, every window saves its documents like 上書き保存 does, if the
//! document is modified and has a name. A document without a name is not saved (Sakura would
//! open the dialog; it does not). Unmodified documents are left alone even when 「無変更でも上書き
//! する」 is on.

use gpui::{App, AsyncWindowContext, Entity, WeakEntity};

use super::check::resume;
use super::state::{WatchMode, modified_time};
use crate::document::SharedDocument;
use crate::file_ops::prefs::{FilePrefs, prefs};
use crate::file_ops::save::{SaveRequest, save};
use crate::view::EditorView;

/// Whether the document is to be saved now, and if so marks it as pausing: the check for changes
/// of its file waits until the save is done (`OnBeforeSave` / `OnAfterSave` of
/// `CAutoReloadAgent`), and a second automatic save does not start while the first one asks a
/// question. A document waits while its file is being looked at: the question may reload the file.
pub(crate) fn begin(shared: &mut SharedDocument) -> bool {
    if shared.path().is_none() || !shared.model().is_modified() || shared.disk().pausing {
        return false;
    }
    shared.disk_mut().pausing = true;
    true
}

/// [`begin`] on the document of `view`.
pub(crate) fn begin_in(view: &Entity<EditorView>, cx: &mut App) -> bool {
    let document = view.read(cx).document().clone();
    document.update(cx, |shared, _| begin(shared))
}

/// The time stamp the file is expected to have, when the file is watched: if the file has another
/// one, a program changed it and the window has not asked yet, so a save would overwrite what the
/// other program wrote.
fn expected_stamp(
    shared: &SharedDocument,
    prefs: &FilePrefs,
) -> Option<(std::path::PathBuf, std::time::SystemTime)> {
    let disk = shared.disk();
    let watched =
        prefs.check_file_time_stamp && prefs.file_share_mode == 0 && disk.mode != WatchMode::None;
    Some((shared.path()?.to_owned(), disk.stamp.filter(|_| watched)?))
}

/// Saves the document that [`begin`] marked, then lets the watching go on. The next interval
/// tries again if the file was changed meanwhile.
pub(crate) async fn run(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    let Ok(document) = view.read_with(cx, |view, _| view.document().downgrade()) else {
        return;
    };
    let prefs = cx.update(|_, app| prefs(app)).unwrap_or_default();
    let expected = document
        .read_with(cx, |shared, _| expected_stamp(shared, &prefs))
        .ok()
        .flatten();
    let changed = match expected {
        Some((path, stamp)) => {
            // The disk may be slow (a network drive): ask it off the window's thread.
            let now = cx
                .background_executor()
                .spawn(async move { modified_time(&path) })
                .await;
            now.is_some_and(|now| now != stamp)
        }
        None => false,
    };
    if !changed {
        save(view, cx, SaveRequest::quiet()).await;
    }
    resume(&document, cx);
}
