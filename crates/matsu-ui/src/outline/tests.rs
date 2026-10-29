//! Tests of the outline pane, the bookmark list and the function list marks in GPUI's headless
//! test windows: open, jump, toggle, follow the tab, and the marks.

use std::path::PathBuf;

use gpui::{AnyWindowHandle, Entity, TestAppContext};
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;
use matsu_core::buffer::{Document, Pos};
use matsu_core::model::DocumentModel;

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
    window_with(cx, text, path, UserSettings::none())
}

/// Like [`window`], with the settings of the user.
fn window_with(
    cx: &mut TestAppContext,
    text: &str,
    path: &str,
    user: UserSettings,
) -> AnyWindowHandle {
    cx.update(gpui_component::init);
    cx.update(|cx| {
        cx.set_global(user);
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
        pane(cx, handle)
            .unwrap()
            .read_with(cx, |p, _| p.title().to_owned()),
        "ブックマーク"
    );
    run(cx, handle, "F_OUTLINE_TOGGLE");
    let p = pane(cx, handle).expect("it was not closed");
    assert_eq!(
        p.read_with(cx, |p, _| p.title().to_owned()),
        "C++ メソッドツリー"
    );
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
fn tex_and_erlang_show_their_outline(cx: &mut TestAppContext) {
    // the type of ".tex" has the outline of TeX, a tree
    let handle = window(cx, "\\section{a}\n\\section{b}\n", "a.tex");
    run(cx, handle, "F_OUTLINE");
    let p = pane(cx, handle).unwrap();
    assert_eq!(p.read_with(cx, |p, _| p.title().to_owned()), "TeX");
    let text = labels(cx, handle);
    assert_eq!(text, ["1. a", "2. b"], "{text:?}");

    // Sakura has no type for Erlang; the basic type can use its outline
    let (dir, user) = rule_file_settings("erlang", 14, None);
    let handle = window_with(cx, "-module(m).\nf(A, B) -> A.\n", "a.zzz", user);
    run(cx, handle, "F_OUTLINE");
    let p = pane(cx, handle).unwrap();
    assert_eq!(
        p.read_with(cx, |p, _| p.title().to_owned()),
        "Erlang 関数一覧"
    );
    let text = labels(cx, handle);
    assert!(text.iter().any(|l| l.contains("f/2")), "{text:?}");
    std::fs::remove_dir_all(dir).ok();
}

/// A folder with the settings file whose basic type has the kind of outline `outline` and the
/// rule file `rules.txt` (`rule`, or none at all when it is `None`).
fn rule_file_settings(name: &str, outline: i32, rule: Option<&[u8]>) -> (PathBuf, UserSettings) {
    let dir = std::env::temp_dir().join(format!("matsu-outline-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    if let Some(rule) = rule {
        std::fs::write(dir.join("rules.txt"), rule).unwrap();
    }
    let ini = format!(
        "[Types(0)]\r\nnDefaultOutline={outline}\r\nszOutlineRuleFilename=rules.txt\r\n\
         [Other]\r\nnTypesCount=17\r\n"
    );
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(ini.as_bytes());
    let profile = matsu_config::Profile::from_bytes(&bytes);
    let user = UserSettings::from_profile(profile, Some(dir.join("matsu.ini")));
    (dir, user)
}

#[gpui::test]
fn the_rule_file_of_the_type_makes_the_outline(cx: &mut TestAppContext) {
    let mut rule = vec![0xEF, 0xBB, 0xBF];
    rule.extend_from_slice(";Title=規則の見出し\r\n#  /// Head\r\n".as_bytes());
    let (dir, user) = rule_file_settings("rule", 12, Some(&rule));
    // a name the types do not know has the basic type; the relative name starts at the folder of
    // the settings file
    let handle = window_with(cx, "# a\nb\n# c\n", "a.zzz", user);
    run(cx, handle, "F_OUTLINE");
    let p = pane(cx, handle).unwrap();
    // the title of the file replaces the title of the kind
    assert_eq!(p.read_with(cx, |p, _| p.title().to_owned()), "規則の見出し");
    assert_eq!(labels(cx, handle), ["# a", "# c"]);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn a_rule_file_that_is_not_there_gives_an_empty_list(cx: &mut TestAppContext) {
    let (dir, user) = rule_file_settings("missing", 12, None);
    let handle = window_with(cx, "# a\n", "a.zzz", user);
    run(cx, handle, "F_OUTLINE");
    let p = pane(cx, handle).unwrap();
    assert!(p.read_with(cx, |p, _| p.rows().is_empty()));
    assert_eq!(
        p.read_with(cx, |p, _| p.title().to_owned()),
        "ルールファイル"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn plsql_cobol_and_assembler_show_their_outline(cx: &mut TestAppContext) {
    // the type of ".sql" has the outline of PL/SQL, a list
    let handle = window(cx, "create procedure p\nis\nbegin null; end;\n", "a.sql");
    run(cx, handle, "F_OUTLINE");
    let p = pane(cx, handle).unwrap();
    assert_eq!(
        p.read_with(cx, |p, _| p.title().to_owned()),
        "PL/SQL 関数一覧"
    );
    let text = labels(cx, handle);
    assert!(text.iter().any(|l| l.contains('p')), "{text:?}");
    assert!(!p.read_with(cx, |p, _| p.rows().is_empty()));

    // ".asm": labels and procedures
    let handle = window(cx, "start:\n  mov eax, 1\nf PROC\n", "a.asm");
    run(cx, handle, "F_OUTLINE");
    let p = pane(cx, handle).unwrap();
    assert_eq!(
        p.read_with(cx, |p, _| p.title().to_owned()),
        "アセンブラ アウトライン"
    );
    let text = labels(cx, handle);
    assert!(text.iter().any(|l| l.contains("start")), "{text:?}");
    assert!(text.iter().any(|l| l.contains('f')), "{text:?}");

    // ".cbl": a tree with the division as a node
    let handle = window(
        cx,
        "       PROCEDURE DIVISION.\n       MAIN-PARA.\n",
        "a.cbl",
    );
    run(cx, handle, "F_OUTLINE");
    let p = pane(cx, handle).unwrap();
    assert_eq!(
        p.read_with(cx, |p, _| p.title().to_owned()),
        "COBOL アウトライン"
    );
    let text = labels(cx, handle);
    assert!(text.iter().any(|l| l == "PROCEDURE DIVISION"), "{text:?}");
    assert!(text.iter().any(|l| l == "MAIN-PARA"), "{text:?}");
}

#[gpui::test]
fn the_heading_marks_of_the_common_settings_pick_the_topics_of_a_text(cx: &mut TestAppContext) {
    // 見出し記号 (`m_szMidashiKigou`): a line that starts with one of them is a topic.
    let handle = window(cx, "■ head\r\nplain\r\n* star\r\n", "a.txt");
    run(cx, handle, "F_OUTLINE");
    let text = labels(cx, handle);
    assert!(text.iter().any(|l| l.contains("head")), "{text:?}");
    assert!(!text.iter().any(|l| l.contains("star")), "{text:?}");
    cx.update(|app| app.set_global(super::HeadingMarks("*".to_owned())));
    let pane = pane(cx, handle).unwrap();
    pane.update(cx, |p, cx| p.reload(cx));
    cx.run_until_parked();
    let text = labels(cx, handle);
    assert!(text.iter().any(|l| l.contains("star")), "{text:?}");
    assert!(!text.iter().any(|l| l.contains("head")), "{text:?}");
}

#[gpui::test]
fn a_file_read_by_ranges_has_no_outline_of_its_text(cx: &mut TestAppContext) {
    use matsu_core::file::{OpenOptions, open_file};

    cx.update(gpui_component::init);
    cx.update(|cx| {
        cx.set_global(UserSettings::none());
        cx.set_global(TabSettings {
            show_bar: true,
            grouped: true,
            new_window: false,
        })
    });
    let dir = std::env::temp_dir().join(format!("matsu-ui-outline-huge-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("a.cpp");
    std::fs::write(&path, CPP).unwrap();
    let options = OpenOptions {
        huge_file_len: Some(1),
        ..OpenOptions::default()
    };
    let file = open_file(&path, &options).unwrap();
    assert!(file.is_ranged());
    let shared = SharedDocument::from_text_file(file);
    let handle = cx
        .update(|cx| open_editor_window(cx, shared, None))
        .unwrap();
    cx.run_until_parked();
    run(cx, handle, "F_OUTLINE");
    for _ in 0..200 {
        cx.run_until_parked();
        if !pane(cx, handle)
            .unwrap()
            .read_with(cx, |p, _| p.is_analyzing())
        {
            break;
        }
    }
    let pane = pane(cx, handle).expect("the pane opens");
    pane.read_with(cx, |p, _| {
        assert!(p.rows().is_empty());
        assert_eq!(
            p.message(),
            Some("巨大なファイルなので、アウトライン解析は使えません")
        );
    });
    // The list of bookmarks does not read the text: it is still there.
    run(cx, handle, "F_BOOKMARK_VIEW");
    std::fs::remove_dir_all(&dir).ok();
}
