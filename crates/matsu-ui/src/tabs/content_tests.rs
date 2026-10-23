//! Tests of the contract every tab keeps (`super::content`), on the editor tab.

use gpui::{Focusable as _, TestAppContext};

use super::content::{CloseCheck, MenuKind};
use super::tests::{run, type_text, window, workspace};

#[gpui::test]
fn an_editor_tab_answers_the_contract(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    let ws = workspace(cx, handle);
    ws.read_with(cx, |ws, cx| {
        let panel = ws.tab_panel(0).unwrap();
        assert_eq!(panel.menu_kind(), MenuKind::Editor);
        assert_eq!(panel.caption(cx), panel.accessible_name(cx));
        assert!(panel.caption(cx).contains("無題"));
        assert_eq!(panel.tooltip(cx), panel.caption(cx));
        assert_eq!(panel.close_check(cx), CloseCheck::None);
        assert_eq!(panel.status(cx).map(|s| s.lines), Some(1));
        // The focus goes to the active pane.
        assert_eq!(panel.focus_handle(cx), ws.editor().focus_handle(cx));
        assert!(ws.tab_panel(1).is_none());
    });
}

#[gpui::test]
fn changes_make_closing_ask(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    type_text(cx, handle, "x");
    workspace(cx, handle).read_with(cx, |ws, cx| {
        assert_eq!(
            ws.tab_panel(0).unwrap().close_check(cx),
            CloseCheck::UnsavedChanges
        );
    });
}

#[gpui::test]
fn a_parked_tab_answers_like_the_active_one(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    run(cx, handle, "F_FILENEW");
    type_text(cx, handle, "second");
    let ws = workspace(cx, handle);
    ws.read_with(cx, |ws, cx| {
        assert_eq!(ws.tab_count(), 2);
        // One tab is parked and the other is the active one: both answer.
        let (active, parked) = if ws.active_tab() == 1 { (1, 0) } else { (0, 1) };
        let active = ws.tab_panel(active).unwrap();
        let parked = ws.tab_panel(parked).unwrap();
        assert_eq!(parked.close_check(cx), CloseCheck::None);
        assert_eq!(active.close_check(cx), CloseCheck::UnsavedChanges);
        assert_eq!(parked.menu_kind(), active.menu_kind());
    });
}
