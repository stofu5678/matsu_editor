// Derived from Sakura Editor (zlib License):
//   sakura_core/env/CSakuraEnvironment.cpp (ExpandParameter, _ExParam_SkipCond,
//     _ExParam_Evaluate, ExParam_LongName)
//   sakura_core/util/string_ex2.cpp (LimitStringLengthW)
//   help/sakura/res/HLP000284.html (the list of the special characters)
// Modified: rewritten in Rust. The state of the editor that Sakura reads from its global
// objects is given in a `HeaderContext`, and the date and the time come as text, formatted by
// the caller.
//
// Copyright (C) 2018-2022, Sakura Editor Organization
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

//! The special characters of the header and the footer: `$f`, `$p`, `${U?...$:...$}` and the
//! like (`ExpandParameter`).

/// Sakura's `STR_NO_TITLE1`.
const NO_TITLE: &str = "(無題)";
/// Sakura's `STR_NOT_SAVED`.
const NOT_SAVED: &str = "(保存されていません)";
/// Sakura's `STR_PREVIEW_ONLY`.
const PREVIEW_ONLY: &str = "(印刷プレビューでのみ使用できます)";

/// What kind of window the document is in (`$w`, `$n`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowKind {
    /// An ordinary editing window.
    #[default]
    Normal,
    /// A Grep result window.
    Grep,
    /// An output window (the debug mode).
    Output,
}

/// How the file can be written (`$R`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WriteState {
    /// Neither of the others.
    #[default]
    Writable,
    /// Set to the view mode.
    ViewMode,
    /// Locked: read-only, or opened by another application with a lock.
    Locked,
}

/// The state of the document and the editor that the special characters show. Sakura reads it
/// from the global objects when it expands the text; here the caller gives it.
///
/// The paths are in the form of the OS; `\` and `/` both separate the names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeaderContext {
    /// `$A`.
    pub app_name: String,
    /// The full path of the file, or none for a new document (`$F`, `$f`, `$g`, `$/`, `$e`,
    /// `$b`).
    pub file_path: Option<String>,
    /// The path in the short form that Sakura makes with the font of the caption (`$N`). Sakura
    /// shortens it to fit the width; when none is given, the full path is used.
    pub file_path_short: Option<String>,
    /// The folder in the short form (`$E`). When none is given, the full folder is used.
    pub folder_short: Option<String>,
    /// The serial number of a new document (`$n`).
    pub untitled_number: Option<u32>,
    /// The name of the document type, or none for the basic type, which Sakura leaves out
    /// (`$B`).
    pub type_name: Option<String>,
    /// The first line of the selected text, or the word at the caret (`$C`).
    pub selected_text: String,
    /// The column of the caret in bytes from the start of the line, from 1 (`$x`, Sakura's
    /// `GetCaretLogicPos().x + 1`).
    pub caret_x: usize,
    /// The line of the caret, from 1 (`$y`).
    pub caret_y: usize,
    /// Today's date in the format of the common settings (`$d`).
    pub date: String,
    /// The time now in the format of the common settings (`$t`).
    pub time: String,
    /// The date of the file's time stamp, or none for a file not saved (`$D`).
    pub file_date: Option<String>,
    /// The time of the file's time stamp (`$T`).
    pub file_time: Option<String>,
    /// The version of the editor (`$V`).
    pub version: String,
    /// The key of the Grep search (`$h`).
    pub grep_key: String,
    /// The path of the editor's executable (`$S`).
    pub exe_path: String,
    /// The path of the settings file (`$I`).
    pub ini_path: String,
    /// The path of the macro running now (`$M`), or none.
    pub macro_path: Option<String>,
    /// The name of the profile, empty for the default one (`$<profile>`).
    pub profile: String,
    /// `${R?...}`.
    pub write_state: WriteState,
    /// The document has changes not saved (`${U?...}`).
    pub modified: bool,
    /// A keyboard macro is being recorded in this window (`${M?...}`).
    pub recording_macro: bool,
    /// `${w?...}`.
    pub window: WindowKind,
}

