//! The tab width and the use of spaces are the document's, not the pane's (Sakura keeps them in
//! the layout and the document), and a change of settings ends the temporary tab width only when
//! the type's tab width changed (`CEditDoc::OnChangeSetting`).

use super::*;

const TWO_SPACES: &[u8] = b"fn a() {\n  b();\n  if c {\n    d();\n  }\n}\n";
const FOUR_SPACES: &[u8] = b"fn a() {\n    b();\n    if c {\n        d();\n    }\n}\n";

/// The tab width and `bInsSpace` of a view.
fn pane(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> (u32, bool) {
    view.read_with(cx, |v, _| {
        (v.view_settings().tab_width, v.view_settings().ins_space)
    })
}

/// The view settings of the type the view has now (what the type settings dialog hands over).
fn type_view_settings(f: &Fixture, cx: &mut VisualTestContext) -> crate::settings::ViewSettings {
    f.view.read_with(cx, |v, app| {
        crate::doc_types::with_user(app, |user| user.view_settings_of_type(v.type_index()))
    })
}

/// A second pane on the document of `f`.
fn second_pane(f: &Fixture, cx: &mut VisualTestContext) -> Entity<EditorView> {
    let document = f.view.read_with(cx, |v, _| v.document().clone());
    cx.new(|cx| EditorView::new(document, cx))
}

#[gpui::test]
fn a_change_of_settings_that_leaves_the_tab_width_keeps_the_detected_width(
    cx: &mut TestAppContext,
) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", TWO_SPACES);
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(UTF8));
    assert_eq!(pane(&f.view, cx), (2, true));
    // The use of spaces is the type's again (the type is copied to the document); the width
    // goes on.
    let same = type_view_settings(&f, cx);
    f.view
        .update(cx, |v, cx| v.apply_changed_settings(same, cx));
    assert_eq!(pane(&f.view, cx), (2, false));
    // The type's tab width changed: the temporary width ends.
    let mut other = type_view_settings(&f, cx);
    other.tab_width = 6;
    f.view
        .update(cx, |v, cx| v.apply_changed_settings(other.clone(), cx));
    assert_eq!(pane(&f.view, cx).0, 6);
    // A width set after that goes on through a change that leaves the type's.
    f.run(cx, "F_CHGTABWIDTH", &[Arg::Int(3)]);
    f.view
        .update(cx, |v, cx| v.apply_changed_settings(other, cx));
    assert_eq!(pane(&f.view, cx).0, 3);
}

#[gpui::test]
fn every_pane_of_the_document_gets_the_detected_indentation_on_a_reload(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", TWO_SPACES);
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(UTF8));
    let second = second_pane(&f, cx);
    std::fs::write(&path, FOUR_SPACES).unwrap();
    f.run(cx, "F_FILE_REOPEN_UTF8", &[Arg::Int(1)]);
    assert_eq!(pane(&f.view, cx), (4, true));
    assert_eq!(pane(&second, cx), (4, true));
    // A file with no style gives the values of the type back to both.
    std::fs::write(&path, b"a\nb\n").unwrap();
    f.run(cx, "F_FILE_REOPEN_UTF8", &[Arg::Int(1)]);
    assert_eq!(pane(&f.view, cx), (4, false));
    assert_eq!(pane(&second, cx), (4, false));
    std::fs::write(&path, TWO_SPACES).unwrap();
    f.run(cx, "F_FILE_REOPEN_UTF8", &[Arg::Int(1)]);
    assert_eq!(pane(&f.view, cx), (2, true));
    assert_eq!(pane(&second, cx), (2, true));
}

#[gpui::test]
fn a_load_that_changes_the_type_still_gives_every_pane_the_detected_indentation(
    cx: &mut TestAppContext,
) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", TWO_SPACES);
    let (f, cx) = setup(cx);
    // The panes are on the untitled document (the basic type); the file has the text type.
    let second = second_pane(&f, cx);
    f.open(cx, &path, Some(UTF8));
    assert_eq!(pane(&f.view, cx), (2, true));
    assert_eq!(pane(&second, cx), (2, true));
}

#[gpui::test]
fn the_tab_width_and_the_spaces_set_by_a_macro_reach_every_pane(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx);
    let second = second_pane(&f, cx);
    f.run(cx, "F_CHGTABWIDTH", &[Arg::Int(7)]);
    assert_eq!(pane(&second, cx).0, 7);
    f.view
        .update(cx, |v, cx| v.share_indent(None, Some(true), cx));
    assert!(pane(&second, cx).1);
}
