//! Editor windows: making them, and finding the ones that are open.
//!
//! Sakura starts a new editor window for a new file, for each file after the first of an open
//! dialog, and for a file when this window already has one (`CControlTray::OpenNewEditor`,
//! `sakura_core/_main/CControlTray.cpp`). A file that is already open in a window is not opened
//! again; that window comes to the front (`CShareData::ActiveAlreadyOpenedWindow`).

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui::{
    AnyWindowHandle, App, AppContext as _, Bounds, Entity, Global, WeakEntity, Window,
    WindowBounds, WindowOptions, px, size,
};
use gpui_component::Root;
use ume_core::encoding::Encoding;

use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::panes::PaneSet;
use crate::view::EditorView;

use super::close;

/// The editor windows of the application, for the flows that look at more than one.
#[derive(Default)]
struct EditorWindows {
    windows: Vec<(AnyWindowHandle, WeakEntity<Workspace>)>,
    /// Files that views are still reading, with the window of each. Their documents have no path
    /// yet, but opening the same file again must not make a second editable copy.
    pending: Vec<(
        AnyWindowHandle,
        WeakEntity<EditorView>,
        gpui::EntityId,
        PathBuf,
    )>,
}

impl Global for EditorWindows {}

/// A file to load into a new window once it is open.
#[derive(Clone, Debug)]
pub(crate) struct LoadRequest {
    pub path: PathBuf,
    /// Read the file in this encoding instead of detecting it.
    pub encoding: Option<Encoding>,
}

/// Opens an editor window with `shared`, and then loads `load` into it if given (the window
/// shows the progress of a big file and the errors, so the load is started from inside).
pub(crate) fn open_editor_window(
    cx: &mut App,
    shared: SharedDocument,
    load: Option<LoadRequest>,
) -> gpui::Result<AnyWindowHandle> {
    let mut shared = shared;
    if load.is_none() {
        // A new document has the settings of the basic type (a file that is loaded replaces it).
        shared.set_new_document_defaults(&crate::settings::new_document_defaults(cx, None));
    }
    // ume.ini, read once at startup (`app::run`); every window applies it.
    let user = cx.try_global::<crate::settings::UserSettings>().cloned();
    open_window_with(cx, move |window, cx| {
        let document = cx.new(|_| shared);
        let workspace = cx.new(|cx| Workspace::new(document, window, cx));
        let editor = workspace.read(cx).editor().clone();
        if let Some(user) = &user {
            editor.update(cx, |view, cx| view.apply_user_settings(user, cx));
        }
        if let Some(load) = load {
            begin_pending(
                cx,
                window.window_handle(),
                &editor.downgrade(),
                load.path.clone(),
            );
            editor.update(cx, |view, cx| {
                view.load_path_here(load.path, load.encoding, window, cx);
            });
        }
        workspace
    })
}

/// Opens a window for a tab that comes from another window (`F_TAB_SEPARATE`, ungrouping): the
/// panes keep their views, carets and everything else. If the window cannot be made, the panes
/// come back so that the caller can put the tab where it was (it may hold unsaved changes).
pub(crate) fn open_window_with_panes(
    cx: &mut App,
    panes: PaneSet,
) -> Result<AnyWindowHandle, Option<Box<PaneSet>>> {
    let slot = Rc::new(std::cell::RefCell::new(Some(panes)));
    let taken = slot.clone();
    open_window_with(cx, move |window, cx| {
        let panes = taken.borrow_mut().take().expect("the panes are used once");
        cx.new(|cx| Workspace::with_panes(panes, window, cx))
    })
    .map_err(|_| slot.borrow_mut().take().map(Box::new))
}

