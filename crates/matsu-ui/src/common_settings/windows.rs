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
