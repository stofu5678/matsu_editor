//! The cases of Sakura's `src/test/cpp/tests1/test-detectindentationstyle.cpp`, and the ones of
//! the text (the other half of `DetectIndentationStyle`).

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::*;

fn style(character: IndentChar, tab_space: i32) -> IndentStyle {
    IndentStyle {
        character,
        tab_space,
    }
}

/// The style of `file` in a temp directory whose `.editorconfig` is `config` and whose text is
/// `text`.
fn detect_with(config: &str, file: &str, text: &str) -> IndentStyle {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".editorconfig"), config).unwrap();
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    detect(Some(&dir.path().join(file)), &doc)
}

/// `root = true` first, so that a file above the temp directory cannot matter.
fn editorconfig(body: &str) -> String {
    format!("root = true\n{body}")
}

#[test]
fn tab_width_is_parsed_from_the_value() {
    let s = detect_with(
        &editorconfig("[*.cpp]\nindent_style = tab\ntab_width = 8\n"),
        "test.cpp",
        "",
    );
    assert_eq!(s, style(IndentChar::Tabs, 8));
}

#[test]
fn values_are_compared_exactly() {
    // "tabs" is not "tab": the config gives nothing and the (empty) text decides.
    let s = detect_with(
        &editorconfig("[*.cpp]\nindent_style = tabs\n"),
        "test.cpp",
        "",
    );
    assert_eq!(s, style(IndentChar::Unknown, -1));
}

#[test]
fn tab_alone_is_applied_and_keeps_the_width() {
    let s = detect_with(
        &editorconfig("[*.cpp]\nindent_style = tab\n"),
        "test.cpp",
        "",
    );
    assert_eq!(s, style(IndentChar::Tabs, -1));
    assert_eq!(
        s.changes(),
        IndentChanges {
            ins_space: Some(false),
            tab_width: None
        }
    );
}

#[test]
fn tab_falls_back_to_indent_size_and_prefers_tab_width() {
    let s = detect_with(
        &editorconfig("[*.cpp]\nindent_style = tab\nindent_size = 2\n"),
        "test.cpp",
        "",
    );
    assert_eq!(s, style(IndentChar::Tabs, 2));
    let s = detect_with(
        &editorconfig("[*.cpp]\nindent_style = tab\nindent_size = 2\ntab_width = 8\n"),
        "test.cpp",
        "",
    );
    assert_eq!(s, style(IndentChar::Tabs, 8));
}

#[test]
fn an_unparsable_number_is_ignored_alone() {
    let s = detect_with(
        &editorconfig("[*.cpp]\nindent_style = tab\nindent_size = unset\ntab_width = 8\n"),
        "test.cpp",
        "",
    );
    assert_eq!(s, style(IndentChar::Tabs, 8));
    let s = detect_with(
        &editorconfig("[*.cpp]\nindent_style = tab\ntab_width = 8x\n"),
        "test.cpp",
        "",
    );
    assert_eq!(s, style(IndentChar::Tabs, -1));
}

#[test]
fn space_uses_indent_size_or_the_tab_width() {
    let s = detect_with(
        &editorconfig("[*.cpp]\nindent_style = space\nindent_size = 4\n"),
        "test.cpp",
        "",
    );
    assert_eq!(s, style(IndentChar::Spaces, 4));
    let s = detect_with(
        &editorconfig("[*.cpp]\nindent_style = space\nindent_size = tab\ntab_width = 8\n"),
        "test.cpp",
        "",
    );
    assert_eq!(s, style(IndentChar::Spaces, 8));
}

#[test]
fn space_without_a_width_leaves_it_to_the_text() {
    let s = detect_with(
        &editorconfig("[*.cpp]\nindent_style = space\n"),
        "test.cpp",
        "",
    );
    assert_eq!(s, style(IndentChar::Unknown, -1));
    let s = detect_with(
        &editorconfig("[*.cpp]\nindent_style = space\n"),
        "test.cpp",
        "a\n  b\n    c\n  d\n",
    );
    assert_eq!(s, style(IndentChar::Spaces, 2));
}

