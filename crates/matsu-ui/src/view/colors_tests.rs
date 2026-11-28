//! Tests of the syntax coloring in the view, in GPUI's headless test window (see `tests.rs`): the
//! rules come from the document type, the states of the lines follow the edits and the type, and
//! painting a colored document does not trip over any of it.

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_coloring::ColorState;
use matsu_core::buffer::{Document, Pos};
use matsu_core::model::DocumentModel;

use super::EditorView;
use crate::document::SharedDocument;
use crate::settings::UserSettings;

/// A view of `text` with the settings of a file called `file_name`.
fn setup<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
    file_name: &str,
) -> (Entity<EditorView>, &'a mut VisualTestContext) {
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let settings = UserSettings::none().view_settings(Some(file_name));
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let mut view = EditorView::new(document, cx);
        view.apply_settings(settings, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    (view, vcx)
}

/// The state at the start of `line`, as the view would paint it.
fn state_at(view: &Entity<EditorView>, cx: &mut VisualTestContext, line: usize) -> ColorState {
    view.read_with(cx, |v, cx| {
        let rules = &v.prefs.settings.coloring;
        let doc = v.document.read(cx).doc();
        v.color_states.state_at(doc, rules, line).0
    })
}

#[gpui::test]
fn the_type_of_the_file_chooses_the_rules(cx: &mut TestAppContext) {
    let (c, cx) = setup(cx, "int a;\n", "a.c");
    c.read_with(cx, |v, _| {
        assert!(v.prefs.settings.coloring.has_range_based());
        assert!(!v.prefs.settings.coloring.colors_nothing());
    });
    let (txt, cx) = setup(cx, "int a;\n", "a.xyz");
    txt.read_with(cx, |v, _| {
        // The basic type colors no comments, and has no keywords.
        assert!(!v.prefs.settings.coloring.has_range_based() || v.prefs.settings.type_index == 0);
    });
}

#[gpui::test]
fn typing_a_comment_start_changes_the_states_below(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "int a;\nint b;\nint c; /* x */\nint d;\n", "a.c");
    cx.run_until_parked();
    assert_eq!(state_at(&view, cx, 1), ColorState::Normal);
    assert_eq!(state_at(&view, cx, 4), ColorState::Normal);

    // `/*` at the start of the first line: the two lines below are in the comment, up to the
    // end of the one on the third.
    view.update(cx, |v, cx| {
        v.cursor.move_to(Pos::new(0, 0), false);
        v.type_char('/', cx);
        v.type_char('*', cx);
    });
    assert_eq!(state_at(&view, cx, 1), ColorState::Block(0));
    assert_eq!(state_at(&view, cx, 2), ColorState::Block(0));
    assert_eq!(state_at(&view, cx, 3), ColorState::Normal);

    // And back.
    view.update(cx, |v, cx| {
        v.run_command("delete_back", &[], cx);
        v.run_command("delete_back", &[], cx);
    });
    assert_eq!(state_at(&view, cx, 1), ColorState::Normal);
    assert_eq!(state_at(&view, cx, 2), ColorState::Normal);
}

#[gpui::test]
fn a_new_line_in_a_comment_moves_the_states_after_it(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "/* a\nb\n*/ c\nd\n", "a.c");
    cx.run_until_parked();
    assert_eq!(state_at(&view, cx, 3), ColorState::Normal);
    view.update(cx, |v, cx| {
        v.cursor.move_to(Pos::new(1, 1), false);
        v.type_char('\r', cx);
    });
    // The comment is a line longer: the lines 1 and 2 are in it, line 3 closes it.
    assert_eq!(state_at(&view, cx, 1), ColorState::Block(0));
    assert_eq!(state_at(&view, cx, 2), ColorState::Block(0));
    assert_eq!(state_at(&view, cx, 3), ColorState::Block(0));
    assert_eq!(state_at(&view, cx, 4), ColorState::Normal);
}

