//! Tests of the input side of the 画面 page: auto indent, smart indent, the other indent chars,
//! the trailing blanks of the line a line break leaves, and the Tab key that inserts spaces.

use gpui::TestAppContext;

use super::*;

/// The text of the document of the view, line endings included.
fn text_of(cx: &mut TestAppContext, view: &Entity<EditorView>) -> String {
    view.read_with(cx, |view, app| {
        let doc = view.document().read(app).doc();
        (0..doc.line_count())
            .map(|i| {
                let line = doc.line(i).unwrap().unwrap();
                format!(
                    "{}{}",
                    line.text,
                    line.eol.map_or("", matsu_core::model::eol_text)
                )
            })
            .collect()
    })
}

/// Types the chars in the window with `F_WCHAR`, as the keyboard does.
fn type_chars(cx: &mut TestAppContext, handle: AnyWindowHandle, chars: &str) {
    for c in chars.chars() {
        run(cx, handle, "F_WCHAR", &[Arg::Int(i64::from(u32::from(c)))]);
    }
}

#[gpui::test]
fn the_controls_of_the_input_side_can_be_used(cx: &mut TestAppContext) {
    let (_ini, a) = setup(cx, "");
    run(cx, a, "F_OPTION_TYPE", &[]);
    for id in [
        AUTO_INDENT,
        AUTO_INDENT_ZENSPACE,
        SMART_INDENT,
        INDENT_CHARS,
        RTRIM_PREV_LINE,
        INS_SPACE,
    ] {
        assert!(super::usable(cx, id), "{id}");
    }
    cancel(cx);
}

#[gpui::test]
fn ok_writes_sakuras_keys_and_the_window_takes_the_options(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "");
    run(cx, a, "F_OPTION_TYPE", &[]);
    check(cx, AUTO_INDENT, false);
    check(cx, AUTO_INDENT_ZENSPACE, false);
    choose(cx, SMART_INDENT, 1);
    type_in(cx, INDENT_CHARS, ">");
    check(cx, RTRIM_PREV_LINE, true);
    check(cx, INS_SPACE, true);
    ok(cx);
    for (key, want) in [
        ("bAutoIndent", "0"),
        ("bAutoIndent_ZENSPACE", "0"),
        ("nSmartIndent", "1"),
        ("szIndentChars", ">"),
        ("bRTrimPrevLine", "1"),
        ("bInsSpace", "1"),
    ] {
        assert_eq!(ini.get("Types(0)", key).as_deref(), Some(want), "{key}");
    }
    let view = view_of(cx, a);
    let (input, ins_space) = view.read_with(cx, |v, _| {
        let s = v.view_settings();
        (s.input.clone(), s.ins_space)
    });
    assert!(!input.auto_indent && !input.auto_indent_zenspace && input.rtrim_prev_line);
    assert_eq!(input.indent_chars, ">");
    assert_eq!(
        input.smart_indent,
        matsu_commands::input_options::SmartIndent::Cpp
    );
    assert!(ins_space);
}

#[gpui::test]
fn the_options_change_what_enter_and_tab_type(cx: &mut TestAppContext) {
    let (_ini, a) = setup(cx, "");
    let view = view_of(cx, a);
    run(cx, a, "F_OPTION_TYPE", &[]);
    choose(cx, SMART_INDENT, 1);
    type_in(cx, INDENT_CHARS, ">");
    check(cx, RTRIM_PREV_LINE, true);
    check(cx, INS_SPACE, true);
    ok(cx);
    // smart indent: one level (4 columns, as the Tab key inserts spaces) after the `{`
    type_chars(cx, a, "if (x) {\r");
    assert_eq!(text_of(cx, &view), "if (x) {\n    ");
    run(cx, a, "F_INDENT_TAB", &[]);
    assert_eq!(text_of(cx, &view), "if (x) {\n        ");
    type_chars(cx, a, "}");
    assert_eq!(text_of(cx, &view), "if (x) {\n}");
}

#[gpui::test]
fn auto_indent_off_and_the_other_chars_change_the_copy(cx: &mut TestAppContext) {
    let (_ini, a) = setup(cx, "");
    let view = view_of(cx, a);
    run(cx, a, "F_OPTION_TYPE", &[]);
    check(cx, AUTO_INDENT, false);
    ok(cx);
    type_chars(cx, a, "  a\r");
    assert_eq!(text_of(cx, &view), "  a\n");
}

#[gpui::test]
fn the_input_options_stay_as_the_file_has_them_without_a_change(cx: &mut TestAppContext) {
    let text = "[Types(0)]\r\nbAutoIndent=0\r\nnSmartIndent=2\r\nszIndentChars=*\r\n";
    let (ini, a) = setup(cx, text);
    let before = ini.bytes();
    run(cx, a, "F_OPTION_TYPE", &[]);
    assert_eq!(value(cx, AUTO_INDENT), Value::Bool(false));
    assert_eq!(value(cx, INDENT_CHARS), Value::Text("*".to_owned()));
    ok(cx);
    assert_eq!(ini.bytes(), before);
}
