//! Closing: the "save changes?" question, closing the file, the window, or every window.
//!
//! `CEditDoc::OnFileClose` (`sakura_core/doc/CEditDoc.cpp`) asks Yes / No / Cancel when the
//! document is modified; `CDocFileOperation::FileClose` and `FileCloseOpen`
//! (`sakura_core/doc/CDocFileOperation.cpp`) do the closing.

use gpui::{AsyncWindowContext, PromptLevel, WeakEntity};

use super::ask::{Answer, ask, document_title, save_before_close_message};
use super::load::{choose_files, load_here, open_paths};
use super::save::{SaveRequest, save};
use crate::document::SharedDocument;
use crate::view::EditorView;

/// Nothing is closed while Grep searches (`CGrepAgent::OnBeforeClose`): the window says so
/// ("Grepの処理中です。") and this returns `true`.
fn grep_blocks_close(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) -> bool {
    let running = view
        .read_with(cx, |v, _| v.is_grep_running())
        .unwrap_or(false);
    if running {
        view.update(cx, |view, cx| {
            view.notice = Some("Grepの処理中です。".to_owned());
            cx.notify();
        })
        .ok();
    }
    running
}

/// Asks what to do with unsaved changes before the document goes away. `true` means go on: the
/// document is unmodified, was saved, or the user does not want to save it. `false` means stop.
/// When it goes on, the file goes to the front of the history with the caret and the bookmarks it
/// has now (`CMruListener::OnBeforeClose`).
pub(crate) async fn confirm_discard(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
) -> bool {
    let go_on = ask_about_changes(view, cx).await;
    if go_on {
        view.update(cx, |view, cx| view.record_history(cx)).ok();
    }
    go_on
}

async fn ask_about_changes(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) -> bool {
    if grep_blocks_close(view, cx) {
        return false;
    }
    let Ok((modified, title)) = view.read_with(cx, |view, app| {
        let shared = view.document().read(app);
        // A Grep window is closed without the question (`m_bGrepExitConfirm` is off).
        (
            shared.model().is_modified() && !view.is_grep_window(),
            document_title(shared.path()),
        )
    }) else {
        return false;
    };
    if !modified {
        return true;
    }
    // Sakura brings the window to the front first.
    cx.update(|window, _| window.activate_window()).ok();
    match ask(
        cx,
        PromptLevel::Warning,
        &save_before_close_message(&title),
        None,
        ["保存する(Y)", "保存しない(N)", "キャンセル"],
    )
    .await
    {
        Answer::Yes => save(view, cx, SaveRequest::default()).await.is_safe(),
        Answer::No => true,
        Answer::Cancel => false,
    }
}

/// `F_WINCLOSE`: closes the window after the question. `false` if the user stopped it.
pub(crate) async fn confirm_and_close_window(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
) -> bool {
    if !confirm_discard(view, cx).await {
        return false;
    }
    cx.update(|window, _| window.remove_window()).is_ok()
}

/// `F_FILESAVECLOSE`: saves and closes the window. An unmodified document is only closed
/// (`CViewCommander::HandleCommand`, `sakura_core/cmd/CViewCommander.cpp`).
pub(crate) async fn save_and_close_window(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
) -> bool {
    if grep_blocks_close(view, cx) {
        return false;
    }
    let modified = view
        .read_with(cx, |view, app| {
            view.document().read(app).model().is_modified()
        })
        .unwrap_or(false);
    if modified && !save(view, cx, SaveRequest::default()).await.is_safe() {
        return false;
    }
    view.update(cx, |view, cx| view.record_history(cx)).ok();
    cx.update(|window, _| window.remove_window()).is_ok()
}

/// `F_FILECLOSE`: closes the file and leaves an untitled document in the window.
pub(crate) async fn close_file(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) -> bool {
    if !confirm_discard(view, cx).await {
        return false;
    }
    view.update(cx, |view, cx| {
        let mut untitled = SharedDocument::empty();
        untitled.set_new_document_defaults(&crate::settings::new_document_defaults(cx, None));
        view.load_document(untitled, None, cx);
    })
    .is_ok()
}

/// `F_FILECLOSE_OPEN`: asks about the changes, shows the open dialog, and replaces the document
/// with the file. Cancelling the dialog keeps the file open (`FileCloseOpen`).
pub(crate) async fn close_and_open(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
) -> bool {
    if !confirm_discard(view, cx).await {
        return false;
    }
    let Some(mut paths) = choose_files(cx).await else {
        return false;
    };
    // The first file replaces this document; the others get new windows.
    let first = paths.remove(0);
    // A file that is open in a window is not opened again; that window comes to the front.
    let existing = cx
        .update(|_, app| super::windows::find_open(app, &first).map(|(h, _)| h))
        .ok()
        .flatten();
    let ok = if let Some(handle) = existing {
        cx.update(|_, app| super::windows::activate(app, handle))
            .ok();
        true
    } else {
        load_here(view, cx, &first, None, None).await
    };
    if !paths.is_empty() {
        open_paths(view, cx, paths, None).await;
    }
    ok
}

/// What `exit_all` checks again before it quits: each window and the version of its document.
fn snapshot(cx: &mut AsyncWindowContext) -> Vec<(gpui::EntityId, u64)> {
    cx.update(|_, app| {
        super::windows::editors(app)
            .into_iter()
            .map(|(_, view)| {
                let version = view.read(app).document().read(app).version();
                (view.entity_id(), version)
            })
            .collect()
    })
    .unwrap_or_default()
}

/// `F_EXITALL`: asks about every window with changes, and quits if none of them is stopped.
///
/// A window that was checked stays editable while the later prompts are open. Before quitting the
/// windows and their versions are compared with what was checked; if anything changed, the
/// round starts again, so no change is thrown away without a question.
pub(crate) async fn exit_all(cx: &mut AsyncWindowContext) {
    loop {
        let editors = cx
            .update(|_, app| super::windows::editors(app))
            .unwrap_or_default();
        let mut checked = Vec::new();
        for (handle, view) in editors {
            let weak = view.downgrade();
            let Ok(mut window_cx) = handle.update(cx, |_, window, app| window.to_async(app)) else {
                continue;
            };
            if !confirm_discard(&weak, &mut window_cx).await {
                return;
            }
            let version = weak
                .read_with(cx, |view, app| view.document().read(app).version())
                .unwrap_or(0);
            checked.push((view.entity_id(), version));
        }
        if snapshot(cx) == checked {
            cx.update(|_, app| app.quit()).ok();
            return;
        }
    }
}
