//! Tests of printing in the window. The OS print dialogs cannot run here; what happens before
//! them (the preview, the keys, an empty document) and after them (`finish_print`) can.

use std::path::PathBuf;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_core::buffer::Document;
use matsu_core::model::DocumentModel;
use matsu_platform::print::Error;
use matsu_print::{LineSource, Printer};

use super::{DocLines, Outcome, Phase};
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

/// `$R` and `$N` of the header are filled as the title fills them (`ExpandParameter`).
#[gpui::test]
fn the_header_has_the_write_state_the_short_path_and_the_number(cx: &mut TestAppContext) {
    use matsu_print::{WriteState, expand_parameter};
    let (view, cx) = setup(cx, "abc", Some("/tmp/memo.txt"));
    let job = view.read_with(cx, |v, cx| v.print_job(cx));
    assert_eq!(job.header.write_state, WriteState::Writable);
    assert_eq!(job.header.file_path_short.as_deref(), Some("/tmp/memo.txt"));
    assert_eq!(
        expand_parameter("$N ${R?(ビューモード)$:(上書き禁止)$}", &job.header, None),
        "/tmp/memo.txt "
    );

    view.update(cx, |v, cx| {
        v.document()
            .update(cx, |shared, cx| shared.set_view_mode(true, cx));
    });
    let job = view.read_with(cx, |v, cx| v.print_job(cx));
    assert_eq!(job.header.write_state, WriteState::ViewMode);
    assert_eq!(
        expand_parameter("${R?(ビューモード)$:(上書き禁止)$}", &job.header, None),
        "(ビューモード)"
    );
    assert_eq!(job.header.app_name, crate::support::about::PRODUCT);

    // A document with no file has its number: `$f$n`.
    let (view, cx) = setup(cx, "", None);
    let number = view.update(cx, |v, cx| {
        v.document()
            .update(cx, |shared, _| shared.number_untitled())
    });
    let job = view.read_with(cx, |v, cx| v.print_job(cx));
    assert_eq!(
        expand_parameter("$f$n", &job.header, None),
        format!("(無題){number}")
    );
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
fn the_preview_function_turns_the_preview_on_and_off(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc", None);
    run(&view, F_PRINT_PREVIEW, cx);
    assert!(view.read_with(cx, |v, _| v.is_print_preview_open()));
    // Called again, it closes the preview (`CEditWnd::PrintPreviewModeONOFF`).
    run(&view, F_PRINT_PREVIEW, cx);
    assert!(!view.read_with(cx, |v, _| v.is_print_preview_open()));
    run(&view, F_PRINT_PREVIEW, cx);
    assert!(view.read_with(cx, |v, _| v.is_print_preview_open()));
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
    assert!(!view.read_with(cx, |v, _| v.is_printing()));
}

#[gpui::test]
fn the_page_setup_opens_its_dialog_and_not_the_preview(cx: &mut TestAppContext) {
    // The dialog itself is tested in `crate::print_setting`.
    let (view, cx) = setup(cx, "abc", None);
    run(&view, F_PRINT_PAGESETUP, cx);
    assert!(view.read_with(cx, |v, _| v.print.setup_dialog.is_some()));
    assert!(!view.read_with(cx, |v, _| v.is_print_preview_open()));
}

#[gpui::test]
fn the_preview_closes_after_printing_and_stays_after_a_cancel(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc", None);
    run(&view, F_PRINT_PREVIEW, cx);
    let preview = view
        .read_with(cx, |v, _| v.print_preview().cloned())
        .unwrap();

    view.update_in(cx, |v, window, cx| {
        v.print.phase = Phase::Printing { slot: 0 };
        v.finish_print(Ok(Outcome::Cancelled), window, cx);
    });
    assert!(view.read_with(cx, |v, _| v.is_print_preview_open() && !v.is_printing()));
    let notice = preview.read_with(cx, |p, _| p.notice.clone());
    assert_eq!(notice.as_deref(), Some("印刷を取り消しました"));

    // A failure is shown on the preview, too.
    view.update_in(cx, |v, window, cx| {
        v.finish_print(Err(Error::from("プリンターがありません")), window, cx);
    });
    let notice = preview.read_with(cx, |p, _| p.notice.clone());
    assert_eq!(notice.as_deref(), Some("プリンターがありません"));
    assert!(view.read_with(cx, |v, _| v.is_print_preview_open()));

    view.update_in(cx, |v, window, cx| {
        v.finish_print(Ok(Outcome::Printed(None)), window, cx)
    });
    assert!(!view.read_with(cx, |v, _| v.is_print_preview_open()));
    let notice = view.read_with(cx, |v, _| v.notice.clone());
    assert_eq!(notice.as_deref(), Some("印刷しました"));
}

#[gpui::test]
fn the_printer_the_dialog_gave_is_remembered_by_the_setting_the_window_prints_with(
    cx: &mut TestAppContext,
) {
    let (view, cx) = setup(cx, "abc", None);
    let printer = Printer {
        driver: "winspool".into(),
        device: "Microsoft Print to PDF".into(),
        output: "PORTPROMPT:".into(),
    };
    assert!(view.read_with(cx, |v, cx| v.current_print_setting(cx).printer.is_none()));

    view.update_in(cx, |v, window, cx| {
        let slot = v.print_slot(cx);
        v.print.phase = Phase::Printing { slot };
        v.finish_print(Ok(Outcome::Printed(Some(printer.clone()))), window, cx);
    });
    assert!(!view.read_with(cx, |v, _| v.is_printing()));
    assert_eq!(
        view.read_with(cx, |v, cx| v.current_print_setting(cx).printer),
        Some(printer.clone())
    );
    // the print job of the next print starts the dialog on it
    assert_eq!(
        view.read_with(cx, |v, cx| v.print_job(cx).setting.printer),
        Some(printer.clone())
    );

    // an OS that does not say which printer leaves it as it was
    view.update_in(cx, |v, window, cx| {
        v.finish_print(Ok(Outcome::Printed(None)), window, cx);
    });
    assert_eq!(
        view.read_with(cx, |v, cx| v.current_print_setting(cx).printer),
        Some(printer)
    );
}

/// The printer that comes back goes to the setting the print started with, even when another
/// window selected another setting meanwhile; a result with no print running remembers nothing.
#[gpui::test]
fn the_printer_goes_to_the_setting_the_print_started_with(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc", None);
    let printer = Printer {
        driver: "winspool".into(),
        device: "PDF".into(),
        output: "FILE:".into(),
    };
    let current = view.read_with(cx, |v, cx| v.print_slot(cx));
    let started_with = (current + 1) % 8;
    view.update_in(cx, |v, window, cx| {
        v.print.phase = Phase::Printing { slot: started_with };
        v.finish_print(Ok(Outcome::Printed(Some(printer.clone()))), window, cx);
    });
    let (at_start, now) = cx.update(|_, cx| {
        (
            EditorView::remembered_printer(started_with, cx),
            EditorView::remembered_printer(current, cx),
        )
    });
    assert_eq!(at_start, Some(printer.clone()));
    assert_eq!(now, None);
    // no print runs: nothing is remembered
    let other = Printer {
        device: "Other".into(),
        ..printer
    };
    view.update_in(cx, |v, window, cx| {
        v.finish_print(Ok(Outcome::Printed(Some(other))), window, cx);
    });
    let (a, b) = cx.update(|_, cx| {
        (
            EditorView::remembered_printer(started_with, cx),
            EditorView::remembered_printer(current, cx),
        )
    });
    assert_eq!(a.unwrap().device, "PDF");
    assert_eq!(b, None);
}

/// Printing a big document opens the preview, whose pages are laid out on a thread; the print
/// dialog waits for them rather than failing, and closing the preview drops the wait.
#[gpui::test]
fn printing_a_big_document_waits_for_its_pages(cx: &mut TestAppContext) {
    let text = "line\n".repeat(30_000);
    let (view, cx) = setup(cx, &text, None);
    run(&view, F_PRINT, cx);
    view.read_with(cx, |v, cx| {
        let preview = v.print_preview().unwrap().read(cx);
        assert!(preview.is_laying_out());
        // No dialog yet: the print waits, with the setting it was asked for.
        assert!(matches!(v.print.phase, Phase::WaitingForLayout { .. }));
    });
    // A macro that waits for the print sees the queued one as printing.
    assert!(view.read_with(cx, |v, _| v.is_printing()));
    view.update_in(cx, |v, window, cx| v.close_print_preview(window, cx));
    assert!(!view.read_with(cx, |v, _| v.is_printing()));
}

/// `$p` and `$P` of a macro or a plugin read the print preview of the window
/// (`CSakuraEnvironment::ExpandParameter`: `m_pPrintPreview`).
#[gpui::test]
fn a_macro_gets_the_page_of_the_preview_while_it_is_open(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc", None);
    let page = |cx: &mut VisualTestContext| {
        view.update(cx, |v, cx| v.host_env("ExpandParameter", cx).print_preview)
    };
    assert_eq!(page(cx), None);

    run(&view, F_PRINT_PREVIEW, cx);
    let preview = view
        .read_with(cx, |v, _| v.print_preview().cloned())
        .unwrap();
    let total = preview.read_with(cx, |p, _| p.page_count());
    assert_eq!(
        page(cx),
        Some(matsu_plugin::host::PreviewPage { number: 1, total })
    );

    view.update_in(cx, |v, window, cx| v.close_print_preview(window, cx));
    assert_eq!(page(cx), None);
}

/// The variables that were empty (#138) come from the window, as `ExpandParameter` reads them.
#[gpui::test]
fn the_header_has_the_selection_the_caret_the_clock_and_the_paths(cx: &mut TestAppContext) {
    use matsu_core::buffer::Pos;
    use matsu_print::expand_parameter;
    let (view, cx) = setup(cx, "あいう abc\r\nsecond line\r\n", None);
    // `$C` is the first line of the selection (`GetCurrentTextForSearch`).
    view.update(cx, |v, cx| {
        v.run_with(cx, |c| c.select_range(Pos::new(0, 3), Pos::new(1, 3)))
            .ok();
    });
    let job = view.read_with(cx, |v, cx| v.print_job(cx));
    assert_eq!(job.header.selected_text, "いう abc");
    // Without a selection it is the word at the caret.
    view.update(cx, |v, cx| {
        v.run_with(cx, |c| c.move_caret(Pos::new(1, 8), false)).ok();
    });
    let job = view.read_with(cx, |v, cx| v.print_job(cx));
    assert_eq!(job.header.selected_text, "line");
    // `$x` counts UTF-16 units from 1, not the bytes of UTF-8: after "あいう " it is 5, not 11.
    view.update(cx, |v, cx| {
        v.run_with(cx, |c| c.move_caret(Pos::new(0, 10), false))
            .ok();
    });
    let job = view.read_with(cx, |v, cx| v.print_job(cx));
    assert_eq!(job.header.caret_x, 5);
    assert_eq!(job.header.caret_y, 1);
    // The clock, in the format of the common settings.
    assert!(!job.header.date.is_empty() && !job.header.time.is_empty());
    // A document that was never saved has no time stamp.
    assert_eq!(
        expand_parameter("$D $T", &job.header, None),
        "(保存されていません) (保存されていません)"
    );
    // The program; no macro is running.
    assert!(!job.header.exe_path.is_empty());
    assert_eq!(job.header.macro_path, None);
}

#[gpui::test]
fn the_title_expands_only_what_it_asks_for(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "word", None);
    view.update(cx, |v, cx| {
        v.run_with(cx, |c| {
            c.move_caret(matsu_core::buffer::Pos::new(0, 1), false)
        })
        .ok();
    });
    let title = view.read_with(cx, |v, cx| v.expand_format("[$C][$x]", cx));
    assert_eq!(title, "[word][2]");
}

#[gpui::test]
fn the_title_has_the_page_and_the_page_count_while_the_preview_is_open(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, &many_lines(200), None);
    let expand = |view: &Entity<EditorView>, cx: &mut VisualTestContext| {
        view.read_with(cx, |v, cx| v.expand_format("$p/$P", cx))
    };
    // Without the preview Sakura says that the codes are for the preview only.
    let only = "(印刷プレビューでのみ使用できます)";
    assert_eq!(expand(&view, cx), format!("{only}/{only}"));
    run(&view, F_PRINT_PREVIEW, cx);
    let preview = view
        .read_with(cx, |v, _| v.print_preview().cloned())
        .unwrap();
    let pages = preview.read_with(cx, |p, _| p.page_count());
    assert!(pages >= 3);
    assert_eq!(expand(&view, cx), format!("1/{pages}"));
    // The page follows the preview; the tab caption has the same expansion.
    cx.simulate_keystrokes("pagedown");
    assert_eq!(expand(&view, cx), format!("2/{pages}"));
    cx.simulate_keystrokes("escape");
    assert_eq!(expand(&view, cx), format!("{only}/{only}"));
}