impl Default for HeaderContext {
    fn default() -> Self {
        Self {
            app_name: "MATSU Editor".into(),
            file_path: None,
            file_path_short: None,
            folder_short: None,
            untitled_number: None,
            type_name: None,
            selected_text: String::new(),
            caret_x: 1,
            caret_y: 1,
            date: String::new(),
            time: String::new(),
            file_date: None,
            file_time: None,
            version: String::new(),
            grep_key: String::new(),
            exe_path: String::new(),
            ini_path: String::new(),
            macro_path: None,
            profile: String::new(),
            write_state: WriteState::Writable,
            modified: false,
            recording_macro: false,
            window: WindowKind::Normal,
        }
    }
}

/// The page that is being printed, for `$p`, `$P` and `$Q`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageInfo<'a> {
    /// The page number from 1 (`$p`).
    pub number: usize,
    /// The number of pages (`$P`).
    pub total: usize,
    /// The name of the print setting (`$Q`).
    pub setting_name: &'a str,
}

/// Expands the special characters in `form` (`CSakuraEnvironment::ExpandParameter`).
///
/// | | |
/// |---|---|
/// | `$$` | `$` |
/// | `$A` | the name of the application |
/// | `$F` `$f` `$g` `$/` | the file's full path, name, name without the extension, full path with `/`; `(無題)` for a new document |
/// | `$N` `$E` `$e` | the short path, the short folder, the folder |
/// | `$n` `$B` `$b` `$Q` | the number of a new document, the document type, the extension, the print setting |
/// | `$C` `$x` `$y` | the selected text, the caret's column and line |
/// | `$d` `$t` `$D` `$T` | the date and time now, and of the file; `(保存されていません)` for a file not saved |
/// | `$p` `$P` | the page and the number of pages (`(印刷プレビューでのみ使用できます)` when `page` is `None`) |
/// | `$V` `$h` `$S` `$I` `$M` `$<profile>` | the version, the Grep key (32 characters), the paths of the application, the settings file and the macro, the profile |
/// | `${R?a$:b$:c$}` | the part chosen by the condition `R` (view mode, locked, other), `U` (changed, not), `M` (recording, not), `w` (Grep, output, other), `N` (new, not) or `I` (always the second part) |
///
/// Other `$` sequences stay as they are. The result is cut at 1024 UTF-16 units, like Sakura's
/// buffer (`DrawHeaderFooter` gives `nWorkLen` 1024), without splitting a character.
pub fn expand_parameter(form: &str, ctx: &HeaderContext, page: Option<&PageInfo<'_>>) -> String {
    let mut out = expand(form, ctx, page);
    limit_units(&mut out, MAX_EXPANDED_UNITS);
    out
}

/// The most UTF-16 units of an expanded text: the buffer of `DrawHeaderFooter` (`nWorkLen`).
const MAX_EXPANDED_UNITS: usize = 1024;

/// Cuts `text` after the last whole character within `max` UTF-16 units.
fn limit_units(text: &mut String, max: usize) {
    let mut units = 0;
    for (i, ch) in text.char_indices() {
        units += ch.len_utf16();
        if units > max {
            text.truncate(i);
            return;
        }
    }
}

