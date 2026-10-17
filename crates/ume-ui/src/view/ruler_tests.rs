//! Tests of the ruler and of the space above the text, in GPUI's headless test window (see
//! `tests.rs`): what the paint measured must follow the settings, because the mouse, the scroll
//! bar and the IME all take their positions from it.
//!
//! The ruler itself, the lines and the bracket pair are tested as arithmetic in `ruler.rs`,
//! `lines.rs` and `bracket_marks.rs`, and the settings that switch them in
//! `settings/view_settings.rs`; the test platform has no fonts, so nothing here looks at glyphs.

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use ume_core::buffer::Document;
use ume_core::model::DocumentModel;

use super::EditorView;
use crate::document::SharedDocument;
use crate::settings::UserSettings;

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

/// Where the text starts and how high its area is, as the last paint measured them.
fn measured(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> (f32, f32) {
    cx.run_until_parked();
    view.read_with(cx, |v, _| {
        (
            v.paint.expect("the window was painted").origin_y,
            v.metrics.view_height,
        )
    })
}

fn apply(view: &Entity<EditorView>, cx: &mut VisualTestContext, ini: &str) {
    let settings = user(ini);
    view.update(cx, |v, cx| v.apply_user_settings(&settings, cx));
}

#[gpui::test]
fn the_ruler_takes_its_height_from_the_top_of_the_view(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    // Sakura's defaults: the ruler is shown and 13 pixels high
    let (shown_y, shown_height) = measured(&view, cx);
    apply(&view, cx, "[Types(0)]\r\nC[RUL]=0,0,000000,efefef,0\r\n");
    let (hidden_y, hidden_height) = measured(&view, cx);
    assert_eq!(shown_y - hidden_y, 13.0);
    assert_eq!(hidden_height - shown_height, 13.0);
}

#[gpui::test]
fn the_height_of_the_ruler_and_the_space_below_it_are_settings(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    apply(&view, cx, "[Types(0)]\r\nC[RUL]=0,0,000000,efefef,0\r\n");
    let (hidden_y, hidden_height) = measured(&view, cx);
    apply(
        &view,
        cx,
        "[Common]\r\nnRulerHeight=20\r\nnRulerBottomSpace=3\r\n",
    );
    let (y, height) = measured(&view, cx);
    assert_eq!(y - hidden_y, 23.0);
    assert_eq!(hidden_height - height, 23.0);
    // the space stays when the ruler is off, the ruler's height does not
    apply(
        &view,
        cx,
        "[Common]\r\nnRulerHeight=20\r\nnRulerBottomSpace=3\r\n\
         [Types(0)]\r\nC[RUL]=0,0,000000,efefef,0\r\n",
    );
    let (y, _) = measured(&view, cx);
    assert_eq!(y - hidden_y, 3.0);
}

#[gpui::test]
fn a_click_below_the_ruler_finds_the_first_row(cx: &mut TestAppContext) {
    use gpui::point;
    let (view, cx) = setup(cx, "abc\ndef");
    cx.run_until_parked();
    let first = view.read_with(cx, |v, cx| {
        let paint = v.paint.expect("the window was painted");
        // the middle of the first row, in the first column
        let at = point(
            gpui::px(paint.origin_x + 1.0),
            gpui::px(paint.origin_y + v.metrics.line_height / 2.0),
        );
        v.position_for_point(at, cx)
    });
    assert_eq!(first, Some(ume_core::buffer::Pos::new(0, 0)));
    // a point in the ruler is above the first row: it maps to the first row too
    let above = view.read_with(cx, |v, cx| {
        let paint = v.paint.expect("the window was painted");
        v.position_for_point(
            point(
                gpui::px(paint.origin_x + 1.0),
                gpui::px(paint.origin_y - 5.0),
            ),
            cx,
        )
    });
    assert_eq!(above, Some(ume_core::buffer::Pos::new(0, 0)));
}
