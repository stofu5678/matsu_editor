// Derived from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/view/CEditView_Search.cpp (GetCurrentTextForSearch, for `$C`)
//
// Copyright (C) 2008, kobake, ryoji
// Copyright (C) 2012, Uchi
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the file above and was modified: the C++ code (wide character
// buffers and pointers) was rewritten in Rust, and the tests are new.
//
// zlib License
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! `ExpandParameter` (`CSakuraEnvironment::ExpandParameter`) for macros, external commands and
//! `TraceOut`.
//!
//! The expansion itself is [`matsu_print::expand_parameter_limited`], the one the window title,
//! the tab caption and the print headers use, so every code (`$d` `$t` `$D` `$T` `$B` `$Q` `$I`
//! `$n` `$h` `$<profile>` `$N` `$E` and the conditions) has one meaning everywhere. This file
//! only collects what the codes read: what a macro can change while it runs (the path, the
//! caret, the selection, the marks) comes from the editor of the call; the rest (the document
//! type, the time stamp of the file, the paths of the settings file, ...) is the window's
//! [`HostEnv::header`], which the window fills as it does for its title.
//!
//! Without a window ([`HostEnv::detached`]) those codes are empty or their plain default.
//! `$p` and `$P` are the text for outside the print preview; a macro does not run in it.

use serde_json::Value;

use super::{EditorFunc, command, utf16_len};
use crate::api::{Args, Ty, req};
use crate::host::Editor;
use crate::protocol::RpcError;

type Result<T = Value> = std::result::Result<T, RpcError>;

/// The size of Sakura's buffer: `ExpandParameter(Source, Buffer, 2047)`.
const BUFFER_LEN: usize = 2047;
/// The most that `$C` takes: `_MAX_PATH - 1` characters (`LimitStringLengthW`).
const CURRENT_TEXT_MAX: usize = 259;

/// At most `max` UTF-16 code units, without cutting a character.
fn truncate_utf16(text: String, max: usize) -> String {
    if utf16_len(&text) <= max {
        return text;
    }
    let mut units = 0;
    let mut end = 0;
    for (i, c) in text.char_indices() {
        units += c.len_utf16();
        if units > max {
            break;
        }
        end = i + c.len_utf8();
    }
    text[..end].to_owned()
}

/// The text of `from` to `to` on the line of `from`, at most `max_bytes` of it: the first line of
/// a range, without reading the rest of a range that may be the whole document. The line ending
/// is not part of it. A cut inside a character moves back to the character before.
fn first_line_of(
    doc: &matsu_core::buffer::Document,
    from: matsu_core::buffer::Pos,
    to: matsu_core::buffer::Pos,
    max_bytes: usize,
) -> Result<String> {
    let line_len = doc.line_len(from.line).map_err(command)?.unwrap_or(0);
    let mut end = line_len.min(from.col.saturating_add(max_bytes));
    if to.line == from.line {
        end = end.min(to.col);
    }
    // At most three steps back: a character is four bytes at most.
    for _ in 0..4 {
        if end < from.col {
            break;
        }
        if let Ok(text) = doc.text_between(from, matsu_core::buffer::Pos::new(from.line, end)) {
            return Ok(text);
        }
        end = end.saturating_sub(1);
    }
    Ok(String::new())
}

/// `$C`: the selection up to its first line break, or the word at the caret
/// (`CEditView::GetCurrentTextForSearch`), at most 259 characters. Only the start of a long
/// selection is read.
fn current_text(ed: &Editor) -> Result<String> {
    let cx = &*ed.cx;
    // A UTF-16 unit is at most 3 bytes of UTF-8 (a pair of units is 4 bytes).
    let max_bytes = CURRENT_TEXT_MAX * 3 + 4;
    // A box selection is its first row inside its columns, not the range between its corners.
    if cx.cursor().box_corners().is_some() {
        let text = cx
            .box_first_row_text()
            .map_err(command)?
            .unwrap_or_default();
        let first = text
            .split(['\r', '\n'])
            .next()
            .unwrap_or_default()
            .to_owned();
        return Ok(truncate_utf16(first, CURRENT_TEXT_MAX));
    }
    let range = match cx.selection() {
        Some(range) => Some(range),
        None => cx.word_range_at(cx.caret()).map_err(command)?,
    };
    let text = match range {
        Some((from, to)) => first_line_of(cx.doc(), from, to, max_bytes)?,
        None => String::new(),
    };
    Ok(truncate_utf16(text, CURRENT_TEXT_MAX))
}

/// The state `ExpandParameter` reads, for the editor of this call: the template of the window,
/// and then what the macro may have changed.
fn header_context(ed: &Editor, format: &str) -> Result<matsu_print::HeaderContext> {
    let caret = ed.cx.caret();
    let mut context = ed.env.header.clone().unwrap_or_default();
    context.version = env!("CARGO_PKG_VERSION").to_owned();
    context.file_path = ed.cx.path().map(|p| p.to_string_lossy().into_owned());
    if let Some(short) = &ed.env.short_name {
        context.file_path_short = context.file_path.as_deref().map(short);
        context.folder_short = context.file_path.as_deref().and_then(|path| {
            path.rfind(std::path::is_separator)
                .map(|i| short(&path[..=i]))
        });
    }
    context.exe_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    context.macro_path = ed.env.macro_path.clone();
    context.modified = ed.cx.is_modified();
    context.recording_macro = ed.env.recording_key_macro;
    // `_ExParam_Evaluate`: the view mode first, then a locked file, else neither.
    if ed.cx.is_view_mode() {
        context.write_state = matsu_print::WriteState::ViewMode;
    } else if context.write_state == matsu_print::WriteState::ViewMode {
        context.write_state = matsu_print::WriteState::Writable;
    }
    context.caret_y = caret.line + 1;
    // `$x` is the number of UTF-16 units before the caret, plus one. Only that format reads the
    // text in front of the caret.
    context.caret_x = if format.contains("$x") {
        let before = ed
            .cx
            .doc()
            .text_between(matsu_core::buffer::Pos::new(caret.line, 0), caret)
            .map_err(command)?;
        utf16_len(&before) + 1
    } else {
        1
    };
    // Only the format strings that have `$C` read the text.
    context.selected_text = if format.contains("$C") {
        current_text(ed)?
    } else {
        String::new()
    };
    // `$d` and `$t` are the time now, in the format of the common settings.
    if format.contains("$d") || format.contains("$t") {
        use matsu_commands::convert::datetime::{Clock, SystemClock, format_date, format_time};
        let options = matsu_commands::format_options::get();
        let now = SystemClock.now();
        context.date = format_date(&now, options.date_picture());
        context.time = format_time(&now, options.time_picture());
    }
    Ok(context)
}

/// F_EXPANDPARAMETER.
fn expand_parameter(ed: &mut Editor, args: &Args) -> Result {
    let format = args.str(0);
    let context = header_context(ed, format)?;
    Ok(matsu_print::expand_parameter_limited(format, &context, None, BUFFER_LEN).into())
}

pub const FUNCS: &[EditorFunc] = &[EditorFunc {
    name: "ExpandParameter",
    params: &[req("format", Ty::Str)],
    result: Ty::Str,
    doc: "Replaces the codes that start with $ in a string, as the window title does: $$ ($), \
          $A (name), $F (full path), $f (file name), $g (file name without the extension), \
          $/ (full path with /), $N (short path), $b (extension), $e (folder), $E (short \
          folder), $n (number of an untitled window), $B (document type), $Q (print setting), \
          $C (the selected text or the word at the caret), $x and $y (caret column in UTF-16 \
          units and line, from 1), $d and $t (date and time now), $D and $T (date and time of \
          the file), $V (version), $h (Grep key), $S (the editor), $I (the settings file), \
          $M (the macro file), $<profile>, and conditions ${R?a$:b$:c$} (R, w, M, U, N, I). A \
          text without a file is (無題). $p and $P are the notice that they are for the print \
          preview. At most 2047 UTF-16 units.",
    run: expand_parameter,
}];

#[cfg(test)]
mod tests {
    use matsu_print::{HeaderContext, WindowKind, WriteState};
    use serde_json::json;

    use super::super::testing::Fixture;
    use super::*;

    /// The template a window gives: every field `ExpandParameter` reads that a macro cannot
    /// change.
    fn template() -> HeaderContext {
        HeaderContext {
            untitled_number: Some(3),
            type_name: Some("C/C++".to_owned()),
            file_date: Some("2026/11/10".to_owned()),
            file_time: Some("12:34:56".to_owned()),
            grep_key: "検索語".to_owned(),
            ini_path: "/home/u/matsu.ini".to_owned(),
            profile: "work".to_owned(),
            print_setting_name: "設定 1".to_owned(),
            ..HeaderContext::default()
        }
    }

    fn fixture(path: Option<&str>) -> Fixture {
        let mut f = Fixture::new("abc 日本語 def\nsecond");
        f.path = path.map(Into::into);
        f.env.macro_path = Some("/m/run.mjs".to_owned());
        f.env.header = Some(template());
        f
    }

    fn expand_in(f: &mut Fixture, format: &str) -> String {
        f.ok("ExpandParameter", json!([format]))
            .as_str()
            .unwrap()
            .to_owned()
    }

    /// What the macro function should read: the header context the title would be given.
    fn header_context(f: &Fixture) -> HeaderContext {
        HeaderContext {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            file_path: f.path.as_ref().map(|p| p.to_string_lossy().into_owned()),
            exe_path: std::env::current_exe()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            macro_path: f.env.macro_path.clone(),
            caret_x: 1,
            caret_y: 1,
            ..template()
        }
    }

    #[test]
    fn every_symbol_is_what_the_header_path_gives() {
        let path = if cfg!(windows) {
            "C:\\work\\日本\\memo.v2.txt"
        } else {
            "/work/日本/memo.v2.txt"
        };
        for path in [Some(path), None] {
            let mut f = fixture(path);
            let header = header_context(&f);
            for code in [
                "$$",
                "$A",
                "$F",
                "$f",
                "$g",
                "$/",
                "$N",
                "$n",
                "$E",
                "$e",
                "$B",
                "$b",
                "$Q",
                "$x",
                "$y",
                "$D",
                "$T",
                "$V",
                "$h",
                "$S",
                "$I",
                "$M",
                "$<profile>",
                "$p",
                "$P",
            ] {
                assert_eq!(
                    expand_in(&mut f, code),
                    matsu_print::expand_parameter(code, &header, None),
                    "{code} for {path:?}"
                );
            }
        }
    }

    #[test]
    fn the_codes_that_were_missing_have_a_value() {
        let mut f = fixture(Some("/work/memo.txt"));
        assert_eq!(expand_in(&mut f, "$B"), "C/C++");
        assert_eq!(expand_in(&mut f, "$Q"), "設定 1");
        assert_eq!(expand_in(&mut f, "$D $T"), "2026/11/10 12:34:56");
        assert_eq!(expand_in(&mut f, "$I"), "/home/u/matsu.ini");
        assert_eq!(expand_in(&mut f, "$h"), "検索語");
        assert_eq!(expand_in(&mut f, "[$<profile>]"), "[work]");
        // an untitled document has a number; one with a file has none
        assert_eq!(expand_in(&mut f, "[$n]"), "[]");
        let mut f = fixture(None);
        assert_eq!(expand_in(&mut f, "$f$n"), "(無題)3");
        // a file without a time stamp
        f.env.header = Some(HeaderContext::default());
        assert_eq!(
            expand_in(&mut f, "$D|$T"),
            "(保存されていません)|(保存されていません)"
        );
        // without a window the codes are empty, `$B` too
        f.env.header = None;
        assert_eq!(expand_in(&mut f, "[$B$h$n$<profile>$I$Q]"), "[]");
        // the program's own values
        assert_eq!(expand_in(&mut f, "$A"), "MATSU Editor");
        assert_eq!(expand_in(&mut f, "$V"), env!("CARGO_PKG_VERSION"));
        assert_eq!(expand_in(&mut f, "$M"), "/m/run.mjs");
        assert_eq!(expand_in(&mut f, "a$$b"), "a$b");
    }

    #[test]
    fn the_date_and_the_time_are_now() {
        use matsu_commands::convert::datetime::{Clock, SystemClock, format_date, format_time};
        let options = matsu_commands::format_options::get();
        let mut f = fixture(None);
        let before = SystemClock.now();
        let text = expand_in(&mut f, "$d|$t");
        let after = SystemClock.now();
        // The clock may pass a second (or midnight) between the reads: accept either end.
        let ends = [&before, &after].map(|t| {
            format!(
                "{}|{}",
                format_date(t, options.date_picture()),
                format_time(t, options.time_picture())
            )
        });
        let (date, time) = text.split_once('|').unwrap();
        assert!(!date.is_empty() && !time.is_empty(), "{text}");
        let date_ends = ends
            .clone()
            .map(|e| e.split('|').next().unwrap().to_owned());
        assert!(date_ends.iter().any(|d| d == date), "{text} {ends:?}");
    }

    #[test]
    fn unknown_codes_stay_as_written() {
        let mut f = fixture(None);
        assert_eq!(expand_in(&mut f, "$z $Z"), "$z $Z");
        assert_eq!(expand_in(&mut f, "日本語"), "日本語");
        // a lone $ at the end, as the header and the title read it
        assert_eq!(expand_in(&mut f, "a$"), "a$");
    }

    #[test]
    fn short_names_come_from_the_window() {
        let mut f = fixture(Some("/home/u/work/memo.txt"));
        // without the window's function: the full path and folder
        assert_eq!(
            expand_in(&mut f, "$N|$E"),
            "/home/u/work/memo.txt|/home/u/work/"
        );
        f.env.short_name = Some(Box::new(|path| format!("<{path}>")));
        assert_eq!(
            expand_in(&mut f, "$N|$E|$e"),
            "</home/u/work/memo.txt>|</home/u/work/>|/home/u/work/"
        );
        // untitled
        f.path = None;
        assert_eq!(expand_in(&mut f, "$N|$E"), "(無題)|(無題)");
    }

    #[test]
    fn the_position_and_the_selection() {
        let mut f = fixture(Some("/tmp/メモ.txt"));
        f.caret_at((0, 4));
        // the word at the caret (日本語 is one word), column in UTF-16 units: 'a','b','c',' ' and 日
        assert_eq!(expand_in(&mut f, "$C"), "日本語");
        assert_eq!(expand_in(&mut f, "$y,$x"), "1,5");
        assert_eq!(expand_in(&mut f, "$f"), "メモ.txt");
        // a selection: its first line only
        f.select((0, 4), (1, 3));
        assert_eq!(expand_in(&mut f, "$C"), "日本語 def");
        assert_eq!(expand_in(&mut f, "$y"), "2");
        assert!(f.call("ExpandParameter", json!([])).is_err());
    }

    #[test]
    fn conditions_choose_a_part() {
        let mut f = fixture(None);
        // U: modified is part 0
        let modified = "${U?*$:-$}";
        assert_eq!(expand_in(&mut f, modified), "-");
        f.model.mark_unsaved();
        assert_eq!(expand_in(&mut f, modified), "*");
        // M: recording is part 0; a part that is not there is empty
        assert_eq!(expand_in(&mut f, "[${M?rec$}]"), "[]");
        f.env.recording_key_macro = true;
        assert_eq!(expand_in(&mut f, "[${M?rec$}]"), "[rec]");
        // N: untitled is part 0
        assert_eq!(expand_in(&mut f, "${N?new$:$f$}"), "new");
        f.path = Some("/a.txt".into());
        assert_eq!(expand_in(&mut f, "${N?new$:$f$}"), "a.txt");
        // I is never minimized
        assert_eq!(expand_in(&mut f, "${I?min$:shown$}"), "shown");
    }

    #[test]
    fn the_condition_r_has_three_cases() {
        let source = "${R?view$:locked$:normal$}";
        let mut f = fixture(None);
        assert_eq!(expand_in(&mut f, source), "normal");
        // a file that cannot be overwritten (the window knows it)
        f.env.header.as_mut().unwrap().write_state = WriteState::Locked;
        assert_eq!(expand_in(&mut f, source), "locked");
        // the view mode is read from the editor of the call, and wins
        f.view_mode = true;
        assert_eq!(expand_in(&mut f, source), "view");
        // the title's own default mark
        f.view_mode = false;
        assert_eq!(
            expand_in(&mut f, "${R?(ビューモード)$:(上書き禁止)$}"),
            "(上書き禁止)"
        );
    }

    #[test]
    fn the_condition_w_chooses_the_kind_of_window() {
        let source = "${w?grep$:out$:normal$}";
        let mut f = fixture(None);
        assert_eq!(expand_in(&mut f, source), "normal");
        for (kind, expected) in [(WindowKind::Grep, "grep"), (WindowKind::Output, "out")] {
            f.env.header.as_mut().unwrap().window = kind;
            assert_eq!(expand_in(&mut f, source), expected);
        }
        // the title of a Grep window: the key
        f.env.header.as_mut().unwrap().window = WindowKind::Grep;
        assert_eq!(expand_in(&mut f, "${w?$h$:out$:$f$}"), "検索語");
    }

    #[test]
    fn conditions_nest() {
        let mut f = fixture(None);
        f.model.mark_unsaved();
        let source = "${U?${M?a$:b$}$:c$}!";
        // modified (part 0), not recording (part 1 of the inner one)
        assert_eq!(expand_in(&mut f, source), "b!");
        f.env.recording_key_macro = true;
        assert_eq!(expand_in(&mut f, source), "a!");
        // an unclosed condition does not hang (the document is modified: part 0 is chosen)
        assert_eq!(expand_in(&mut f, "x${U?a"), "xa");
        assert_eq!(expand_in(&mut f, "x${U"), "x");
    }

    #[test]
    fn the_result_is_cut_at_2047_units() {
        let mut f = fixture(None);
        let long = "あ".repeat(3000);
        assert_eq!(utf16_len(&expand_in(&mut f, &long)), 2047);
        let emoji = "😀".repeat(2000);
        // 2047 units hold 1023 pairs, not a cut pair
        assert_eq!(expand_in(&mut f, &emoji).chars().count(), 1023);
    }

    #[test]
    fn a_long_selection_is_read_only_as_far_as_the_first_line_needs() {
        let long = "あ".repeat(100_000);
        let mut f = Fixture::new(&format!("{long}\nsecond"));
        f.select((0, 0), (1, 3));
        let text = f.ok("ExpandParameter", json!(["$C"]));
        assert_eq!(text, json!("あ".repeat(259)));
        // a selection that starts in the middle of the line, and one inside a line
        f.select((0, 6), (1, 3));
        assert_eq!(
            f.ok("ExpandParameter", json!(["$C"])),
            json!("あ".repeat(259))
        );
        f.select((0, 6), (0, 12));
        assert_eq!(f.ok("ExpandParameter", json!(["$C"])), json!("ああ"));
        // other codes do not read the selection at all
        assert_eq!(f.ok("ExpandParameter", json!(["$y"])), json!("1"));
        // 4-byte characters are cut by units: 259 units hold 129 of them
        let mut f = Fixture::new(&"😀".repeat(1000));
        f.select((0, 0), (0, 4000));
        assert_eq!(
            f.ok("ExpandParameter", json!(["$C"])),
            json!("😀".repeat(129))
        );
    }

    #[test]
    fn a_box_selection_gives_its_first_row() {
        use matsu_core::buffer::Pos;
        let mut f = Fixture::new("abcdef\nabcdef\nabcdef");
        f.cursor.move_to(Pos::new(0, 1), false);
        f.cursor.begin_box(1, false);
        f.cursor.move_box_to(Pos::new(1, 3), 3);
        assert_eq!(f.ok("ExpandParameter", json!(["[$C]"])), json!("[bc]"));
    }
}
