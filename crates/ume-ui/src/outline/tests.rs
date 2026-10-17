//! Tests of the outline pane, the bookmark list and the function list marks in GPUI's headless
//! test windows: open, jump, toggle, follow the tab, and the marks.

use std::path::PathBuf;

use gpui::{AnyWindowHandle, Entity, TestAppContext};
use ume_commands::command::Arg;
use ume_commands::sakura::FUNCTIONS;
use ume_core::buffer::{Document, Pos};
use ume_core::model::DocumentModel;

use super::pane::{OutlinePane, Row};
use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::file_ops::windows::{open_editor_window, workspace_of};
use crate::settings::UserSettings;
use crate::tabs::settings::TabSettings;
use crate::view::EditorView;

const CPP: &str = "namespace ns {\nclass Foo {\n  void bar();\n};\n}\nvoid ns::Foo::bar() {\n}\nint main() {\n  return 0;\n}\n";

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

/// A window with the document `text` in a file called `path` (so that the type is chosen by it).
fn window(cx: &mut TestAppContext, text: &str, path: &str) -> AnyWindowHandle {
    cx.update(gpui_component::init);
    cx.update(|cx| {
        cx.set_global(UserSettings::none());
        cx.set_global(TabSettings {
            show_bar: true,
            grouped: true,
            new_window: false,
        })
    });
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let shared = SharedDocument::new(DocumentModel::new(doc), Some(PathBuf::from(path)));
    let handle = cx
        .update(|cx| open_editor_window(cx, shared, None))
        .unwrap();
    cx.run_until_parked();
    handle
}

