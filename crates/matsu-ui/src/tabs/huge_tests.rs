//! Tests of a file opened by ranges (ADR-0020): the file is scanned in the background, the document
//! is read-only meanwhile, and the status bar says how far the scan is.

use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::{AnyWindowHandle, Entity, TestAppContext};
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
/// every 150 ms), so that the tests can look at the document while it runs.
fn slow_scan(cx: &mut TestAppContext) {
    cx.update(|app| {
        app.set_global(FilePrefs {
            huge_file_size_mb: 1,
            huge_scan: ScanOptions {
                block_size: 64 * 1024,
                window_blocks: 1,
                step_delay: Some(Duration::from_millis(150)),
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
    for _ in 0..600 {
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
    let dir = temp_dir("huge");
    let path = write_big(&dir, name, bytes);
    let handle = window(cx, true);
    slow_scan(cx);
    open_file(cx, handle, &path);
    // The file is read on a background thread that the test executor does not wait for.
    for _ in 0..500 {
        let view = active_view(cx, handle);
        if view.read_with(cx, |v, cx| v.document().read(cx).path().is_some()) {
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
