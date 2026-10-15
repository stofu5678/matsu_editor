//! Tests of the settings in the view, in GPUI's headless test window (see `tests.rs`).

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use ume_core::buffer::Document;
use ume_core::model::DocumentModel;
use ume_layout::Wrap;

use super::EditorView;
use crate::document::SharedDocument;
use crate::geometry::Metrics;
use crate::settings::{UserSettings, ViewSettings, WrapMethod};

fn setup<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
) -> (Entity<EditorView>, &'a mut VisualTestContext) {
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(document, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    (view, vcx)
}

fn user(text: &str) -> UserSettings {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    UserSettings::from_profile(ume_config::Profile::from_bytes(&bytes), None)
}

fn metrics(view_width: f32) -> Metrics {
    Metrics {
        cell_width: 10.0,
        line_height: 20.0,
        view_width,
        view_height: 400.0,
    }
}

fn wrap(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> Wrap {
    view.read_with(cx, |v, _| v.layout.config().wrap)
}

fn run(view: &Entity<EditorView>, cx: &mut VisualTestContext, code: u16) {
    view.update_in(cx, |v, window, cx| v.run_function(code, &[], window, cx));
}

#[gpui::test]
fn the_file_sets_the_font_the_tab_width_and_the_wrap(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "a\tb");
    let settings = user(
        "[Font]\r\nnPointSize=120\r\nlf=-16,0,0,0,400,0,0,0,128,3,2,1,49\r\nlfFaceName=Consolas\r\n\
         [Types(0)]\r\nnInts=0,40,0,8,-1,-1,0,1,1,0,0,0\r\nnTextWrapMethod=1\r\n",
    );
    view.update(cx, |v, cx| v.apply_user_settings(&settings, cx));
    view.read_with(cx, |v, _| {
        assert_eq!(v.font_px(), 16.0);
        assert_eq!(v.font_family.as_ref(), "Consolas");
        assert_eq!(v.layout.config().tab_width, 8);
        assert_eq!(v.layout.config().wrap, Wrap::Columns(40));
    });
}

#[gpui::test]
fn user_key_bindings_replace_sakuras(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    // The row of the key A: Ctrl+A (index 2) is 30210 (F_UNDO) instead of select all.
    let settings = user("[KeyBind]\r\nKeyBind[000]=0041,0,0,30210,0,0,0,0,0,A\r\n");
    view.update(cx, |v, cx| v.apply_user_settings(&settings, cx));
    cx.simulate_input("x");
    cx.simulate_keystrokes("secondary-a");
    let text = view.read_with(cx, |v, cx| {
        v.document.read(cx).doc().line(0).unwrap().unwrap().text
    });
    assert_eq!(text, "abc");
}

#[gpui::test]
fn the_font_size_functions_follow_the_zoom_table(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let before = view.read_with(cx, |v, _| v.font_px());
    run(&view, cx, 31122); // F_SETFONTSIZEUP
    let up = view.read_with(cx, |v, _| v.font_px());
    assert!(up > before, "{up} > {before}");
    run(&view, cx, 31123); // F_SETFONTSIZEDOWN
    run(&view, cx, 31123);
    let down = view.read_with(cx, |v, _| v.font_px());
    assert!(down < before, "{down} < {before}");
}

#[gpui::test]
fn the_temporary_wraps_change_the_layout_not_the_settings(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, &"x".repeat(300));
    view.update(cx, |v, _| v.set_metrics(metrics(800.0)));
    let rows = |cx: &mut VisualTestContext| view.read_with(cx, |v, _| v.layout.row_count());
    assert_eq!(rows(cx), 1);
    run(&view, cx, 31143); // F_TMPWRAPWINDOW: 80 columns less the wrap mark's
    assert_eq!(wrap(&view, cx), Wrap::Columns(79));
    assert_eq!(rows(cx), 4); // 300 / 79, rounded up
    run(&view, cx, 31141); // F_TMPWRAPNOWRAP
    assert_eq!(rows(cx), 1);
    view.read_with(cx, |v, _| {
        assert_eq!(v.view_settings().wrap_method, WrapMethod::None);
    });
}

