//! The indentation style that a loaded file gets (`CDocEditor::OnAfterLoad`,
//! `m_bDetectIndentationStyleOnFileLoad`): the tab width and the spaces of the Tab key change for
//! that document only.

use super::*;
use crate::settings::UserSettings;

/// A settings file where every type has the lines `body` (`a.txt` has the text type, not the
/// basic one).
fn settings(body: &str) -> UserSettings {
    let mut text = String::from("[Other]\r\nnTypesCount=17\r\n");
    for i in 0..17 {
        text.push_str(&format!("[Types({i})]\r\n{body}"));
    }
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    UserSettings::from_profile(matsu_config::Profile::from_bytes(&bytes), None)
}

/// The tab width and `bInsSpace` of the view, and the same two values of its type.
fn widths(f: &Fixture, cx: &mut VisualTestContext) -> ((u32, bool), (i32, bool)) {
    f.view.read_with(cx, |v, app| {
        let t = v.type_settings(app);
        (
            (v.view_settings().tab_width, v.view_settings().ins_space),
            (t.tab_space, t.ins_space),
        )
    })
}

const TWO_SPACES: &[u8] = b"fn a() {\n  b();\n  if c {\n    d();\n  }\n}\n";
const TABS: &[u8] = b"fn a() {\n\tb();\n\tif c {\n\t\td();\n\t}\n}\n";

#[gpui::test]
fn a_file_indented_with_spaces_gets_that_width_and_spaces_for_the_tab_key(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", TWO_SPACES);
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(UTF8));
    // The document has 2 and spaces; the type still says 4 and a tab.
    assert_eq!(widths(&f, cx), ((2, true), (4, false)));
    // The Tab key inserts spaces up to the next stop of 2.
    f.run(cx, "F_GOFILETOP", &[]);
    f.view
        .update(cx, |v, cx| v.run_command("indent_tab", &[], cx));
    assert_eq!(f.lines(cx)[0], "  fn a() {");
}

#[gpui::test]
fn a_file_indented_with_tabs_turns_the_spaces_off_and_keeps_the_width(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", TABS);
    cx.update(|app| {
        app.set_global(settings(
            "nInts=0,80,0,8,-1,-1,0,1,1,0,0,0\r\nbInsSpace=1\r\n",
        ))
    });
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(UTF8));
    assert_eq!(widths(&f, cx), ((8, false), (8, true)));
}

#[gpui::test]
fn a_file_without_a_clear_style_changes_nothing(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", b"a\nb\nc\n");
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(UTF8));
    assert_eq!(widths(&f, cx), ((4, false), (4, false)));
}

#[gpui::test]
fn a_mixed_file_changes_nothing(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    // As many lines with tabs as with spaces.
    let path = dir.write("a.txt", b"  a\n\tb\n  c\n\td\n");
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(UTF8));
    assert_eq!(widths(&f, cx), ((4, false), (4, false)));
}

#[gpui::test]
fn nothing_changes_when_the_type_does_not_detect(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", TWO_SPACES);
    cx.update(|app| app.set_global(settings("bDetectIndentationStyleOnFileLoad=0\r\n")));
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(UTF8));
    assert_eq!(widths(&f, cx), ((4, false), (4, false)));
}

#[gpui::test]
fn reading_the_file_again_forgets_what_the_first_read_found(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let path = dir.write("a.txt", TWO_SPACES);
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(UTF8));
    assert_eq!(widths(&f, cx).0, (2, true));
    std::fs::write(&path, b"a\nb\n").unwrap();
    f.run(cx, "F_FILE_REOPEN_UTF8", &[Arg::Int(1)]);
    assert_eq!(widths(&f, cx).0, (4, false));
}

#[gpui::test]
fn an_editorconfig_wins_over_the_text(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    dir.write(
        ".editorconfig",
        b"root = true\n[*.txt]\nindent_style = space\nindent_size = 3\n",
    );
    let path = dir.write("a.txt", TABS);
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(UTF8));
    assert_eq!(widths(&f, cx), ((3, true), (4, false)));
}

#[gpui::test]
fn the_settings_file_is_not_written(cx: &mut TestAppContext) {
    let dir = TempDir::new();
    let ini = dir.write("matsu.ini", b"\xEF\xBB\xBF[Types(0)]\r\nnTabSpace=4\r\n");
    let before = std::fs::read(&ini).unwrap();
    cx.update(|app| app.set_global(UserSettings::load_from(std::slice::from_ref(&ini))));
    let path = dir.write("a.txt", TWO_SPACES);
    let (f, cx) = setup(cx);
    f.open(cx, &path, Some(UTF8));
    assert_eq!(widths(&f, cx).0, (2, true));
    assert_eq!(std::fs::read(&ini).unwrap(), before);
}
