//! What a key costs inside a very long line of a normal document is bounded (#614, and the fix
//! that followed it).
//!
//! `long_line_measure.rs` measures; this asserts. A line of 1 MiB or more of a document that is
//! read whole is laid out in rows of fixed size and read only where the screen and the caret are,
//! so the bytes that a key makes the layer below lay out and decode must stay under a fixed bound,
//! and must not grow when the line grows four times. There is no timing in it.
//!
//! The counters exist only with the `measure` feature, which `cargo test --workspace` does not
//! turn on, so the module is compiled only with it. CI runs it in one job:
//! `cargo test -p matsu-ui --features measure --lib long_line_bounds`.

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_core::buffer::Pos;
use matsu_core::file::{OpenOptions, open_file};
use matsu_core::measure::{self, Entry};

use super::EditorView;
use super::long_line_measure::{MIB, paint, put_caret, text_bytes};
use crate::document::SharedDocument;

/// The bytes that the layers below read, decode or lay out, from the counters: the text decoded
/// into a `String`, the text decoded only to count it, the text walked to find a column, and the
/// text laid out.
fn work(spans: &[Entry]) -> u64 {
    spans
        .iter()
        .filter(|e| {
            matches!(
                e.name,
                "core.decode_range"
                    | "core.decode_for_len"
                    | "core.raw_for_col"
                    | "layout.layout_line"
            )
        })
        .map(|e| e.bytes)
        .sum()
}

/// What the work of one key may be, whatever the length of the line: the rows of a screen are
/// 4096 bytes each (a few dozen of them), the rest is a few characters.
const KEY_BOUND: u64 = 2 * MIB as u64;

