//! The whole window takes dropped files (`DragAcceptFiles( GetHwnd(), TRUE )`,
//! `sakura_core/window/CEditWnd.cpp`; `WM_DROPFILES` runs `CEditWnd::OnDropFiles`).
//!
//! GPUI hands a drop from the OS to the element under the mouse (`on_drop`), and only if the
//! last input was not a key: the check `Hitbox::is_hovered` fails after typing, and a drop sends
//! no mouse event that would reset it. A drop right after typing would be lost. So the window
//! follows the drag itself: [`Workspace::note_file_drag`] remembers the paths while they move
//! over the window, and a listener of the mouse button released (see [`drop_zone`]) takes them
//! on the drop.

use std::path::PathBuf;

use gpui::{
    Context, DispatchPhase, DragMoveEvent, ExternalPaths, IntoElement, MouseButton, MouseDownEvent,
    MouseUpEvent, Styled as _, Window, canvas,
};

use crate::app::Workspace;

impl Workspace {
    /// Remembers the files that are being dragged over the window.
    pub(crate) fn note_file_drag(
        &mut self,
        event: &DragMoveEvent<ExternalPaths>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.file_drop = Some(event.drag(cx).paths().to_vec());
    }

    /// Files dropped on the window go to the document of the active pane (`CEditWnd::OnDropFiles`).
    fn drop_files(&mut self, paths: Vec<PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
        let active = self.panes().active().clone();
        active.update(cx, |view, cx| {
            view.on_external_drop(&ExternalPaths(paths.into()), window, cx);
        });
    }
}

/// An element over the window that draws nothing and takes the drop of the files remembered by
/// [`Workspace::note_file_drag`]. It listens in the capture phase, before the elements under it.
pub(crate) fn drop_zone(cx: &mut Context<Workspace>) -> impl IntoElement {
    let workspace = cx.entity();
    canvas(
        |_, _, _| {},
        move |_, (), window, _| {
            // A press is a new action: the files of a drag that left the window are forgotten.
            let forget = workspace.clone();
            window.on_mouse_event(move |_: &MouseDownEvent, phase, _, cx| {
                if phase == DispatchPhase::Capture {
                    forget.update(cx, |this, _| this.file_drop = None);
                }
            });
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                if phase != DispatchPhase::Capture || event.button != MouseButton::Left {
                    return;
                }
                let paths = workspace.update(cx, |this, _| this.file_drop.take());
                // The drag may be another one (a tab) when the files left the window.
                if let Some(paths) = paths
                    && cx.has_active_drag()
                {
                    cx.stop_active_drag(window);
                    cx.stop_propagation();
                    workspace.update(cx, |this, cx| this.drop_files(paths, window, cx));
                }
            });
        },
    )
    .absolute()
    .size_full()
}
