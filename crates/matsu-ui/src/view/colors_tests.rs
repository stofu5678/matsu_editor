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
