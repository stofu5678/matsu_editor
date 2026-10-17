//! The document type belongs to the document, not to a pane.

use gpui::{TestAppContext, VisualTestContext};
use ume_commands::command::Arg;

use super::tests::{TABS, locked, run, tab_width, type_index, user, view_of, window};

/// Sakura keeps the type and the lock per document (`CEditDoc::m_cDocType`): the panes of a split
/// window have the same type, whichever pane was asked.
#[gpui::test]
fn all_panes_of_a_document_share_the_type_and_the_lock(cx: &mut TestAppContext) {
    let handle = window(cx, Some(user(TABS)));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let first = view_of(cx);
    run(&first, cx, "F_SPLIT_V", &[]);
    let workspace = cx
        .update(|_, app| crate::file_ops::windows::workspace_of(app, handle))
        .unwrap();
    let panes = workspace.read_with(cx, |ws, _| ws.panes.shown());
    assert_eq!(panes.len(), 2);
    let (a, b) = (panes[0].clone(), panes[1].clone());
    run(&a, cx, "F_CHANGETYPE", &[Arg::Int(3)]);
    assert_eq!((type_index(&a, cx), tab_width(&a, cx)), (2, 2));
    assert_eq!(
        (type_index(&b, cx), tab_width(&b, cx)),
        (2, 2),
        "the other pane follows"
    );
    assert!(locked(&a, cx) && locked(&b, cx));
    // asking the other pane works the same way
    run(&b, cx, "F_CHANGETYPE", &[Arg::Int(2)]);
    assert_eq!((type_index(&a, cx), type_index(&b, cx)), (1, 1));
}
