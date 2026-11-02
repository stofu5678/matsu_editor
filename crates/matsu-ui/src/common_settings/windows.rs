//! What OK does to the windows that are open.

use gpui::App;

use crate::file_ops::windows::workspaces;

/// Shows or hides the status bar of every pane of every tab of every window
/// (`CEditWnd::OnChangeSetting`: each window takes `m_bDispSTATUSBAR`).
pub(super) fn set_status_bar(cx: &mut App, shown: bool) {
    for (_, workspace) in workspaces(cx) {
        for view in workspace.read(cx).all_views() {
            view.update(cx, |view, cx| view.set_status_bar(shown, cx));
        }
        workspace.update(cx, |_, cx| cx.notify());
    }
}

/// Gives the views of every window the horizontal scroll bar, or takes it away
/// (`CEditWnd::OnChangeSetting`: the bars of the views are made again when `bScrollBarHorz`
/// differs from what a view has).
pub(super) fn set_h_scrollbar(cx: &mut App, shown: bool) {
    crate::view::hscroll::set_shown(cx, shown);
    for (_, workspace) in workspaces(cx) {
        for view in workspace.read(cx).all_views() {
            view.update(cx, |_, cx| cx.notify());
        }
        workspace.update(cx, |_, cx| cx.notify());
    }
}