fn expand(form: &str, ctx: &HeaderContext, page: Option<&PageInfo<'_>>) -> String {
    let src: Vec<char> = form.chars().collect();
    let mut out = String::new();
    let mut p = 0;
    while p < src.len() {
        if src[p] != '$' {
            out.push(src[p]);
            p += 1;
            continue;
        }
        p += 1;
        let Some(&c) = src.get(p) else {
            // A lone `$` at the end. (Sakura reads past the end of the text here.)
            out.push('$');
            break;
        };
        match c {
            '$' => {
                out.push('$');
                p += 1;
            }
            '{' => {
                // `${cond?part0$:part1$:...$}`: go to the chosen part. The text of the part is
                // expanded by this loop, and a `$:` at its end skips to the `$}`.
                let cond = evaluate(src.get(p + 1).copied(), ctx);
                while p < src.len() && src[p] != '?' {
                    p += 1;
                }
                if p >= src.len() {
                    break;
                }
                p = skip_cond(&src, p + 1, cond as i32);
            }
            ':' => p = skip_cond(&src, p + 1, -1),
            '}' => p += 1,
            '<' => {
                // `$<LongName>`
                p += 1;
                let begin = p;
                while p < src.len() && src[p] != '>' {
                    p += 1;
                }
                if p >= src.len() {
                    break;
                }
                let name: String = src[begin..p].iter().collect();
                if name.eq_ignore_ascii_case("profile") {
                    out.push_str(&ctx.profile);
                }
                p += 1;
            }
            _ => {
                match expand_one(c, ctx, page) {
                    Some(text) => out.push_str(&text),
                    None => {
                        out.push('$');
                        out.push(c);
                    }
                }
                p += 1;
            }
        }
    }
    out
}

/// The text of `$c`, or none when `c` is not a special character.
fn expand_one(c: char, ctx: &HeaderContext, page: Option<&PageInfo<'_>>) -> Option<String> {
    let path = ctx.file_path.as_deref();
    let or_no_title = |f: &dyn Fn(&str) -> String| path.map_or_else(|| NO_TITLE.to_string(), f);
    Some(match c {
        'A' => ctx.app_name.clone(),
        'F' => or_no_title(&|p| p.to_string()),
        'f' => or_no_title(&|p| file_name(p).to_string()),
        'g' => or_no_title(&|p| file_stem(file_name(p))),
        // A backslash is a separator only on Windows; elsewhere it is part of a name.
        '/' => or_no_title(&|p| {
            if cfg!(windows) {
                p.replace('\\', "/")
            } else {
                p.to_owned()
            }
        }),
        'N' => or_no_title(&|p| ctx.file_path_short.clone().unwrap_or_else(|| p.to_string())),
        // Only a document without a file, in an ordinary window, has a number.
        'n' => match (path, ctx.window, ctx.untitled_number) {
            (None, WindowKind::Normal, Some(n)) if n > 0 => n.to_string(),
            _ => String::new(),
        },
        'E' => or_no_title(&|p| {
            ctx.folder_short
                .clone()
                .unwrap_or_else(|| folder(p).to_string())
        }),
        'e' => or_no_title(&|p| folder(p).to_string()),
        // Sakura shows the name of the type unless it is the basic one.
        'B' => ctx.type_name.clone().unwrap_or_default(),
        'b' => path.map(extension).unwrap_or_default().to_string(),
        'Q' => page.map_or_else(String::new, |p| p.setting_name.to_string()),
        'C' => ctx.selected_text.clone(),
        'x' => ctx.caret_x.to_string(),
        'y' => ctx.caret_y.to_string(),
        'd' => ctx.date.clone(),
        't' => ctx.time.clone(),
        'p' => page.map_or_else(|| PREVIEW_ONLY.to_string(), |p| p.number.to_string()),
        'P' => page.map_or_else(|| PREVIEW_ONLY.to_string(), |p| p.total.to_string()),
        'D' => ctx.file_date.clone().unwrap_or_else(|| NOT_SAVED.into()),
        'T' => ctx.file_time.clone().unwrap_or_else(|| NOT_SAVED.into()),
        'V' => ctx.version.clone(),
        'h' => limit_grep_key(&ctx.grep_key),
        'S' => ctx.exe_path.clone(),
        'I' => ctx.ini_path.clone(),
        'M' => ctx.macro_path.clone().unwrap_or_default(),
        _ => return None,
    })
}

/// The first 32 UTF-16 units of the Grep key, whole characters only, and `...` when it was
/// cut (the code of `$h` with `LimitStringLengthW`).
fn limit_grep_key(key: &str) -> String {
    let mut units = 0;
    let mut out = String::new();
    for ch in key.chars() {
        units += ch.len_utf16();
        if units > 32 {
            out.push_str("...");
            break;
        }
        out.push(ch);
    }
    out
}

