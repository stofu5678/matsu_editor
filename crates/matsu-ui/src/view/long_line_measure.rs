//! A measurement, not a test: what one key costs inside a very long line of a normal document
//! (a file under `nHugeFileSizeMB`, so one that is read whole, not by ranges).
//!
//! The results and how to read them are in `docs/measurements/long-line-normal.md`. The test is
//! `#[ignore]`: CI does not run it, and it asserts nothing about time. Run it with
//!
//! ```sh
//! cargo test --release -p matsu-ui --features measure --lib -- --ignored --nocapture --test-threads=1 long_line_measure_
//! ```
//!
//! (`--test-threads=1`: the resident memory is that of the whole process.) A name such as
//! `long_line_measure_16_mib` runs one length.
//!
//! Without `--features measure` it still runs, but the counters of the layers below (decoded
//! bytes, laid-out bytes) are not compiled in and the tables show only the time.
//! `MATSU_LONG_LINE_ROUNDS` (default 3) is how many times a step is repeated; the best time is
//! shown, and `MATSU_LONG_LINE_MIN_US` (default 100) is the least time in microseconds that a
//! layer must have taken to be listed. Nothing here changes what the product does.

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_core::buffer::{Document, Pos};
use matsu_core::file::{OpenOptions, open_file};
use matsu_core::measure::{self, Entry};

use super::EditorView;
use crate::document::SharedDocument;

pub(super) const MIB: usize = 1024 * 1024;

fn status_kib(key: &str) -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix(key)?.split_whitespace().next()?.parse().ok())
        })
        .unwrap_or(0)
}

/// Starts the peak of the resident memory again (Linux; elsewhere the peak stays as it was).
fn reset_peak() {
    let _ = std::fs::write("/proc/self/clear_refs", "5");
}

fn rss_mib() -> f64 {
    status_kib("VmRSS:") as f64 / 1024.0
}

fn peak_mib() -> f64 {
    status_kib("VmHWM:") as f64 / 1024.0
}

/// A document of three parts: a short line, one line of `len` bytes of ASCII, short lines.
pub(super) fn text_bytes(len: usize) -> Vec<u8> {
    let unit = b"0123456789abcdef";
    let mut bytes = Vec::with_capacity(len + 64);
    bytes.extend_from_slice(b"first line\n");
    while bytes.len() < 11 + len {
        bytes.extend_from_slice(unit);
    }
    bytes.truncate(11 + len);
    bytes.extend_from_slice(b"\nthird line\nfourth line\nfifth line\n");
    bytes
}

/// What one part of a step cost.
#[derive(Clone)]
struct Phase {
    /// A short name for the table of the layers below.
    id: &'static str,
    name: &'static str,
    wall: Duration,
    /// How far the peak of the resident memory rose above what was resident before, in MiB (Linux).
    peak_rise: f64,
    spans: Vec<Entry>,
}

