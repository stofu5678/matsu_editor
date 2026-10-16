// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/env/CSakuraEnvironment.cpp (ExpandParameter, _ExParam_SkipCond,
//   _ExParam_Evaluate)
//   sakura_core/view/CEditView_Search.cpp (GetCurrentTextForSearch, for `$C`)
//
// Copyright (C) 2008, kobake, ryoji
// Copyright (C) 2012, Uchi
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (wide character
// buffers and pointers) was rewritten in Rust on character vectors, the codes that need
// something this editor does not have are left out (see the list below), and the tests are new.
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

//! `ExpandParameter`: Sakura's format string with `$F`, `$f`, `$g`, ... (the help of
//! `S_ExpandParameter`).
//!
//! # What is supported
//!
//! `$$` `$A` `$F` `$f` `$g` `$/` `$N` `$b` `$e` `$E` `$n` `$C` `$x` `$y` `$p` `$P` `$V` `$S` `$M`
//! `$h` `$<profile>`, and the conditions `${R?..$:..$:..$}`, `${w?...}`, `${M?...}`, `${U?...}`,
//! `${N?...}`, `${I?...}`.
//!
//! # What is not, and what it does instead
//!
//! * `$d` `$t` `$D` `$T` (the date and the time in the common settings' format), `$B` (the name
//!   of the document type), `$Q` (the print setting), `$I` (the path of the settings file) are
//!   not supported: they stay in the result as written, which is what Sakura does with a letter
//!   it does not know.
//! * `$N` and `$E` show the path in full: Sakura's short form (`GetTransformFileNameFast`, the
//!   "common settings: file name display") does not exist here.
//! * `$p` and `$P` are the text for outside the print preview; a macro does not run in it.
//! * `$h` is empty (no Grep window) and `$<profile>` is empty (no profile name).
//! * `${R?}` has no read-only attribute of the file to test: it is 0 in view mode and 2 (neither)
//!   otherwise. `${w?}` is always 2 (a normal window). `${I?}` is 1: a window is never minimized
//!   while a macro runs.

use serde_json::Value;

use super::{EditorFunc, command, utf16_len};
use crate::api::{Args, Ty, req};
use crate::host::Editor;
use crate::protocol::RpcError;

type Result<T = Value> = std::result::Result<T, RpcError>;

/// The size of Sakura's buffer: `ExpandParameter(Source, Buffer, 2047)`.
const BUFFER_LEN: usize = 2047;
/// `STR_NO_TITLE1`.
const NO_TITLE: &str = "(無題)";
/// `STR_PREVIEW_ONLY`.
const PREVIEW_ONLY: &str = "(印刷プレビューでのみ使用できます)";
/// The name of the application (`GSTR_APPNAME_W` is "sakura" there).
const APP_NAME: &str = "UME Editor";
/// The most that `$C` takes: `_MAX_PATH - 1` characters (`LimitStringLengthW`).
const CURRENT_TEXT_MAX: usize = 259;

/// What `$x` and the like read from the editor.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Context {
    pub path: Option<String>,
    pub macro_path: Option<String>,
    pub exe_path: String,
    /// `$C`: the selected text up to its first line break, or the word at the caret.
    pub current_text: String,
    /// `$x`: the logical column of the caret in UTF-16 units, counted from 1.
    pub column: usize,
    /// `$y`: the logical line of the caret, counted from 1.
    pub line: usize,
    pub view_mode: bool,
    pub recording: bool,
    pub modified: bool,
}

/// The directory separators of a path: Sakura's are the backslash only; the slash is one too
/// here.
fn is_separator(c: char) -> bool {
    c == '\\' || c == '/'
}

fn file_name(path: &str) -> &str {
    path.rsplit(is_separator).next().unwrap_or(path)
}

/// `$g`: the file name without the last extension. A name that starts with a dot and has no
/// other dot, such as `.bashrc`, has none.
fn name_without_extension(name: &str) -> &str {
    match name.rfind('.') {
        Some(dot) if dot > 0 => &name[..dot],
        _ => name,
    }
}

/// `$b`: the extension of the file name, without the dot, or nothing.
fn extension(name: &str) -> &str {
    name.rfind('.').map_or("", |dot| &name[dot + 1..])
}

/// `_ExParam_Evaluate`: the number of the branch that a condition chooses.
fn evaluate(condition: Option<char>, cx: &Context) -> i32 {
    match condition {
        // read-only view mode, a file that cannot be overwritten, or neither
        Some('R') => {
            if cx.view_mode {
                0
            } else {
                2
            }
        }
        // grep window, output window, or a normal window
        Some('w') => 2,
        // recording a key macro
        Some('M') => {
            if cx.recording {
                0
            } else {
                1
            }
        }
        // modified
        Some('U') => {
            if cx.modified {
                0
            } else {
                1
            }
        }
        // untitled
        Some('N') => {
            if cx.path.is_none() {
                0
            } else {
                1
            }
        }
        // minimized
        Some('I') => 1,
        _ => 0,
    }
}