/// The name after the last separator (`GetFileName`).
fn file_name(path: &str) -> &str {
    path.rsplit(std::path::is_separator).next().unwrap_or(path)
}

/// The name without the extension: it ends at the last `.`, which is not the first character
/// (`$g`: "the dot at the head is not an extension").
fn file_stem(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut dot = chars.len().saturating_sub(1);
    while dot > 0 && chars[dot] != '.' {
        dot -= 1;
    }
    if dot == 0 {
        dot = chars.len();
    }
    chars[..dot].iter().collect()
}

/// The folder with its last separator (`$e`), or nothing for a name without one.
fn folder(path: &str) -> &str {
    path.rfind(std::path::is_separator)
        .map_or("", |i| &path[..=i])
}

/// The extension without the dot (`$b`); the whole name of `.profile` is one.
fn extension(path: &str) -> &str {
    let name = file_name(path);
    name.rfind('.').map_or("", |i| &name[i + 1..])
}

/// The number of the part that the condition chooses (`_ExParam_Evaluate`); 0 for an unknown
/// condition.
fn evaluate(cond: Option<char>, ctx: &HeaderContext) -> usize {
    match cond {
        Some('R') => match ctx.write_state {
            WriteState::ViewMode => 0,
            WriteState::Locked => 1,
            WriteState::Writable => 2,
        },
        Some('w') => match ctx.window {
            WindowKind::Grep => 0,
            WindowKind::Output => 1,
            WindowKind::Normal => 2,
        },
        Some('M') => usize::from(!ctx.recording_macro),
        Some('U') => usize::from(!ctx.modified),
        Some('N') => usize::from(ctx.file_path.is_some()),
        // The preview is never iconic; the window is "not active" only in a window of its own.
        Some('I') => 1,
        _ => 0,
    }
}