#[gpui::test]
fn a_change_of_type_starts_the_states_again(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "/* a\nb\n", "a.c");
    cx.run_until_parked();
    assert_eq!(state_at(&view, cx, 1), ColorState::Block(0));
    // As text, a `/*` means nothing.
    view.update(cx, |v, cx| {
        v.change_type(2, cx); // F_CHANGETYPE with the type テキスト
    });
    cx.run_until_parked();
    assert_eq!(state_at(&view, cx, 1), ColorState::Normal);
    view.update(cx, |v, cx| {
        v.change_type(3, cx); // C/C++
    });
    cx.run_until_parked();
    assert_eq!(state_at(&view, cx, 1), ColorState::Block(0));
}

#[gpui::test]
fn another_views_edit_starts_the_states_again(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "int a;\nint b;\n", "a.c");
    cx.run_until_parked();
    let document = view.read_with(cx, |v, _| v.document.clone());
    // An edit that is not a command of this view (another view of the document).
    document.update(cx, |shared, cx| {
        shared
            .parts_mut()
            .0
            .replace(Pos::new(0, 0), Pos::new(0, 0), "/*")
            .unwrap();
        shared.changed(cx);
    });
    cx.run_until_parked();
    assert_eq!(state_at(&view, cx, 1), ColorState::Block(0));
}

#[gpui::test]
fn the_colors_of_a_long_line_are_kept_between_frames(cx: &mut TestAppContext) {
    use std::sync::Arc;

    use super::colors::LineColorRequest;

    let text = format!("{} /* x */\nint b;\n", "int a; ".repeat(500));
    let (view, cx) = setup(cx, &text, "a.c");
    cx.run_until_parked();
    let ask = |cx: &mut VisualTestContext, upto: usize, want_next: bool| {
        view.read_with(cx, |v, cx| {
            let shared = v.document.read(cx);
            let ll = v.layout.line_layout(shared.doc(), 0).unwrap().unwrap();
            v.colored_line(LineColorRequest {
                rules: &v.prefs.settings.coloring,
                version: shared.version(),
                line: 0,
                layout: &ll,
                state: &ColorState::Normal,
                upto,
                want_next,
                next_known: None,
                raw_len: None,
            })
        })
    };
    let first = ask(cx, 100, false);
    assert!(first.next.is_none());
    // The same question is answered from the cache; the next line's state is added when asked.
    let again = ask(cx, 100, true);
    assert!(Arc::ptr_eq(&first.runs, &again.runs));
    assert_eq!(again.next, Some(ColorState::Normal));
    // Another end of the last row is another question.
    let more = ask(cx, 200, false);
    assert!(!Arc::ptr_eq(&first.runs, &more.runs));
    // A change of the type forgets it.
    let settings = UserSettings::none().view_settings(Some("a.c"));
    view.update(cx, |v, cx| v.apply_settings(settings, cx));
    let after = ask(cx, 200, false);
    assert!(!Arc::ptr_eq(&more.runs, &after.runs));
}

#[gpui::test]
fn a_colored_document_paints(cx: &mut TestAppContext) {
    let text = "#include <stdio.h>\n/* comment\n over lines */\nint main(void) {\n\tprintf(\"a // b %d\\n\", 0x1F); // http://example.com\n\treturn 0;\n}\n";
    let (view, cx) = setup(cx, text, "a.c");
    cx.run_until_parked();
    view.update(cx, |v, cx| {
        v.cursor.move_to(Pos::new(4, 3), false);
        cx.notify();
    });
    cx.run_until_parked();
    // The states are there for every line, and the painting used them.
    view.read_with(cx, |v, cx| {
        let doc = v.document.read(cx).doc();
        assert!(v.color_states.is_complete(doc, &v.prefs.settings.coloring));
    });
    assert_eq!(state_at(&view, cx, 2), ColorState::Block(0));
    assert_eq!(state_at(&view, cx, 3), ColorState::Normal);
}

