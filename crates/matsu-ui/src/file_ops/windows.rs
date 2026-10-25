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
use matsu_core::encoding::Encoding;

use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::panes::PaneSet;
use crate::view::EditorView;

use super::close;
use super::start_options::{
    self, LaunchOptions, Placement, set_next_placement, take_next_placement,
};

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
    if load.is_none() && !shared.is_restored() {
        // A new document has the settings of the basic type (a file that is loaded replaces it).
        shared.set_new_document_defaults(&crate::settings::new_document_defaults(cx, None));
    }
    // matsu.ini, read once at startup (`app::run`); every window applies it.
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
    // `-SX=` `-SY=` `-WX=` `-WY=` of the launch that opens this window.
    let placement = super::start_options::take_next_placement(cx);
    // The size and the position that the windows kept (`CEditWnd::_GetWindowRectForInit`); the
    // command line wins over them.
    let saved = crate::saved_state::window::state(cx);
    let default_size = saved.size().unwrap_or_else(|| size(px(1000.), px(700.)));
    let window_size = placement.map_or(default_size, |p| p.size_or(default_size));
    let mut bounds = Bounds::centered(None, window_size, cx);
    match saved.origin() {
        Some(origin) => bounds.origin = origin,
        None => {
            // Windows after the first are shifted so that they do not hide each other.
            let shift = px(30. * (count % 8) as f32);
            bounds.origin.x += shift;
            bounds.origin.y += shift;
        }
    }
    if let Some(placement) = placement {
        placement.place(&mut bounds);
    }
    let window_bounds = if saved.maximized() {
        WindowBounds::Maximized(bounds)
    } else {
        WindowBounds::Windowed(bounds)
    };
    let options = WindowOptions {
        window_bounds: Some(window_bounds),
        icon: crate::app_icon::window_icon(),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some("MATSU Editor".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let handle = cx.open_window(options, move |window, cx| {
        let workspace = build(window, cx);
        install(window, cx, &workspace);
        // The windows keep their size and position when the settings say so (`OnSize`, `OnMove`).
        workspace.update(cx, |_, workspace_cx| {
            workspace_cx
                .observe_window_bounds(window, |_, window, cx| {
                    // macOS reports a zoomed window as `Windowed`; its own flag says it is zoomed.
                    let bounds = if cfg!(target_os = "macos") && window.is_maximized() {
                        WindowBounds::Maximized(window.bounds())
                    } else if cfg!(target_os = "macos") {
                        // macOS gives the frame with the title bar, and opens a window from the
                        // size of its content.
                        crate::saved_state::window::with_content_size(
                            window.window_bounds(),
                            window.viewport_size(),
                        )
                    } else {
                        window.window_bounds()
                    };
                    crate::saved_state::window::bounds_changed(cx, bounds);
                })
                .detach();
        });
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
        // `m_bTab_CloseOneWin`: in tab mode the close button of the window closes the file that is
        // shown and nothing else (`CKeyBind::GetDefFuncCode`: `F_WINCLOSE` instead of
        // `F_GROUPCLOSE`).
        if crate::tabs::settings::current(cx).bound()
            && crate::tabs::settings::options(cx).close_one_win
            && workspace.read(cx).tab_count() > 1
        {
            if !asking.replace(true) {
                let view = workspace.read(cx).editor().downgrade();
                let asking = asking.clone();
                window
                    .spawn(cx, async move |cx| {
                        close::confirm_and_close_window(&view, cx).await;
                        asking.set(false);
                    })
                    .detach();
            }
            return false;
        }
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
            let shared = view.read(cx).document().read(cx);
            // a changed encoding is asked about too (`CDocFile::IsChgCodeSet`)
            !view.read(cx).is_grep_window()
                && (shared.model().is_modified() || shared.format_changed())
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
    // Closing the last window ends the editor: with `KeepUnsavedNoAsk` the copies of the documents are enough
    // (ADR-0022, section 5).
    let hot = cx
        .update(|_, app| crate::unsaved::hot_exit_enabled(app) && workspaces(app).len() == 1)
        .unwrap_or(false);
    loop {
        if hot {
            crate::unsaved::settle(cx).await;
        }
        let Ok(views) = workspace.read_with(cx, |ws, _| ws.tab_editors()) else {
            return;
        };
        for view in &views {
            let version = view.read_with(cx, |v, app| v.document().read(app).version());
            if asked.contains(&(view.entity_id(), version)) {
                continue;
            }
            // Asked again at the moment of the question: windows may have opened while the copies were written.
            let ends =
                |app: &mut App| crate::unsaved::hot_exit_enabled(app) && workspaces(app).len() == 1;
            if !close::confirm_discard_ending(&view.downgrade(), cx, &ends).await {
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
            cx.update(|window, app| {
                for view in &views {
                    crate::unsaved::apply_close(view, app);
                }
                window.remove_window();
            })
            .ok();
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
    find_open_except(cx, path, None)
}

/// Like [`find_open`], without looking at the view `skip`: for a command that runs in that view,
/// which is being updated and cannot be read.
pub(crate) fn find_open_except(
    cx: &mut App,
    path: &Path,
    skip: Option<gpui::EntityId>,
) -> Option<(AnyWindowHandle, Entity<EditorView>)> {
    let found = editors(cx).into_iter().find(|(_, view)| {
        Some(view.entity_id()) != skip
            && view
                .read(cx)
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

/// The one entry for documents opened from outside: the command line of this process, the files
/// of a later launch that was handed over (`crate::launch`), and the files that the OS sends to the
/// running application (Finder, the Dock). `paths` are absolute, or relative to the folder of this
/// process.
///
/// Sakura's rules: a file that is open already brings its tab and window to the front instead of
/// opening a second copy (`CNormalProcess::ActiveAlreadyOpenedWindow`, as `open_paths` does). Any
/// other file, and a launch without a file (an empty document), opens the way
/// `CControlTray::OpenNewEditor` does: in tab mode as a new tab of the active window, else as a new
/// window, and always a new window with `bNewWindow` (`tabs::flows::open_document`). Without any
/// window yet (the command line of the first launch), the first document opens a window and the
/// others follow it. A file that is named twice (`a.txt ./a.txt`) is opened once. The window of the
/// last document opened comes to the front.
pub(crate) fn open_from_outside(cx: &mut App, paths: Vec<PathBuf>) -> gpui::Result<()> {
    open_from_outside_with(cx, paths, &LaunchOptions::default())
}

/// [`open_from_outside`] for a launch with Sakura's command-line options
/// (`file_ops::start_options` says what each does).
pub(crate) fn open_from_outside_with(
    cx: &mut App,
    paths: Vec<PathBuf>,
    launch: &LaunchOptions,
) -> gpui::Result<()> {
    let new_window = crate::tabs::settings::current(cx).new_window;
    let mut from = workspaces(cx)
        .into_iter()
        .map(|(handle, _)| handle)
        .rev()
        .find(|handle| Some(*handle) == cx.active_window())
        .or_else(|| workspaces(cx).last().map(|(handle, _)| *handle));
    // Opens a document; `placement` is for the window if this makes one.
    let mut open = |cx: &mut App,
                    load: Option<LoadRequest>,
                    placement: Placement|
     -> gpui::Result<(AnyWindowHandle, Option<Entity<EditorView>>)> {
        set_next_placement(cx, placement);
        let opened = match from {
            Some(from) => crate::tabs::flows::open_document(
                cx,
                None,
                from,
                new_window,
                SharedDocument::empty(),
                load,
            )
            .map(|(handle, view)| (handle, Some(view))),
            None => open_editor_window(cx, SharedDocument::empty(), load).map(|handle| {
                let view = editors(cx)
                    .into_iter()
                    .rfind(|(h, _)| *h == handle)
                    .map(|(_, view)| view);
                (handle, view)
            }),
        };
        // A tab does not take the place of a window.
        take_next_placement(cx);
        let (handle, view) = opened?;
        from.get_or_insert(handle);
        activate(cx, handle);
        Ok((handle, view))
    };
    if launch.is_grep() {
        // `-GREPMODE`: a window for the result; the files are not opened.
        let (handle, view) = open(cx, None, launch.placement())?;
        if let Some(view) = view {
            launch.start_grep(handle, &view, cx);
        }
        return Ok(());
    }
    if paths.is_empty() {
        let (handle, view) = open(cx, None, launch.placement())?;
        if let Some(view) = view {
            launch.start_untitled(handle, &view, cx);
        }
        return Ok(());
    }
    let count = paths.len();
    let mut first = true;
    for (i, path) in paths.into_iter().enumerate() {
        if let Some((handle, found)) = find_open(cx, &path) {
            // A file that is still being read gets the caret when the read is over.
            if is_pending(cx, &found.downgrade()) {
                if let Some(after) = launch.after_load(false) {
                    start_options::expect(cx, &path, after);
                }
            } else {
                launch.jump_in_open_file(&found, cx);
            }
            // `reveal` acts later than opening does, so only the last file of the request may use
            // it: the others would come to the front after the document opened after them.
            if i + 1 == count {
                crate::tabs::flows::reveal(cx, handle, found);
            }
            continue;
        }
        if launch.file.view_mode {
            super::view_mode_request::request(cx, &path);
        }
        if let Some(after) = launch.after_load(first) {
            start_options::expect(cx, &path, after);
        }
        let placement = if first {
            launch.placement()
        } else {
            Placement::default()
        };
        first = false;
        let load = LoadRequest {
            path: path.clone(),
            encoding: launch.file.encoding,
        };
        if let Err(error) = open(cx, Some(load), placement) {
            start_options::forget(cx, &path);
            super::view_mode_request::take(cx, &path);
            return Err(error);
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
