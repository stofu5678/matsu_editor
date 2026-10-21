//! Tests of the clipboard functions that need the window, in GPUI's headless window. The HTML
//! itself is tested in `crate::copy_html`; these check that the functions run through the
//! function code and that the text reaches the clipboard.

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use ume_commands::sakura::FUNCTIONS;
use ume_core::buffer::{Document, Pos};
use ume_core::model::DocumentModel;

use crate::document::SharedDocument;
use crate::view::EditorView;

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

fn setup<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
) -> (Entity<EditorView>, &'a mut VisualTestContext) {
    cx.update(gpui_component::init);
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(document, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    vcx.run_until_parked();
    (view, vcx)
}

fn run(view: &Entity<EditorView>, cx: &mut VisualTestContext, function: &str) -> bool {
    let code = code_of(function);
    let ok = view.update_in(cx, |v, window, cx| v.run_function(code, &[], window, cx));
    cx.run_until_parked();
    ok
}

fn clipboard_text(cx: &mut VisualTestContext) -> Option<String> {
    cx.read_from_clipboard().and_then(|item| item.text())
}

#[gpui::test]
fn the_selection_is_copied_as_html_text(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "a<b\r\ncd\r\n");
    view.update(cx, |v, _| {
        v.cursor.move_to(Pos::new(0, 0), false);
        v.cursor.move_to(Pos::new(1, 1), true);
    });
    assert!(run(&view, cx, "F_COPY_COLOR_HTML"));
    let text = clipboard_text(cx).unwrap();
    assert!(
        text.starts_with("<pre style=\"background-color:#"),
        "{text}"
    );
    assert!(text.contains("a&lt;b\r\nc"), "{text}");
    assert!(text.ends_with("</pre>"), "{text}");
    assert!(!text.contains("1:"), "{text}");

    assert!(run(&view, cx, "F_COPY_COLOR_HTML_LINENUMBER"));
    let numbered = clipboard_text(cx).unwrap();
    assert!(
        numbered.contains(">1:") || numbered.contains("1:<span"),
        "{numbered}"
    );
    assert!(numbered.contains("2:<span"), "{numbered}");
}

#[gpui::test]
fn nothing_is_copied_without_a_selection(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc\r\n");
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("before".to_owned()));
    assert!(run(&view, cx, "F_COPY_COLOR_HTML"));
    assert_eq!(clipboard_text(cx).as_deref(), Some("before"));
}

#[gpui::test]
fn the_key_list_is_copied_as_text(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc\r\n");
    assert!(run(&view, cx, "F_CREATEKEYBINDLIST"));
    let text = clipboard_text(cx).unwrap();
    let mut lines = text.split("\r\n");
    assert_eq!(
        lines.next(),
        Some("キー\t機能名\t関数名\t機能番号\tキーマクロ記録可/不可")
    );
    assert_eq!(lines.next(), Some("-----\t-----\t-----\t-----\t-----"));
    assert!(
        text.contains("\r\nCtrl+C\tコピー\tCopy\t30602\t○\r\n"),
        "{text}"
    );
}