#[test]
fn a_braced_list_of_extensions_is_understood() {
    let s = detect_with(
        &editorconfig("[*.{h,cpp}]\nindent_style = tab\ntab_width = 3\n"),
        "test.cpp",
        "",
    );
    assert_eq!(s, style(IndentChar::Tabs, 3));
}

#[test]
fn a_glob_matches_the_extension_exactly() {
    // Sakura matched `[*.cpp]` for a `.c` file too (a prefix test).
    let s = detect_with(
        &editorconfig("[*.cpp]\nindent_style = tab\ntab_width = 3\n"),
        "test.c",
        "",
    );
    assert_eq!(s, style(IndentChar::Unknown, -1));
    // The case does not matter.
    let s = detect_with(
        &editorconfig("[*.CPP]\nindent_style = tab\ntab_width = 3\n"),
        "test.cpp",
        "",
    );
    assert_eq!(s, style(IndentChar::Tabs, 3));
}

#[test]
fn the_search_goes_up_and_root_stops_it() {
    let dir = TempDir::new().unwrap();
    let child = dir.path().join("child");
    fs::create_dir(&child).unwrap();
    fs::write(
        dir.path().join(".editorconfig"),
        "root = true\n[*.cpp]\nindent_style = tab\ntab_width = 5\n",
    )
    .unwrap();
    fs::write(
        child.join(".editorconfig"),
        "[*.txt]\nindent_style = space\nindent_size = 2\n",
    )
    .unwrap();
    let doc = Document::empty();
    // The file of the child has no section for .cpp: the parent's is taken.
    assert_eq!(
        detect(Some(&child.join("test.cpp")), &doc),
        style(IndentChar::Tabs, 5)
    );
    // `root = true` in the child stops the search.
    fs::write(
        child.join(".editorconfig"),
        "root = true\n[*.txt]\nindent_style = space\nindent_size = 2\n",
    )
    .unwrap();
    assert_eq!(
        detect(Some(&child.join("test.cpp")), &doc),
        style(IndentChar::Unknown, -1)
    );
}

#[test]
fn a_file_without_an_extension_is_not_searched() {
    let s = detect_with(
        &editorconfig("[*]\nindent_style = tab\ntab_width = 5\n"),
        "Makefile",
        "",
    );
    assert_eq!(s, style(IndentChar::Unknown, -1));
}

#[test]
fn a_bom_and_crlf_in_the_config_are_fine() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join(".editorconfig"),
        "\u{feff}root = true\r\n[*.cpp]\r\nindent_style = tab\r\ntab_width = 6\r\n",
    )
    .unwrap();
    let doc = Document::empty();
    assert_eq!(
        detect(Some(&dir.path().join("a.cpp")), &doc),
        style(IndentChar::Tabs, 6)
    );
}

fn from_text(text: &str) -> IndentStyle {
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    detect(None, &doc)
}

#[test]
fn a_file_indented_with_spaces_gets_the_most_frequent_step() {
    let s = from_text("void f()\n    int a;\n        int b;\n    int c;\n");
    assert_eq!(s, style(IndentChar::Spaces, 4));
    let s = from_text("a\n  b\n    c\n      d\n  e\n");
    assert_eq!(s, style(IndentChar::Spaces, 2));
    assert_eq!(
        s.changes(),
        IndentChanges {
            ins_space: Some(true),
            tab_width: Some(2)
        }
    );
}

#[test]
fn a_file_indented_with_tabs_has_no_width() {
    let s = from_text("a\n\tb\n\t\tc\n\td\n");
    assert_eq!(s, style(IndentChar::Tabs, -1));
    assert_eq!(
        s.changes(),
        IndentChanges {
            ins_space: Some(false),
            tab_width: None
        }
    );
}

