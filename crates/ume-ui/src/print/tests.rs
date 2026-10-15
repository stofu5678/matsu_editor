//! Tests of printing in the window. The OS print dialogs cannot run here; what happens before
//! them (the preview, the keys, an empty document) and after them (`finish_print`) can.

use std::path::PathBuf;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use ume_core::buffer::Document;
use ume_core::model::DocumentModel;
use ume_platform::print::Error;
use ume_print::LineSource;

use super::DocLines;
use crate::document::SharedDocument;
use crate::ui_command::UiRegistry;
use crate::view::EditorView;

/// Sakura's function codes (`sakura_core/Funccode_enum.h`).
const F_PRINT: u16 = 30150;
const F_PRINT_PREVIEW: u16 = 30151;
const F_PRINT_PAGESETUP: u16 = 30152;

fn setup<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
    path: Option<&str>,
) -> (Entity<EditorView>, &'a mut VisualTestContext) {
    // The bar of the preview uses gpui-component's buttons and theme.
    cx.update(gpui_component::init);
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let path = path.map(PathBuf::from);
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), path));
    cx.add_window_view(move |window, cx| {
        let view = EditorView::new(document, cx);
        window.focus(&view.focus_handle, cx);
        view
    })
}

fn run(view: &Entity<EditorView>, code: u16, cx: &mut VisualTestContext) {
    view.update_in(cx, |v, window, cx| v.run_function(code, &[], window, cx));
}

fn many_lines(n: usize) -> String {
    (0..n).map(|i| format!("line {i}\r\n")).collect()
}

#[test]
fn the_print_functions_are_ui_commands() {
    let registry = UiRegistry::global();
    for (code, id) in [
        (F_PRINT, "print"),
        (F_PRINT_PREVIEW, "print_preview"),
        (F_PRINT_PAGESETUP, "print_pagesetup"),
    ] {
        assert_eq!(registry.get_by_code(code).map(|r| r.id), Some(id));
    }
}

#[test]
fn the_document_is_read_as_lines_with_their_endings() {
    let doc = Document::from_utf8(b"a\r\nb\r\nc".to_vec()).unwrap();
    let lines = DocLines(doc);
    let mut buf = String::from("old");
    assert_eq!(lines.line_count(), 3);
    assert!(lines.read_line(0, &mut buf));
    assert_eq!(buf, "a");
    // The last line has no line ending.
    assert!(!lines.read_line(2, &mut buf));
    assert_eq!(buf, "c");
    // Past the end reads as an empty line without a line ending.
    assert!(!lines.read_line(9, &mut buf));
    assert_eq!(buf, "");
    // A text that ends with a line ending has no empty line after it.
    let doc = Document::from_utf8(b"a\r\n".to_vec()).unwrap();
    assert_eq!(DocLines(doc).line_count(), 1);
}

#[gpui::test]
fn the_job_has_what_the_header_shows(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc\r\ndef", Some("/tmp/memo.txt"));
    let job = view.read_with(cx, |v, cx| v.print_job(cx));
    assert_eq!(job.title, "memo.txt");
    assert_eq!(job.header.file_path.as_deref(), Some("/tmp/memo.txt"));
    assert_eq!(job.source.line_count(), 2);
    let (view, cx) = setup(cx, "", None);
    let job = view.read_with(cx, |v, cx| v.print_job(cx));
    assert_eq!(job.title, "(無題)");
    assert_eq!(job.header.file_path, None);
}

