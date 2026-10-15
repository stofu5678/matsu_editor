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
use crate::view::EditorView;

use super::close;

/// The editor windows of the application, for the flows that look at more than one.
#[derive(Default)]
struct EditorWindows {
    windows: Vec<(AnyWindowHandle, WeakEntity<EditorView>)>,
    /// Files that windows are still reading. Their documents have no path yet, but opening the
    /// same file again must not make a second editable copy.
    pending: Vec<(AnyWindowHandle, PathBuf)>,
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
    let count = editors(cx).len();
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
    // ume.ini, read once at startup (`app::run`); every window applies it.
    let user = cx.try_global::<crate::settings::UserSettings>().cloned();
    let handle = cx.open_window(options, move |window, cx| {
        let document = cx.new(|_| shared);
        let workspace = cx.new(|cx| Workspace::new(document, window, cx));
        let editor = workspace.read(cx).editor().clone();
        install(window, cx, &editor);
        if let Some(user) = &user {
            editor.update(cx, |view, cx| view.apply_user_settings(user, cx));
        }
        if let Some(load) = load {
            cx.default_global::<EditorWindows>()
                .pending
                .push((window.window_handle(), load.path.clone()));
            editor.update(cx, |view, cx| {
                view.load_path_here(load.path, load.encoding, window, cx);
            });
        }
        cx.new(|cx| Root::new(workspace, window, cx))
    })?;
    Ok(handle.into())
}

/// Registers the window, and asks about unsaved changes when the user closes it.
fn install(window: &mut Window, cx: &mut App, editor: &Entity<EditorView>) {
    cx.default_global::<EditorWindows>()
        .windows
        .push((window.window_handle(), editor.downgrade()));
    let view = editor.downgrade();
    let asking = Rc::new(Cell::new(false));
    window.on_window_should_close(cx, move |window, cx| {
        let Some(editor) = view.upgrade() else {
            return true;
        };
        if !editor.read(cx).document().read(cx).model().is_modified() {
            return true;
        }
        if !asking.replace(true) {
            let view = view.clone();
            let asking = asking.clone();
            window
                .spawn(cx, async move |cx| {
                    if !close::confirm_and_close_window(&view, cx).await {
                        asking.set(false);
                    }
                })
                .detach();
        }
        false
    });
}

/// The editor windows that are open, oldest first.
pub(crate) fn editors(cx: &mut App) -> Vec<(AnyWindowHandle, Entity<EditorView>)> {
    let open = cx.windows();
    let list = cx.default_global::<EditorWindows>();
    list.windows
        .retain(|(handle, view)| open.contains(handle) && view.upgrade().is_some());
    list.pending.retain(|(handle, _)| open.contains(handle));
    list.windows
        .iter()
        .filter_map(|(handle, view)| Some((*handle, view.upgrade()?)))
        .collect()
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
            .find(|(_, open)| same_file(open, path))
            .map(|(handle, _)| *handle)?;
        editors(cx)
            .into_iter()
            .find(|(handle, _)| *handle == pending)
    })
}

/// Whether two paths are the same file. Compares the real paths when both exist, else the paths.
fn same_file(a: &Path, b: &Path) -> bool {
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

/// The window has finished reading its file (or failed): the file is no longer pending.
pub(crate) fn finish_pending(cx: &mut App, handle: AnyWindowHandle) {
    cx.default_global::<EditorWindows>()
        .pending
        .retain(|(h, _)| *h != handle);
}
