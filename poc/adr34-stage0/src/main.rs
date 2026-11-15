//! ADR-0034 stage 0: what GPUI can measure without a `Window`, from other threads, and what
//! drawing one character at a time costs. Throwaway code; see README.md.
//!
//! `cargo run --release -- [family ...]` (default: a proportional and a monospaced family that
//! this machine has). Prints a report to the standard output.

use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{Font, FontId, SharedString, TextRun, TextSystem, WindowTextSystem, px};

const SIZE: f32 = 13.0;
const ROWS: usize = 60;
const COLUMNS: usize = 120;
const REPEATS: usize = 15;

fn assert_send_sync<T: Send + Sync>() {}

fn main() {
    // Compile-time check: what the ADR wants to hand to a measuring function.
    assert_send_sync::<TextSystem>();
    assert_send_sync::<Arc<TextSystem>>();

    let platform = gpui_platform::bench_text_system();
    let system = Arc::new(TextSystem::new(platform));
    let names = system.all_font_names();
    println!("fonts installed: {}", names.len());

    let args: Vec<String> = std::env::args().skip(1).collect();
    let families: Vec<String> = if args.is_empty() {
        let pick = |cands: &[&str]| {
            cands
                .iter()
                .find(|c| names.iter().any(|n| n == *c))
                .map(|c| (*c).to_owned())
        };
        [
            pick(&[
                "Noto Sans JP",
                "Noto Sans CJK JP",
                "Yu Gothic UI",
                "Hiragino Sans",
            ]),
            pick(&[
                "BIZ UDGothic",
                "MS Gothic",
                "Noto Sans Mono",
                "DejaVu Sans Mono",
                "Menlo",
            ]),
        ]
        .into_iter()
        .flatten()
        .collect()
    } else {
        args
    };

    for family in &families {
        println!("\n=== {family} at {SIZE} px ===");
        let font = gpui::font(family.clone());
        let id = system.resolve_font(&font);
        println!("FontId = {}", id.0);
        measure_without_window(&system, id);
        threads(&system, &font, id);
        bench(&system, &font, id);
    }
}

/// Measures with `TextSystem` only: no `Window`, no `App`.
fn measure_without_window(system: &Arc<TextSystem>, id: FontId) {
    println!("-- measuring without a Window (TextSystem only) --");
    for ch in [
        '0',
        'i',
        'm',
        'あ',
        '漢',
        '\u{3000}',
        'ｱ',
        '\u{1F600}',
        '\u{0301}',
    ] {
        let adv = system.advance(id, px(SIZE), ch);
        let lay = system.layout_width(id, px(SIZE), ch);
        println!(
            "  U+{:04X} {ch:?}: advance() = {}, layout_width() = {:.3}",
            ch as u32,
            match &adv {
                Ok(s) => format!("{:.3}", f32::from(s.width)),
                Err(e) => format!("error ({e})"),
            },
            f32::from(lay)
        );
    }
}

/// Measures every character of the benchmark text from other threads, while this thread shapes
/// lines, and compares with what this thread measured first.
fn threads(system: &Arc<TextSystem>, font: &Font, id: FontId) {
    println!("-- measuring from other threads --");
    let text = rows();
    let chars = unique_chars(&text);
    let reference: Vec<f32> = chars
        .iter()
        .map(|&c| f32::from(system.layout_width(id, px(SIZE), c)))
        .collect();
    let window_system = WindowTextSystem::new(system.clone());
    let started = Instant::now();
    let handles: Vec<_> = (0..4)
        .map(|n| {
            let system = system.clone();
            let chars = chars.clone();
            std::thread::spawn(move || {
                // Each thread walks the characters from a different place, 20 times.
                let mut out = Vec::new();
                for _ in 0..20 {
                    out = chars
                        .iter()
                        .cycle()
                        .skip(n * 7)
                        .take(chars.len())
                        .map(|&c| (c, f32::from(system.layout_width(id, px(SIZE), c))))
                        .collect::<Vec<_>>();
                }
                out
            })
        })
        .collect();
    // This thread shapes whole lines in the meantime.
    for row in &text {
        shape(&window_system, font, row);
    }
    let mut same = true;
    for h in handles {
        for (c, w) in h.join().expect("thread panicked") {
            let i = chars.iter().position(|&x| x == c).unwrap();
            same &= w == reference[i];
        }
    }
    println!(
        "  4 threads x {} characters x 20 rounds + {} lines shaped here: {:?}; same widths as the first measurement: {same}",
        chars.len(),
        text.len(),
        started.elapsed()
    );
}

fn shape(system: &WindowTextSystem, font: &Font, text: &str) -> usize {
    let text: SharedString = text.to_owned().into();
    let runs = [TextRun {
        len: text.len(),
        font: font.clone(),
        color: gpui::black(),
        background_color: None,
        underline: None,
        strikethrough: None,
    }];
    system.shape_line(text, px(SIZE), &runs, None).len()
}

