//! Tests of the tabs in GPUI's headless test windows: the tab functions of Sakura on windows that
//! hold several documents.

use gpui::{AnyWindowHandle, Entity, EntityId, TestAppContext, VisualTestContext};
use ume_commands::sakura::FUNCTIONS;

use super::settings::TabSettings;
use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::file_ops::windows::{editors, open_editor_window, workspace_of};
use crate::view::EditorView;

const DONT_SAVE: &str = "保存しない(N)";
const CANCEL: &str = "キャンセル";

pub(super) fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

pub(super) fn tab_mode(cx: &mut TestAppContext, show_bar: bool, grouped: bool) {
    cx.update(|cx| {
        cx.set_global(TabSettings {
            show_bar,
            grouped,
            new_window: false,
        })
    });
}

/// A window with one untitled document, in the tab mode of Sakura (the bar shown, grouped) or not.
pub(super) fn window(cx: &mut TestAppContext, show_bar: bool) -> AnyWindowHandle {
    cx.update(gpui_component::init);
    tab_mode(cx, show_bar, true);
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    cx.run_until_parked();
    handle
}

pub(super) fn workspace(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<Workspace> {
    cx.update(|app| workspace_of(app, handle)).unwrap()
}

/// The view that runs the functions of the window: the active pane of the active tab.
pub(super) fn active_view(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<EditorView> {
    workspace(cx, handle).read_with(cx, |ws, _| ws.editor().clone())
}

pub(super) fn run(cx: &mut TestAppContext, handle: AnyWindowHandle, function: &str) {
    let code = code_of(function);
    let view = active_view(cx, handle);
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, &[], window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

pub(super) fn visual(cx: &mut TestAppContext, handle: AnyWindowHandle) -> &mut VisualTestContext {
    VisualTestContext::from_window(handle, cx).into_mut()
}

pub(super) fn type_text(cx: &mut TestAppContext, handle: AnyWindowHandle, text: &str) {
    visual(cx, handle).simulate_input(text);
    cx.run_until_parked();
}

pub(super) fn answer(cx: &mut TestAppContext, handle: AnyWindowHandle, button: &str) {
    let vcx = visual(cx, handle);
    assert!(vcx.has_pending_prompt(), "no prompt to answer {button:?}");
    vcx.simulate_prompt_answer(button);
    vcx.run_until_parked();
}

pub(super) fn tab_count(cx: &mut TestAppContext, handle: AnyWindowHandle) -> usize {
    workspace(cx, handle).read_with(cx, |ws, _| ws.tab_count())
}

pub(super) fn active_tab(cx: &mut TestAppContext, handle: AnyWindowHandle) -> usize {
    workspace(cx, handle).read_with(cx, |ws, _| ws.active_tab())
}

/// The ids of the views that stand for the tabs of the window, in the order of the bar.
pub(super) fn tab_ids(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Vec<EntityId> {
    workspace(cx, handle).read_with(cx, |ws, _| {
        ws.tab_editors().iter().map(|v| v.entity_id()).collect()
    })
}

pub(super) fn captions(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Vec<String> {
    let ws = workspace(cx, handle);
    ws.read_with(cx, |ws, app| {
        ws.tab_editors()
            .iter()
            .map(|v| Workspace::caption_of(v.read(app), app))
            .collect()
    })
}

pub(super) fn windows(cx: &mut TestAppContext) -> usize {
    cx.windows().len()
}

/// A window with `count` tabs; the tab `n` has the text `tab n` typed in (so it is modified).
pub(super) fn tabs(cx: &mut TestAppContext, count: usize) -> AnyWindowHandle {
    let handle = window(cx, true);
    for i in 1..count {
        run(cx, handle, "F_FILENEW");
        assert_eq!(tab_count(cx, handle), i + 1);
    }
    handle
}

/// `bTab_RetainEmptyWin` off: closing the last file closes the window (Sakura's default keeps an
/// untitled document, which the tests of the common settings cover).
fn keep_no_empty_window(cx: &mut TestAppContext) {
    cx.update(|app| {
        app.set_global(super::settings::TabOptions {
            retain_empty: false,
            ..Default::default()
        })
    });
}

// ---- opening documents as tabs ----

#[gpui::test]
fn a_new_document_is_a_tab_when_the_bar_is_shown(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    run(cx, handle, "F_FILENEW");
    assert_eq!(windows(cx), 1);
    assert_eq!(tab_count(cx, handle), 2);
    assert_eq!(active_tab(cx, handle), 1, "the new tab is the active one");
    // the documents are two, and each window list entry is a tab
    assert_eq!(cx.update(|app| editors(app).len()), 2);
}

#[gpui::test]
fn a_new_document_is_a_window_when_the_bar_is_hidden(cx: &mut TestAppContext) {
    let handle = window(cx, false);
    run(cx, handle, "F_FILENEW");
    assert_eq!(windows(cx), 2);
    assert_eq!(tab_count(cx, handle), 1);
}

#[gpui::test]
fn a_new_document_is_a_window_when_the_windows_are_not_grouped(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    tab_mode(cx, true, false);
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    cx.run_until_parked();
    run(cx, handle, "F_FILENEW");
    assert_eq!(windows(cx), 2);
}

#[gpui::test]
fn new_window_makes_a_window_even_in_tab_mode(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    run(cx, handle, "F_FILENEW_NEWWINDOW");
    assert_eq!(windows(cx), 2);
    assert_eq!(tab_count(cx, handle), 1);
}

#[gpui::test]
fn the_captions_are_the_names_with_the_marks(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    assert_eq!(captions(cx, handle), ["(無題)1"]);
    type_text(cx, handle, "a");
    assert_eq!(captions(cx, handle), ["(無題)1(更新)"]);
    run(cx, handle, "F_FILENEW");
    assert_eq!(captions(cx, handle), ["(無題)1(更新)", "(無題)2"]);
}

#[gpui::test]
fn the_tab_bar_draws_with_several_tabs(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    // a frame is drawn by the next step
    let vcx = visual(cx, handle);
    vcx.simulate_resize(gpui::size(gpui::px(900.), gpui::px(600.)));
    vcx.run_until_parked();
    assert_eq!(tab_count(cx, handle), 3);
}

// ---- moving tabs ----

#[gpui::test]
fn the_tab_moves_left_and_right_and_stops_at_the_ends(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    let ids = tab_ids(cx, handle);
    // the active tab is the last
    run(cx, handle, "F_TAB_MOVERIGHT");
    assert_eq!(tab_ids(cx, handle), ids, "nothing at the right end");
    run(cx, handle, "F_TAB_MOVELEFT");
    assert_eq!(tab_ids(cx, handle), [ids[0], ids[2], ids[1]]);
    assert_eq!(active_tab(cx, handle), 1, "the tab stays the active one");
    run(cx, handle, "F_TAB_MOVELEFT");
    assert_eq!(tab_ids(cx, handle), [ids[2], ids[0], ids[1]]);
    run(cx, handle, "F_TAB_MOVELEFT");
    assert_eq!(tab_ids(cx, handle), [ids[2], ids[0], ids[1]]);
    run(cx, handle, "F_TAB_MOVERIGHT");
    assert_eq!(tab_ids(cx, handle), [ids[0], ids[2], ids[1]]);
}

#[gpui::test]
fn dragging_a_tab_reorders_the_bar(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    let ids = tab_ids(cx, handle);
    workspace(cx, handle).update(cx, |ws, cx| ws.move_tab(0, 2, cx));
    assert_eq!(tab_ids(cx, handle), [ids[1], ids[2], ids[0]]);
    assert_eq!(
        active_tab(cx, handle),
        1,
        "the active tab is still the same document"
    );
    assert_eq!(
        active_view(cx, handle).entity_id(),
        ids[2],
        "the active tab is still the last one that was opened"
    );
}

#[gpui::test]
fn the_tab_commands_do_nothing_without_the_bar(cx: &mut TestAppContext) {
    let handle = window(cx, false);
    for f in [
        "F_TAB_MOVERIGHT",
        "F_TAB_MOVELEFT",
        "F_TAB_SEPARATE",
        "F_TAB_JOINTNEXT",
        "F_TAB_JOINTPREV",
        "F_NEXTGROUP",
        "F_PREVGROUP",
        "F_GROUPCLOSE",
        "F_TAB_CLOSELEFT",
        "F_TAB_CLOSERIGHT",
    ] {
        run(cx, handle, f);
        assert_eq!(windows(cx), 1, "{f}");
        assert_eq!(tab_count(cx, handle), 1, "{f}");
    }
}

// ---- activating, closing ----

#[gpui::test]
fn the_next_and_previous_window_functions_walk_the_tabs(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    assert_eq!(active_tab(cx, handle), 2);
    run(cx, handle, "F_NEXTWINDOW");
    assert_eq!(active_tab(cx, handle), 0, "after the last comes the first");
    run(cx, handle, "F_PREVWINDOW");
    assert_eq!(active_tab(cx, handle), 2);
    run(cx, handle, "F_PREVWINDOW");
    assert_eq!(active_tab(cx, handle), 1);
}

#[gpui::test]
fn closing_a_tab_goes_back_to_the_one_that_was_active_before(cx: &mut TestAppContext) {
    let handle = tabs(cx, 4);
    let ids = tab_ids(cx, handle);
    // the order of use: tab 3 (now), then 1, then 2
    workspace(cx, handle).update_in(visual(cx, handle), |ws, window, cx| {
        ws.activate_tab(2, window, cx);
        ws.activate_tab(1, window, cx);
        ws.activate_tab(3, window, cx);
    });
    cx.run_until_parked();
    run(cx, handle, "F_WINCLOSE");
    assert_eq!(tab_count(cx, handle), 3);
    assert_eq!(windows(cx), 1);
    assert_eq!(active_view(cx, handle).entity_id(), ids[1]);
}

#[gpui::test]
fn closing_the_only_tab_closes_the_window(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    keep_no_empty_window(cx);
    run(cx, handle, "F_WINCLOSE");
    assert_eq!(windows(cx), 0);
}

#[gpui::test]
fn closing_a_modified_tab_asks_and_cancel_keeps_it(cx: &mut TestAppContext) {
    let handle = tabs(cx, 2);
    type_text(cx, handle, "x");
    run(cx, handle, "F_WINCLOSE");
    answer(cx, handle, CANCEL);
    assert_eq!(tab_count(cx, handle), 2);
    run(cx, handle, "F_WINCLOSE");
    answer(cx, handle, DONT_SAVE);
    assert_eq!(tab_count(cx, handle), 1);
    assert_eq!(windows(cx), 1);
}

#[gpui::test]
fn close_left_and_close_right_leave_the_tab_and_ask_about_each(cx: &mut TestAppContext) {
    let handle = tabs(cx, 4);
    let ids = tab_ids(cx, handle);
    // modify tab 0 and make tab 2 the active one
    workspace(cx, handle).update_in(visual(cx, handle), |ws, window, cx| {
        ws.activate_tab(0, window, cx)
    });
    cx.run_until_parked();
    type_text(cx, handle, "x");
    workspace(cx, handle).update_in(visual(cx, handle), |ws, window, cx| {
        ws.activate_tab(2, window, cx)
    });
    cx.run_until_parked();
    run(cx, handle, "F_TAB_CLOSELEFT");
    answer(cx, handle, CANCEL);
    assert_eq!(tab_count(cx, handle), 4, "cancel stops it");
    // the question brought the tab it is about to the front
    assert_eq!(active_tab(cx, handle), 0);
    workspace(cx, handle).update_in(visual(cx, handle), |ws, window, cx| {
        ws.activate_tab(2, window, cx)
    });
    cx.run_until_parked();
    run(cx, handle, "F_TAB_CLOSELEFT");
    answer(cx, handle, DONT_SAVE);
    assert_eq!(tab_ids(cx, handle), [ids[2], ids[3]]);
    run(cx, handle, "F_TAB_CLOSERIGHT");
    assert_eq!(tab_ids(cx, handle), [ids[2]]);
    assert_eq!(windows(cx), 1);
}

#[gpui::test]
fn close_other_keeps_only_this_tab(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    let ids = tab_ids(cx, handle);
    workspace(cx, handle).update_in(visual(cx, handle), |ws, window, cx| {
        ws.activate_tab(1, window, cx)
    });
    cx.run_until_parked();
    run(cx, handle, "F_TAB_CLOSEOTHER");
    assert_eq!(tab_ids(cx, handle), [ids[1]]);
    assert_eq!(windows(cx), 1);
}

#[gpui::test]
fn close_all_and_group_close_close_every_tab_of_the_window(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    keep_no_empty_window(cx);
    run(cx, handle, "F_GROUPCLOSE");
    assert_eq!(windows(cx), 0);

    let handle = tabs(cx, 2);
    type_text(cx, handle, "x");
    run(cx, handle, "F_WIN_CLOSEALL");
    answer(cx, handle, CANCEL);
    assert_eq!(windows(cx), 1, "cancel stops it");
    // the tab without changes was closed before the question; the one asked about stays
    assert_eq!(tab_count(cx, handle), 1);
}

#[gpui::test]
fn the_other_windows_are_not_closed_by_the_group_functions(cx: &mut TestAppContext) {
    let first = window(cx, true);
    run(cx, first, "F_FILENEW_NEWWINDOW");
    assert_eq!(windows(cx), 2);
    run(cx, first, "F_FILENEW");
    assert_eq!(tab_count(cx, first), 2);
    run(cx, first, "F_GROUPCLOSE");
    assert_eq!(windows(cx), 1, "the other window (group) stays");
}

// ---- groups: separate, join, next and previous ----

#[gpui::test]
fn separate_moves_the_tab_to_a_new_window_with_its_document(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    type_text(cx, handle, "moved");
    let moved = active_view(cx, handle);
    let document = moved.read_with(cx, |v, _| v.document().clone());
    run(cx, handle, "F_TAB_SEPARATE");
    assert_eq!(windows(cx), 2);
    assert_eq!(tab_count(cx, handle), 2);
    let all = cx.update(editors);
    let (new_handle, view) = all
        .iter()
        .find(|(_, v)| *v == moved)
        .cloned()
        .expect("the tab is in the list of documents");
    assert_ne!(new_handle, handle);
    assert_eq!(
        view.read_with(cx, |v, _| v.document().clone()),
        document,
        "the same document, not a copy"
    );
    assert_eq!(tab_count(cx, new_handle), 1);
    let text = document.read_with(cx, |s, _| s.doc().line(0).unwrap().unwrap().text);
    assert_eq!(text, "moved");
    // the moved tab works in its new window
    type_text(cx, new_handle, "!");
    let text = document.read_with(cx, |s, _| s.doc().line(0).unwrap().unwrap().text);
    assert_eq!(text, "moved!");
}

#[gpui::test]
fn separate_does_nothing_for_the_only_tab(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    run(cx, handle, "F_TAB_SEPARATE");
    assert_eq!(windows(cx), 1);
}

#[gpui::test]
fn join_next_and_previous_move_the_tab_to_the_neighbour_window(cx: &mut TestAppContext) {
    let first = window(cx, true);
    run(cx, first, "F_FILENEW");
    run(cx, first, "F_FILENEW_NEWWINDOW");
    let second = cx.windows()[1];
    assert_eq!(windows(cx), 2);
    let moved = active_view(cx, first);
    run(cx, first, "F_TAB_JOINTNEXT");
    assert_eq!(tab_count(cx, first), 1);
    assert_eq!(tab_count(cx, second), 2);
    assert_eq!(active_view(cx, second), moved, "it is the active tab there");
    // the other way, from the window that has two
    run(cx, second, "F_TAB_JOINTPREV");
    assert_eq!(tab_count(cx, first), 2);
    assert_eq!(tab_count(cx, second), 1);
}

#[gpui::test]
fn joining_the_only_tab_closes_the_window_it_leaves(cx: &mut TestAppContext) {
    let first = window(cx, true);
    run(cx, first, "F_FILENEW_NEWWINDOW");
    assert_eq!(windows(cx), 2);
    let second = cx.windows()[1];
    run(cx, second, "F_TAB_JOINTPREV");
    assert_eq!(windows(cx), 1);
    assert_eq!(tab_count(cx, first), 2);
}

#[gpui::test]
fn next_and_previous_group_bring_the_other_window_to_the_front(cx: &mut TestAppContext) {
    let first = window(cx, true);
    run(cx, first, "F_FILENEW_NEWWINDOW");
    let second = cx.windows()[1];
    run(cx, first, "F_NEXTGROUP");
    assert_eq!(cx.update(|app| app.active_window()), Some(second));
    run(cx, second, "F_PREVGROUP");
    assert_eq!(cx.update(|app| app.active_window()), Some(first));
    // F_NEXTWINDOW does not leave the group in tab mode
    run(cx, first, "F_NEXTWINDOW");
    assert_eq!(cx.update(|app| app.active_window()), Some(first));
}

// ---- showing and hiding the bar, grouping ----

#[gpui::test]
fn hiding_the_bar_spreads_the_tabs_into_windows(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    run(cx, handle, "F_SHOWTAB");
    assert_eq!(windows(cx), 3);
    assert_eq!(tab_count(cx, handle), 1);
    assert!(!cx.update(|app| super::settings::current(app)).show_bar);
    // and the other way: showing it gathers the windows into the tabs of this one
    run(cx, handle, "F_SHOWTAB");
    assert_eq!(windows(cx), 1);
    assert_eq!(tab_count(cx, handle), 3);
}

#[gpui::test]
fn the_bind_function_gathers_and_spreads_the_windows(cx: &mut TestAppContext) {
    let handle = tabs(cx, 2);
    run(cx, handle, "F_BIND_WINDOW");
    assert_eq!(windows(cx), 2, "not grouped: every document has a window");
    run(cx, handle, "F_FILENEW");
    assert_eq!(windows(cx), 3, "and a new document gets a window");
    run(cx, handle, "F_BIND_WINDOW");
    assert_eq!(windows(cx), 1);
    assert_eq!(tab_count(cx, handle), 3);
}

#[gpui::test]
fn the_bind_function_needs_the_bar(cx: &mut TestAppContext) {
    let handle = tabs(cx, 2);
    run(cx, handle, "F_SHOWTAB");
    assert_eq!(windows(cx), 2);
    run(cx, handle, "F_BIND_WINDOW");
    let settings = cx.update(|app| super::settings::current(app));
    assert!(settings.grouped, "nothing changed while the bar is hidden");
}

// ---- what belongs to a tab ----

#[gpui::test]
fn a_key_macro_belongs_to_the_tab_where_it_was_started(cx: &mut TestAppContext) {
    let handle = tabs(cx, 2);
    run(cx, handle, "F_RECKEYMACRO");
    assert!(active_view(cx, handle).read_with(cx, |v, _| v.is_recording_key_macro()));
    workspace(cx, handle).update_in(visual(cx, handle), |ws, window, cx| {
        ws.activate_tab(0, window, cx)
    });
    cx.run_until_parked();
    assert!(!active_view(cx, handle).read_with(cx, |v, _| v.is_recording_key_macro()));
    assert_eq!(
        captions(cx, handle),
        ["(無題)1", "(無題)2【キーマクロの記録中】"]
    );
}

#[gpui::test]
fn split_view_is_per_tab(cx: &mut TestAppContext) {
    let handle = tabs(cx, 2);
    run(cx, handle, "F_SPLIT_V");
    let ws = workspace(cx, handle);
    assert_eq!(ws.read_with(cx, |w, _| w.panes.grid()), (2, 1));
    ws.update_in(visual(cx, handle), |ws, window, cx| {
        ws.activate_tab(0, window, cx)
    });
    cx.run_until_parked();
    assert_eq!(
        ws.read_with(cx, |w, _| w.panes.grid()),
        (1, 1),
        "the other tab is not split"
    );
    ws.update_in(visual(cx, handle), |ws, window, cx| {
        ws.activate_tab(1, window, cx)
    });
    cx.run_until_parked();
    assert_eq!(
        ws.read_with(cx, |w, _| w.panes.grid()),
        (2, 1),
        "and the split is kept"
    );
    // a pane of the split tab is a pane of that tab
    let pane = ws
        .read_with(cx, |w, _| w.panes.pane_at(2).cloned())
        .unwrap();
    assert_eq!(ws.read_with(cx, |w, _| w.tab_of(&pane)), Some(1));
}

#[gpui::test]
fn a_split_tab_keeps_its_panes_when_it_moves_to_another_window(cx: &mut TestAppContext) {
    let handle = tabs(cx, 2);
    run(cx, handle, "F_SPLIT_V");
    run(cx, handle, "F_TAB_SEPARATE");
    let new_handle = cx.windows()[1];
    let ws = workspace(cx, new_handle);
    assert_eq!(ws.read_with(cx, |w, _| w.panes.grid()), (2, 1));
    // typing in the moved window reaches both panes of the document
    type_text(cx, new_handle, "q");
    let shown = ws.read_with(cx, |w, _| w.panes.shown());
    for pane in shown {
        let line = pane.read_with(cx, |v, cx| {
            v.document().read(cx).doc().line(0).unwrap().unwrap().text
        });
        assert_eq!(line, "q");
    }
}

#[gpui::test]
fn the_window_list_has_a_row_for_every_tab(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    // The popup is the tab list while the bar is shown; the dialog is the window list.
    run(cx, handle, "F_DLGWINLIST");
    let has_list = active_view(cx, handle).read_with(cx, |v, _| v.window_list.is_some());
    assert!(has_list);
    assert_eq!(cx.update(|app| editors(app).len()), 3);
}

#[gpui::test]
fn exit_all_asks_about_every_tab(cx: &mut TestAppContext) {
    let handle = tabs(cx, 2);
    type_text(cx, handle, "x");
    workspace(cx, handle).update_in(visual(cx, handle), |ws, window, cx| {
        ws.activate_tab(0, window, cx)
    });
    cx.run_until_parked();
    type_text(cx, handle, "y");
    run(cx, handle, "F_EXITALL");
    answer(cx, handle, CANCEL);
    assert_eq!(tab_count(cx, handle), 2, "nothing was closed");
}