fn timed(id: &'static str, name: &'static str, f: impl FnOnce()) -> Phase {
    measure::reset();
    reset_peak();
    let before = rss_mib();
    let start = Instant::now();
    f();
    let wall = start.elapsed();
    let peak_rise = (peak_mib() - before).max(0.0);
    Phase {
        id,
        name,
        wall,
        peak_rise,
        spans: measure::take(),
    }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn mib(bytes: u64) -> f64 {
    bytes as f64 / MIB as f64
}

pub(super) fn paint(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
}

pub(super) fn put_caret(view: &Entity<EditorView>, cx: &mut VisualTestContext, pos: Pos) {
    view.update(cx, |v, cx| {
        v.run_with(cx, |c| c.move_caret(pos, false)).unwrap();
    });
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Step {
    Right,
    Type,
    Backspace,
}

impl Step {
    const ALL: [Step; 3] = [Step::Right, Step::Type, Step::Backspace];

    fn label(self) -> &'static str {
        match self {
            Step::Right => "Right (one column)",
            Step::Type => "typing `x`",
            Step::Backspace => "Backspace",
        }
    }
}

/// The parts of one step: what the app runs for the key, then for the next paint, then what the
/// panes around the text ask the view, which the test window does not have.
fn run_step(
    view: &Entity<EditorView>,
    cx: &mut VisualTestContext,
    step: Step,
    search_marks: bool,
) -> Vec<Phase> {
    let mut phases = Vec::new();
    phases.push(timed(
        "key",
        "key (command, layout update, caret into view, and the paint GPUI runs after it)",
        || match step {
            Step::Right => cx.simulate_keystrokes("right"),
            Step::Type => cx.simulate_input("x"),
            Step::Backspace => cx.simulate_keystrokes("backspace"),
        },
    ));
    phases.push(timed(
        "paint",
        "a second paint (nothing changed since the key)",
        || paint(cx),
    ));
    // What the status bar shows: line, column and the code of the character.
    phases.push(timed(
        "status",
        "status bar (column, character code, selection)",
        || {
            view.read_with(cx, |v, cx| {
                let _ = v.status(cx);
            });
        },
    ));
    // The ruler's caret mark and the horizontal scroll bar's thumb: the caret's x and the range.
    phases.push(timed(
        "ruler",
        "caret x + h-scroll range (ruler, scroll bar)",
        || {
            view.read_with(cx, |v, cx| {
                let _ = v.caret_x(cx);
                let _ = v.h_right_edge();
            });
        },
    ));
    // The minimap draws the colors of the rows around the view in its own pane.
    phases.push(timed(
        "minimap",
        "minimap rows (when the minimap is on)",
        || {
            view.read_with(cx, |v, cx| {
                let first = (v.scroll_y() / v.metrics().line_height).floor() as usize;
                let _ = v.minimap_rows(first, 120, 120, cx);
            });
        },
    ));
    // The bracket pair of the caret.
    phases.push(timed("bracket", "bracket pair at the caret", || {
        view.read_with(cx, |v, cx| {
            let doc = v.document().read(cx).doc();
            let first = (v.scroll_y() / v.metrics().line_height).floor() as usize;
            let screen = super::bracket_marks::Screen {
                first_row: first,
                last_row: first + 40,
                first_x: matsu_layout::X::ZERO,
                end_x: v.layout().config().width.to_x(120),
            };
            let _ = super::bracket_marks::marks(doc, v.layout(), v.cursor().caret(), screen);
        });
    }));
    // The marks of changed lines and of the diff, as the paint asks for each visible line.
    phases.push(timed("marks", "marks of changed lines", || {
        view.read_with(cx, |v, cx| {
            let shared = v.document().read(cx);
            let model = shared.model();
            for line in 0..6 {
                let _ = model.is_line_modified(line);
                let _ = model.bookmarks().contains(line);
                let _ = model.diff_marks().get(line);
            }
        });
    }));
    // Whatever a timer does after an edit: a few seconds pass, and the tasks of the test run.
    phases.push(timed("timers", "timers (clock +10 s, tasks run)", || {
        cx.executor().advance_clock(Duration::from_secs(10));
        cx.run_until_parked();
    }));
    if search_marks {
        // The next paint again, which is the one that shows the search marks of the new text.
        phases.push(timed("paint2", "a second paint (search marks on)", || {
            paint(cx)
        }));
    }
    phases
}

/// The best of the rounds, a part at a time.
fn best(rounds: &[Vec<Phase>]) -> Vec<Phase> {
    let n = rounds[0].len();
    (0..n)
        .map(|i| {
            rounds
                .iter()
                .map(|r| &r[i])
                .min_by_key(|p| p.wall)
                .unwrap()
                .clone()
        })
        .collect()
}

fn print_phases(out: &mut String, phases: &[Phase]) {
    let total: Duration = phases.iter().map(|p| p.wall).sum();
    let peak = phases.iter().map(|p| p.peak_rise).fold(0.0, f64::max);
    let _ = writeln!(
        out,
        "| part | time (ms) | peak resident rise (MiB) |\n|---|---:|---:|"
    );
    for p in phases {
        let _ = writeln!(
            out,
            "| {} | {:.2} | {:.0} |",
            p.name,
            ms(p.wall),
            p.peak_rise
        );
    }
    let _ = writeln!(
        out,
        "| **total** | **{:.2}** | **{:.0}** (the largest) |",
        ms(total),
        peak
    );
}

/// The spans of the layers below, summed over the parts and sorted by the time they took.
/// A span counts the time of the spans inside it too.
fn print_spans(out: &mut String, phases: &[Phase]) {
    if !measure::ENABLED {
        let _ = writeln!(out, "(the counters are off: run with `--features measure`)");
        return;
    }
    let _ = writeln!(
        out,
        "| what (called from) | part | calls | bytes (MiB) | time (ms, with what is inside) |\n|---|---|---:|---:|---:|"
    );
    let mut rows: Vec<(&'static str, &Entry)> = Vec::new();
    for p in phases {
        for e in &p.spans {
            rows.push((p.id, e));
        }
    }
    rows.sort_by_key(|a| std::cmp::Reverse(a.1.nanos));
    for (part, e) in rows
        .iter()
        .filter(|r| r.1.nanos >= min_nanos() || r.1.bytes >= 1 << 20)
    {
        let _ = writeln!(
            out,
            "| `{}` {} | {part} | {} | {:.2} | {:.2} |",
            e.name,
            site(e),
            e.calls,
            mib(e.bytes),
            e.nanos as f64 / 1e6
        );
    }
}

/// Runs `f` and adds a line to the table of the opening: its time, the resident memory after it and
/// the peak during it (Linux: `VmRSS` and `VmHWM`).
fn stage<R>(out: &mut String, name: &str, f: impl FnOnce() -> R) -> R {
    reset_peak();
    let base = rss_mib();
    let t = Instant::now();
    let r = f();
    let wall = t.elapsed();
    let _ = writeln!(
        out,
        "| {name} | {:.1} | {:.0} (+{:.0}) | {:.0} |",
        ms(wall),
        rss_mib(),
        rss_mib() - base,
        peak_mib()
    );
    r
}

/// The cost of the piece tree and the line index alone, with no view: an insert and a delete
/// inside a line of `len` bytes, and the questions the layout asks about the line.
fn document_layer(out: &mut String, doc: &Document, rounds: usize) {
    let mut doc = doc.clone();
    let mut rows: Vec<(&'static str, Phase)> = Vec::new();
    let mut run = |label: &'static str, doc: &mut Document, f: &dyn Fn(&mut Document)| {
        let best = (0..rounds)
            .map(|_| timed("doc", label, || f(doc)))
            .min_by_key(|p| p.wall)
            .unwrap();
        rows.push((label, best));
    };
    run("line_raw_len(1) (index only)", &mut doc, &|d| {
        let _ = d.line_raw_len(1);
    });
    run("line_len(1) (decoded length)", &mut doc, &|d| {
        let _ = d.line_len(1);
    });
    run(
        "line(1) (decode the whole line into a String)",
        &mut doc,
        &|d| {
            let _ = d.line(1);
        },
    );
    run("line_slice(1, 0, 4096)", &mut doc, &|d| {
        let _ = d.line_slice(1, 0, 4096);
    });
    run("rest_of_line(1, 6, 64)", &mut doc, &|d| {
        let _ = d.rest_of_line(1, 6, 64);
    });
    run("end_pos()", &mut doc, &|d| {
        let _ = d.end_pos();
    });
    // The edit pair leaves the text as it was.
    run("insert `x` at (1, 6)", &mut doc, &|d| {
        let _ = d.insert(Pos::new(1, 6), "x");
        let _ = d.delete(Pos::new(1, 6), Pos::new(1, 7));
    });
    let _ = writeln!(
        out,
        "| call | time (ms) | what the counters saw |\n|---|---:|---|"
    );
    for (label, p) in rows {
        let seen = p
            .spans
            .iter()
            .filter(|e| e.nanos >= 50_000 || e.bytes >= 1 << 20)
            .map(|e| {
                format!(
                    "`{}`{} x{} {:.1} MiB {:.2} ms",
                    e.name,
                    site(e),
                    e.calls,
                    mib(e.bytes),
                    e.nanos as f64 / 1e6
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        let _ = writeln!(out, "| {label} | {:.2} | {seen} |", ms(p.wall));
    }
    let _ = writeln!(out, "\npiece count after the edits: {}", doc.piece_count());
}

fn measure_one(cx: &mut TestAppContext, mib_len: usize, rounds: usize) -> String {
    let mut out = String::new();
    let len = mib_len * MIB;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("long-line.txt");
    let _ = writeln!(out, "## The line is {mib_len} MiB\n");

    // ---- opening ----
    let bytes = text_bytes(len);
    let file_len = bytes.len();
    std::fs::write(&path, &bytes).unwrap();
    drop(bytes);
    let _ = writeln!(
        out,
        "File: {:.1} MB ({} bytes), line 2 is {len} bytes. Resident memory at the start: {:.0} MiB.\n",
        file_len as f64 / 1e6,
        file_len,
        rss_mib()
    );
    let _ = writeln!(
        out,
        "### Opening\n\n| stage | time (ms) | resident after (MiB) | peak resident during (MiB) |\n|---|---:|---:|---:|"
    );
    measure::reset();
    let file = stage(
        &mut out,
        "`open_file` (read the file, find the encoding, index the line endings)",
        || open_file(&path, &OpenOptions::default()).unwrap(),
    );
    let opening_spans = measure::take();
    let shared = stage(&mut out, "`SharedDocument::from_text_file`", || {
        SharedDocument::from_text_file(file)
    });
    let doc_for_layer = shared.doc().clone();
    let document = cx.new(|_| shared);
    measure::reset();
    let (view, cx) = stage(
        &mut out,
        "the window and the view (`EditorView::new`)",
        || {
            cx.add_window_view({
                let document = document.clone();
                move |window, cx| {
                    let view = EditorView::new(document, cx);
                    window.focus(&view.focus_handle, cx);
                    view
                }
            })
        },
    );
    let window_spans = measure::take();
    cx.update(|window, _| window.activate_window());
    cx.simulate_resize(gpui::size(gpui::px(900.), gpui::px(420.)));
    stage(&mut out, "first paint, caret on line 1", || paint(cx));
    stage(
        &mut out,
        "frames until the layout and the text width are done",
        || {
            for _ in 0..200 {
                paint(cx);
                cx.run_until_parked();
                let done = view.read_with(cx, |v, _| {
                    v.layout().is_complete() && v.text_width.is_complete()
                });
                if done {
                    break;
                }
            }
        },
    );
    stage(
        &mut out,
        "caret to line 2, column 6, and paint (the first time the long line is read)",
        || {
            put_caret(&view, cx, Pos::new(1, 6));
            paint(cx);
        },
    );
    stage(
        &mut out,
        "the first edit (typing `x`) and its paint",
        || {
            cx.simulate_input("x");
            paint(cx);
        },
    );
    stage(
        &mut out,
        "Backspace and its paint (the text is as it was)",
        || {
            cx.simulate_keystrokes("backspace");
            paint(cx);
        },
    );
    let _ = writeln!(out);
    if measure::ENABLED {
        let _ = writeln!(
            out,
            "Counters of `open_file` and of the window and the view:\n"
        );
        for e in opening_spans
            .iter()
            .chain(&window_spans)
            .filter(|e| e.nanos >= 100_000 || e.bytes >= 1 << 20)
        {
            let _ = writeln!(
                out,
                "- `{}`{} x{} {:.1} MiB {:.2} ms",
                e.name,
                site(e),
                e.calls,
                mib(e.bytes),
                e.nanos as f64 / 1e6
            );
        }
        let _ = writeln!(out);
    }

    // ---- the layout of the line ----
    view.read_with(cx, |v, cx| {
        let doc = v.document().read(cx).doc();
        let l = v.layout().line_layout(doc, 1).unwrap().unwrap();
        let _ = writeln!(
            out,
            "Layout of the line: {} rows at {} columns, `LineLayout::text` is {:.1} MiB, `rows` is {:.2} MiB ({} bytes a row). Document: {} lines, {} rows; pieces {}.\n",
            l.rows.len(),
            v.layout().config().max_columns(),
            l.text.len() as f64 / MIB as f64,
            (l.rows.len() * std::mem::size_of::<matsu_layout::Row>()) as f64 / MIB as f64,
            std::mem::size_of::<matsu_layout::Row>(),
            doc.line_count(),
            v.layout().row_count(),
            doc.piece_count(),
        );
    });

    // ---- the steps ----
    for search_marks in [false, true] {
        if search_marks {
            // A search that finds nothing in the line, with the marks on: the paint reads the line
            // for it (`matches_of_line`).
            view.update(cx, |v, _| {
                v.search.set_pattern("zzz-not-there");
                v.search.set_mark(true);
            });
            let _ = writeln!(
                out,
                "### The same steps with a search whose marks are on (pattern `zzz-not-there`)\n"
            );
        } else {
            let _ = writeln!(out, "### Keys, caret at line 2, column 6\n");
        }
        let mut per_step: Vec<Vec<Vec<Phase>>> = vec![Vec::new(); Step::ALL.len()];
        // one round to warm the caches the way a user has them after the first key
        for round in 0..rounds + 1 {
            put_caret(&view, cx, Pos::new(1, 6));
            paint(cx);
            cx.run_until_parked();
            for (i, step) in Step::ALL.into_iter().enumerate() {
                let phases = run_step(&view, cx, step, search_marks);
                if round > 0 {
                    per_step[i].push(phases);
                }
            }
        }
        for (i, step) in Step::ALL.into_iter().enumerate() {
            let phases = best(&per_step[i]);
            let _ = writeln!(out, "#### {}\n", step.label());
            print_phases(&mut out, &phases);
            let _ = writeln!(out, "\nThe layers below (best round):\n");
            print_spans(&mut out, &phases);
            let _ = writeln!(out);
        }
        if search_marks {
            view.update(cx, |v, _| v.search.set_mark(false));
        }
    }

    // ---- the document layer alone ----
    let _ = writeln!(out, "### The document layer alone (no view)\n");
    document_layer(&mut out, &doc_for_layer, rounds);
    let _ = writeln!(out);
    out
}

fn run(cx: &mut TestAppContext, mib_len: usize) {
    let rounds = env_rounds();
    let mut out = String::new();
    let _ = writeln!(
        out,
        "counters: {}; build: {}; rounds: {rounds}\n",
        if measure::ENABLED { "on" } else { "off" },
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
    out.push_str(&measure_one(cx, mib_len, rounds));
    println!("{out}");
}

/// "(from crates/x/src/y.rs:12)" for a span that knows its caller.
fn site(e: &Entry) -> String {
    if e.file.is_empty() {
        return String::new();
    }
    let file = e.file.replace('\\', "/");
    let file = file.find("crates/").map_or(file.as_str(), |i| &file[i..]);
    format!("(from {file}:{})", e.line)
}

/// The least time a span must have taken to be listed: 100 microseconds, or `MATSU_LONG_LINE_MIN_US`
/// (0 lists every span, to see the small consumers too).
fn min_nanos() -> u64 {
    std::env::var("MATSU_LONG_LINE_MIN_US")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map_or(100_000, |us| us * 1000)
}

fn env_rounds() -> usize {
    std::env::var("MATSU_LONG_LINE_ROUNDS")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(3)
}

#[gpui::test]
#[ignore = "a measurement: run by hand, see the module documentation"]
fn long_line_measure_4_mib(cx: &mut TestAppContext) {
    run(cx, 4);
}

#[gpui::test]
#[ignore = "a measurement: run by hand, see the module documentation"]
fn long_line_measure_16_mib(cx: &mut TestAppContext) {
    run(cx, 16);
}

#[gpui::test]
#[ignore = "a measurement: run by hand, see the module documentation"]
fn long_line_measure_64_mib(cx: &mut TestAppContext) {
    run(cx, 64);
}

/// The size of the file of the Windows Sandbox measurement (200 MiB of line, 209.7 MB in all).
#[gpui::test]
#[ignore = "a measurement: run by hand, see the module documentation"]
fn long_line_measure_200_mib(cx: &mut TestAppContext) {
    run(cx, 200);
}