/// Opens a window whose content `build` makes, and registers it.
fn open_window_with(
    cx: &mut App,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<Workspace> + 'static,
) -> gpui::Result<AnyWindowHandle> {
    let count = window_count(cx);
    let mut bounds = Bounds::centered(None, size(px(1000.), px(700.)), cx);
    // Windows after the first are shifted so that they do not hide each other.
    let shift = px(30. * (count % 8) as f32);
    bounds.origin.x += shift;
    bounds.origin.y += shift;
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some("UME Editor".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let handle = cx.open_window(options, move |window, cx| {
        let workspace = build(window, cx);
        install(window, cx, &workspace);
        cx.new(|cx| Root::new(workspace, window, cx))
    })?;
    Ok(handle.into())
}

/// Registers the window, and asks about unsaved changes when the user closes it.
fn install(window: &mut Window, cx: &mut App, workspace: &Entity<Workspace>) {
    cx.default_global::<EditorWindows>()
        .windows
        .push((window.window_handle(), workspace.downgrade()));
    let workspace = workspace.downgrade();
    let asking = Rc::new(Cell::new(false));
    window.on_window_should_close(cx, move |window, cx| {
        let Some(workspace) = workspace.upgrade() else {
            return true;
        };
        // Every tab is asked about (`RequestCloseEditor` goes to each window of the group).
        let views = workspace.read(cx).tab_editors();
        // A Grep window cannot be closed while it searches (`CGrepAgent::OnBeforeClose`), and
        // it is closed without the question about saving (`CEditDoc::OnFileClose`).
        if let Some(busy) = views.iter().find(|v| v.read(cx).is_grep_running()) {
            busy.update(cx, |view, cx| {
                view.notice = Some("Grepの処理中です。".to_owned());
                cx.notify();
            });
            return false;
        }
        let needs_question = |view: &Entity<EditorView>, cx: &App| {
            !view.read(cx).is_grep_window()
                && view.read(cx).document().read(cx).model().is_modified()
        };
        if !views.iter().any(|v| needs_question(v, cx)) {
            for view in &views {
                view.update(cx, |view, cx| view.record_history(cx));
            }
            return true;
        }
        if !asking.replace(true) {
            let workspace = workspace.downgrade();
            let asking = asking.clone();
            window
                .spawn(cx, async move |cx| {
                    close_after_questions(&workspace, cx).await;
                    asking.set(false);
                })
                .detach();
        }
        false
    });
}

/// Asks about each tab with unsaved changes, then closes the window. Another window can move a
/// tab here while a question is open, so the tabs and their versions are checked again before the
/// window goes (`exit_all` does the same): nothing is thrown away without a question.
async fn close_after_questions(
    workspace: &WeakEntity<Workspace>,
    cx: &mut gpui::AsyncWindowContext,
) {
    let mut asked: Vec<(gpui::EntityId, u64)> = Vec::new();
    loop {
        let Ok(views) = workspace.read_with(cx, |ws, _| ws.tab_editors()) else {
            return;
        };
        for view in &views {
            let version = view.read_with(cx, |v, app| v.document().read(app).version());
            if asked.contains(&(view.entity_id(), version)) {
                continue;
            }
            if !close::confirm_discard(&view.downgrade(), cx).await {
                return;
            }
            let version = view.read_with(cx, |v, app| v.document().read(app).version());
            asked.push((view.entity_id(), version));
        }
        let unchanged = workspace
            .read_with(cx, |ws, app| {
                ws.tab_editors().iter().all(|v| {
                    asked.contains(&(v.entity_id(), v.read(app).document().read(app).version()))
                })
            })
            .unwrap_or(true);
        if unchanged {
            cx.update(|window, _| window.remove_window()).ok();
            return;
        }
    }
}

/// The windows that are open with their workspaces, oldest first.
pub(crate) fn workspaces(cx: &mut App) -> Vec<(AnyWindowHandle, Entity<Workspace>)> {
    let open = cx.windows();
    let list = cx.default_global::<EditorWindows>();
    list.windows
        .retain(|(handle, ws)| open.contains(handle) && ws.upgrade().is_some());
    list.pending
        .retain(|(handle, view, _, _)| open.contains(handle) && view.upgrade().is_some());
    list.windows
        .iter()
        .filter_map(|(handle, ws)| Some((*handle, ws.upgrade()?)))
        .collect()
}

/// The registered windows as they are, without forgetting the ones that look closed. A window
/// that is being drawn for the first time is not among the open windows yet, so what draws (the
/// tab bar) must not prune the list.
pub(crate) fn listed_workspaces(cx: &App) -> Vec<(AnyWindowHandle, Entity<Workspace>)> {
    cx.try_global::<EditorWindows>()
        .map_or_else(Vec::new, |list| {
            list.windows
                .iter()
                .filter_map(|(handle, ws)| Some((*handle, ws.upgrade()?)))
                .collect()
        })
}

/// The number of editor windows.
pub(crate) fn window_count(cx: &mut App) -> usize {
    workspaces(cx).len()
}

/// The workspace of a window.
pub(crate) fn workspace_of(cx: &mut App, handle: AnyWindowHandle) -> Option<Entity<Workspace>> {
    workspaces(cx)
        .into_iter()
        .find(|(h, _)| *h == handle)
        .map(|(_, ws)| ws)
}

/// The documents that are open, oldest window first, the tabs of a window in the order of its
/// bar: the window and the view of each tab (its active pane). Without tabs this is one view for
/// each window.
pub(crate) fn editors(cx: &mut App) -> Vec<(AnyWindowHandle, Entity<EditorView>)> {
    workspaces(cx)
        .into_iter()
        .flat_map(|(handle, ws)| {
            ws.read(cx)
                .tab_editors()
                .into_iter()
                .map(move |view| (handle, view))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The window and the view (the active pane of the tab) of the tab that has the pane `view`.
pub(crate) fn find_tab_of(
    cx: &mut App,
    view: &WeakEntity<EditorView>,
) -> Option<(AnyWindowHandle, Entity<EditorView>)> {
    let view = view.upgrade()?;
    workspaces(cx).into_iter().find_map(|(handle, ws)| {
        let ws = ws.read(cx);
        let tab = ws.tab_of(&view)?;
        Some((handle, ws.tab_editor(tab)?))
    })
}

/// The window that has `path` open.
pub(crate) fn find_open(
    cx: &mut App,
    path: &Path,
) -> Option<(AnyWindowHandle, Entity<EditorView>)> {
    let found = editors(cx).into_iter().find(|(_, view)| {
        view.read(cx)
            .document()
            .read(cx)
            .path()
            .is_some_and(|open| same_file(open, path))
    });
    found.or_else(|| {
        let pending = cx
            .default_global::<EditorWindows>()
            .pending
            .iter()
            .find(|(_, _, _, open)| same_file(open, path))
            .map(|(_, view, _, _)| view.clone())?;
        find_tab_of(cx, &pending)
    })
}

/// Whether two paths are the same file. Compares the real paths when both exist, else the paths.
pub(crate) fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// Brings a window to the front.
pub(crate) fn activate(cx: &mut App, handle: AnyWindowHandle) {
    handle
        .update(cx, |_, window, _| window.activate_window())
        .ok();
}

/// The view starts reading `path` into its document: until `finish_pending`, opening the same
/// file again brings the tab of this view to the front instead of making a second copy.
pub(crate) fn begin_pending(
    cx: &mut App,
    handle: AnyWindowHandle,
    view: &WeakEntity<EditorView>,
    path: PathBuf,
) {
    // The document is noted now: the view may be borrowed when it is asked about.
    let Some(document) = view.upgrade().map(|v| v.read(cx).document().entity_id()) else {
        return;
    };
    cx.default_global::<EditorWindows>()
        .pending
        .push((handle, view.clone(), document, path));
}

/// The view has finished reading its file (or failed): the file is no longer pending.
pub(crate) fn finish_pending(cx: &mut App, view: &WeakEntity<EditorView>) {
    cx.default_global::<EditorWindows>()
        .pending
        .retain(|(_, v, _, _)| v.entity_id() != view.entity_id());
}

/// Opens the documents of the command line: one per file, or one untitled window without files.
/// A file that is named twice (`a.txt ./a.txt`) is opened once, as in `open_paths`. In tab mode the
/// files after the first are tabs of the first window, unless `bNewWindow` says that files from
/// outside get new windows (`CControlTray::OpenNewEditor`).
pub(crate) fn open_startup_windows(cx: &mut App, paths: Vec<PathBuf>) -> gpui::Result<()> {
    if paths.is_empty() {
        open_editor_window(cx, SharedDocument::empty(), None)?;
    }
    let new_window = crate::tabs::settings::current(cx).new_window;
    let mut first: Option<AnyWindowHandle> = None;
    for path in paths {
        if find_open(cx, &path).is_some() {
            continue;
        }
        let load = LoadRequest {
            path,
            encoding: None,
        };
        match first {
            Some(from) => {
                crate::tabs::flows::open_document(
                    cx,
                    None,
                    from,
                    new_window,
                    SharedDocument::empty(),
                    Some(load),
                )?;
            }
            None => first = Some(open_editor_window(cx, SharedDocument::empty(), Some(load))?),
        }
    }
    Ok(())
}

/// Whether a view of the window is still reading a file into its document.
pub(crate) fn has_pending(cx: &mut App, handle: AnyWindowHandle) -> bool {
    cx.default_global::<EditorWindows>()
        .pending
        .iter()
        .any(|(h, _, _, _)| *h == handle)
}

/// Whether this view, or another pane of its document, is still reading a file into the
/// document. The view is read, so it must not be borrowed; [`is_pending_document`] does not.
pub(crate) fn is_pending(cx: &mut App, view: &WeakEntity<EditorView>) -> bool {
    match view.upgrade().map(|v| v.read(cx).document().entity_id()) {
        Some(document) => is_pending_document(cx, document),
        None => false,
    }
}

/// Whether a file is still being read into the document.
pub(crate) fn is_pending_document(cx: &mut App, document: gpui::EntityId) -> bool {
    cx.default_global::<EditorWindows>()
        .pending
        .iter()
        .any(|(_, _, d, _)| *d == document)
}
