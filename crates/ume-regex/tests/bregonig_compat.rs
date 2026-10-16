//! Replays the cases of `tools/bregonig-compat/cases.json` through this crate and compares them
//! with what the real bregonig.dll 4.20 (Sakura's package) did with them. The DLL's results were
//! measured on Windows with the harness in `tools/bregonig-compat/` and are stored in
//! `bregonig-4.20-x64.json` there (the x86 file must agree; see `x86_and_x64_agree`). The DLL is
//! not needed to run this test.
//!
//! A case with `known_difference` is one where UME Editor differs on purpose. The test then
//! requires that the two still differ, so the entry goes when the difference does.

use std::collections::BTreeMap;
use std::fs;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;
use ume_core::buffer::{Document, Pos};
use ume_core::search::{Control, Matcher, ReplaceTarget};
use ume_regex::{RegexMatcher, RegexReplacer, RegexSearchOptions, ReplaceAllMode};

fn data_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/bregonig-compat")
        .join(name)
}

fn read_json(name: &str) -> Value {
    let path = data_path(name);
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

struct Case {
    id: String,
    kind: String,
    pattern: String,
    replacement: String,
    subject: String,
    /// In UTF-16 units.
    start: usize,
    /// In UTF-16 units.
    end: usize,
    match_case: bool,
    known_difference: Option<String>,
}

fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

fn parse_cases(v: &Value) -> Vec<Case> {
    v["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .map(|c| {
            let text = |key: &str| c[key].as_str().unwrap_or_default().to_owned();
            let subject = text("subject");
            let end = c["end"]
                .as_u64()
                .map_or_else(|| utf16_len(&subject), |n| n as usize);
            Case {
                id: text("id"),
                kind: text("kind"),
                pattern: text("pattern"),
                replacement: text("replacement"),
                start: c["start"].as_u64().unwrap_or(0) as usize,
                end,
                match_case: c["match_case"].as_bool().unwrap_or(false),
                known_difference: c["known_difference"].as_str().map(str::to_owned),
                subject,
            }
        })
        .collect()
}

/// What a run came to; the same shape for the DLL and for this crate.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// The pattern or the replacement was refused.
    Rejected,
    /// The DLL crashed or reported an error while running (never expected for this crate).
    RunError(String),
    /// No match (or nothing replaced).
    NoMatch,
    Done {
        /// Group 0 and the groups, as UTF-16 ranges; `None` for a group that did not take part.
        /// Empty for the replace-all kind, which has no single match.
        groups: Vec<Option<(usize, usize)>>,
        /// The number of groups of the pattern, not counting the whole match.
        nparens: usize,
        /// The replaced text for the replace kinds, in UTF-16 units (the DLL can give a lone
        /// surrogate, which no `String` holds).
        out: Option<Vec<u16>>,
    },
}