/// 60 rows of about 120 half-width columns: ASCII pieces and Japanese phrases in turn.
fn rows() -> Vec<String> {
    let ascii = [
        "let mut buffer = String::new();",
        "fn main() {",
        "println!(\"hello\");",
        "// Wiki Maximum",
        "if (x < 10) return;",
        "WWW milk illicit",
        "0123456789",
        "for i in 0..n {",
    ];
    let japanese = [
        "吾輩は猫である。名前はまだ無い。",
        "どこで生れたかとんと見当がつかぬ。",
        "ここで文字ごとの幅を測る",
        "プロポーショナルフォント",
        "全角スペース\u{3000}を含む行",
        "ｱｲｳｴｵ半角カナ",
    ];
    let columns = |piece: &str| -> usize {
        piece
            .chars()
            .map(|c| {
                if c.is_ascii() || ('ｱ'..='ﾟ').contains(&c) {
                    1
                } else {
                    2
                }
            })
            .sum()
    };
    (0..ROWS)
        .map(|r| {
            let mut row = String::new();
            let mut cols = 0;
            let mut k = r;
            loop {
                let piece = if k % 2 == 0 {
                    ascii[(k / 2 + r) % ascii.len()]
                } else {
                    japanese[(k / 2 + r) % japanese.len()]
                };
                let w = columns(piece) + 1;
                if cols + w > COLUMNS {
                    break;
                }
                row.push_str(piece);
                row.push(' ');
                cols += w;
                k += 1;
            }
            row
        })
        .collect()
}

fn unique_chars(rows: &[String]) -> Vec<char> {
    let mut chars: Vec<char> = rows.iter().flat_map(|r| r.chars()).collect();
    chars.sort_unstable();
    chars.dedup();
    chars
}

/// Splits a row into runs of the same class (ASCII or not).
fn runs_of(row: &str) -> Vec<&str> {
    let mut runs = Vec::new();
    let mut start = 0;
    let mut last: Option<bool> = None;
    for (i, c) in row.char_indices() {
        let class = c.is_ascii();
        if last.is_some_and(|l| l != class) {
            runs.push(&row[start..i]);
            start = i;
        }
        last = Some(class);
    }
    runs.push(&row[start..]);
    runs
}

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

type Mode = (
    &'static str,
    fn(&WindowTextSystem, &Font, &[String]) -> usize,
);

fn per_line(s: &WindowTextSystem, f: &Font, rows: &[String]) -> usize {
    rows.iter().map(|r| shape(s, f, r)).sum()
}

fn per_run(s: &WindowTextSystem, f: &Font, rows: &[String]) -> usize {
    rows.iter()
        .flat_map(|r| runs_of(r))
        .map(|r| shape(s, f, r))
        .sum()
}

fn per_char(s: &WindowTextSystem, f: &Font, rows: &[String]) -> usize {
    let mut buf = [0u8; 4];
    rows.iter()
        .flat_map(|r| r.chars())
        .map(|c| shape(s, f, c.encode_utf8(&mut buf)))
        .sum()
}

fn bench(system: &Arc<TextSystem>, font: &Font, id: FontId) {
    let rows = rows();
    let chars: usize = rows.iter().map(|r| r.chars().count()).sum();
    let runs: usize = rows.iter().map(|r| runs_of(r).len()).sum();
    let unique = unique_chars(&rows);
    println!(
        "-- cost of shaping {ROWS} rows (~{COLUMNS} columns): {chars} characters, {runs} runs, {} distinct characters --",
        unique.len()
    );

    // The width table: each distinct character measured once.
    let t = Instant::now();
    for &c in &unique {
        system.layout_width(id, px(SIZE), c);
    }
    println!(
        "  width table, {} distinct characters (first time): {:?}",
        unique.len(),
        t.elapsed()
    );

    let modes: [Mode; 3] = [
        ("per line", per_line),
        ("per run", per_run),
        ("per char", per_char),
    ];
    for (name, f) in modes {
        // A fresh line cache every time: the first frame (the platform's own caches are warm).
        let mut cold = Vec::new();
        for _ in 0..REPEATS {
            let ws = WindowTextSystem::new(system.clone());
            let t = Instant::now();
            f(&ws, font, &rows);
            cold.push(t.elapsed());
        }
        // The same cache, again: a repeated frame when the lines are cached.
        let ws = WindowTextSystem::new(system.clone());
        f(&ws, font, &rows);
        let mut warm = Vec::new();
        for _ in 0..REPEATS {
            let t = Instant::now();
            f(&ws, font, &rows);
            warm.push(t.elapsed());
        }
        println!(
            "  {name:9}: first frame {:>10.3?}   repeated frame {:>10.3?}",
            median(cold),
            median(warm)
        );
    }
}
