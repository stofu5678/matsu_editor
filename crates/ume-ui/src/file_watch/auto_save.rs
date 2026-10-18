//! The automatic save (`CAutoSaveAgent::CheckAutoSave`, `sakura_core/agent/CAutoSaveAgent.cpp`).
//!
//! When the interval has passed, every window saves its document like 上書き保存 does, if the
//! document is modified and has a name. A document without a name is not saved (Sakura would
//! open the dialog; it does not). Unmodified documents are left alone even when 「無変更でも上書き
//! する」 is on.

use gpui::{AsyncWindowContext, WeakEntity};

use crate::file_ops::save::{SaveRequest, save};
use crate::view::EditorView;

/// Saves the document of `view` if it is modified and has a file name.
pub(crate) async fn save_if_needed(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    let wanted = view
        .read_with(cx, |view, app| {
            let shared = view.document().read(app);
            shared.model().is_modified() && shared.path().is_some()
        })
        .unwrap_or(false);
    if wanted {
        save(view, cx, SaveRequest::quiet()).await;
    }
}