#[gpui::test]
fn a_very_long_line_of_a_file_read_by_ranges_has_no_colors(cx: &mut TestAppContext) {
    use super::colors::LineColorRequest;

    let long = "int a; ".repeat(10_000);
    let text = format!("{long}\nint b;\n");
    let (view, cx) = setup(cx, &text, "a.c");
    cx.run_until_parked();
    let runs_of = |cx: &mut VisualTestContext, line: usize, raw_len: Option<usize>| {
        view.read_with(cx, |v, cx| {
            let shared = v.document.read(cx);
            let ll = v.layout.line_layout(shared.doc(), line).unwrap().unwrap();
            let upto = ll.text.len();
            v.colored_line(LineColorRequest {
                rules: &v.prefs.settings.coloring,
                version: shared.version(),
                line,
                layout: &ll,
                state: &ColorState::Normal,
                upto,
                want_next: true,
                next_known: None,
                raw_len,
            })
        })
    };
    // Read whole, the line is colored.
    assert!(!runs_of(cx, 0, None).runs.is_empty());
    view.read_with(cx, |v, _| v.line_color_cache.clear());
    // Read by ranges, a line of the length at which the states skip it is not (and its state
    // goes on unchanged); a short line still is.
    let skip = matsu_coloring::SKIP_LINE_BYTES;
    let long_line = runs_of(cx, 0, Some(skip));
    assert!(long_line.runs.is_empty());
    assert_eq!(long_line.next, Some(ColorState::Normal));
    assert!(!runs_of(cx, 1, Some(skip - 1)).runs.is_empty());
}

/// The text of line `line` of the document of the view.
fn line_text(view: &Entity<EditorView>, cx: &mut VisualTestContext, line: usize) -> String {
    view.read_with(cx, |v, cx| {
        let doc = v.document.read(cx).doc();
        doc.line(line).unwrap().map(|l| l.text).unwrap_or_default()
    })
}

#[gpui::test]
fn the_cpp_smart_indent_knows_a_comment_that_goes_on_from_the_lines_before(
    cx: &mut TestAppContext,
) {
    // The `}` is typed in a block comment that began two lines up: it is only a char there, and the
    // indentation stays (Sakura asks `GetColorTypePrev` of the line).
    let (view, cx) = setup(cx, "void f() {\n\t/* a\n\t\t", "a.c");
    cx.run_until_parked();
    view.update(cx, |v, cx| v.run_command("go_file_end", &[], cx));
    cx.simulate_input("}");
    assert_eq!(line_text(&view, cx, 2), "\t\t}");
    // After a line comment the line is code: the `}` goes back to the level of its `{`.
    let (view, cx) = setup(cx, "void f() {\n\t// a\n\t\t", "a.c");
    cx.run_until_parked();
    view.update(cx, |v, cx| v.run_command("go_file_end", &[], cx));
    cx.simulate_input("}");
    assert_eq!(line_text(&view, cx, 2), "}");
    // The `}` is in the comment too, so it closes nothing and the line break after the code is
    // indented from the `{` of the function.
    let (view, cx) = setup(cx, "void f() {\n\t/* a\n\t } */ g();", "a.c");
    cx.run_until_parked();
    view.update(cx, |v, cx| v.run_command("go_file_end", &[], cx));
    cx.simulate_input("\r");
    assert_eq!(line_text(&view, cx, 3), "\t");
}

/// The runs of the first `n` rows of line 0 of the view, each scanned from its own text
/// (`colored_row`), in the order of the screen: the state goes from a row to the next. `first` is
/// the first row that is on the screen: the rows before it are not scanned.
fn row_runs_from(
    view: &Entity<EditorView>,
    cx: &mut VisualTestContext,
    first: usize,
    n: usize,
) -> Vec<Vec<matsu_coloring::ColorRun>> {
    view.read_with(cx, |v, cx| {
        let doc = v.document.read(cx).doc();
        let rules = &v.prefs.settings.coloring;
        let rows = v.layout.rows(doc, first, n).unwrap();
        let mut carry = None;
        rows.iter()
            .map(|row| {
                assert_eq!(row.line, 0);
                assert!(super::colors::is_colored_by_row(doc, row.line));
                let runs = super::colors::colored_row(rules, row, &mut carry, || {
                    v.color_states.state_at(doc, rules, row.line).0
                });
                // the columns are those of the line
                assert!(runs.iter().all(|r| r.start >= row.start.col));
                runs.to_vec()
            })
            .collect()
    })
}

