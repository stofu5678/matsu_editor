//! OK of the type settings keeps the temporary wrap and the temporary font size of a view of the
//! type, unless they end (`CEditDoc::OnChangeSetting`, `sakura_core/doc/CEditDoc.cpp`).

use gpui::{Entity, TestAppContext, VisualTestContext};
use ume_commands::command::Arg;

use super::tests::{run, user, view_of, window};
use crate::settings::view_settings::WrapMethod;
use crate::view::EditorView;

/// A settings file whose basic type wraps by `wrap` (`nTextWrapMethod`, with `columns` for
/// 指定桁), with a common font of `points` (1/10 point).
fn ini(wrap: i32, columns: i32, points: i32) -> String {
    format!(
        "[Font]\r\nlfFaceName=MS Gothic\r\nnPointSize={points}\r\nlfOutPrecision=3\r\n\
         [Other]\r\nnTypesCount=17\r\n\
         [Types(0)]\r\nnTextWrapMethod={wrap}\r\nnInts=0,{columns},0,4,-1,-1,0,1,1,0,0,0\r\n"
    )
}

/// The settings file changes and the type settings dialog says OK for type 0.
fn change_settings(view: &Entity<EditorView>, cx: &mut VisualTestContext, text: &str) {
    cx.update(|_, app| app.set_global(user(text)));
    view.update(cx, |view, cx| view.refresh_type(0, cx));
}

fn wrap(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> (WrapMethod, u32) {
    view.read_with(cx, |v, _| (v.prefs.wrap_method, v.prefs.wrap_columns))
}

#[gpui::test]
fn a_temporary_wrap_stays_when_the_type_changes_to_another_wrap(cx: &mut TestAppContext) {
    let handle = window(cx, Some(user(&ini(0, 80, 100))));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    run(&view, cx, "F_TMPWRAPWINDOW", &[]);
    assert_eq!(wrap(&view, cx).0, WrapMethod::WindowWidth);
    change_settings(&view, cx, &ini(1, 60, 100));
    assert_eq!(wrap(&view, cx).0, WrapMethod::WindowWidth);
}

#[gpui::test]
fn a_temporary_wrap_ends_when_it_equals_the_type(cx: &mut TestAppContext) {
    let handle = window(cx, Some(user(&ini(0, 80, 100))));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    run(&view, cx, "F_TMPWRAPWINDOW", &[]);
    change_settings(&view, cx, &ini(2, 80, 100));
    assert_eq!(wrap(&view, cx).0, WrapMethod::WindowWidth);
    // No longer temporary: the next change of the type is taken.
    change_settings(&view, cx, &ini(0, 80, 100));
    assert_eq!(wrap(&view, cx).0, WrapMethod::None);
}

#[gpui::test]
fn a_temporary_column_stays_unless_the_type_has_the_same_column(cx: &mut TestAppContext) {
    let handle = window(cx, Some(user(&ini(0, 80, 100))));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    run(&view, cx, "F_TMPWRAPSETTING", &[]);
    assert_eq!(wrap(&view, cx), (WrapMethod::SettingWidth, 80));
    // The same method with another column stays temporary (2013.05.29).
    change_settings(&view, cx, &ini(1, 40, 100));
    assert_eq!(wrap(&view, cx), (WrapMethod::SettingWidth, 80));
    // The type now has the column: the temporary wrap ends and the type is followed.
    change_settings(&view, cx, &ini(1, 80, 100));
    change_settings(&view, cx, &ini(1, 50, 100));
    assert_eq!(wrap(&view, cx), (WrapMethod::SettingWidth, 50));
}

#[gpui::test]
fn without_a_temporary_wrap_the_type_is_followed(cx: &mut TestAppContext) {
    let handle = window(cx, Some(user(&ini(0, 80, 100))));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    change_settings(&view, cx, &ini(1, 30, 100));
    assert_eq!(wrap(&view, cx), (WrapMethod::SettingWidth, 30));
}

#[gpui::test]
fn a_temporary_font_size_stays_unless_the_base_size_changes(cx: &mut TestAppContext) {
    let handle = window(cx, Some(user(&ini(0, 80, 100))));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let base = view.read_with(cx, |v, _| v.font_px());
    run(&view, cx, "F_SETFONTSIZEUP", &[Arg::Int(1)]);
    let zoomed = view.read_with(cx, |v, _| v.font_px());
    assert!(zoomed > base);
    // Another setting of the type changed: the size stays.
    change_settings(&view, cx, &ini(1, 60, 100));
    assert_eq!(view.read_with(cx, |v, _| v.font_px()), zoomed);
    // The base size changed: the temporary size ends.
    change_settings(&view, cx, &ini(1, 60, 160));
    let after = view.read_with(cx, |v, _| v.font_px());
    assert!(after > base && after != zoomed);
    assert_eq!(
        after,
        view.read_with(cx, |v, _| v.view_settings().font_px())
    );
}