#[gpui::test]
fn the_preview_takes_the_place_of_the_text_and_esc_goes_back(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, &many_lines(200), None);
    assert!(!view.read_with(cx, |v, _| v.is_print_preview_open()));
    run(&view, F_PRINT_PREVIEW, cx);
    let preview = view
        .read_with(cx, |v, _| v.print_preview().cloned())
        .unwrap();
    let pages = preview.read_with(cx, |p, _| p.page_count());
    assert!(pages >= 3);

    // The keys of the preview (Sakura's bar): page down, the access key N, and page up.
    cx.simulate_keystrokes("pagedown");
    assert_eq!(preview.read_with(cx, |p, _| p.state().0), 1);
    cx.simulate_keystrokes("n");
    assert_eq!(preview.read_with(cx, |p, _| p.state().0), 2);
    cx.simulate_keystrokes("pageup");
    assert_eq!(preview.read_with(cx, |p, _| p.state().0), 1);
    // The editor does not get the keys: nothing was typed.
    let text = view.read_with(cx, |v, cx| {
        v.document().read(cx).doc().line(0).unwrap().unwrap().text
    });
    assert_eq!(text, "line 0");
    // Zoom out with the access key O, in steps of 10 %.
    cx.simulate_keystrokes("o");
    assert_eq!(preview.read_with(cx, |p, _| p.state().1), 90);

    cx.simulate_keystrokes("escape");
    assert!(!view.read_with(cx, |v, _| v.is_print_preview_open()));
    // The caret is back in the editor: typing edits the text.
    cx.simulate_input("X");
    let text = view.read_with(cx, |v, cx| {
        v.document().read(cx).doc().line(0).unwrap().unwrap().text
    });
    assert_eq!(text, "Xline 0");
}

#[gpui::test]
fn opening_the_preview_twice_keeps_the_first(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc", None);
    run(&view, F_PRINT_PREVIEW, cx);
    let first = view
        .read_with(cx, |v, _| v.print_preview().cloned())
        .unwrap();
    // Opening again keeps the preview that is open.
    run(&view, F_PRINT_PREVIEW, cx);
    let second = view
        .read_with(cx, |v, _| v.print_preview().cloned())
        .unwrap();
    assert_eq!(first, second);
}

#[gpui::test]
fn printing_an_empty_document_says_so_and_does_not_ask_the_os(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "", None);
    run(&view, F_PRINT, cx);
    // The preview is open (Sakura opens it first) and shows Sakura's STR_ERR_DLGPRNPRVW7.
    let preview = view
        .read_with(cx, |v, _| v.print_preview().cloned())
        .unwrap();
    let notice = preview.read_with(cx, |p, _| p.notice.clone());
    assert_eq!(notice.as_deref(), Some("印刷するページがありません。"));
    assert!(!view.read_with(cx, |v, _| v.print.printing));
}

#[gpui::test]
fn the_page_setup_opens_the_preview_with_a_notice(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc", None);
    run(&view, F_PRINT_PAGESETUP, cx);
    let preview = view
        .read_with(cx, |v, _| v.print_preview().cloned())
        .unwrap();
    let notice = preview.read_with(cx, |p, _| p.notice.clone()).unwrap();
    assert!(notice.contains("印刷ページ設定"), "{notice}");
}

#[gpui::test]
fn the_preview_closes_after_printing_and_stays_after_a_cancel(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc", None);
    run(&view, F_PRINT_PREVIEW, cx);
    let preview = view
        .read_with(cx, |v, _| v.print_preview().cloned())
        .unwrap();

    view.update_in(cx, |v, window, cx| {
        v.print.printing = true;
        v.finish_print(Ok(false), window, cx);
    });
    assert!(view.read_with(cx, |v, _| v.is_print_preview_open() && !v.print.printing));
    let notice = preview.read_with(cx, |p, _| p.notice.clone());
    assert_eq!(notice.as_deref(), Some("印刷を取り消しました"));

    // A failure is shown on the preview, too.
    view.update_in(cx, |v, window, cx| {
        v.finish_print(Err(Error::from("プリンターがありません")), window, cx);
    });
    let notice = preview.read_with(cx, |p, _| p.notice.clone());
    assert_eq!(notice.as_deref(), Some("プリンターがありません"));
    assert!(view.read_with(cx, |v, _| v.is_print_preview_open()));

    view.update_in(cx, |v, window, cx| v.finish_print(Ok(true), window, cx));
    assert!(!view.read_with(cx, |v, _| v.is_print_preview_open()));
    let notice = view.read_with(cx, |v, _| v.notice.clone());
    assert_eq!(notice.as_deref(), Some("印刷しました"));
}