#[test]
fn a_mixed_file_is_decided_only_by_four_times_as_many_lines() {
    // 4 tab lines and 1 space line: tabs are not more than 4 times as many as the spaces... 4 > 4
    // is false, so it is unknown.
    assert_eq!(
        from_text("\ta\n\tb\n\tc\n\td\n  e\n").character,
        IndentChar::Unknown
    );
    // 5 tab lines and 1 space line.
    assert_eq!(
        from_text("\ta\n\tb\n\tc\n\td\n\te\n  f\n").character,
        IndentChar::Tabs
    );
    // The same for spaces.
    assert_eq!(
        from_text("  a\n  b\n  c\n  d\n  e\n\tf\n").character,
        IndentChar::Spaces
    );
    assert_eq!(
        from_text("  a\n  b\n  c\n  d\n\te\n").character,
        IndentChar::Unknown
    );
    // A half of each.
    let s = from_text("  a\n\tb\n  c\n\td\n");
    assert_eq!(s, style(IndentChar::Unknown, -1));
    assert_eq!(s.changes(), IndentChanges::default());
}

#[test]
fn a_file_without_indentation_is_unknown() {
    assert_eq!(from_text("a\nb\nc\n"), style(IndentChar::Unknown, -1));
    assert_eq!(from_text(""), style(IndentChar::Unknown, -1));
}

#[test]
fn spaces_without_a_step_change_nothing() {
    // Steps of 1 only (the difference must be more than 1): spaces, but no width.
    let s = from_text(" a\n  b\n   c\n    d\n");
    assert_eq!(s, style(IndentChar::Spaces, -1));
    assert_eq!(s.changes(), IndentChanges::default());
}

#[test]
fn blank_lines_do_not_count_and_the_first_most_frequent_step_wins() {
    // The empty lines are skipped (the "last" stays): the steps are 2 and 2.
    let s = from_text("a\n  b\n\n\n    c\n");
    assert_eq!(s, style(IndentChar::Spaces, 2));
}

#[test]
fn only_the_first_256_lines_count() {
    let mut text = String::new();
    for _ in 0..256 {
        text.push_str("a\n");
    }
    for _ in 0..1000 {
        text.push_str("\tb\n");
    }
    assert_eq!(from_text(&text).character, IndentChar::Unknown);
    // The 256th line is the last one looked at.
    let mut text = String::new();
    for _ in 0..255 {
        text.push_str("a\n");
    }
    text.push_str("\tb\n\tb\n");
    assert_eq!(from_text(&text).character, IndentChar::Tabs);
}

#[test]
fn a_tab_width_the_dialog_cannot_have_is_not_taken() {
    for bad in [0, -3, 65, 1000] {
        let tabs = style(IndentChar::Tabs, bad);
        assert_eq!(tabs.changes().tab_width, None, "{bad}");
        assert_eq!(tabs.changes().ins_space, Some(false), "{bad}");
        let spaces = style(IndentChar::Spaces, bad);
        assert_eq!(spaces.changes(), IndentChanges::default(), "{bad}");
    }
    assert_eq!(style(IndentChar::Spaces, 64).changes().tab_width, Some(64));
    assert_eq!(style(IndentChar::Spaces, 1).changes().tab_width, Some(1));
}

#[test]
fn a_huge_first_line_is_not_read_whole() {
    // A line of 3 MiB with leading spaces: only the start of it is decoded.
    let mut text = " ".repeat(8);
    text.push_str(&"x".repeat(3 << 20));
    text.push_str("\n    y\n");
    let doc = Document::from_utf8(text.into_bytes()).unwrap();
    let s = detect(Some(Path::new("/nonexistent-dir-for-test/a")), &doc);
    // 2 lines, both start with a space: spaces, the step is 4 (8 to 4).
    assert_eq!(s, style(IndentChar::Spaces, 4));
}
