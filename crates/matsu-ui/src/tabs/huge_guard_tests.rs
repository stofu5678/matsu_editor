//! The paths that took a very long line (1 MiB or more of a file read by ranges, ADR-0020) whole
//! are checked with the largest range that one decode covered ([`largest_decode`]): `$x`, the box
//! selection and what the copy of one does, and the places where a copy is refused. A file that is
//! read by ranges wraps after its scan, as any document does.

use std::time::Duration;

use gpui::{AnyWindowHandle, Entity, TestAppContext};
use matsu_commands::clipboard::with_copy_limit;
use matsu_core::buffer::{Pos, largest_decode, reset_largest_decode};

use super::file_tests::{open_file, temp_dir};
use super::huge_tests::wait_for_scan;
use super::tests::{active_view, run, window};
use crate::file_ops::prefs::FilePrefs;
use crate::view::EditorView;

/// Opens a file with `text` (3 MiB of one line is 2 MiB over the line of a file that is read by
/// ranges here) and waits for the scan. Gives the window, the view and the folder.
fn open_file_of(
    cx: &mut TestAppContext,
    name: &str,
    text: &str,
) -> (AnyWindowHandle, Entity<EditorView>, std::path::PathBuf) {
    open_file_of_with(cx, name, text, 1)
}

/// Like `open_file_of`, with the size from which a file is read by ranges given (MB).
fn open_file_of_with(
    cx: &mut TestAppContext,
    name: &str,
    text: &str,
    huge_file_size_mb: i32,
) -> (AnyWindowHandle, Entity<EditorView>, std::path::PathBuf) {
    let dir = temp_dir(name);
    let path = dir.join("one.txt");
    std::fs::write(&path, text).unwrap();
    let handle = window(cx, true);
    cx.update(|app| {
        app.set_global(FilePrefs {
            huge_file_size_mb,
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
    cx.run_until_parked();
    (handle, view, dir)
}

fn long_text() -> String {
    "0123456789abcdef".repeat(3 * 1024 * 1024 / 16)
}

fn text_of(cx: &mut TestAppContext, view: &Entity<EditorView>, from: Pos, to: Pos) -> String {
    view.read_with(cx, |v, app| {
        v.document().read(app).doc().text_between(from, to).unwrap()
    })
}

fn clipboard_text(cx: &mut TestAppContext) -> Option<String> {
    cx.read_from_clipboard().and_then(|item| item.text())
}

/// The text of the message box that is on the screen, which is closed.
fn shown_box(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Option<String> {
    let vcx = super::tests::visual(cx, handle);
    let (_, detail) = vcx.pending_prompt()?;
    vcx.simulate_prompt_answer("OK");
    vcx.run_until_parked();
    Some(detail)
}

#[gpui::test]
fn the_box_and_the_seed_of_a_search_do_not_decode_a_very_long_line(cx: &mut TestAppContext) {
    let (handle, view, dir) = open_file_of(cx, "huge-guard-box", &long_text());
    let at_most = |what: &str, limit: usize| {
        let largest = largest_decode();
        assert!(largest <= limit, "{what} decoded {largest} bytes at once");
        reset_largest_decode();
    };
    view.update(cx, |v, cx| v.go_to(Pos::new(0, 1_500_000), cx));
    reset_largest_decode();
    // the word at the caret: the part of the line around it
    let seed = view.read_with(cx, |v, app| v.search_seed(app));
    let seed = seed.expect("a word");
    assert!(
        seed.starts_with("0123") || seed.len() > 1000,
        "{}",
        seed.len()
    );
    assert!(seed.len() <= 8 * 1024 + 4, "{}", seed.len());
    at_most("the seed of a search", 16 * 1024);
    // a box over rows of the line: copied, pasted back as a box, and copied as rows
    view.update(cx, |v, cx| v.run_command("begin_box", &[], cx));
    for _ in 0..3 {
        view.update(cx, |v, cx| v.run_command("cursor_right_box", &[], cx));
    }
    for _ in 0..5 {
        view.update(cx, |v, cx| v.run_command("cursor_down_box", &[], cx));
    }
    at_most("the moves of a box", 64 * 1024);
    run(cx, handle, "F_COPY");
    at_most("the copy of a box", 64 * 1024);
    assert_eq!(clipboard_text(cx).map(|t| t.len()), Some(6 * 3 + 6 * 2));
    run(cx, handle, "F_PASTE");
    at_most("the paste of a box", 64 * 1024);
    run(cx, handle, "F_COPYLINES");
    at_most("the copy of rows", 64 * 1024);
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_column_x_of_a_very_long_line_is_not_read_from_its_text(cx: &mut TestAppContext) {
    let (_, view, dir) = open_file_of(cx, "huge-guard-x", &long_text());
    view.update(cx, |v, cx| v.go_to(Pos::new(0, 1_500_000), cx));
    reset_largest_decode();
    let x = view.read_with(cx, |v, app| v.header_context_for(Some("$x"), app).caret_x);
    // the bytes stand for the UTF-16 units here, and the text is ASCII
    assert_eq!(x, 1_500_001);
    assert!(
        largest_decode() <= 64 * 1024,
        "decoded {} bytes at once",
        largest_decode()
    );
    // a short line is counted in UTF-16 units from its text, as before
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn a_copy_over_the_limit_says_so_and_a_cut_deletes_nothing(cx: &mut TestAppContext) {
    let (handle, view, dir) = open_file_of(cx, "huge-guard-copy", &long_text());
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("old".to_owned()));
    // 100 000 bytes of the line, with a limit of 50 000
    view.update(cx, |v, cx| v.go_to(Pos::new(0, 1_000_000), cx));
    view.update(cx, |v, cx| {
        v.run_with(cx, |c| {
            c.select_range(Pos::new(0, 1_000_000), Pos::new(0, 1_100_000))
        })
        .unwrap();
    });
    let before = text_of(cx, &view, Pos::new(0, 999_990), Pos::new(0, 1_100_010));
    with_copy_limit(50_000, || {
        for (function, said) in [
            ("F_COPY", "コピーできません"),
            ("F_CUT", "切り取りできません"),
            ("F_COPY_COLOR_HTML", "コピーできません"),
        ] {
            run(cx, handle, function);
            assert_eq!(clipboard_text(cx).as_deref(), Some("old"), "{function}");
            let text = shown_box(cx, handle).unwrap_or_else(|| panic!("{function}: no message"));
            assert!(text.contains(said), "{function}: {text}");
            assert!(text.contains("上限は 1 MB"), "{function}: {text}");
        }
    });
    let after = text_of(cx, &view, Pos::new(0, 999_990), Pos::new(0, 1_100_010));
    assert_eq!(after, before, "nothing was deleted");
    assert!(!view.read_with(cx, |v, app| v.document().read(app).model().is_modified()));
    // under the limit the same selection is copied
    run(cx, handle, "F_COPY");
    assert_eq!(clipboard_text(cx).map(|t| t.len()), Some(100_000));
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn a_file_that_was_read_by_ranges_wraps_after_its_scan(cx: &mut TestAppContext) {
    // 36 000 lines of 107 chars, a wrap at 40: every line is 3 rows
    let text: String = (0..36_000)
        .map(|i| format!("{i:06} {}\r\n", "x".repeat(100)))
        .collect();
    assert!(text.len() > 3 * 1024 * 1024);
    let (_, view, dir) = open_file_of(cx, "huge-guard-wrap", &text);
    view.update(cx, |v, cx| {
        v.prefs.wrap_method = crate::settings::view_settings::WrapMethod::SettingWidth;
        v.prefs.wrap_columns = 40;
        v.relayout_in(cx);
        let doc = v.document().read(cx).doc().clone();
        assert!(doc.is_ranged() && !doc.is_read_only(), "the scan is over");
        v.layout.layout_all(&doc).unwrap();
    });
    let (lines, rows) = view.read_with(cx, |v, app| {
        (
            v.document().read(app).doc().line_count(),
            v.layout().row_count(),
        )
    });
    assert_eq!(lines, 36_000);
    assert_eq!(rows, 3 * lines + 1, "{rows} rows for {lines} lines");
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn a_narrow_box_over_a_lot_of_text_is_copied_as_html_by_the_text_of_its_rows(
    cx: &mut TestAppContext,
) {
    let (handle, view, dir) = open_file_of(cx, "huge-guard-box-html", &long_text());
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("old".to_owned()));
    view.update(cx, |v, cx| v.go_to(Pos::new(0, 1_000_000), cx));
    view.update(cx, |v, cx| v.run_command("begin_box", &[], cx));
    for _ in 0..3 {
        view.update(cx, |v, cx| v.run_command("cursor_right_box", &[], cx));
    }
    for _ in 0..20 {
        view.update(cx, |v, cx| v.run_command("cursor_down_box", &[], cx));
    }
    // 21 rows of 4096 bytes lie between the corners, over the limit, but the box has 63 bytes
    with_copy_limit(50_000, || {
        run(cx, handle, "F_COPY_COLOR_HTML");
        assert!(shown_box(cx, handle).is_none());
        let html = clipboard_text(cx).expect("the clipboard has the HTML");
        assert!(html.starts_with("<pre"), "{html}");
        // the plain copy of the same box is not refused either
        cx.write_to_clipboard(gpui::ClipboardItem::new_string("old".to_owned()));
        run(cx, handle, "F_COPY");
        assert!(shown_box(cx, handle).is_none());
        assert_ne!(clipboard_text(cx).as_deref(), Some("old"));
    });
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn the_html_copy_of_a_very_long_line_of_a_document_read_whole_is_refused(cx: &mut TestAppContext) {
    // 1.2 MiB in one line, under the size of a file that is read by ranges: a whole document
    let text = "0123456789abcdef".repeat(1_200_000 / 16 + 1);
    let (handle, view, dir) = open_file_of_with(cx, "huge-guard-html-line", &text, 100);
    assert!(!view.read_with(cx, |v, app| v.document().read(app).doc().is_ranged()));
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("old".to_owned()));
    run(cx, handle, "F_SELECTALL");
    run(cx, handle, "F_COPY_COLOR_HTML");
    assert_eq!(clipboard_text(cx).as_deref(), Some("old"));
    let text = shown_box(cx, handle).expect("a message");
    assert!(text.contains("長すぎる行"), "{text}");
    assert!(text.contains("上限は 1 MB"), "{text}");
    // the plain copy is not refused
    run(cx, handle, "F_COPY");
    assert_eq!(
        clipboard_text(cx).map(|t| t.len()),
        Some(1_200_000 / 16 * 16 + 16)
    );
    std::fs::remove_dir_all(dir).ok();
}

// ---- the measurement of the PR ----

/// The peak resident memory of this process so far, in MB (Linux), or 0.
fn peak_rss_mb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find_map(|l| l.strip_prefix("VmHWM:"))
                .and_then(|l| l.split_whitespace().next()?.parse::<u64>().ok())
        })
        .map_or(0, |kb| kb / 1024)
}

/// Not a test: the peak memory and the time of the paths of this PR over a file with a very long
/// line, for the PR. Run it in a release build, one step at a time (the peak memory is of the
/// whole process):
/// `MATSU_GUARD_FILE=/path/one-line-1gib.txt MATSU_GUARD_STEP=grep_after cargo test -p matsu-ui
/// --release --lib measure_the_long_line_paths -- --ignored --nocapture`.
/// (The Grep is measured by `measure_a_big_file` in `grep/huge_tests.rs`.)
/// Steps: `x_before` and `x_after` (`$x` in the middle of the line), `compare_before` and
/// `compare_after` (the comparison of the file with itself, with a difference at the end of the
/// line; "before" is the old way, a line decoded whole on each side).
#[test]
#[ignore = "measures the file in MATSU_GUARD_FILE; run in a release build"]
fn measure_the_long_line_paths() {
    let Ok(path) = std::env::var("MATSU_GUARD_FILE") else {
        return;
    };
    let step = std::env::var("MATSU_GUARD_STEP").unwrap_or_default();
    let path = std::path::PathBuf::from(path);
    let mb = std::fs::metadata(&path).unwrap().len() / (1024 * 1024);
    println!(
        "file {mb} MB, step {step}, memory before {} MB",
        peak_rss_mb()
    );
    let opening = std::time::Instant::now();
    // the document of the file, the way the scan of the app makes it
    let options = matsu_core::file::OpenOptions {
        huge_file_len: Some(1),
        ..Default::default()
    };
    let mut opened = matsu_core::file::open_file(&path, &options).unwrap();
    let doc = match opened.take_scan() {
        Some(job) => job.finish().unwrap(),
        None => opened.document.clone(),
    };
    println!(
        "open and scan: {:.2} s, {} lines, peak memory {} MB",
        opening.elapsed().as_secs_f64(),
        doc.line_count(),
        peak_rss_mb()
    );
    let started = std::time::Instant::now();
    let report = |what: &str| {
        println!(
            "{what}: {:.2} s, peak memory {} MB",
            started.elapsed().as_secs_f64(),
            peak_rss_mb()
        );
    };
    let middle = doc.line_raw_len(0).unwrap().unwrap() / 2;
    match step.as_str() {
        "x_before" => {
            // the old code of `$x`: the line decoded, and its units counted
            let line = doc.line(0).unwrap().unwrap();
            let upto = line.text.get(..middle).unwrap_or(&line.text);
            report(&format!("x_before: {}", upto.encode_utf16().count() + 1));
        }
        "x_after" => report(&format!("x_after: {}", doc.utf16_col(0, middle) + 1)),
        "compare_before" => {
            // the old comparison: both lines decoded whole, then compared char by char
            let (a, b) = (doc.line(0).unwrap().unwrap(), doc.line(0).unwrap().unwrap());
            report(&format!("compare_before: {}", a.text == b.text));
        }
        "compare_after" => {
            let found =
                matsu_core::diff::first_difference(&doc, Pos::new(0, 0), &doc, Pos::new(0, 0))
                    .unwrap();
            report(&format!("compare_after: {found:?}"));
        }
        other => panic!("no such step: {other}"),
    }
}
