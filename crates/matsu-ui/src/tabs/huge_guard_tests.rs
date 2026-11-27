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

// ---- the IME and the line commands on a very long line (ADR-0020) ----

/// "abc", a very long line (3 MiB) with a `(` at column 1 000 000 and its `)` at 2 000 000, an
/// empty line, and two short lines.
fn lines_around_a_long_one() -> String {
    let mut long = long_text().into_bytes();
    long[1_000_000] = b'(';
    long[2_000_000] = b')';
    format!(
        "abc\n{}\n\nxyz\nlast\n",
        String::from_utf8(long).expect("ASCII")
    )
}

/// Runs `f`, and checks that no decode of it covered more than `limit` bytes.
fn within(what: &str, limit: usize, f: impl FnOnce()) {
    reset_largest_decode();
    f();
    let largest = largest_decode();
    assert!(largest <= limit, "{what} decoded {largest} bytes at once");
    reset_largest_decode();
}

fn go_and_clear(cx: &mut TestAppContext, view: &Entity<EditorView>, pos: Pos) {
    view.update(cx, |v, cx| {
        v.go_to(pos, cx);
        v.cursor.clear_selection();
    });
}

fn select_lines(cx: &mut TestAppContext, view: &Entity<EditorView>) {
    view.update(cx, |v, cx| {
        v.go_to(Pos::new(0, 0), cx);
        v.run_with(cx, |c| c.select_range(Pos::new(0, 0), Pos::new(3, 1)))
            .unwrap();
    });
}

fn line_len_of(cx: &mut TestAppContext, view: &Entity<EditorView>, line: usize) -> usize {
    view.read_with(cx, |v, a| {
        v.document().read(a).doc().line_len(line).unwrap().unwrap()
    })
}

