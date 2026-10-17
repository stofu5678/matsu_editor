//! What the tab functions do across windows: which documents a group-wide function reaches, where
//! a new document opens, gathering the windows into tabs and spreading them again, and moving a
//! tab to another window.
//!
//! Sakura: a group is the set of windows that share a frame (`EditNode::m_nGroup`,
//! `sakura_core/env/CAppNodeManager.cpp`). Here a group is one window. In the tab mode of Sakura
//! (`m_bDispTabWnd && !m_bDispTabWndMultiWin`, [`TabSettings::bound`]) every window is its own
//! group; otherwise all windows are in one group (`ResetGroupId` gives them the same number), and
//! each holds one document.

use gpui::{AnyWindowHandle, App, AppContext as _, Context, Entity, Window};

use super::order;
use super::settings::{self, TabSettings};
use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::file_ops::close::show_in_window;
use crate::file_ops::windows::{
    LoadRequest, editors, has_pending, open_editor_window, open_window_with_panes, workspace_of,
    workspaces,
};
use crate::panes::PaneSet;
use crate::view::EditorView;

/// The documents of the group of the window `handle`, in the order of the bar (or of the
/// windows): the tabs of the window in tab mode, else every document.
pub(crate) fn group_views(
    cx: &mut App,
    handle: AnyWindowHandle,
) -> Vec<(AnyWindowHandle, Entity<EditorView>)> {
    if settings::current(cx).bound() {
        editors(cx)
            .into_iter()
            .filter(|(h, _)| *h == handle)
            .collect()
    } else {
        editors(cx)
    }
}

/// Brings the document of `view` to the front: its window, and its tab. It happens after the
/// function that asks for it ends (the window of that function may be the one to change).
pub(crate) fn reveal(cx: &mut App, handle: AnyWindowHandle, view: Entity<EditorView>) {
    cx.spawn(async move |cx| {
        cx.update(|app| {
            handle
                .update(app, |_, window, app| {
                    show_in_window(&view, window, app);
                    window.activate_window();
                })
                .ok();
        });
    })
    .detach();
}

/// Opens a document the way Sakura's `CControlTray::OpenNewEditor` does
/// (`sakura_core/_main/CControlTray.cpp`): in tab mode it joins the group of the window `from`
/// (a new tab there); otherwise, or with `new_window` (`bNewWindow`, `F_FILENEW_NEWWINDOW`), it
/// gets a new window. `here` is the window that runs the function, if it has one in hand.
///
/// Returns the window and the view of the new document.
pub(crate) fn open_document(
    cx: &mut App,
    here: Option<&mut Window>,
    from: AnyWindowHandle,
    new_window: bool,
    shared: SharedDocument,
    load: Option<LoadRequest>,
) -> gpui::Result<(AnyWindowHandle, Entity<EditorView>)> {
    let mut payload = Some((shared, load));
    if settings::current(cx).bound()
        && !new_window
        && let Some(workspace) = workspace_of(cx, from)
    {
        let user = cx.try_global::<crate::settings::UserSettings>().cloned();
        let mut make = |workspace: &mut Workspace,
                        window: &mut Window,
                        cx: &mut Context<Workspace>| {
            let (mut shared, load) = payload.take()?;
            if load.is_none() {
                shared.set_new_document_defaults(&crate::settings::new_document_defaults(cx, None));
            }
            let document = cx.new(|_| shared);
            let editor = cx.new(|cx| EditorView::new(document, cx));
            if let Some(user) = &user {
                editor.update(cx, |view, cx| view.apply_user_settings(user, cx));
            }
            workspace.insert_tab(PaneSet::new(editor.clone()), usize::MAX, true, window, cx);
            if let Some(load) = load {
                crate::file_ops::windows::begin_pending(
                    cx,
                    window.window_handle(),
                    &editor.downgrade(),
                    load.path.clone(),
                );
                editor.update(cx, |view, cx| {
                    view.load_path_here(load.path, load.encoding, window, cx);
                });
            }
            Some(editor)
        };
        let made = match here {
            Some(window) if window.window_handle() == from => {
                workspace.update(cx, |ws, wcx| make(ws, window, wcx))
            }
            _ => from
                .update(cx, |_, window, app| {
                    workspace.update(app, |ws, wcx| make(ws, window, wcx))
                })
                .ok()
                .flatten(),
        };
        if let Some(view) = made {
            return Ok((from, view));
        }
    }
    let (shared, load) = payload.take().expect("the document was not used");
    let handle = open_editor_window(cx, shared, load)?;
    let view = editors(cx)
        .into_iter()
        .rfind(|(h, _)| *h == handle)
        .map(|(_, view)| view)
        .ok_or_else(anyhow_closed)?;
    Ok((handle, view))
}

