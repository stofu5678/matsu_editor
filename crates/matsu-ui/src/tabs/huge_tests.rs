//! Tests of a file opened by ranges (ADR-0020): the file is scanned in the background, the document
//! is read-only meanwhile, and the status bar says how far the scan is.

use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::{AnyWindowHandle, AppContext as _, Entity, TestAppContext};
use matsu_commands::command::Arg;
use matsu_core::buffer::ScanOptions;

use super::file_tests::{open_file, temp_dir};
use super::tests::{active_view, run, window};
use crate::file_ops::prefs::FilePrefs;
use crate::view::EditorView;

/// A file of about 1.2 MiB: 100 000 lines, `line 000000` and so on.
fn big_text() -> String {
    (0..100_000).map(|i| format!("line {i:06}\n")).collect()
}

fn write_big(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

/// Files of 1 MiB or more are opened by ranges, and the scan takes a while (a window of 64 KiB
/// every 300 ms), so that the tests can look at the document while it runs.
fn slow_scan(cx: &mut TestAppContext) {
    cx.update(|app| {
        app.set_global(FilePrefs {
            huge_file_size_mb: 1,
            huge_scan: ScanOptions {
                block_size: 64 * 1024,
                window_blocks: 1,
                step_delay: Some(Duration::from_millis(300)),
                ..ScanOptions::default()
            },
            ..FilePrefs::default()
        });
    });
}

fn scanning(cx: &mut TestAppContext, view: &Entity<EditorView>) -> bool {
    view.read_with(cx, |v, cx| v.document().read(cx).is_scanning())
}

fn notice(cx: &mut TestAppContext, view: &Entity<EditorView>) -> Option<String> {
    view.read_with(cx, |v, cx| v.status(cx).notice)
}

fn line_count(cx: &mut TestAppContext, view: &Entity<EditorView>) -> usize {
    view.read_with(cx, |v, cx| v.document().read(cx).doc().line_count())
}

fn caret_line(cx: &mut TestAppContext, view: &Entity<EditorView>) -> usize {
    view.read_with(cx, |v, _| v.cursor.caret().line)
}

/// Lets the scan thread and the task of the view go on until the scan is done.
fn wait_for_scan(cx: &mut TestAppContext, view: &Entity<EditorView>) {
    for _ in 0..6000 {
        if !scanning(cx, view) {
            return;
        }
        cx.executor().advance_clock(Duration::from_millis(150));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the scan did not end");
}

fn opened(cx: &mut TestAppContext, name: &str, bytes: &[u8]) -> (AnyWindowHandle, PathBuf) {
    opened_with(cx, name, bytes, |_, _| {})
}

/// Like `opened`; `before` runs with the path just before the file is opened.
fn opened_with(
    cx: &mut TestAppContext,
    name: &str,
    bytes: &[u8],
    before: impl FnOnce(&mut TestAppContext, &Path),
) -> (AnyWindowHandle, PathBuf) {
    // A folder of its own: the tests run at the same time.
    static COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let dir = temp_dir(&format!(
        "huge-{}",
        COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let path = write_big(&dir, name, bytes);
    let handle = window(cx, true);
    slow_scan(cx);
    before(cx, &path);
    open_file(cx, handle, &path);
    // The file is read on a background thread that the test executor does not wait for.
    for _ in 0..6000 {
        let view = active_view(cx, handle);
        // The name is on the tab while the file is still read: wait for the text.
        if view.read_with(cx, |v, cx| v.document().read(cx).doc().raw_len() > 0) {
            break;
        }
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(10));
    }
    (handle, dir)
}

#[gpui::test]
fn a_file_over_the_threshold_is_read_only_while_it_is_scanned(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    assert!(scanning(cx, &view), "the scan has just started");
    // The first lines can be read at once, and the status bar says how far the scan is.
    let first = view.read_with(cx, |v, cx| {
        v.document().read(cx).doc().line(0).unwrap().unwrap().text
    });
    assert_eq!(first, "line 000000");
    let text = notice(cx, &view).unwrap();
    assert!(text.starts_with("改行位置を調べています "), "{text}");
    assert!(text.contains('%') && text.contains("約"), "{text}");
    // Typing does nothing and says why.
    view.update(cx, |v, cx| v.type_char('x', cx));
    let first = view.read_with(cx, |v, cx| {
        v.document().read(cx).doc().line(0).unwrap().unwrap().text
    });
    assert_eq!(first, "line 000000");
    assert_eq!(
        notice(cx, &view).as_deref(),
        Some("改行位置を調べている間は編集できません")
    );
    // A save is refused too (the file would lose the rest): `partial_reason` is what it tells.
    assert!(view.read_with(cx, |v, cx| v.document().read(cx).is_partial()));
    drop(dir);
}

#[gpui::test]
fn the_document_can_be_edited_when_the_scan_is_done(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    let scanned_at_open = line_count(cx, &view);
    assert!(scanned_at_open < 100_000);
    wait_for_scan(cx, &view);
    assert_eq!(line_count(cx, &view), 100_000);
    assert!(!view.read_with(cx, |v, cx| v.document().read(cx).is_partial()));
    view.update(cx, |v, cx| v.type_char('x', cx));
    let first = view.read_with(cx, |v, cx| {
        v.document().read(cx).doc().line(0).unwrap().unwrap().text
    });
    assert_eq!(first, "xline 000000");
    // The lines far down are the ones of the file.
    let last = view.read_with(cx, |v, cx| {
        v.document()
            .read(cx)
            .doc()
            .line(99_999)
            .unwrap()
            .unwrap()
            .text
    });
    assert_eq!(last, "line 099999");
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn a_jump_to_the_end_waits_for_the_scan(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    run(cx, handle, "F_GOFILEEND");
    assert_eq!(
        caret_line(cx, &view),
        0,
        "the caret stays while the scan runs"
    );
    assert_eq!(
        notice(cx, &view).as_deref(),
        Some("改行位置を調べ終えてから移動します")
    );
    // A line the scan has reached is a jump as usual.
    view.update(cx, |v, cx| v.run_command("jump", &[Arg::Int(2)], cx));
    assert_eq!(caret_line(cx, &view), 1);
    // The PL/SQL count and the count of rows cannot be placed in a start of the file: they wait.
    view.update(cx, |v, cx| {
        v.run_command("jump", &[Arg::Int(3), Arg::Int(2)], cx)
    });
    assert_eq!(caret_line(cx, &view), 1);
    assert_eq!(
        notice(cx, &view).as_deref(),
        Some("改行位置を調べ終えてから移動します")
    );
    // A line it has not reached waits too.
    view.update(cx, |v, cx| v.run_command("jump", &[Arg::Int(70_000)], cx));
    assert_eq!(caret_line(cx, &view), 1);
    wait_for_scan(cx, &view);
    // The last request waited, and ran when the scan was done.
    assert_eq!(caret_line(cx, &view), 69_999);
    run(cx, handle, "F_GOFILEEND");
    assert_eq!(
        caret_line(cx, &view),
        100_000,
        "the end of the text, after the last line"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn a_file_under_the_threshold_opens_as_before(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "small.txt", b"one\ntwo\n");
    let view = active_view(cx, handle);
    assert!(!scanning(cx, &view));
    assert!(!view.read_with(cx, |v, cx| v.document().read(cx).is_partial()));
    view.update(cx, |v, cx| v.type_char('x', cx));
    assert_eq!(
        view.read_with(cx, |v, cx| v
            .document()
            .read(cx)
            .doc()
            .line(0)
            .unwrap()
            .unwrap()
            .text),
        "xone"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn iso_2022_jp_over_the_threshold_is_read_whole(cx: &mut TestAppContext) {
    let line = b"\x1b$B$\"$$$&$($*\x1b(B\n";
    let bytes: Vec<u8> = line
        .iter()
        .copied()
        .cycle()
        .take(line.len() * 70_000)
        .collect();
    assert!(bytes.len() > 1024 * 1024);
    let (handle, dir) = opened(cx, "jis.txt", &bytes);
    let view = active_view(cx, handle);
    assert!(!scanning(cx, &view));
    assert!(!view.read_with(cx, |v, cx| v.document().read(cx).is_partial()));
    assert_eq!(line_count(cx, &view), 70_000);
    assert_eq!(
        view.read_with(cx, |v, cx| v
            .document()
            .read(cx)
            .doc()
            .line(0)
            .unwrap()
            .unwrap()
            .text),
        "あいうえお"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_go_to_line_dialog_waits_for_the_scan_too(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    view.update(cx, |v, cx| v.go_to_line(90_000, cx));
    assert_eq!(caret_line(cx, &view), 0);
    wait_for_scan(cx, &view);
    assert_eq!(caret_line(cx, &view), 89_999);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn another_pane_of_the_document_runs_its_own_waiting_jump(cx: &mut TestAppContext) {
    let (handle, dir) = opened(cx, "big.txt", big_text().as_bytes());
    let view = active_view(cx, handle);
    // A second view of the same document, as a split pane is.
    let document = view.read_with(cx, |v, _| v.document().clone());
    let pane = cx.new(|cx| EditorView::new(document.clone(), cx));
    pane.update(cx, |v, cx| v.go_to_line(95_000, cx));
    assert_eq!(caret_line(cx, &pane), 0);
    wait_for_scan(cx, &view);
    cx.run_until_parked();
    assert_eq!(caret_line(cx, &pane), 94_999);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn a_place_and_bookmarks_past_the_scanned_start_come_back_when_the_scan_is_done(
    cx: &mut TestAppContext,
) {
    use crate::history::{HistoryOptions, HistoryState, view::path_text};
    use matsu_config::history::FileEntry;
    use matsu_core::buffer::Pos;

    let (handle, dir) = opened_with(cx, "big.txt", big_text().as_bytes(), |cx, path| {
        cx.update(|app| {
            app.set_global(HistoryState::in_memory(HistoryOptions::default()));
            let mut entry = FileEntry::new(path_text(path));
            entry.caret_x = 3;
            entry.caret_y = 89_999;
            entry.marks = matsu_config::history::marks::encode_marks([50_000, 95_000]);
            crate::history::add_file(app, entry);
        });
    });
    let view = active_view(cx, handle);
    assert!(scanning(cx, &view));
    let marks = |cx: &mut TestAppContext, view: &Entity<EditorView>| -> Vec<usize> {
        view.read_with(cx, |v, app| {
            v.document().read(app).model().bookmarks().iter().collect()
        })
    };
    // Past what is scanned: nothing is cut to the start yet.
    assert!(marks(cx, &view).is_empty());
    // Closing now would write the history again: with what it had, not the start of the file.
    let kept = view.read_with(cx, |v, app| v.history_entry(app)).unwrap();
    assert_eq!(kept.caret_y, 89_999);
    assert_eq!(
        kept.marks,
        matsu_config::history::marks::encode_marks([50_000, 95_000])
    );
    wait_for_scan(cx, &view);
    assert_eq!(
        view.read_with(cx, |v, _| v.cursor.caret()),
        Pos::new(89_999, 3)
    );
    assert_eq!(marks(cx, &view), [50_000, 95_000]);
    // The history was written with the restored place, not one cut to the start.
    let entry = cx.update(|app| {
        crate::history::current(app).and_then(|h| h.file(&path_text(&dir.join("big.txt"))).cloned())
    });
    assert_eq!(entry.map(|e| e.caret_y), Some(89_999));
    std::fs::remove_dir_all(dir).ok();
}

// ---- a very long line ----

#[gpui::test]
fn a_file_of_one_very_long_line_opens_scrolls_and_takes_the_caret_and_a_selection(
    cx: &mut TestAppContext,
) {
    use matsu_core::buffer::Pos;

    // 3 MiB without a line ending: a very long line (from 1 MiB, ADR-0020 9.10).
    let text = "0123456789abcdef".repeat(3 * 1024 * 1024 / 16);
    let dir = temp_dir("huge-long-line");
    let path = write_big(&dir, "one.txt", text.as_bytes());
    let handle = window(cx, true);
    cx.update(|app| {
        app.set_global(FilePrefs {
            huge_file_size_mb: 1,
            ..FilePrefs::default()
        });
    });
    open_file(cx, handle, &path);
    let view = {
        let mut found = None;
        for _ in 0..6000 {
            let view = active_view(cx, handle);
            if view.read_with(cx, |v, cx| v.document().read(cx).doc().raw_len() > 0) {
                found = Some(view);
                break;
            }
            cx.run_until_parked();
            std::thread::sleep(Duration::from_millis(10));
        }
        found.expect("the file was not read")
    };
    wait_for_scan(cx, &view);
    let caret = |cx: &mut TestAppContext| view.read_with(cx, |v, _| v.cursor.caret());
    // One line of the file, in as many rows as the text has parts of the length of a row.
    assert_eq!(line_count(cx, &view), 1);
    let rows = text.len().div_ceil(matsu_layout::LONG_ROW_BYTES);
    assert_eq!(view.read_with(cx, |v, _| v.layout().row_count()), rows);
    // The rows give the text.
    let shown = view.read_with(cx, |v, cx| {
        let doc = v.document().read(cx).doc();
        v.layout().rows(doc, 100, 5).unwrap()
    });
    assert_eq!(shown.len(), 5);
    assert_eq!(shown[0].text(), &text[100 * 4096..101 * 4096]);
    // The caret goes down by rows and to the end, and comes back.
    for _ in 0..3 {
        run(cx, handle, "F_DOWN");
    }
    assert_eq!(caret(cx), Pos::new(0, 3 * 4096));
    run(cx, handle, "F_GOFILEEND");
    assert_eq!(caret(cx), Pos::new(0, text.len()));
    run(cx, handle, "F_LEFT");
    assert_eq!(caret(cx), Pos::new(0, text.len() - 1));
    run(cx, handle, "F_UP");
    assert_eq!(caret(cx).line, 0);
    assert!(caret(cx).col < text.len() - 4000);
    // A selection from the caret to the end of the file.
    run(cx, handle, "F_GOFILEEND_SEL");
    let selection = view.read_with(cx, |v, _| v.cursor.selection());
    assert_eq!(selection.map(|(_, b)| b), Some(Pos::new(0, text.len())));
    // A char typed in the middle goes there.
    view.update(cx, |v, cx| {
        v.cursor.move_to(Pos::new(0, 1_000_000), false);
        v.type_char('Z', cx);
    });
    let len = view.read_with(cx, |v, cx| v.document().read(cx).doc().line_len(0).unwrap());
    assert_eq!(len, Some(text.len() + 1));
    std::fs::remove_dir_all(dir).ok();
}