/// `_ExParam_SkipCond`: from `start` (just after `?`, or after a `$:`), the index of the beginning
/// of part number `part` of the condition, or just after the closing `$}` if there is no such
/// part. `-1` skips to the end. A nested `${` ... `$}` is skipped as a whole.
fn skip_condition(chars: &[char], start: usize, mut part: i32) -> usize {
    if part == 0 {
        return start;
    }
    let mut nest = 0;
    let mut p = start;
    let mut next = true;
    while next && p < chars.len() {
        if chars[p] == '$' && p + 1 < chars.len() {
            p += 1;
            match chars[p] {
                '{' => nest += 1,
                '}' => {
                    if nest == 0 {
                        next = false;
                    } else {
                        nest -= 1;
                    }
                }
                ':' if nest == 0 => {
                    part -= 1;
                    if part == 0 {
                        next = false;
                    }
                }
                _ => {}
            }
        }
        p += 1;
    }
    p
}

/// `CSakuraEnvironment::ExpandParameter`.
pub(crate) fn expand(source: &str, cx: &Context) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::new();
    let mut p = 0;
    let path = cx.path.as_deref();
    let name = path.map(file_name);
    // The text for a file-dependent code: the value, or "(無題)" for a text without a file.
    let or_untitled = |f: &dyn Fn(&str) -> String| path.map_or_else(|| NO_TITLE.to_owned(), f);
    while p < chars.len() && utf16_len(&out) < BUFFER_LEN {
        if chars[p] != '$' {
            out.push(chars[p]);
            p += 1;
            continue;
        }
        p += 1;
        // A `$` at the end of the string: Sakura reads the end mark as the code and stops.
        let Some(&code) = chars.get(p) else {
            break;
        };
        match code {
            '$' => {
                out.push('$');
                p += 1;
            }
            'A' => {
                out.push_str(APP_NAME);
                p += 1;
            }
            'F' | 'N' => {
                out.push_str(&or_untitled(&|path| path.to_owned()));
                p += 1;
            }
            'f' => {
                out.push_str(&or_untitled(&|path| file_name(path).to_owned()));
                p += 1;
            }
            'g' => {
                out.push_str(&or_untitled(&|path| {
                    name_without_extension(file_name(path)).to_owned()
                }));
                p += 1;
            }
            '/' => {
                out.push_str(&or_untitled(&|path| path.replace('\\', "/")));
                p += 1;
            }
            'n' => {
                // The number of an untitled window: this editor has none.
                p += 1;
            }
            'E' | 'e' => {
                out.push_str(&or_untitled(&|path| {
                    // The folder with its last separator, or nothing for a bare name.
                    path.rfind(is_separator)
                        .map_or_else(String::new, |i| path[..=i].to_owned())
                }));
                p += 1;
            }
            'b' => {
                if let Some(name) = name {
                    out.push_str(extension(name));
                }
                p += 1;
            }
            'C' => {
                out.push_str(&cx.current_text);
                p += 1;
            }
            'x' => {
                out.push_str(&cx.column.to_string());
                p += 1;
            }
            'y' => {
                out.push_str(&cx.line.to_string());
                p += 1;
            }
            'p' | 'P' => {
                out.push_str(PREVIEW_ONLY);
                p += 1;
            }
            'V' => {
                out.push_str(env!("CARGO_PKG_VERSION"));
                p += 1;
            }
            'h' => {
                // The Grep key: there is no Grep window.
                p += 1;
            }
            'S' => {
                out.push_str(&cx.exe_path);
                p += 1;
            }
            'M' => {
                if let Some(path) = &cx.macro_path {
                    out.push_str(path);
                }
                p += 1;
            }
            '{' => {
                // ${c?A0$:A1$:A2$}
                let choice = evaluate(chars.get(p + 1).copied(), cx);
                while p < chars.len() && chars[p] != '?' {
                    p += 1;
                }
                if p >= chars.len() {
                    break;
                }
                p = skip_condition(&chars, p + 1, choice);
            }
            ':' => {
                // The end of the chosen part: skip to the end of the condition.
                p = skip_condition(&chars, p + 1, -1);
            }
            '}' => {
                p += 1;
            }
            '<' => {
                // $<LongName>: only `profile`, which is empty.
                p += 1;
                while p < chars.len() && chars[p] != '>' {
                    p += 1;
                }
                if p >= chars.len() {
                    break;
                }
                p += 1;
            }
            other => {
                // A code that is not known stays as it was written.
                out.push('$');
                out.push(other);
                p += 1;
            }
        }
    }
    truncate_utf16(out, BUFFER_LEN)
}

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
    doc: &ume_core::buffer::Document,
    from: ume_core::buffer::Pos,
    to: ume_core::buffer::Pos,
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
        if let Ok(text) = doc.text_between(from, ume_core::buffer::Pos::new(from.line, end)) {
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

/// F_EXPANDPARAMETER.
fn expand_parameter(ed: &mut Editor, args: &Args) -> Result {
    let caret = ed.cx.caret();
    // `$x` is the number of UTF-16 units before the caret, plus one. Only that format reads the
    // text in front of the caret.
    let column = if args.str(0).contains("$x") {
        let before = ed
            .cx
            .doc()
            .text_between(ume_core::buffer::Pos::new(caret.line, 0), caret)
            .map_err(command)?;
        utf16_len(&before) + 1
    } else {
        0
    };
    let context = Context {
        path: ed.cx.path().map(|p| p.to_string_lossy().into_owned()),
        macro_path: ed.env.macro_path.clone(),
        exe_path: std::env::current_exe()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
        // Only the format strings that have `$C` read the text.
        current_text: if args.str(0).contains("$C") {
            current_text(ed)?
        } else {
            String::new()
        },
        column,
        line: caret.line + 1,
        view_mode: ed.cx.is_view_mode(),
        recording: ed.env.recording_key_macro,
        modified: ed.cx.is_modified(),
    };
    Ok(expand(args.str(0), &context).into())
}

pub const FUNCS: &[EditorFunc] = &[EditorFunc {
    name: "ExpandParameter",
    params: &[req("format", Ty::Str)],
    result: Ty::Str,
    doc: "Replaces the codes that start with $ in a string: $$ ($), $F (full path), $f (file \
          name), $g (file name without the extension), $/ (full path with /), $N (as $F), \
          $b (extension), $e and $E (folder), $C (the selected text or the word at the caret), \
          $x and $y (caret column in UTF-16 units and line, from 1), $S (the editor), \
          $M (the macro file), $V (version), $A (name), and conditions ${R?a$:b$:c$} (R, w, M, \
          U, N, I). A text without a file is (無題). $d $t $D $T $B $Q $I are not supported \
          and stay as written. At most 2047 UTF-16 units.",
    run: expand_parameter,
}];

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::super::testing::Fixture;
    use super::*;

    fn file_context() -> Context {
        Context {
            path: Some("C:\\work\\日本\\memo.v2.txt".to_owned()),
            macro_path: Some("/m/run.mjs".to_owned()),
            exe_path: "/bin/ume".to_owned(),
            current_text: "語".to_owned(),
            column: 5,
            line: 7,
            ..Context::default()
        }
    }

    #[test]
    fn the_file_codes() {
        let cx = file_context();
        assert_eq!(expand("$F", &cx), "C:\\work\\日本\\memo.v2.txt");
        assert_eq!(expand("$f", &cx), "memo.v2.txt");
        assert_eq!(expand("$g", &cx), "memo.v2");
        assert_eq!(expand("$/", &cx), "C:/work/日本/memo.v2.txt");
        assert_eq!(expand("$b", &cx), "txt");
        assert_eq!(expand("$e", &cx), "C:\\work\\日本\\");
        assert_eq!(expand("$E", &cx), "C:\\work\\日本\\");
        assert_eq!(expand("$N", &cx), expand("$F", &cx));
    }

    #[test]
    fn a_text_without_a_file_is_untitled() {
        let cx = Context::default();
        for code in ["$F", "$f", "$g", "$/", "$N", "$e", "$E"] {
            assert_eq!(expand(code, &cx), "(無題)", "{code}");
        }
        assert_eq!(expand("[$b]", &cx), "[]");
        assert_eq!(expand("[$M]", &cx), "[]");
    }

    #[test]
    fn names_with_dots() {
        let mut cx = Context {
            path: Some("/home/u/.bashrc".to_owned()),
            ..Context::default()
        };
        assert_eq!(expand("$g", &cx), ".bashrc");
        assert_eq!(expand("$b", &cx), "bashrc");
        cx.path = Some("README".to_owned());
        assert_eq!(expand("$g", &cx), "README");
        assert_eq!(expand("$b", &cx), "");
        assert_eq!(expand("$e", &cx), "");
    }

    #[test]
    fn the_position_text_and_paths() {
        let cx = file_context();
        assert_eq!(expand("$y:$x", &cx), "7:5");
        assert_eq!(expand("$C", &cx), "語");
        assert_eq!(expand("$M", &cx), "/m/run.mjs");
        assert_eq!(expand("$S", &cx), "/bin/ume");
        assert_eq!(expand("$A", &cx), "UME Editor");
        assert_eq!(expand("$V", &cx), env!("CARGO_PKG_VERSION"));
        assert_eq!(
            expand("$p/$P", &cx),
            format!("{PREVIEW_ONLY}/{PREVIEW_ONLY}")
        );
        assert_eq!(expand("a$$b$$$$c", &cx), "a$b$$c");
        assert_eq!(expand("$h$n$<profile>.", &cx), ".");
    }

    #[test]
    fn codes_that_are_not_known_stay_as_written() {
        let cx = Context::default();
        assert_eq!(
            expand("$d $t $D $T $B $Q $I $z", &cx),
            "$d $t $D $T $B $Q $I $z"
        );
        // a lone $ at the end ends the string
        assert_eq!(expand("a$", &cx), "a");
        assert_eq!(expand("日本語", &cx), "日本語");
    }

    #[test]
    fn conditions_choose_a_part() {
        let mut cx = Context::default();
        // U: modified is part 0
        let source = "${U?*$:-$}";
        assert_eq!(expand(source, &cx), "-");
        cx.modified = true;
        assert_eq!(expand(source, &cx), "*");
        // R has three parts
        let source = "${R?view$:locked$:normal$}";
        assert_eq!(expand(source, &cx), "normal");
        cx.view_mode = true;
        assert_eq!(expand(source, &cx), "view");
        // a part that is not there is empty; text before and after stays
        assert_eq!(expand("[${M?rec$}]", &cx), "[]");
        cx.recording = true;
        assert_eq!(expand("[${M?rec$}]", &cx), "[rec]");
        // N: untitled is part 0
        assert_eq!(expand("${N?new$:$f$}", &cx), "new");
        cx.path = Some("a.txt".to_owned());
        assert_eq!(expand("${N?new$:$f$}", &cx), "a.txt");
        // w and I
        assert_eq!(expand("${w?grep$:out$:normal$}", &cx), "normal");
        assert_eq!(expand("${I?min$:shown$}", &cx), "shown");
    }

    #[test]
    fn conditions_nest() {
        let mut cx = Context {
            modified: true,
            ..Context::default()
        };
        let source = "${U?${M?a$:b$}$:c$}!";
        // modified (part 0), not recording (part 1 of the inner one)
        assert_eq!(expand(source, &cx), "b!");
        cx.recording = true;
        assert_eq!(expand(source, &cx), "a!");
        cx.modified = false;
        assert_eq!(expand(source, &cx), "c!");
        // an unclosed condition does not hang
        assert_eq!(expand("x${U?a", &cx), "x");
        assert_eq!(expand("x${U", &cx), "x");
    }

    #[test]
    fn the_result_is_cut_at_2047_units() {
        let cx = Context::default();
        let long = "あ".repeat(3000);
        assert_eq!(utf16_len(&expand(&long, &cx)), 2047);
        let emoji = "😀".repeat(2000);
        // 2047 units hold 1023 pairs, not a cut pair
        assert_eq!(expand(&emoji, &cx).chars().count(), 1023);
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
        use ume_core::buffer::Pos;
        let mut f = Fixture::new("abcdef\nabcdef\nabcdef");
        f.cursor.move_to(Pos::new(0, 1), false);
        f.cursor.begin_box(1, false);
        f.cursor.move_box_to(Pos::new(1, 3), 3);
        assert_eq!(f.ok("ExpandParameter", json!(["[$C]"])), json!("[bc]"));
    }

    #[test]
    fn the_function_reads_the_editor() {
        let mut f = Fixture::new("abc 日本語 def\nsecond");
        f.path = Some("/tmp/メモ.txt".into());
        f.env.macro_path = Some("/m/x.js".to_owned());
        f.caret_at((0, 4));
        // the word at the caret (日本語 is one word), column in UTF-16 units: 'a','b','c',' ' and 日
        assert_eq!(f.ok("ExpandParameter", json!(["$C"])), json!("日本語"));
        assert_eq!(f.ok("ExpandParameter", json!(["$y,$x"])), json!("1,5"));
        assert_eq!(f.ok("ExpandParameter", json!(["$f"])), json!("メモ.txt"));
        assert_eq!(f.ok("ExpandParameter", json!(["$M"])), json!("/m/x.js"));
        // a selection: its first line only
        f.select((0, 4), (1, 3));
        assert_eq!(f.ok("ExpandParameter", json!(["$C"])), json!("日本語 def"));
        assert_eq!(f.ok("ExpandParameter", json!(["${U?m$:u$}"])), json!("u"));
        assert!(f.call("ExpandParameter", json!([])).is_err());
    }
}