fn anyhow_closed() -> gpui::private::anyhow::Error {
    gpui::private::anyhow::anyhow!("the new window closed at once")
}

/// `ResetGroupId` after `F_SHOWTAB` and `F_BIND_WINDOW` (`CAppNodeManager::ResetGroupId`): in tab
/// mode the documents of all windows gather in the tabs of `window`; otherwise every tab of every
/// window gets a window of its own. A window that is still reading a file is left as it is.
/// `window` is the window that runs the function.
pub(crate) fn regroup(cx: &mut App, window: &mut Window) {
    let current = window.window_handle();
    let bound = settings::current(cx).bound();
    let all = workspaces(cx);
    if bound {
        let Some(me) = all
            .iter()
            .find(|(h, _)| *h == current)
            .map(|(_, ws)| ws.clone())
        else {
            return;
        };
        let at = all.iter().position(|(h, _)| *h == current).unwrap_or(0);
        // Windows opened before this one put their tabs in front of its own, the later ones
        // after (Sakura orders the tabs by the number of the window).
        let mut front = 0;
        for (i, (handle, other)) in all.iter().enumerate() {
            if *handle == current || has_pending(cx, *handle) {
                continue;
            }
            let sets = handle
                .update(cx, |_, other_window, app| {
                    let sets = other.update(app, |ws, wcx| ws.take_all_tabs(other_window, wcx));
                    other_window.remove_window();
                    sets
                })
                .unwrap_or_default();
            for set in sets {
                let place = if i < at {
                    front += 1;
                    front - 1
                } else {
                    usize::MAX
                };
                me.update(cx, |ws, wcx| ws.insert_tab(set, place, false, window, wcx));
            }
        }
    } else {
        for (handle, ws) in all {
            if ws.read(cx).tab_count() < 2 || has_pending(cx, handle) {
                continue;
            }
            let sets = if handle == current {
                ws.update(cx, |ws, wcx| ws.take_other_tabs(window, wcx))
            } else {
                handle
                    .update(cx, |_, other, app| {
                        ws.update(app, |ws, wcx| ws.take_other_tabs(other, wcx))
                    })
                    .unwrap_or_default()
            };
            for set in sets {
                // A tab that cannot get a window stays where it was.
                if let Err(Some(set)) = open_window_with_panes(cx, set) {
                    let set = *set;
                    if handle == current {
                        ws.update(cx, |ws, wcx| {
                            ws.insert_tab(set, usize::MAX, false, window, wcx)
                        });
                    } else {
                        handle
                            .update(cx, |_, other, app| {
                                ws.update(app, |ws, wcx| {
                                    ws.insert_tab(set, usize::MAX, false, other, wcx)
                                })
                            })
                            .ok();
                    }
                }
            }
        }
    }
}

/// Switches the tab bar and the grouping, and gathers or spreads the windows
/// (`Command_SHOWTAB`, `Command_BIND_WINDOW` in `sakura_core/cmd/CViewCommander_Settings.cpp` and
/// `CViewCommander_Window.cpp`). Both end with `ResetGroupId` and a message to all windows.
pub(crate) fn change_settings(
    cx: &mut App,
    window: &mut Window,
    change: impl FnOnce(&mut TabSettings),
) {
    // A tab that is being read cannot move: its task belongs to the window it started in. The
    // bar stays as it is until the reading ends.
    let reading = workspaces(cx)
        .into_iter()
        .any(|(handle, _)| has_pending(cx, handle));
    if reading {
        if let Some(workspace) = workspace_of(cx, window.window_handle()) {
            let editor = workspace.read(cx).editor().clone();
            editor.update(cx, |view, cx| {
                view.notice =
                    Some("ファイルの読み込み中は、タブの表示を切り替えられない".to_owned());
                cx.notify();
            });
        }
        return;
    }
    settings::update(cx, change);
    regroup(cx, window);
}