/// The index after the `part`-th `$:` of the condition that starts at `start`, or after its
/// `$}` when there are not so many parts; -1 skips to the `$}` (`_ExParam_SkipCond`). Nested
/// conditions are skipped whole.
fn skip_cond(src: &[char], start: usize, mut part: i32) -> usize {
    if part == 0 {
        return start;
    }
    let mut nest = 0;
    let mut p = start;
    let mut next = true;
    while next && p < src.len() {
        // A `$` at the very end is ignored.
        if src[p] == '$' && p + 1 < src.len() {
            p += 1;
            match src[p] {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> HeaderContext {
        HeaderContext {
            file_path: Some(r"D:\source\Main.cpp".into()),
            ..HeaderContext::default()
        }
    }

    fn ex(form: &str, ctx: &HeaderContext) -> String {
        let page = PageInfo {
            number: 2,
            total: 5,
            setting_name: "Setting 1",
        };
        expand_parameter(form, ctx, Some(&page))
    }

    #[cfg(windows)]
    #[test]
    fn file_names() {
        // The examples of HLP000284.html, with D:\source\Main.cpp.
        let c = ctx();
        assert_eq!(ex("$F", &c), r"D:\source\Main.cpp");
        assert_eq!(ex("$f", &c), "Main.cpp");
        assert_eq!(ex("$g", &c), "Main");
        assert_eq!(ex("$/", &c), "D:/source/Main.cpp");
        assert_eq!(ex("$e", &c), r"D:\source\");
        assert_eq!(ex("$b", &c), "cpp");
        // Without the short forms, the full ones stand in.
        assert_eq!(ex("$N", &c), r"D:\source\Main.cpp");
        assert_eq!(ex("$E", &c), r"D:\source\");
        let short = HeaderContext {
            file_path_short: Some(r"ソース\Main.cpp".into()),
            folder_short: Some(r"ソース\".into()),
            ..c
        };
        assert_eq!(ex("$N|$E", &short), r"ソース\Main.cpp|ソース\");
    }

    /// Elsewhere a backslash is a character of the name, not a separator.
    #[cfg(not(windows))]
    #[test]
    fn a_backslash_is_part_of_the_name_outside_windows() {
        let c = HeaderContext {
            file_path: Some(r"dir/a\b.txt".into()),
            ..HeaderContext::default()
        };
        assert_eq!(ex("$f", &c), r"a\b.txt");
        assert_eq!(ex("$g", &c), r"a\b");
        assert_eq!(ex("$b", &c), "txt");
        assert_eq!(ex("$e", &c), "dir/");
        assert_eq!(ex("$/", &c), r"dir/a\b.txt");
    }

    #[test]
    fn unix_paths_work_too() {
        let c = HeaderContext {
            file_path: Some("/home/me/a.b.txt".into()),
            ..HeaderContext::default()
        };
        assert_eq!(ex("$f $g $b $e", &c), "a.b.txt a.b txt /home/me/");
        assert_eq!(ex("$/", &c), "/home/me/a.b.txt");
    }

    #[test]
    fn a_dot_at_the_head_is_not_an_extension_for_g() {
        let c = HeaderContext {
            file_path: Some("/h/.profile".into()),
            ..HeaderContext::default()
        };
        assert_eq!(ex("$g", &c), ".profile");
        let none = HeaderContext {
            file_path: Some("/h/Makefile".into()),
            ..HeaderContext::default()
        };
        assert_eq!(ex("[$g][$b]", &none), "[Makefile][]");
    }

    #[test]
    fn a_new_document_has_no_title() {
        // The note of HLP000284.html: $F, $f, $g and $/ are (無題), and $D and $T are
        // (保存されていません).
        let c = HeaderContext::default();
        assert_eq!(ex("$F|$f|$g|$/", &c), "(無題)|(無題)|(無題)|(無題)");
        assert_eq!(ex("$D|$T", &c), "(保存されていません)|(保存されていません)");
        assert_eq!(ex("[$b][$e]", &c), "[][(無題)]");
    }

    #[test]
    fn serial_number_of_a_new_document() {
        let c = HeaderContext {
            untitled_number: Some(3),
            ..HeaderContext::default()
        };
        assert_eq!(ex("$n", &c), "3");
        let grep = HeaderContext {
            window: WindowKind::Grep,
            ..c.clone()
        };
        assert_eq!(ex("$n", &grep), "");
        // A saved file has none.
        assert_eq!(
            ex(
                "$n",
                &HeaderContext {
                    file_path: Some("a".into()),
                    ..c
                }
            ),
            ""
        );
    }

    #[test]
    fn pages_and_the_setting() {
        let c = ctx();
        assert_eq!(ex("- $p / $P - $Q", &c), "- 2 / 5 - Setting 1");
        // Outside the preview Sakura says so.
        assert_eq!(
            expand_parameter("$p", &c, None),
            "(印刷プレビューでのみ使用できます)"
        );
    }

    #[test]
    fn state_and_time() {
        let c = HeaderContext {
            date: "2026/10/15".into(),
            time: "09:30:00".into(),
            file_date: Some("2026/10/14".into()),
            file_time: Some("22:00:00".into()),
            version: "0.1.0".into(),
            exe_path: "/usr/bin/matsu".into(),
            ini_path: "/home/me/matsu.ini".into(),
            macro_path: Some("/m/a.js".into()),
            selected_text: "word".into(),
            caret_x: 7,
            caret_y: 42,
            profile: "work".into(),
            type_name: Some("C/C++".into()),
            ..ctx()
        };
        assert_eq!(
            ex("$d $t|$D $T", &c),
            "2026/10/15 09:30:00|2026/10/14 22:00:00"
        );
        assert_eq!(
            ex("$A $V $S $I $M", &c),
            "MATSU Editor 0.1.0 /usr/bin/matsu /home/me/matsu.ini /m/a.js"
        );
        assert_eq!(
            ex("$C($x,$y) $B $<profile> $<PROFILE> $<nothing>.", &c),
            "word(7,42) C/C++ work work ."
        );
    }

    #[test]
    fn grep_key_is_cut_at_32_units() {
        let c = HeaderContext {
            grep_key: "a".repeat(40),
            ..HeaderContext::default()
        };
        assert_eq!(ex("$h", &c), format!("{}...", "a".repeat(32)));
        // A character of two UTF-16 units is not cut in two.
        let c = HeaderContext {
            grep_key: format!("{}{}", "a".repeat(31), "𠮷b"),
            ..HeaderContext::default()
        };
        assert_eq!(ex("$h", &c), format!("{}...", "a".repeat(31)));
        let c = HeaderContext {
            grep_key: "short".into(),
            ..HeaderContext::default()
        };
        assert_eq!(ex("$h", &c), "short");
    }

    #[test]
    fn the_result_is_cut_at_1024_units() {
        let c = HeaderContext {
            selected_text: "あ".repeat(2000),
            ..HeaderContext::default()
        };
        assert_eq!(ex("$C", &c).chars().count(), 1024);
        // A character of two units is not cut in two.
        let c = HeaderContext {
            selected_text: format!("{}{}", "a".repeat(1023), "𠮷"),
            ..HeaderContext::default()
        };
        assert_eq!(ex("$C", &c), "a".repeat(1023));
    }

    #[test]
    fn dollars_that_mean_nothing_stay() {
        let c = ctx();
        assert_eq!(ex("$$ $z $", &c), "$ $z $");
        assert_eq!(ex("$$f", &c), "$f");
    }

    #[test]
    fn conditions() {
        // The example of HLP000284.html. `U` is 0 for a document with changes not saved
        // (_ExParam_Evaluate: IsModified), so that one is "未更新" (not saved yet).
        let mut c = ctx();
        let form = "「$F」は、${U?未更新で$:更新されていま$}す。";
        c.modified = true;
        assert_eq!(ex(form, &c), r"「D:\source\Main.cpp」は、未更新です。");
        c.modified = false;
        assert_eq!(
            ex(form, &c),
            r"「D:\source\Main.cpp」は、更新されています。"
        );
    }

    #[test]
    fn conditions_with_three_parts_and_nesting() {
        let mut c = ctx();
        let form = "${R?view$:locked$:rw$}!";
        assert_eq!(ex(form, &c), "rw!");
        c.write_state = WriteState::ViewMode;
        assert_eq!(ex(form, &c), "view!");
        c.write_state = WriteState::Locked;
        assert_eq!(ex(form, &c), "locked!");

        let nested = "${U?${N?new$:old$}$:clean$}.";
        c.modified = true;
        assert_eq!(ex(nested, &c), "old.");
        c.file_path = None;
        assert_eq!(ex(nested, &c), "new.");
        c.modified = false;
        assert_eq!(ex(nested, &c), "clean.");

        c.window = WindowKind::Grep;
        assert_eq!(ex("${w?grep$:output$:normal$}", &c), "grep");
        c.window = WindowKind::Output;
        assert_eq!(ex("${w?grep$:output$:normal$}", &c), "output");
        c.recording_macro = true;
        assert_eq!(ex("${M?rec$:no$}", &c), "rec");
        // A part that is not there is empty.
        assert_eq!(ex("a${U?x$}b", &HeaderContext::default()), "ab");
        // `I` is true for an iconified window only, which a preview never is.
        assert_eq!(ex("${I?inactive$:active$}", &c), "active");
    }

    #[test]
    fn broken_conditions_do_not_loop() {
        let c = ctx();
        assert_eq!(ex("a${U", &c), "a");
        // The document is not changed, so `U` chooses part 1, which is not there.
        assert_eq!(ex("a${U?b", &c), "a");
        assert_eq!(ex("a$<profile", &c), "a");
        // `$}` is nothing; `$:` skips the rest.
        assert_eq!(ex("a$}b$:c", &c), "ab");
    }
}