#[gpui::test]
fn the_wrap_follows_the_width_of_the_window(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, &"x".repeat(300));
    run(&view, cx, 31143);
    view.update(cx, |v, cx| {
        v.set_metrics(metrics(400.0));
        v.follow_window_width(cx);
    });
    assert_eq!(wrap(&view, cx), Wrap::Columns(39));
}

#[gpui::test]
fn wrap_window_width_toggles_window_then_largest_then_the_types_column(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let settings = user("[Types(0)]\r\nnInts=0,60,0,4,-1,-1,0,1,1,0,0,0\r\nnTextWrapMethod=1\r\n");
    view.update(cx, |v, cx| v.apply_user_settings(&settings, cx));
    // The test window is painted with its own size; the wrap is worked out from that.
    let window = view.read_with(cx, |v, _| v.window_columns().saturating_sub(1));
    assert_ne!(window, 60);
    assert_eq!(wrap(&view, cx), Wrap::Columns(60));
    run(&view, cx, 31140); // F_WRAPWINDOWWIDTH: 60 is not the window's, so the window
    assert_eq!(wrap(&view, cx), Wrap::Columns(window));
    run(&view, cx, 31140); // the window's width: the largest
    assert_eq!(wrap(&view, cx), Wrap::Columns(10240));
    run(&view, cx, 31140); // the largest, and the type's differs: the type's
    assert_eq!(wrap(&view, cx), Wrap::Columns(60));
}

#[gpui::test]
fn the_status_bar_toggles_and_a_type_change_keeps_it(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    assert!(view.read_with(cx, |v, _| v.status_bar_shown()));
    run(&view, cx, 31102); // F_SHOWSTATUSBAR
    assert!(!view.read_with(cx, |v, _| v.status_bar_shown()));
    view.update(cx, |v, cx| v.apply_settings(ViewSettings::default(), cx));
    assert!(!view.read_with(cx, |v, _| v.status_bar_shown()));
}

#[gpui::test]
fn a_file_that_hides_the_status_bar_starts_without_it(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let settings = user("[Common]\r\nbDispSTATUSBAR=0\r\n");
    view.update(cx, |v, cx| v.apply_user_settings(&settings, cx));
    assert!(!view.read_with(cx, |v, _| v.status_bar_shown()));
}

#[gpui::test]
fn ctrl_wheel_zooms_and_the_plain_wheel_scrolls(cx: &mut TestAppContext) {
    use gpui::{Modifiers, ScrollDelta, ScrollWheelEvent, TouchPhase, point, px};
    let (view, cx) = setup(cx, &"line\n".repeat(200));
    let wheel = |modifiers| ScrollWheelEvent {
        position: point(px(100.), px(100.)),
        delta: ScrollDelta::Lines(point(0., 1.)),
        modifiers,
        touch_phase: TouchPhase::Moved,
    };
    let before = view.read_with(cx, |v, _| v.font_px());
    cx.simulate_event(wheel(Modifiers::secondary_key()));
    let (after, scroll) = view.read_with(cx, |v, _| (v.font_px(), v.scroll_y()));
    assert!(after > before, "wheel up with Ctrl is F_SETFONTSIZEUP");
    assert_eq!(scroll, 0.0);
    view.update(cx, |v, _| v.scroll_by(0.0, -100.0));
    let scrolled = view.read_with(cx, |v, _| v.scroll_y());
    cx.simulate_event(wheel(Modifiers::default()));
    let (size, now) = view.read_with(cx, |v, _| (v.font_px(), v.scroll_y()));
    assert_eq!(size, after);
    assert!(now < scrolled);
}

#[gpui::test]
fn a_wrap_change_keeps_the_caret_in_view(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, &"x".repeat(3000));
    view.update(cx, |v, cx| {
        v.set_metrics(metrics(800.0));
        v.run_command("go_line_end", &[], cx);
    });
    run(&view, cx, 31143); // F_TMPWRAPWINDOW: the caret moves to row 37 or so
    view.read_with(cx, |v, cx| {
        let row = v.caret_row(cx).row as f32 * v.metrics().line_height;
        assert!(row >= v.scroll_y() && row < v.scroll_y() + v.metrics().view_height);
    });
}