impl Workspace {
    /// Takes every tab but the active one out of the window, left to right.
    pub(crate) fn take_other_tabs(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<PaneSet> {
        let mut taken = Vec::new();
        for index in (0..self.tab_count()).rev() {
            if index != self.active_tab()
                && let Some(set) = self.take_tab(index, window, cx)
            {
                taken.push(set);
            }
        }
        taken.reverse();
        taken
    }
}

/// `F_NEXTGROUP` / `F_PREVGROUP`: brings the next or previous window to the front. Only in tab
/// mode, where a window is a group (`CTabWnd::GetNextGroupWnd` answers nothing otherwise).
pub(crate) fn go_to_group(cx: &mut App, current: AnyWindowHandle, next: bool) {
    if !settings::current(cx).bound() {
        return;
    }
    let all = workspaces(cx);
    let Some(at) = all.iter().position(|(h, _)| *h == current) else {
        return;
    };
    let target = if next {
        order::next_group(at, all.len())
    } else {
        order::prev_group(at, all.len())
    };
    if let Some(target) = target {
        crate::file_ops::windows::activate(cx, all[target].0);
    }
}

/// `F_TAB_SEPARATE` (新規グループ): the tab goes to a window of its own
/// (`CTabWnd::Separate`). A window with one tab has nothing to separate from. Only in tab mode.
pub(crate) fn separate_tab(cx: &mut App, window: &mut Window) -> bool {
    let handle = window.window_handle();
    if !settings::current(cx).bound() || has_pending(cx, handle) {
        return false;
    }
    let Some(workspace) = workspace_of(cx, handle) else {
        return false;
    };
    if workspace.read(cx).tab_count() < 2 {
        return false;
    }
    let set = workspace.update(cx, |ws, wcx| {
        let active = ws.active_tab();
        ws.take_tab(active, window, wcx)
    });
    match set {
        Some(set) => match open_window_with_panes(cx, set) {
            Ok(_) => true,
            Err(back) => {
                // The tab stays where it was.
                if let Some(set) = back.map(|b| *b) {
                    workspace.update(cx, |ws, wcx| {
                        ws.insert_tab(set, usize::MAX, true, window, wcx)
                    });
                }
                false
            }
        },
        None => false,
    }
}

/// `F_TAB_JOINTNEXT` / `F_TAB_JOINTPREV` (次・前のグループに移動): the tab moves to the next or
/// previous window and becomes its active tab (`CTabWnd::JoinNext`). A window left without tabs
/// closes. Only in tab mode, and when there is another window.
pub(crate) fn join_tab(cx: &mut App, window: &mut Window, next: bool) -> bool {
    let handle = window.window_handle();
    if !settings::current(cx).bound() || has_pending(cx, handle) {
        return false;
    }
    let all = workspaces(cx);
    let Some(at) = all.iter().position(|(h, _)| *h == handle) else {
        return false;
    };
    let target = if next {
        order::next_group(at, all.len())
    } else {
        order::prev_group(at, all.len())
    };
    let Some(target) = target else {
        return false;
    };
    let (target_handle, target_ws) = all[target].clone();
    let workspace = all[at].1.clone();
    let alone = workspace.read(cx).tab_count() < 2;
    let set = workspace.update(cx, |ws, wcx| {
        if alone {
            ws.take_all_tabs(window, wcx).pop()
        } else {
            let active = ws.active_tab();
            ws.take_tab(active, window, wcx)
        }
    });
    let Some(set) = set else {
        return false;
    };
    target_handle
        .update(cx, |_, target_window, app| {
            target_ws.update(app, |ws, wcx| {
                ws.insert_tab(set, usize::MAX, true, target_window, wcx)
            });
            target_window.activate_window();
        })
        .ok();
    if alone {
        window.remove_window();
    }
    true
}

/// `F_TAB_CLOSELEFT` / `F_TAB_CLOSERIGHT`: closes the tabs on one side of this one, asking about
/// each that has unsaved changes (`Command_TAB_CLOSELEFT`: `RequestCloseEditor`). Sakura does it
/// when the tab bar is shown.
pub(crate) async fn close_side(
    cx: &mut gpui::AsyncWindowContext,
    handle: AnyWindowHandle,
    left: bool,
) -> bool {
    let plan = cx
        .update(|_, app| {
            if !settings::current(app).show_bar {
                return None;
            }
            let current = workspace_of(app, handle)?.read(app).editor().clone();
            let views = group_views(app, handle);
            let at = views.iter().position(|(_, v)| *v == current)?;
            let range = if left {
                order::left_range(at)
            } else {
                order::right_range(at, views.len())
            };
            Some(views[range].to_vec())
        })
        .ok()
        .flatten();
    match plan {
        Some(views) => crate::window_ops::close_views(cx, views).await,
        None => false,
    }
}
