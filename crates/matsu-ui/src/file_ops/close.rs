//! Closing: the "save changes?" question, closing the file, the window, or every window.
//!
//! `CEditDoc::OnFileClose` (`sakura_core/doc/CEditDoc.cpp`) asks Yes / No / Cancel when the
//! document is modified; `CDocFileOperation::FileClose` and `FileCloseOpen`
//! (`sakura_core/doc/CDocFileOperation.cpp`) do the closing.

use gpui::{App, AsyncWindowContext, Entity, PromptLevel, WeakEntity, Window};

use super::ask::{Answer, ask, charset_change_message, document_title, save_before_close_message};
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
        view.update(cx, |view, cx| {
            view.record_history(cx);
            // The user saved it or does not want it: the kept copy goes now (ADR-0022).
            crate::unsaved::discard_document(view.document().entity_id(), cx);
        })
        .ok();
    }
    go_on
}

async fn ask_about_changes(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) -> bool {
    if grep_blocks_close(view, cx) {
        return false;
    }
    let Ok((modified, title, only_format)) = view.read_with(cx, |view, app| {
        let shared = view.document().read(app);
        // A Grep window asks only when 「Grepモードで保存確認する」 is on, and then always, also
        // when it is unmodified; any other window asks when it is modified
        // (`CEditDoc::OnFileClose`, `m_bGrepExitConfirm`).
        let title = view
            .grep_title()
            .unwrap_or_else(|| document_title(shared.path(), shared.untitled_number()));
        // Only the encoding or the BOM changed (`IsChgCodeSet`): asked with its own message.
        let only_format =
            !view.is_grep_window() && !shared.model().is_modified() && shared.format_changed();
        let ask = if view.is_grep_window() {
            crate::search::prefs::prefs(app).grep_exit_confirm
        } else {
            shared.model().is_modified() || shared.format_changed()
        };
        (ask, title, only_format)
    }) else {
        return false;
    };
    if !modified {
        return true;
    }
    // Sakura brings the window to the front first; with tabs, the tab too.
    if let Some(entity) = view.upgrade() {
        cx.update(|window, app| {
            show_in_window(&entity, window, app);
            window.activate_window();
        })
        .ok();
    }
    match ask(
        cx,
        PromptLevel::Warning,
        &if only_format {
            charset_change_message(&title)
        } else {
            save_before_close_message(&title)
        },
        None,
        ["保存する(Y)", "保存しない(N)", "キャンセル"],
    )
    .await
    {
        Answer::Yes => save(view, cx, SaveRequest::default()).await.is_safe(),
        Answer::No => true,
        Answer::Cancel => {
            if only_format {
                // `CancelChgCodeSet`: the change of the encoding is given up.
                view.update(cx, |view, cx| {
                    view.document().update(cx, |shared, cx| {
                        shared.cancel_format_change();
                        cx.notify();
                    });
                    cx.notify();
                })
                .ok();
            }
            false
        }
    }
}

/// Makes the tab of `view` the active one in its window.
pub(crate) fn show_in_window(view: &Entity<EditorView>, window: &mut Window, app: &mut App) {
    let host = view.read(app).split_host.as_ref().and_then(|h| h.upgrade());
    if let Some(workspace) = host {
        workspace.update(app, |workspace, cx| workspace.show_view(view, window, cx));
    }
}

/// Closes the tab of `view` without a question, or the window when it is the only tab. The tab
/// may have moved to another window since the flow started, so the window of the view is looked
/// up now, not taken from the flow.
pub(crate) fn close_tab_or_window(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
) -> bool {
    let Some(entity) = view.upgrade() else {
        return false;
    };
    cx.update(|window, app| {
        let Some((handle, _)) = super::windows::find_tab_of(app, view) else {
            return;
        };
        if handle == window.window_handle() {
            close_in(&entity, window, app);
        } else {
            handle
                .update(app, |_, other, app| close_in(&entity, other, app))
                .ok();
        }
    })
    .is_ok()
}

/// Closes the tab of `view` in `window`, which shows it, or the window if it has no other tab.
fn close_in(view: &Entity<EditorView>, window: &mut Window, app: &mut App) {
    let host = view.read(app).split_host.as_ref().and_then(|h| h.upgrade());
    if let Some(workspace) = host {
        let (count, index) = {
            let w = workspace.read(app);
            (w.tab_count(), w.tab_of(view))
        };
        if let (true, Some(index)) = (count > 1, index) {
            workspace.update(app, |w, cx| w.close_tab_now(index, window, cx));
            return;
        }
    }
    // `m_bTab_RetainEmptyWin` (`CEditWnd::DispatchEvent`, `MYWM_CLOSE`): in tab mode the last file
    // of the group closes into an untitled document, and the window stays.
    if crate::tabs::settings::current(app).bound()
        && crate::tabs::settings::options(app).retain_empty
        && super::windows::workspaces(app).len() == 1
    {
        view.update(app, |view, cx| {
            let mut untitled = SharedDocument::empty();
            untitled.set_new_document_defaults(&crate::settings::new_document_defaults(cx, None));
            view.load_document(untitled, None, cx);
        });
        return;
    }
    window.remove_window();
}

/// `F_WINCLOSE`: closes the document after the question: its tab, or the window when it has no
/// other tab (`Command_WINCLOSE` closes the edit window, which is a tab of the group). `false`
/// if the user stopped it.
pub(crate) async fn confirm_and_close_window(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
) -> bool {
    if !confirm_discard(view, cx).await {
        return false;
    }
    close_tab_or_window(view, cx)
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
    view.update(cx, |view, cx| {
        view.record_history(cx);
        // Saved: the kept copy is not wanted any more (ADR-0022).
        crate::unsaved::discard_document(view.document().entity_id(), cx);
    })
    .ok();
    close_tab_or_window(view, cx)
}

/// `F_FILECLOSE`: closes the file and leaves an untitled document in the window.
pub(crate) async fn close_file(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) -> bool {
    if !confirm_discard(view, cx).await {
        return false;
    }
    let closed = view
        .update(cx, |view, cx| {
            let mut untitled = SharedDocument::empty();
            untitled.set_new_document_defaults(&crate::settings::new_document_defaults(cx, None));
            view.load_document(untitled, None, cx);
        })
        .is_ok();
    if closed {
        // `CDocFileOperation::FileClose`: the macro of オープン後 runs for the untitled document.
        crate::macros::run_opened(view, cx).await;
    }
    closed
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
    let Some(mut paths) = choose_files(view, cx, None).await else {
        return false;
    };
    // The first file replaces this document; the others get new windows.
    let first = paths.remove(0);
    // A file that is open in a window is not opened again; that window comes to the front.
    let existing = cx
        .update(|_, app| super::windows::find_open(app, &first))
        .ok()
        .flatten();
    let ok = if let Some((handle, found)) = existing {
        cx.update(|_, app| crate::tabs::flows::reveal(app, handle, found))
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