/// A window over a file of a line of `len` bytes (line 2, as `long_line_measure` makes it), the
/// caret at column 6 of it, everything laid out, and the caches as a user has them after a key.
fn open_view(
    cx: &mut TestAppContext,
    len: usize,
) -> (
    Entity<EditorView>,
    &mut VisualTestContext,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("long-line.txt");
    std::fs::write(&path, text_bytes(len)).unwrap();
    let file = open_file(&path, &OpenOptions::default()).unwrap();
    assert!(!file.is_ranged(), "the file is read whole");
    let document = cx.new(|_| SharedDocument::from_text_file(file));
    let (view, cx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(document, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    cx.update(|window, _| window.activate_window());
    cx.simulate_resize(gpui::size(gpui::px(900.), gpui::px(420.)));
    for _ in 0..200 {
        paint(cx);
        cx.run_until_parked();
        if view.read_with(cx, |v, _| {
            v.layout().is_complete() && v.text_width.is_complete()
        }) {
            break;
        }
    }
    put_caret(&view, cx, Pos::new(1, 6));
    paint(cx);
    cx.run_until_parked();
    // one key of each kind, as the measurement warms its caches
    cx.simulate_keystrokes("right");
    cx.simulate_input("x");
    cx.simulate_keystrokes("backspace left");
    (view, cx, dir)
}

/// The work of the three keys, in the order Right, typing `x`, Backspace: the key itself with the
/// paint GPUI runs after it, another paint, and the status bar, the ruler and the minimap, which
/// the window asks for after a key.
fn keys(cx: &mut TestAppContext, len: usize) -> [u64; 3] {
    let (view, cx, _dir) = open_view(cx, len);
    let mut out = [0; 3];
    for (i, key) in ["right", "x", "backspace"].into_iter().enumerate() {
        measure::reset();
        if key == "x" {
            cx.simulate_input("x");
        } else {
            cx.simulate_keystrokes(key);
        }
        paint(cx);
        view.read_with(cx, |v, cx| {
            let _ = v.status(cx);
            let _ = v.caret_x(cx);
            let _ = v.h_right_edge();
            let first = (v.scroll_y() / v.metrics().line_height).floor() as usize;
            let _ = v.minimap_rows(first, 120, 120, cx);
        });
        let spans = measure::take();
        if std::env::var_os("MATSU_BOUNDS_SHOW").is_some() {
            for e in spans.iter().filter(|e| e.bytes > 0) {
                eprintln!(
                    "{key} {} {}:{} x{} {} bytes",
                    e.name, e.file, e.line, e.calls, e.bytes
                );
            }
        }
        out[i] = work(&spans);
    }
    out
}

#[gpui::test]
fn a_key_in_a_very_long_line_reads_a_bounded_amount(cx: &mut TestAppContext) {
    let small = keys(cx, 4 * MIB);
    let big = keys(cx, 16 * MIB);
    for (i, name) in ["Right", "typing", "Backspace"].into_iter().enumerate() {
        assert!(
            small[i] < KEY_BOUND && big[i] < KEY_BOUND,
            "{name}: {} bytes in a line of 4 MiB, {} in one of 16 MiB (bound {KEY_BOUND})",
            small[i],
            big[i]
        );
        // four times the line, the same work (a little more for a longer index to search)
        assert!(
            big[i] <= small[i] + small[i] / 8 + 4096,
            "{name}: {} bytes in a line of 4 MiB, {} in one of 16 MiB",
            small[i],
            big[i]
        );
    }
}

/// What each of the five places that read a few bytes of the line of the caret costs there, at the
/// start of the line and in its middle: the width of the caret in overwrite mode (`element.rs`),
/// the code of the character at the caret (`status_info.rs`), the bracket pair (`bracket_marks.rs`),
/// whether an edit joins line endings (`context/mod.rs`) and a move by a character (`cursor.rs`).
fn places(cx: &mut TestAppContext, len: usize) -> Vec<(&'static str, u64)> {
    let (view, cx, _dir) = open_view(cx, len);
    let mut out = Vec::new();
    for col in [6, len / 2] {
        put_caret(&view, cx, Pos::new(1, col));
        paint(cx);
        view.read_with(cx, |v, cx| {
            let doc = v.document().read(cx).doc();
            let caret = v.cursor().caret();
            let mut one = |name: &'static str, f: &dyn Fn()| {
                measure::reset();
                f();
                out.push((name, work(&measure::take())));
            };
            one("overwrite_caret_width", &|| {
                let _ = super::element::overwrite_caret_width(doc, v.layout(), caret);
            });
            one("caret_char_code", &|| {
                let _ = v.caret_char_code(crate::status_options::StatusbarOptions::default(), cx);
            });
            one("bracket_marks", &|| {
                let screen = super::bracket_marks::Screen {
                    first_row: 0,
                    last_row: 40,
                    first_x: matsu_layout::X::ZERO,
                    end_x: v.layout().config().width.to_x(120),
                };
                let _ = super::bracket_marks::marks(doc, v.layout(), caret, screen);
            });
            one("char_right", &|| {
                let _ = matsu_core::cursor::char_right(doc, caret);
            });
            one("char_left", &|| {
                let _ = matsu_core::cursor::char_left(doc, caret);
            });
        });
        // The edit asks whether it joins line endings: an insert, then a delete of what it put.
        measure::reset();
        view.update(cx, |v, cx| {
            v.run_with(cx, |c| c.replace(Pos::new(1, col), Pos::new(1, col), "x"))
                .unwrap();
            v.run_with(cx, |c| {
                c.replace(Pos::new(1, col), Pos::new(1, col + 1), "")
            })
            .unwrap();
        });
        out.push(("joined_by (an edit)", work(&measure::take())));
    }
    out
}

/// A few bytes are what the five places need: in a line of 4 MiB and in one of 16 MiB they read
/// the same few KiB, and in a short line no more than the line.
#[gpui::test]
fn the_places_that_read_a_few_bytes_of_the_line_read_a_bounded_amount(cx: &mut TestAppContext) {
    let small = places(cx, 4 * MIB);
    let big = places(cx, 16 * MIB);
    for ((name, a), (_, b)) in small.iter().zip(&big) {
        // An edit also updates the layout, which reads the rows around it: about a screen.
        let limit = if name.starts_with("joined_by") {
            KEY_BOUND
        } else {
            64 * 1024
        };
        assert!(*a < limit && *b < limit, "{name}: {a} and {b} bytes");
        assert!(b <= a, "{name}: {a} bytes in 4 MiB, {b} in 16 MiB");
    }
    // a short line (the file is about a hundred bytes)
    for (name, bytes) in places(cx, 64) {
        assert!(bytes <= 8192, "{name}: {bytes} bytes in a line of 64 bytes");
    }
}
