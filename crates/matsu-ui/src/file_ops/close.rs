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
use crate::plugin_events::fire_and_wait;
use crate::view::EditorView;
use matsu_config::plugins::def::Jack;

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
    view.update(cx, |view, cx| crate::unsaved::unmark_close(view, cx))
        .ok();
    let go_on = ask_about_changes(view, cx).await;
    if go_on {
        view.update(cx, |view, cx| {
            view.record_history(cx);
            // The user saved it or does not want it. The kept copy goes when the close really happens, not
            // now: the flow may still be cancelled (ADR-0022).
            let this = cx.weak_entity();
            crate::unsaved::mark_close(view, this, cx);
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
        // The close happens: the copy of a document the user did not want goes now.
        crate::unsaved::apply_close(&entity, app);
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

/// Whether closing the tab of `view` closes its window too: it is the only tab, and the window is not
/// the one that stays empty (`m_bTab_RetainEmptyWin`).
fn closes_window(view: &Entity<EditorView>, app: &mut App) -> bool {
    let host = view.read(app).split_host.as_ref().and_then(|h| h.upgrade());
    if let Some(workspace) = host
        && workspace.read(app).tab_count() > 1
    {
        return false;
    }
    !(crate::tabs::settings::current(app).bound()
        && crate::tabs::settings::options(app).retain_empty
        && super::windows::workspaces(app).len() == 1)
}

/// [`close_tab_or_window`] after the plugins of `DocumentClose`, and of `EditorEnd` when the window goes
/// (`MYWM_CLOSE`, `CEditWnd::DispatchEvent`). The question was asked already.
async fn close_with_events(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) -> bool {
    fire_and_wait(view, cx, Jack::DocumentClose).await;
    let ends = match view.upgrade() {
        Some(entity) => cx
            .update(|_, app| closes_window(&entity, app))
            .unwrap_or(false),
        None => false,
    };
    if ends {
        fire_and_wait(view, cx, Jack::EditorEnd).await;
    }
    close_tab_or_window(view, cx)
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

/// [`confirm_discard`] for a flow that ends the editor (an exit of all windows, the close of the last window). When
/// `ends` says that this close really ends the editor (`KeepUnsavedNoAsk`, ADR-0022 section 5): a document whose
/// copy has its latest content is not asked about, it closes and comes back at the next start. `ends` is asked here,
/// right before the question would be left out, not earlier: windows can open while the copies are written. A
/// document that has no such copy (a Grep window, one that is too big, a write that failed, the folder in use by
/// another process) is asked about as always.
pub(crate) async fn confirm_discard_ending(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    ends: &dyn Fn(&mut App) -> bool,
) -> bool {
    confirm_discard_ending_by(view, cx, ends).await.0
}

/// [`confirm_discard_ending`] that also says whether the question was left out because the copy covers the
/// document (the second value), so that a flow that goes on asking about other documents can ask again about this
/// one if the close stops being the end of the editor meanwhile.
pub(crate) async fn confirm_discard_ending_by(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    ends: &dyn Fn(&mut App) -> bool,
) -> (bool, bool) {
    // An answer of an earlier flow that was cancelled does not count any more.
    view.update(cx, |view, cx| crate::unsaved::unmark_close(view, cx))
        .ok();
    let covered = cx.update(|_, app| ends(app)).unwrap_or(false)
        && view.read_with(cx, crate::unsaved::covered).unwrap_or(false);
    if covered {
        // The file goes into the history with its caret, bookmarks and encoding, as it does after the question
        // (`CMruListener::OnBeforeClose`); the copy of the document is not marked for discard.
        view.update(cx, |view, cx| {
            view.record_history(cx);
            // The copy stays when the document goes, also if the process lives on a while (a dialog).
            crate::unsaved::retain_for_exit(view, cx);
        })
        .ok();
        return (true, true);
    }
    (confirm_discard(view, cx).await, false)
}

/// A flow that left questions out for the copies (`confirm_discard_ending`) was cancelled: the documents are not
/// closed, so the copy of none of them stays for the end of the editor any more (a document that goes back to its
/// saved text and closes later must not leave its old copy behind).
pub(crate) fn forget_left_out(views: &[Entity<EditorView>], cx: &mut AsyncWindowContext) {
    cx.update(|_, app| {
        for view in views {
            view.update(app, |view, cx| crate::unsaved::unmark_close(view, cx));
        }
    })
    .ok();
}

/// Whether closing the document of `view` ends the editor without a question about its copy: the setting is on,
/// and the document is the only tab of the last window.
pub(crate) fn closing_ends_the_editor(app: &mut App, view: &WeakEntity<EditorView>) -> bool {
    crate::unsaved::hot_exit_enabled(app)
        // The last tab of a window that stays (`m_bTab_RetainEmptyWin`, `close_in`) is not the end.
        && !(crate::tabs::settings::current(app).bound()
            && crate::tabs::settings::options(app).retain_empty)
        && super::windows::workspaces(app).len() == 1
        && super::windows::find_tab_of(app, view)
            .and_then(|(handle, _)| super::windows::workspace_of(app, handle))
            .is_some_and(|ws| ws.read(app).tab_count() == 1)
}

/// `F_WINCLOSE`: closes the document after the question: its tab, or the window when it has no
/// other tab (`Command_WINCLOSE` closes the edit window, which is a tab of the group). `false`
/// if the user stopped it.
pub(crate) async fn confirm_and_close_window(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
) -> bool {
    // The last tab of the last window ends the editor: with `KeepUnsavedNoAsk` the copy is enough.
    let hot = cx
        .update(|_, app| closing_ends_the_editor(app, view))
        .unwrap_or(false);
    if hot {
        crate::unsaved::settle(cx).await;
    }
    if !confirm_discard_ending(view, cx, &|app| closing_ends_the_editor(app, view)).await {
        return false;
    }
    close_with_events(view, cx).await
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
        // Saved or unchanged: the close that follows makes the discard of its copy final.
        let this = cx.weak_entity();
        crate::unsaved::mark_close(view, this, cx);
    })
    .ok();
    close_with_events(view, cx).await
}

/// `F_FILECLOSE`: closes the file and leaves an untitled document in the window.
pub(crate) async fn close_file(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) -> bool {
    if !confirm_discard(view, cx).await {
        return false;
    }
    fire_and_wait(view, cx, Jack::DocumentClose).await;
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
    // `FileCloseOpen`: the plugins of DocumentClose come before the dialog.
    fire_and_wait(view, cx, Jack::DocumentClose).await;
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

/// Which of Sakura's two ways to quit (the tray process that Sakura also ends is not here).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExitKind {
    /// `F_EXITALL` (`Command_EXITALL`): `CControlTray::TerminateApplication`.
    Application,
    /// `F_EXITALLEDITORS` (`Command_EXITALLEDITORS`): `CControlTray::CloseAllEditor(TRUE, .., TRUE, 0)`.
    Editors,
}

/// `F_EXITALL` and `F_EXITALLEDITORS`: asks about every window with changes, and quits if none of
/// them is stopped.
///
/// A window that was checked stays editable while the later prompts are open. Before quitting the
/// windows and their versions are compared with what was checked; if anything changed, the
/// round starts again, so no change is thrown away without a question.
///
/// `kind` tells which question comes first (see [`ExitKind`]).
pub(crate) async fn exit_all(cx: &mut AsyncWindowContext, kind: ExitKind) {
    let (count, confirm) = cx
        .update(|_, app| {
            (
                super::windows::editors(app).len(),
                crate::common_settings::close_confirm(app),
            )
        })
        .unwrap_or_default();
    match kind {
        // `CControlTray::TerminateApplication`: with `bExitConfirm` and a window open, asks once,
        // and then no longer asks about the number of windows (`STR_TRAY_EXITALL`).
        ExitKind::Application => {
            if confirm.exit
                && count > 0
                && !super::ask::confirm(
                    cx,
                    "現在開いている編集用のウィンドウをすべて閉じて終了しますか?",
                    None,
                    ["はい(Y)", "いいえ(N)"],
                )
                .await
            {
                return;
            }
        }
        // `CloseAllEditor(TRUE, ..)`: the question of `bCloseAllConfirm`.
        ExitKind::Editors => {
            if !crate::window_ops::confirm_closing_several(cx, count).await {
                return;
            }
        }
    }
    // `KeepUnsavedNoAsk`: a document whose copy is up to date is not asked about (ADR-0022, section 5).
    let hot = cx
        .update(|_, app| crate::unsaved::hot_exit_enabled(app))
        .unwrap_or(false);
    loop {
        if hot {
            // The latest content of every document is written before the questions, and again when a round
            // starts over because something changed meanwhile.
            crate::unsaved::settle(cx).await;
        }
        let editors = cx
            .update(|_, app| super::windows::editors(app))
            .unwrap_or_default();
        let mut checked = Vec::new();
        for (handle, view) in editors.clone() {
            let weak = view.downgrade();
            let Ok(mut window_cx) = handle.update(cx, |_, window, app| window.to_async(app)) else {
                continue;
            };
            if !confirm_discard_ending(&weak, &mut window_cx, &|app| {
                crate::unsaved::hot_exit_enabled(app)
            })
            .await
            {
                // The flow is over: what it left out for the copies is taken back.
                let views: Vec<_> = editors.iter().map(|(_, v)| v.clone()).collect();
                forget_left_out(&views, cx);
                return;
            }
            let version = weak
                .read_with(cx, |view, app| view.document().read(app).version())
                .unwrap_or(0);
            checked.push((view.entity_id(), version));
        }
        if snapshot(cx) == checked {
            cx.update(|_, app| {
                // Every question is answered: what the user threw away leaves no copy.
                crate::unsaved::apply_all_closes(app);
            })
            .ok();
            // The windows go: the plugins of DocumentClose for each tab and of EditorEnd for each window
            // (`MYWM_CLOSE` of each editor).
            let windows = cx
                .update(|_, app| {
                    super::windows::workspaces(app)
                        .into_iter()
                        .map(|(_, ws)| {
                            ws.read(app)
                                .tab_editors()
                                .iter()
                                .map(Entity::downgrade)
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            for views in &windows {
                crate::plugin_events::window_closing(views, cx).await;
            }
            cx.update(|_, app| app.quit()).ok();
            return;
        }
    }
}
