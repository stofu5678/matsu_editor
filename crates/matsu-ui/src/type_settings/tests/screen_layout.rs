//! The display side of the page スクリーン: the kinsoku, the indent of wrapped rows and the marks
//! of a tab are read and written as Sakura's `Types(N)` keys, and the windows of the type lay out
//! with them.

use gpui::TestAppContext;
use matsu_layout::WrapIndent;

use super::*;
use crate::settings::view_settings::TabMark;
use crate::type_settings::values::tab_view_string;

const CONTROLS: [&str; 10] = [
    KINSOKU_KUTO_CHARS,
    KINSOKU_HEAD,
    KINSOKU_HEAD_CHARS,
    KINSOKU_TAIL,
    KINSOKU_TAIL_CHARS,
    INDENT_LAYOUT,
    TAB_ARROW,
    TAB_VIEW_STRING,
    KINSOKU_KUTO,
    KINSOKU_RET,
];

#[gpui::test]
fn the_controls_of_the_display_side_are_available(cx: &mut TestAppContext) {
    let (_ini, a) = setup(cx, "");
    run(cx, a, "F_TYPE_SCREEN", &[]);
    with_dialog(cx, |d, _, _| {
        for id in CONTROLS {
            let control = d
                .page_controls()
                .into_iter()
                .find(|c| c.id == id)
                .unwrap_or_else(|| panic!("{id}"));
            assert!(d.is_usable(&control), "{id}");
        }
    });
    // The string of a tab is for the string only (`IDC_EDIT_TABVIEWSTRING`).
    choose(cx, TAB_ARROW, 1);
    with_dialog(cx, |d, _, _| {
        let control = d
            .page_controls()
            .into_iter()
            .find(|c| c.id == TAB_VIEW_STRING)
            .unwrap();
        assert!(!d.is_usable(&control));
    });
    cancel(cx);
}

#[gpui::test]
fn ok_writes_the_keys_of_sakura_and_the_windows_lay_out_with_them(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "");
    let view = view_of(cx, a);
    run(cx, a, "F_TYPE_SCREEN", &[]);
    check(cx, KINSOKU_HEAD, true);
    check(cx, KINSOKU_TAIL, true);
    check(cx, KINSOKU_KUTO, true);
    type_in(cx, KINSOKU_HEAD_CHARS, "ー。");
    type_in(cx, KINSOKU_TAIL_CHARS, "「");
    type_in(cx, KINSOKU_KUTO_CHARS, "、");
    choose(cx, INDENT_LAYOUT, 2);
    choose(cx, TAB_ARROW, 0);
    type_in(cx, TAB_VIEW_STRING, "ab");
    ok(cx);

    let get = |key: &str| ini.get("Types(0)", key);
    assert_eq!(get("bKinsokuHead").as_deref(), Some("1"));
    assert_eq!(get("bKinsokuTail").as_deref(), Some("1"));
    assert_eq!(get("bKinsokuKuto").as_deref(), Some("1"));
    assert_eq!(get("szKinsokuHead").as_deref(), Some("ー。"));
    assert_eq!(get("szKinsokuTail").as_deref(), Some("「"));
    assert_eq!(get("szKinsokuKuto").as_deref(), Some("、"));
    assert_eq!(get("nIndentLayout").as_deref(), Some("2"));
    assert_eq!(get("bTabArrow").as_deref(), Some("0"));
    // `GetData`: what is typed over the default.
    assert_eq!(get("szTabViewString").as_deref(), Some("ab      "));

    // The window has them in its layout and in how it draws a tab.
    let (config, mark) = view.read_with(cx, |v, _| {
        (
            v.layout.config().clone(),
            v.view_settings().tab_mark.clone(),
        )
    });
    assert!(config.kinsoku.head && config.kinsoku.tail && config.kinsoku.kuto);
    assert!(config.kinsoku.head_chars.contains('ー'));
    assert!(config.kinsoku.tail_chars.contains('「'));
    assert!(config.kinsoku.kuto_chars.contains('、'));
    assert!(matches!(
        config.indent,
        WrapIndent::LeadingWhiteSpace { .. }
    ));
    assert_eq!(mark, TabMark::Chars("ab      ".to_owned()));

    // Read again, the sheet shows the same, and a second OK changes nothing.
    let before = ini.bytes();
    run(cx, a, "F_TYPE_SCREEN", &[]);
    assert_eq!(
        value(cx, KINSOKU_HEAD_CHARS),
        Value::Text("ー。".to_owned())
    );
    assert_eq!(
        value(cx, TAB_VIEW_STRING),
        Value::Text("ab      ".to_owned())
    );
    ok(cx);
    assert_eq!(ini.bytes(), before);
}

#[gpui::test]
fn the_defaults_are_sakuras(cx: &mut TestAppContext) {
    let (_ini, a) = setup(cx, "");
    run(cx, a, "F_TYPE_SCREEN", &[]);
    assert_eq!(value(cx, TAB_ARROW), Value::Int(0));
    assert_eq!(
        value(cx, TAB_VIEW_STRING),
        Value::Text("^       ".to_owned())
    );
    assert_eq!(value(cx, INDENT_LAYOUT), Value::Int(0));
    assert_eq!(value(cx, KINSOKU_HEAD), Value::Bool(false));
    assert_eq!(value(cx, KINSOKU_HEAD_CHARS), Value::Text(String::new()));
    assert_eq!(
        value(cx, KINSOKU_KUTO_CHARS),
        Value::Text("、。，．､｡,.".to_owned())
    );
    cancel(cx);
}

#[test]
fn the_string_of_a_tab_is_typed_over_the_default() {
    assert_eq!(tab_view_string(""), "^       ");
    assert_eq!(tab_view_string("ab"), "ab      ");
    assert_eq!(tab_view_string("123456789"), "12345678");
    // The characters a tab cannot show end the text (`IsTabAvailableCode`).
    assert_eq!(tab_view_string("a\tb"), "a       ");
}

#[test]
fn a_tab_is_drawn_by_its_kind() {
    let string = TabMark::from_setting(0, "^       ");
    assert_eq!(string.text(4), "^   ");
    assert_eq!(string.text(1), "^");
    // Shorter than the tab: filled with spaces.
    assert_eq!(TabMark::Chars("ab".to_owned()).text(4), "ab  ");
    let short = TabMark::from_setting(1, "^");
    assert_eq!(short.text(4), "\u{2192}");
    let long = TabMark::from_setting(2, "^");
    assert_eq!(long.text(4), "\u{2500}\u{2500}\u{2500}\u{2192}");
    assert_eq!(long.text(1), "\u{2192}");
    assert_ne!(string, short);
    assert_ne!(short, long);
}