#[gpui::test]
fn a_comment_on_the_screen_in_a_very_long_line_is_colored(cx: &mut TestAppContext) {
    use matsu_coloring::ColorKind;

    // 1.2 MB in the line, which is over the limit of 1 MiB; a comment at column 98.
    let line = format!(
        "{}/* c */{}",
        "int a; ".repeat(14),
        "int a; ".repeat(170_000)
    );
    assert!(line.len() > 1 << 20);
    let (view, cx) = setup(cx, &format!("{line}\nint b;\n"), "a.c");
    cx.run_until_parked();
    let rows = row_runs_from(&view, cx, 0, 1);
    let comments: Vec<_> = rows[0]
        .iter()
        .filter(|r| r.kind == ColorKind::Comment)
        .map(|r| (r.start, r.end))
        .collect();
    assert_eq!(comments, vec![(98, 105)]);
    // the rest of the row has keywords, as a short line has
    assert!(
        rows[0]
            .iter()
            .any(|r| matches!(r.kind, ColorKind::Keyword(_)))
    );
}

#[gpui::test]
fn a_comment_goes_on_over_the_rows_of_the_screen_but_not_from_before_it(cx: &mut TestAppContext) {
    use matsu_coloring::ColorKind;

    // A comment from column 98 to column 10,000: rows 0 to 2 (of 4096 bytes) hold it.
    let line = format!(
        "{}/*{}*/ {}",
        "int a; ".repeat(14),
        "z".repeat(10_000 - 100),
        "int a; ".repeat(170_000)
    );
    let (view, cx) = setup(cx, &format!("{line}\nint b;\n"), "a.c");
    cx.run_until_parked();
    let comment_of = |runs: &[matsu_coloring::ColorRun]| -> Vec<(usize, usize)> {
        runs.iter()
            .filter(|r| r.kind == ColorKind::Comment)
            .map(|r| (r.start, r.end))
            .collect()
    };
    // The screen starts at the top of the line: the comment is on every row it covers.
    let top = row_runs_from(&view, cx, 0, 3);
    assert_eq!(comment_of(&top[0]), vec![(98, 4096)]);
    assert_eq!(comment_of(&top[1]), vec![(4096, 8192)]);
    assert_eq!(comment_of(&top[2]), vec![(8192, 10_002)]);
    // The screen starts at row 1: the state at the start of the line is all that is known, so the
    // comment that began before the screen is not seen there, until its end (`*/`), which is
    // text. This is the difference that the user chose (2026-11-29, E).
    let below = row_runs_from(&view, cx, 1, 2);
    assert!(comment_of(&below[0]).is_empty());
    assert!(comment_of(&below[1]).is_empty());
}

#[gpui::test]
fn an_open_comment_in_a_very_long_line_does_not_reach_the_next_line(cx: &mut TestAppContext) {
    // A line that is short: the open comment goes on into the line below.
    let (view, cx) = setup(cx, "int a; /* open\nint b;\n", "a.c");
    cx.run_until_parked();
    assert_eq!(state_at(&view, cx, 1), ColorState::Block(0));
    // A very long line is not scanned for the state of the next line: it starts as the long line
    // did.
    let line = format!("{}/* open", "int a; ".repeat(170_000));
    let (view, cx) = setup(cx, &format!("{line}\nint b;\n"), "a.c");
    cx.run_until_parked();
    assert_eq!(state_at(&view, cx, 1), ColorState::Normal);
}

#[gpui::test]
fn a_line_just_under_one_mebibyte_is_colored_as_before(cx: &mut TestAppContext) {
    // 1 MiB minus one byte is not a very long line: it is scanned from its start and its comment
    // goes on into the next line.
    let head = "int a; ".repeat(14);
    let open = "/* open";
    let filler = "x".repeat((1 << 20) - 1 - head.len() - open.len());
    let line = format!("{head}{open}{filler}");
    assert_eq!(line.len(), (1 << 20) - 1);
    let (view, cx) = setup(cx, &format!("{line}\nint b;\n"), "a.c");
    cx.run_until_parked();
    view.read_with(cx, |v, cx| {
        let doc = v.document.read(cx).doc();
        assert!(!doc.is_long_line(0));
        assert!(!super::colors::is_colored_by_row(doc, 0));
    });
    assert_eq!(state_at(&view, cx, 1), ColorState::Block(0));
}