#[gpui::test]
fn the_ime_does_not_decode_a_very_long_line_when_composing_and_committing(cx: &mut TestAppContext) {
    use gpui::EntityInputHandler as _;

    let (handle, view, dir) = open_file_of(cx, "huge-line-ime", &lines_around_a_long_one());
    // insert mode, then overwrite mode (the chars after the composition are looked at)
    for overwrite in [false, true] {
        if overwrite {
            view.update(cx, |v, cx| v.run_command("toggle_overwrite_mode", &[], cx));
        }
        let place = if overwrite { 1_700_000 } else { 1_500_000 };
        go_and_clear(cx, &view, Pos::new(1, place));
        let before = line_len_of(cx, &view, 1);
        within("the IME", 64 * 1024, || {
            handle
                .update(cx, |_, window, cx| {
                    view.update(cx, |v, cx| {
                        let selected = v.selected_text_range(false, window, cx).unwrap();
                        let at = selected.range.start;
                        v.replace_and_mark_text_in_range(None, "に", None, window, cx);
                        v.replace_and_mark_text_in_range(
                            Some(at..at + 1),
                            "にほ",
                            None,
                            window,
                            cx,
                        );
                        let marked = v.marked_text_range(window, cx).unwrap();
                        assert_eq!(marked, at..at + 2);
                        let mut actual = None;
                        let seen = v.text_for_range(0..usize::MAX, &mut actual, window, cx);
                        assert!(seen.unwrap().len() <= 5000);
                        v.replace_text_in_range(Some(marked), "日本", window, cx);
                    })
                })
                .unwrap();
        });
        let after = line_len_of(cx, &view, 1);
        // the text has 6 bytes; overwrite mode also replaces the 2 chars after it
        let expected = if overwrite { before + 4 } else { before + 6 };
        assert_eq!(after, expected, "overwrite {overwrite}");
        let near = view.read_with(cx, |v, a| {
            let doc = v.document().read(a).doc();
            doc.line_slice(1, place - 2, place + 10)
                .unwrap()
                .unwrap()
                .text
        });
        assert!(near.contains("日本"), "{near}");
    }
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn line_commands_do_not_decode_a_very_long_line(cx: &mut TestAppContext) {
    use matsu_commands::command::Arg;

    let (handle, view, dir) = open_file_of(cx, "huge-line-commands", &lines_around_a_long_one());
    let limit = 64 * 1024;
    let caret = |cx: &mut TestAppContext| view.read_with(cx, |v, _| v.cursor.caret());
    // the paragraph commands look at the start of the line only
    go_and_clear(cx, &view, Pos::new(1, 1_500_000));
    within("the next paragraph", limit, || {
        run(cx, handle, "F_GONEXTPARAGRAPH")
    });
    assert_eq!(caret(cx).line, 3, "past the empty line, to the next text");
    within("the previous paragraph", limit, || {
        run(cx, handle, "F_GOPREVPARAGRAPH")
    });
    assert!(caret(cx).line < 3, "{:?}", caret(cx));
    // the bracket at the caret and its pair, 1 000 000 bytes away
    go_and_clear(cx, &view, Pos::new(1, 1_000_000));
    within("the bracket pair", 2 * limit, || {
        run(cx, handle, "F_BRACKETPAIR")
    });
    assert_eq!(caret(cx), Pos::new(1, 2_000_000));
    // the row of the caret is trimmed and duplicated: a row, not the line
    go_and_clear(cx, &view, Pos::new(1, 1_500_000));
    let before = line_len_of(cx, &view, 1);
    for function in ["F_LTRIM", "F_RTRIM"] {
        within(function, limit, || run(cx, handle, function));
    }
    assert_eq!(line_len_of(cx, &view, 1), before, "nothing to trim");
    within("the duplicate of a row", limit, || {
        run(cx, handle, "F_DUPLICATELINE")
    });
    // a row of the long line is copied into the line (it has no line ending of its own)
    let grown = line_len_of(cx, &view, 1) - before;
    assert!(grown > 0 && grown <= 4096, "the line grew by {grown} bytes");
    // the macro MoveCursor counts in units and does not decode the line
    within("MoveCursor", limit, || {
        view.update(cx, |v, cx| {
            v.run_command(
                "movecursor",
                &[Arg::Int(2), Arg::Int(1_234_568), Arg::Int(0)],
                cx,
            )
        });
    });
    let at = caret(cx);
    assert_eq!(at.line, 1);
    assert!(at.col.abs_diff(1_234_567) <= 4096, "{at:?}");
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn indent_and_unindent_over_a_very_long_line_read_only_its_start(cx: &mut TestAppContext) {
    let (handle, view, dir) = open_file_of(cx, "huge-line-indent", &lines_around_a_long_one());
    let start = |cx: &mut TestAppContext| {
        view.read_with(cx, |v, a| {
            let doc = v.document().read(a).doc();
            doc.line_slice(1, 0, 3).unwrap().unwrap().text
        })
    };
    select_lines(cx, &view);
    within("indent", 64 * 1024, || run(cx, handle, "F_INDENT_TAB"));
    assert_eq!(start(cx), "\t01");
    select_lines(cx, &view);
    within("unindent", 64 * 1024, || run(cx, handle, "F_UNINDENT_TAB"));
    assert_eq!(start(cx), "012");
    std::fs::remove_dir_all(dir).ok();
}

#[gpui::test]
fn commands_that_need_the_whole_text_refuse_over_the_limit_and_change_nothing(
    cx: &mut TestAppContext,
) {
    let (handle, view, dir) = open_file_of(cx, "huge-line-refuse", &lines_around_a_long_one());
    let raw =
        |cx: &mut TestAppContext| view.read_with(cx, |v, a| v.document().read(a).doc().raw_len());
    let size = raw(cx);
    // a limit of 1 MB: the line of 3 MiB is over it
    with_copy_limit(1024 * 1024, || {
        for (function, said) in [
            ("F_SORT_ASC", "並べ替え"),
            ("F_SORT_DESC", "並べ替え"),
            ("F_MERGE", "マージ"),
            ("F_LTRIM", "トリム"),
            ("F_RTRIM", "トリム"),
            ("F_TOUPPER", "変換"),
            ("F_TABTOSPACE", "変換"),
        ] {
            select_lines(cx, &view);
            within(function, 64 * 1024, || run(cx, handle, function));
            let text = shown_box(cx, handle).unwrap_or_else(|| panic!("{function}: no message"));
            assert!(
                text.contains(said) && text.contains("上限は 1 MB"),
                "{function}: {text}"
            );
            assert_eq!(raw(cx), size, "{function} changed the text");
        }
    });
    // under the limit a sort of the same lines goes on (the text is decoded as a whole)
    select_lines(cx, &view);
    run(cx, handle, "F_SORT_ASC");
    assert!(shown_box(cx, handle).is_none());
    assert_eq!(raw(cx), size);
    std::fs::remove_dir_all(dir).ok();
}

/// Not a test: the time and the peak memory of a sort over a very long line, for the PR. Run it
/// alone: `MATSU_LINE_MB=100 cargo test -p matsu-ui --lib measure_the_sort_of_a_long_line --
/// --ignored --nocapture` (the peak is read from `/proc/self/status`, so on Linux only).
#[gpui::test]
#[ignore = "a measurement for the PR, not a test"]
fn measure_the_sort_of_a_long_line(cx: &mut TestAppContext) {
    let mb: usize = std::env::var("MATSU_LINE_MB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    let peak = || {
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("VmHWM:"))
                    .map(str::to_owned)
            })
            .unwrap_or_default()
    };
    let text = format!(
        "b\n{}\na\n",
        "0123456789abcdef".repeat(mb * 1024 * 1024 / 16)
    );
    // like `open_file_of_with`, with more patience: the scan of a big line takes a while in a
    // debug build
    let dir = temp_dir("huge-line-measure");
    let path = dir.join("one.txt");
    std::fs::write(&path, &text).unwrap();
    drop(text);
    let handle = window(cx, true);
    cx.update(|app| {
        app.set_global(FilePrefs {
            huge_file_size_mb: 1,
            ..FilePrefs::default()
        });
    });
    open_file(cx, handle, &path);
    let mut view = active_view(cx, handle);
    for _ in 0..60_000 {
        view = active_view(cx, handle);
        let scanned = view.read_with(cx, |v, a| {
            let shared = v.document().read(a);
            shared.doc().raw_len() > 0 && !shared.is_scanning()
        });
        if scanned {
            break;
        }
        cx.executor().advance_clock(Duration::from_millis(150));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(10));
    }
    eprintln!("after the open: {}", peak());
    view.update(cx, |v, cx| {
        v.go_to(Pos::new(0, 0), cx);
        v.run_with(cx, |c| c.select_range(Pos::new(0, 0), Pos::new(3, 0)))
            .unwrap();
    });
    let started = std::time::Instant::now();
    run(cx, handle, "F_SORT_ASC");
    eprintln!("sort of {mb} MiB: {:?}, {}", started.elapsed(), peak());
    std::fs::remove_dir_all(dir).ok();
}