fn workspace(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<Workspace> {
    cx.update(|app| workspace_of(app, handle)).unwrap()
}

fn active_view(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<EditorView> {
    workspace(cx, handle).read_with(cx, |ws, _| ws.editor().clone())
}

fn run_with(cx: &mut TestAppContext, handle: AnyWindowHandle, function: &str, args: &[Arg]) {
    let code = code_of(function);
    let view = active_view(cx, handle);
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, args, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

fn run(cx: &mut TestAppContext, handle: AnyWindowHandle, function: &str) {
    run_with(cx, handle, function, &[]);
}

fn pane(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Option<Entity<OutlinePane>> {
    workspace(cx, handle).read_with(cx, |ws, _| ws.outline.clone())
}

fn rows(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Vec<Row> {
    pane(cx, handle)
        .expect("the pane is open")
        .read_with(cx, |p, _| p.rows().to_vec())
}

fn labels(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Vec<String> {
    rows(cx, handle).into_iter().map(|r| r.label).collect()
}

fn caret(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Pos {
    active_view(cx, handle).read_with(cx, |v, _| v.cursor().caret())
}

fn move_caret(cx: &mut TestAppContext, handle: AnyWindowHandle, line: usize) {
    let view = active_view(cx, handle);
    view.update(cx, |view, cx| {
        view.run_with(cx, |c| c.move_caret(Pos::new(line, 0), false))
            .unwrap();
    });
    cx.run_until_parked();
}

fn jump(cx: &mut TestAppContext, handle: AnyWindowHandle, row: usize) {
    let pane = pane(cx, handle).unwrap();
    handle
        .update(cx, |_, window, app| {
            pane.update(app, |p, cx| p.jump(row, false, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

#[gpui::test]
fn the_outline_of_the_type_is_shown_as_a_tree(cx: &mut TestAppContext) {
    let handle = window(cx, CPP, "a.cpp");
    assert!(pane(cx, handle).is_none());
    run(cx, handle, "F_OUTLINE");
    let pane = pane(cx, handle).expect("F_OUTLINE opens the pane");
    pane.read_with(cx, |p, _| {
        assert_eq!(p.title(), "C++ メソッドツリー");
        assert!(!p.is_analyzing());
    });
    // the nodes at the top are open: the namespace, its class (the item "ns::Foo" takes the node
    // that the name made), and the members; "Foo クラス" made by "Foo::bar" is a node of its own
    let text = labels(cx, handle);
    assert_eq!(
        text,
        [
            "ns 名前空間",
            "定義位置",
            "Foo クラス",
            "グローバル",
            "main"
        ]
    );
}

#[gpui::test]
fn a_click_on_a_row_moves_the_caret_to_the_line(cx: &mut TestAppContext) {
    let handle = window(cx, CPP, "a.cpp");
    run(cx, handle, "F_OUTLINE");
    let all = rows(cx, handle);
    let main_row = all.iter().position(|r| r.label == "main").unwrap();
    jump(cx, handle, main_row);
    assert_eq!(caret(cx, handle), Pos::new(7, 0));
    // the row is selected, and a node without an item does nothing
    let selected = pane(cx, handle)
        .unwrap()
        .read_with(cx, |p, _| p.selected_row());
    assert_eq!(selected, Some(main_row));
    let node_row = all.iter().position(|r| r.item.is_none()).unwrap();
    jump(cx, handle, node_row);
    assert_eq!(caret(cx, handle), Pos::new(7, 0));
}

#[gpui::test]
fn the_selection_follows_the_caret_until_the_text_changes(cx: &mut TestAppContext) {
    let handle = window(cx, "第1章\ntext\n第2章\nmore\n", "a.txt");
    run(cx, handle, "F_OUTLINE");
    let selected = |cx: &mut TestAppContext| {
        let p = pane(cx, handle).unwrap();
        let (row, rows) = p.read_with(cx, |p, _| (p.selected_row(), p.rows().to_vec()));
        row.map(|r| rows[r].label.clone())
    };
    assert_eq!(selected(cx).as_deref(), Some("第1章"));
    move_caret(cx, handle, 3);
    assert_eq!(selected(cx).as_deref(), Some("第2章"));
    move_caret(cx, handle, 0);
    assert_eq!(selected(cx).as_deref(), Some("第1章"));
    // an edit makes the result old: it stays, and the selection stops following
    let view = active_view(cx, handle);
    view.update(cx, |view, cx| {
        view.run_with(cx, |c| c.insert_text("x").map(|_| ()))
            .unwrap();
        cx.notify();
    });
    cx.run_until_parked();
    let p = pane(cx, handle).unwrap();
    assert!(p.read_with(cx, |p, _| p.is_stale()));
    move_caret(cx, handle, 3);
    assert_eq!(selected(cx).as_deref(), Some("第1章"));
    // F_OUTLINE with the action 1 analyzes again
    run_with(cx, handle, "F_OUTLINE", &[Arg::Int(1)]);
    assert!(!p.read_with(cx, |p, _| p.is_stale()));
    assert_eq!(selected(cx).as_deref(), Some("第2章"));
}

#[gpui::test]
fn the_toggle_closes_the_pane_of_the_same_kind_only(cx: &mut TestAppContext) {
    let handle = window(cx, CPP, "a.cpp");
    run(cx, handle, "F_OUTLINE_TOGGLE");
    assert!(pane(cx, handle).is_some());
    run(cx, handle, "F_OUTLINE_TOGGLE");
    assert!(pane(cx, handle).is_none());
    // a pane of the bookmarks is not closed by the toggle of the outline: it shows the outline
    run(cx, handle, "F_BOOKMARK_VIEW");
    assert_eq!(
        pane(cx, handle).unwrap().read_with(cx, |p, _| p.title()),
        "ブックマーク"
    );
    run(cx, handle, "F_OUTLINE_TOGGLE");
    let p = pane(cx, handle).expect("it was not closed");
    assert_eq!(p.read_with(cx, |p, _| p.title()), "C++ メソッドツリー");
    // F_OUTLINE with the same kind only gives the focus; the pane is the same
    run(cx, handle, "F_OUTLINE");
    assert_eq!(pane(cx, handle).unwrap().entity_id(), p.entity_id());
}

#[gpui::test]
fn the_bookmark_list_shows_the_bookmarked_lines(cx: &mut TestAppContext) {
    let handle = window(cx, "  one  \ntwo\n\nfour\n", "a.txt");
    for line in [0, 1, 2, 3] {
        move_caret(cx, handle, line);
        run(cx, handle, "F_BOOKMARK_SET");
    }
    run(cx, handle, "F_BOOKMARK_VIEW");
    let pane_ = pane(cx, handle).unwrap();
    let all = pane_.read_with(cx, |p, _| p.rows().to_vec());
    // a list: the line, the column and the text without the blanks at both ends
    let columns: Vec<_> = all
        .iter()
        .map(|r| r.columns.clone().unwrap().to_vec())
        .collect();
    assert_eq!(
        columns,
        [
            ["1", "1", ""],
            ["2", "1", ""],
            ["3", "1", ""],
            ["4", "1", ""]
        ]
        .map(|c| c.map(String::from).to_vec())
    );
    assert_eq!(
        all.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(),
        ["one", "two", "", "four"]
    );
    // 空行を無視する leaves the empty line out
    pane_.update(cx, |p, cx| {
        cx.set_global(super::OutlineOptions {
            ignore_blank_bookmarks: true,
        });
        p.reload(cx);
    });
    cx.run_until_parked();
    let after = labels(cx, handle);
    assert_eq!(after, ["one", "two", "four"]);
    cx.update(|cx| cx.set_global(super::OutlineOptions::default()));
}

#[gpui::test]
fn the_pane_follows_the_tab(cx: &mut TestAppContext) {
    let handle = window(cx, "第1章\n第2章\n", "a.txt");
    run(cx, handle, "F_OUTLINE");
    assert_eq!(labels(cx, handle), ["第1章", "第2章"]);
    // a new tab: an empty document
    run(cx, handle, "F_FILENEW");
    cx.run_until_parked();
    let ws = workspace(cx, handle);
    assert_eq!(ws.read_with(cx, |w, _| w.tab_count()), 2);
    // the pane analyzes the document of the active tab when the window is drawn
    cx.run_until_parked();
    assert!(labels(cx, handle).is_empty());
    run(cx, handle, "F_PREVWINDOW");
    cx.run_until_parked();
    assert_eq!(labels(cx, handle), ["第1章", "第2章"]);
}

#[gpui::test]
fn the_function_list_marks_move_between_the_functions(cx: &mut TestAppContext) {
    let handle = window(cx, CPP, "a.cpp");
    // without an analysis there is no mark
    move_caret(cx, handle, 0);
    run(cx, handle, "F_FUNCLIST_NEXT");
    assert_eq!(caret(cx, handle).line, 0);
    run(cx, handle, "F_OUTLINE");
    let marks: Vec<usize> = active_view(cx, handle).read_with(cx, |v, cx| {
        v.document()
            .read(cx)
            .model()
            .funclist_marks()
            .iter()
            .collect()
    });
    // the lines of ns, Foo, bar (declared), bar, main
    assert_eq!(marks, [0, 1, 2, 5, 7]);
    move_caret(cx, handle, 0);
    run(cx, handle, "F_FUNCLIST_NEXT");
    assert_eq!(caret(cx, handle), Pos::new(1, 0));
    run(cx, handle, "F_FUNCLIST_NEXT");
    run(cx, handle, "F_FUNCLIST_NEXT");
    assert_eq!(caret(cx, handle), Pos::new(5, 0));
    run(cx, handle, "F_FUNCLIST_PREV");
    assert_eq!(caret(cx, handle), Pos::new(2, 0));
    // the marks follow the lines: a line inserted at the top moves them
    move_caret(cx, handle, 0);
    let view = active_view(cx, handle);
    view.update(cx, |view, cx| {
        view.run_with(cx, |c| c.insert_text("\n").map(|_| ()))
            .unwrap();
    });
    cx.run_until_parked();
    run(cx, handle, "F_FUNCLIST_NEXT");
    assert_eq!(caret(cx, handle), Pos::new(2, 0));
}

#[gpui::test]
fn a_kind_that_is_not_ported_says_so(cx: &mut TestAppContext) {
    // the type of ".sql" has the outline of PL/SQL
    let handle = window(cx, "create procedure p;\n", "a.sql");
    run(cx, handle, "F_OUTLINE");
    let p = pane(cx, handle).unwrap();
    assert!(p.read_with(cx, |p, _| p.rows().is_empty()));
    assert_eq!(p.read_with(cx, |p, _| p.title()), "PL/SQL 関数一覧");
}