fn dll_outcome(case: &Case, r: &Value) -> Outcome {
    if r["crashed"].as_bool() == Some(true) {
        return Outcome::RunError(format!("the DLL crashed (exit code {})", r["exit_code"]));
    }
    if r["ran"].as_bool() != Some(true) {
        return Outcome::Rejected;
    }
    let msg = r["msg"].as_str().unwrap_or_default();
    // The DLL refuses a search whose start is the end of the text. Sakura's `CBregexp::Match`
    // and `Replace` treat any error as "no match", and so does this test.
    if msg == "invalid target parameter" {
        return Outcome::NoMatch;
    }
    if !msg.is_empty() {
        return Outcome::RunError(msg.to_owned());
    }
    let ret = r["ret"].as_i64().expect("ret");
    if ret <= 0 {
        return Outcome::NoMatch;
    }
    let groups = r["groups"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|g| {
                    g.as_array().map(|p| {
                        (
                            p[0].as_u64().unwrap() as usize,
                            p[1].as_u64().unwrap() as usize,
                        )
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let out = if case.kind == "match" {
        None
    } else {
        Some(dll_text(r))
    };
    Outcome::Done {
        groups: if case.kind == "replace_all" {
            Vec::new()
        } else {
            groups
        },
        nparens: r["nparens"].as_u64().unwrap_or(0) as usize,
        out,
    }
}

/// The `out` or `out_units` of a result, as UTF-16 units. No text (a null) is an empty text.
fn dll_text(r: &Value) -> Vec<u16> {
    if let Some(units) = r["out_units"].as_array() {
        return units.iter().map(|u| u.as_u64().unwrap() as u16).collect();
    }
    r["out"]
        .as_str()
        .unwrap_or_default()
        .encode_utf16()
        .collect()
}

fn to_utf16(text: &str, byte: usize) -> usize {
    utf16_len(&text[..byte])
}

fn to_byte(text: &str, unit: usize) -> usize {
    let mut units = 0;
    for (i, c) in text.char_indices() {
        if units >= unit {
            return i;
        }
        units += c.len_utf16();
    }
    text.len()
}

fn our_outcome(case: &Case) -> Outcome {
    let options = RegexSearchOptions {
        match_case: case.match_case,
    };
    let matcher = match RegexMatcher::new(&case.pattern, options) {
        Ok(m) => Arc::new(m),
        Err(_) => return Outcome::Rejected,
    };
    if case.kind == "replace_all" {
        return our_replace_all(case, matcher);
    }
    // Sakura matches one line; a subject with a line ending is cut at it, because this crate
    // matches the line without its ending (the cases that say so are known differences).
    let line_end = case
        .subject
        .find(['\r', '\n'])
        .unwrap_or(case.subject.len());
    let line = &case.subject[..line_end];
    let start = to_byte(line, case.start);
    let end = to_byte(line, case.end);
    let found = match matcher.find_at(line, start) {
        Ok(Some(m)) if m.end <= end => m,
        Ok(_) => return Outcome::NoMatch,
        Err(e) => return Outcome::RunError(e.to_string()),
    };
    let mut ranges: Vec<Option<Range<usize>>> = vec![Some(found.start..found.end)];
    ranges.extend(found.captures.iter().cloned());
    let groups = ranges
        .iter()
        .map(|g| {
            g.as_ref()
                .map(|r| (to_utf16(line, r.start), to_utf16(line, r.end)))
        })
        .collect();
    let out = if case.kind == "match" {
        None
    } else {
        let replacer =
            match RegexReplacer::new(matcher.clone(), &case.replacement, ReplaceTarget::Match) {
                Ok(r) => r,
                Err(_) => return Outcome::Rejected,
            };
        let expanded = replacer.template().expand(line, &ranges);
        // The DLL gives the text from the start offset on.
        let out = format!(
            "{}{}{}",
            &line[start..found.start],
            expanded,
            &line[found.end..]
        );
        Some(out.encode_utf16().collect())
    };
    Outcome::Done {
        groups,
        nparens: matcher.capture_count(),
        out,
    }
}

fn our_replace_all(case: &Case, matcher: Arc<RegexMatcher>) -> Outcome {
    let replacer =
        match RegexReplacer::new(matcher.clone(), &case.replacement, ReplaceTarget::Match) {
            Ok(r) => r,
            Err(_) => return Outcome::Rejected,
        };
    let mut doc = Document::from_utf8(case.subject.as_bytes().to_vec()).unwrap();
    let result = replacer
        .replace_all(&mut doc, ReplaceAllMode::Batch, None, Control::new())
        .expect("replace_all");
    let text = doc
        .text_between(Pos::new(0, 0), doc.end_pos().unwrap())
        .unwrap();
    if result.edits.is_empty() {
        return Outcome::NoMatch;
    }
    Outcome::Done {
        groups: Vec::new(),
        nparens: matcher.capture_count(),
        out: Some(text.encode_utf16().collect()),
    }
}

/// The DLL's text when nothing was replaced is not compared: only whether anything was.
fn describe(o: &Outcome) -> String {
    format!("{o:?}")
}

fn results_by_id(v: &Value) -> BTreeMap<String, &Value> {
    v["results"]
        .as_array()
        .expect("results")
        .iter()
        .map(|r| (r["id"].as_str().expect("id").to_owned(), r))
        .collect()
}

#[test]
fn matches_bregonig_dll_4_20() {
    let cases = parse_cases(&read_json("cases.json"));
    let measured = read_json("bregonig-4.20-x64.json");
    assert_eq!(
        measured["dll_version"].as_str(),
        Some("bregonig.dll Ver.4.20 with Onigmo 6.2.0")
    );
    let results = results_by_id(&measured);
    assert_eq!(results.len(), cases.len(), "every case has a result");

    let mut unexpected = Vec::new();
    let mut stale = Vec::new();
    let mut known = 0;
    for case in &cases {
        let r = results
            .get(&case.id)
            .unwrap_or_else(|| panic!("no result for {}", case.id));
        let dll = dll_outcome(case, r);
        let ours = our_outcome(case);
        let same = dll == ours;
        match (&case.known_difference, same) {
            (None, true) => {}
            (None, false) => unexpected.push(format!(
                "{} [{}] /{}/ {:?} on {:?}\n    dll:  {}\n    ours: {}",
                case.id,
                case.kind,
                case.pattern,
                case.replacement,
                case.subject,
                describe(&dll),
                describe(&ours)
            )),
            (Some(_), false) => known += 1,
            (Some(_), true) => stale.push(case.id.clone()),
        }
    }
    assert!(
        unexpected.is_empty(),
        "{} cases differ from bregonig.dll:\n{}",
        unexpected.len(),
        unexpected.join("\n")
    );
    assert!(
        stale.is_empty(),
        "known_difference is set but the results agree: {stale:?}"
    );
    eprintln!("{} cases, {known} known differences", cases.len());
}

/// The cases where the 32-bit and the 64-bit DLL gave different text. All of them are
/// `$-{name}[n]` with an `n` past the groups of that name: the x64 DLL gives an empty text and
/// the x86 DLL gives the text of a group (it looks like a read past the end of a list), so the
/// x64 result is the reference and these are not compared with this crate in any other way.
const X86_X64_DIFFER: &[&str] = &[
    "dupname-alt-minus2-a",
    "dupname-alt-minus2-b",
    "dupname-alt-minus99-a",
    "dupname-alt-minus99-b",
    "dupname-seq-empty-minus2",
    "dupname-seq-minus2",
    "minus-single-01",
    "minus-single-1",
];

#[test]
fn x86_and_x64_agree() {
    let x64 = read_json("bregonig-4.20-x64.json");
    let x86 = read_json("bregonig-4.20-x86.json");
    assert_eq!(x64["dll_version"], x86["dll_version"]);
    let (r64, r86) = (results_by_id(&x64), results_by_id(&x86));
    assert_eq!(r64.len(), r86.len());
    let differ: Vec<&str> = r64
        .iter()
        .filter(|(id, r)| r86.get(*id) != Some(r))
        .map(|(id, _)| id.as_str())
        .collect();
    let mut expected = X86_X64_DIFFER.to_vec();
    expected.sort_unstable();
    assert_eq!(differ, expected);
}
