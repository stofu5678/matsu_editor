//! The report: Markdown for people, JSON for tools. Both carry the same numbers.

use std::fmt::Write as _;

/// One value of the report.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Num(f64),
    Text(String),
    /// Not measured (the reason is in the text of the row, or the step was skipped).
    Missing,
}

impl Value {
    pub fn text(text: impl Into<String>) -> Self {
        Value::Text(text.into())
    }

    fn markdown(&self) -> String {
        match self {
            Value::Num(n) => number(*n),
            Value::Text(t) => t.replace('|', "\\|").replace('\n', " "),
            Value::Missing => "-".to_owned(),
        }
    }

    fn json(&self) -> String {
        match self {
            Value::Num(n) if n.is_finite() => number(*n),
            Value::Num(_) | Value::Missing => "null".to_owned(),
            Value::Text(t) => json_string(t),
        }
    }
}

/// A number with 3 significant decimals at most, and no exponent.
fn number(n: f64) -> String {
    if n == n.trunc() && n.abs() < 1e15 {
        return format!("{n:.0}");
    }
    let text = format!("{n:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

pub fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// What was measured for one kind of file and one block size.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    /// `S`, `C`, ...
    pub kind: String,
    pub file_bytes: u64,
    /// The block size in KiB (the same for the cache of the file and the index of the scan).
    pub block_kib: u64,
    /// `(name, unit, value)` in the order they were measured.
    pub metrics: Vec<(String, String, Value)>,
    pub notes: Vec<String>,
}

impl Run {
    pub fn new(kind: &str, file_bytes: u64, block_kib: u64) -> Self {
        Self {
            kind: kind.to_owned(),
            file_bytes,
            block_kib,
            metrics: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn num(&mut self, name: &str, unit: &str, value: f64) {
        self.metrics
            .push((name.to_owned(), unit.to_owned(), Value::Num(value)));
    }

    pub fn text(&mut self, name: &str, value: impl Into<String>) {
        self.metrics
            .push((name.to_owned(), String::new(), Value::text(value)));
    }

    pub fn missing(&mut self, name: &str, unit: &str) {
        self.metrics
            .push((name.to_owned(), unit.to_owned(), Value::Missing));
    }
}

/// The result of one OS check (ADR-0020, 8.1, 「確かめる OS の挙動」).
#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    pub name: String,
    /// What the call of the OS did (`ok` or the error).
    pub action: String,
    /// What a read of a place gave afterwards through a source that had loaded nothing of it.
    pub read_uncached: String,
    /// What a read of a place that the cache of another source held gave afterwards (that source
    /// had read nothing else since the change, so this read could not learn of it from another one).
    pub read_cached: String,
}

/// What the report says about the machine and the run.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Machine {
    pub os: String,
    pub arch: String,
    pub cpu: String,
    pub cpus: usize,
    pub memory_mib: Option<u64>,
    pub commit: String,
    pub rustc: String,
    pub dir: String,
    pub filesystem: String,
    pub started: String,
    pub cache_dropped: String,
    pub release_build: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    pub machine: Machine,
    /// The arguments of the run.
    pub arguments: String,
    pub runs: Vec<Run>,
    pub checks: Vec<Check>,
    /// `(kind, MB/s)`: a plain read of the file in pieces of 1 MiB, the speed of the disk.
    pub baselines: Vec<(String, f64)>,
}

impl Report {
    pub fn markdown(&self) -> String {
        let m = &self.machine;
        let mut out = String::new();
        out.push_str("# Huge file measurements (ADR-0020, 8.1)\n\n");
        out.push_str("## Machine\n\n| item | value |\n|---|---|\n");
        let memory = m.memory_mib.map_or("-".to_owned(), |v| format!("{v} MiB"));
        for (k, v) in [
            ("OS", format!("{} {}", m.os, m.arch)),
            ("CPU", format!("{} ({} threads)", m.cpu, m.cpus)),
            ("Memory", memory),
            ("Commit", m.commit.clone()),
            ("Rust", m.rustc.clone()),
            (
                "Build",
                if m.release_build {
                    "release".to_owned()
                } else {
                    "DEBUG (do not trust the numbers)".to_owned()
                },
            ),
            ("Folder", m.dir.clone()),
            ("File system", m.filesystem.clone()),
            ("Started", m.started.clone()),
            ("OS cache", m.cache_dropped.clone()),
            ("Arguments", format!("`{}`", self.arguments)),
        ] {
            let _ = writeln!(out, "| {k} | {} |", Value::text(v).markdown());
        }

        let mut kinds: Vec<&str> = Vec::new();
        for run in &self.runs {
            if !kinds.contains(&run.kind.as_str()) {
                kinds.push(&run.kind);
            }
        }
        for kind in kinds {
            let runs: Vec<&Run> = self.runs.iter().filter(|r| r.kind == kind).collect();
            let _ = writeln!(
                out,
                "\n## {kind}: {} MiB\n",
                runs[0].file_bytes / (1024 * 1024)
            );
            if let Some((_, speed)) = self.baselines.iter().find(|(k, _)| k == kind) {
                let _ = writeln!(
                    out,
                    "A plain read of the file in 1 MiB pieces: {} MB/s.\n",
                    number(*speed)
                );
            }
            out.push_str("| measure |");
            for r in &runs {
                let _ = write!(out, " block {} KiB |", r.block_kib);
            }
            out.push_str("\n|---|");
            for _ in &runs {
                out.push_str("---:|");
            }
            out.push('\n');
            // A run that stopped early or failed has fewer metrics than a full one: the rows are
            // the names of all the runs (in the order they first appear), and each cell is looked
            // up by name.
            let mut rows: Vec<(&str, &str)> = Vec::new();
            for r in &runs {
                for (name, unit, _) in &r.metrics {
                    if !rows.iter().any(|(n, _)| n == name) {
                        rows.push((name, unit));
                    }
                }
            }
            for (name, unit) in rows {
                let label = if unit.is_empty() {
                    name.to_owned()
                } else {
                    format!("{name} ({unit})")
                };
                let _ = write!(out, "| {} |", Value::text(label).markdown());
                for r in &runs {
                    let cell = r
                        .metrics
                        .iter()
                        .find(|m| m.0 == name)
                        .map_or("-".to_owned(), |m| m.2.markdown());
                    let _ = write!(out, " {cell} |");
                }
                out.push('\n');
            }
            for r in &runs {
                for note in &r.notes {
                    let _ = writeln!(out, "\n- block {} KiB: {note}", r.block_kib);
                }
            }
        }

        if !self.checks.is_empty() {
            out.push_str("\n## OS checks\n\n");
            out.push_str(
                "| check | the call | read of a place not in the cache (a source of its own) | read of a place in the cache (another source of its own) |\n",
            );
            out.push_str("|---|---|---|---|\n");
            for c in &self.checks {
                let _ = writeln!(
                    out,
                    "| {} | {} | {} | {} |",
                    Value::text(&c.name).markdown(),
                    Value::text(&c.action).markdown(),
                    Value::text(&c.read_uncached).markdown(),
                    Value::text(&c.read_cached).markdown()
                );
            }
        }
        out
    }

    pub fn json(&self) -> String {
        let m = &self.machine;
        let mut out = String::from("{\n");
        let _ = writeln!(out, "  \"machine\": {{");
        let fields = [
            ("os", json_string(&m.os)),
            ("arch", json_string(&m.arch)),
            ("cpu", json_string(&m.cpu)),
            ("cpus", m.cpus.to_string()),
            (
                "memory_mib",
                m.memory_mib.map_or("null".to_owned(), |v| v.to_string()),
            ),
            ("commit", json_string(&m.commit)),
            ("rustc", json_string(&m.rustc)),
            ("dir", json_string(&m.dir)),
            ("filesystem", json_string(&m.filesystem)),
            ("started", json_string(&m.started)),
            ("cache_dropped", json_string(&m.cache_dropped)),
            ("release_build", m.release_build.to_string()),
            ("arguments", json_string(&self.arguments)),
        ];
        for (i, (k, v)) in fields.iter().enumerate() {
            let comma = if i + 1 < fields.len() { "," } else { "" };
            let _ = writeln!(out, "    \"{k}\": {v}{comma}");
        }
        out.push_str("  },\n  \"baselines_mb_per_s\": {");
        for (i, (k, v)) in self.baselines.iter().enumerate() {
            let comma = if i > 0 { "," } else { "" };
            let _ = write!(out, "{comma} {}: {}", json_string(k), number(*v));
        }
        out.push_str(" },\n  \"runs\": [\n");
        for (i, r) in self.runs.iter().enumerate() {
            let _ = write!(
                out,
                "    {{ \"kind\": {}, \"file_bytes\": {}, \"block_kib\": {}, \"metrics\": {{",
                json_string(&r.kind),
                r.file_bytes,
                r.block_kib
            );
            for (j, (name, unit, value)) in r.metrics.iter().enumerate() {
                let comma = if j > 0 { "," } else { "" };
                let key = if unit.is_empty() {
                    name.clone()
                } else {
                    format!("{name} ({unit})")
                };
                let _ = write!(out, "{comma} {}: {}", json_string(&key), value.json());
            }
            out.push_str(" }, \"notes\": [");
            for (j, note) in r.notes.iter().enumerate() {
                let comma = if j > 0 { ", " } else { "" };
                let _ = write!(out, "{comma}{}", json_string(note));
            }
            let comma = if i + 1 < self.runs.len() { "," } else { "" };
            let _ = writeln!(out, "] }}{comma}");
        }
        out.push_str("  ],\n  \"os_checks\": [\n");
        for (i, c) in self.checks.iter().enumerate() {
            let comma = if i + 1 < self.checks.len() { "," } else { "" };
            let _ = writeln!(
                out,
                "    {{ \"name\": {}, \"action\": {}, \"read_uncached\": {}, \"read_cached\": {} }}{comma}",
                json_string(&c.name),
                json_string(&c.action),
                json_string(&c.read_uncached),
                json_string(&c.read_cached)
            );
        }
        out.push_str("  ]\n}\n");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Report {
        let mut a = Run::new("S", 10 * 1024 * 1024, 64);
        a.num("scan", "s", 1.23456);
        a.num("count", "", 42.0);
        a.text("search", "cancelled | at 5 %");
        a.missing("save", "s");
        let mut b = Run::new("S", 10 * 1024 * 1024, 256);
        b.num("scan", "s", 0.9);
        b.num("count", "", 42.0);
        b.text("search", "done");
        b.missing("save", "s");
        b.notes.push("a \"quoted\" note".to_owned());
        Report {
            machine: Machine {
                os: "linux".into(),
                arch: "x86_64".into(),
                cpu: "Test CPU".into(),
                cpus: 4,
                memory_mib: Some(8192),
                commit: "abc123".into(),
                rustc: "rustc 1.0".into(),
                dir: "/tmp/x".into(),
                filesystem: "ext4".into(),
                started: "2026-11-28T00:00:00Z".into(),
                cache_dropped: "no".into(),
                release_build: true,
            },
            arguments: "--kinds s".into(),
            runs: vec![a, b],
            checks: vec![Check {
                name: "truncate".into(),
                action: "ok".into(),
                read_uncached: "changed outside".into(),
                read_cached: "ok".into(),
            }],
            baselines: vec![("S".into(), 812.5)],
        }
    }

    #[test]
    fn numbers_are_short_and_have_no_exponent() {
        assert_eq!(number(1.0), "1");
        assert_eq!(number(1.23456), "1.235");
        assert_eq!(number(0.5), "0.5");
        assert_eq!(number(0.0001), "0");
        assert_eq!(number(1234567.0), "1234567");
    }

    #[test]
    fn json_strings_are_escaped() {
        assert_eq!(json_string("a\"b\\c\nd\u{1}"), "\"a\\\"b\\\\c\\nd\\u0001\"");
        assert_eq!(json_string("日本"), "\"日本\"");
    }

    #[test]
    fn markdown_has_one_table_per_kind_with_a_column_per_block_size() {
        let md = sample().markdown();
        assert!(md.starts_with("# Huge file measurements"));
        assert!(md.contains("| Commit | abc123 |"), "{md}");
        assert!(md.contains("## S: 10 MiB"), "{md}");
        assert!(
            md.contains("| measure | block 64 KiB | block 256 KiB |"),
            "{md}"
        );
        assert!(md.contains("| scan (s) | 1.235 | 0.9 |"), "{md}");
        assert!(md.contains("| count | 42 | 42 |"), "{md}");
        // A pipe in a value must not break the table.
        assert!(md.contains("cancelled \\| at 5 %"), "{md}");
        assert!(md.contains("| save (s) | - | - |"), "{md}");
        assert!(md.contains("812.5 MB/s"), "{md}");
        assert!(md.contains("- block 256 KiB: a \"quoted\" note"), "{md}");
        assert!(
            md.contains("| truncate | ok | changed outside | ok |"),
            "{md}"
        );
    }

    #[test]
    fn a_run_with_other_metrics_is_matched_by_name() {
        // The first run stopped early: it lacks "scan" and "save". The second has an extra metric
        // that the first never reached, and a different order.
        let mut a = Run::new("S", 1024 * 1024, 64);
        a.num("open", "ms", 1.0);
        let mut b = Run::new("S", 1024 * 1024, 256);
        b.num("scan", "s", 2.0);
        b.num("open", "ms", 3.0);
        b.num("save", "s", 4.0);
        let report = Report {
            runs: vec![a, b],
            ..Report::default()
        };
        let md = report.markdown();
        assert!(md.contains("| open (ms) | 1 | 3 |"), "{md}");
        assert!(md.contains("| scan (s) | - | 2 |"), "{md}");
        assert!(md.contains("| save (s) | - | 4 |"), "{md}");
    }

    #[test]
    fn json_has_the_same_numbers() {
        let json = sample().json();
        assert!(json.contains("\"commit\": \"abc123\""), "{json}");
        assert!(json.contains("\"memory_mib\": 8192"), "{json}");
        assert!(json.contains("\"scan (s)\": 1.235"), "{json}");
        assert!(json.contains("\"scan (s)\": 0.9"), "{json}");
        assert!(json.contains("\"save (s)\": null"), "{json}");
        assert!(json.contains("\"S\": 812.5"), "{json}");
        assert!(json.contains("a \\\"quoted\\\" note"), "{json}");
        assert!(json.contains("\"name\": \"truncate\""), "{json}");
        // Brackets and braces balance (outside strings, there are none in this sample's text).
        let open = json.matches('{').count() + json.matches('[').count();
        let close = json.matches('}').count() + json.matches(']').count();
        assert_eq!(open, close, "{json}");
    }

    #[test]
    fn a_report_without_runs_still_renders() {
        let r = Report::default();
        assert!(r.markdown().contains("## Machine"));
        assert!(r.json().contains("\"runs\": [\n  ]"));
    }
}
