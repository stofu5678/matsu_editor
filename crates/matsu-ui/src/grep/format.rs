// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/grep/GrepMessageFormat.cpp (SetGrepResult, OutputPathInfo)
//   sakura_core/agent/CGrepAgent.cpp (the heading of the result in DoGrep, the heading of a file
//   in DoGrepFile, FormatPathList, EscapeStringLiteral)
//   sakura_core/sakura_rc.rc (STR_GREP_* strings)
//
// sakura_core/grep/GrepMessageFormat.cpp:
//   Copyright (C) 2026, Sakura Editor Organization
// sakura_core/agent/CGrepAgent.cpp:
//   Copyright (C) 2018-2026, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (UTF-16 buffers,
// `CNativeW` and `auto_sprintf`) was rewritten in Rust on `String`, and the tests are new.
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

//! The text of Grep's result: the heading with the conditions, the lines of the hits, and the
//! count at the end.
//!
//! Sakura writes the lines with `\r\n` and counts the columns in UTF-16 code units, and so does
//! this, so that the tag jump (`matsu_core::tag_jump`) reads the result as it reads Sakura's.
//! Nothing here touches a file or a window.

use matsu_core::encoding::Encoding;

use super::keys::GrepKeys;

/// The line break of the result (Sakura appends `\r\n` as it is).
pub const EOL: &str = "\r\n";

/// 出力形式 (`nGrepOutputStyle`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputStyle {
    /// ノーマル: `path(line,col)  [SJIS]: text`, 1.
    #[default]
    Normal,
    /// ファイル毎 (the "WZ style"): `■"path"`, then `・(  line,col   ): text`, 2.
    PerFile,
    /// 結果のみ: only the text, 3.
    ResultOnly,
}

impl OutputStyle {
    /// Sakura's number (`m_nGrepOutputStyle`).
    pub fn number(self) -> i32 {
        match self {
            OutputStyle::Normal => 1,
            OutputStyle::PerFile => 2,
            OutputStyle::ResultOnly => 3,
        }
    }

    /// The style for Sakura's number; anything else is the normal style (`CDlgGrep::SetData`).
    pub fn from_number(n: i32) -> OutputStyle {
        match n {
            2 => OutputStyle::PerFile,
            3 => OutputStyle::ResultOnly,
            _ => OutputStyle::Normal,
        }
    }
}

/// 結果出力 (`nGrepOutputLineType`): what is written for a hit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineType {
    /// 該当部分: the matched text, 0.
    MatchPart,
    /// 該当行: the whole line, 1.
    #[default]
    Line,
    /// 否該当行: the lines without a hit, 2.
    NoHitLine,
}

impl LineType {
    /// Sakura's number (`m_nGrepOutputLineType`).
    pub fn number(self) -> i32 {
        match self {
            LineType::MatchPart => 0,
            LineType::Line => 1,
            LineType::NoHitLine => 2,
        }
    }

    pub fn from_number(n: i32) -> LineType {
        match n {
            1 => LineType::Line,
            2 => LineType::NoHitLine,
            _ => LineType::MatchPart,
        }
    }
}

/// The options that decide the form of the lines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Layout {
    pub style: OutputStyle,
    pub line_type: LineType,
    /// ベースフォルダー表示 (`bGrepOutputBaseFolder`).
    pub base_folder: bool,
    /// フォルダー毎に表示 (`bGrepSeparateFolder`).
    pub separate_folder: bool,
}

/// What was written about the folders and the file already (`bOutputBaseFolder`,
/// `bOutputFolderName`, `bOutFileName`). The first two last for the whole search, the last for
/// one file.
#[derive(Clone, Copy, Debug, Default)]
pub struct PathInfoState {
    pub base_folder_written: bool,
    pub folder_written: bool,
    pub file_written: bool,
}

/// The names of a file for the lines (`pszFullPath`, `pszBaseFolder`, `pszFolder`,
/// `pszRelPath`).
#[derive(Clone, Copy, Debug)]
pub struct FilePaths<'a> {
    /// `C:\Folder\SubFolder\File.ext`.
    pub full: &'a str,
    /// The folder that was searched, `C:\Folder`.
    pub base_folder: &'a str,
    /// `SubFolder` when the folders are written apart from the files and the base folder is
    /// written too, else the full folder `C:\Folder\SubFolder`.
    pub folder: &'a str,
    /// `File.ext` when the folders are written apart, else `SubFolder\File.ext`.
    pub relative: &'a str,
}

impl FilePaths<'_> {
    /// The name written in front of each line (`pszDispFilePath`).
    fn shown(&self, layout: &Layout) -> &str {
        if layout.separate_folder || layout.base_folder {
            self.relative
        } else {
            self.full
        }
    }
}

/// `OutputPathInfo`: the lines before the first hit of a file (and of the search, and of the
/// folder): the base folder, the folder, and in the per-file style the file. `code_name` is
/// `  [SJIS]` or empty.
pub fn push_path_info(
    out: &mut String,
    layout: &Layout,
    paths: &FilePaths<'_>,
    code_name: &str,
    state: &mut PathInfoState,
) {
    if layout.style == OutputStyle::ResultOnly {
        return;
    }
    if !state.base_folder_written && layout.base_folder {
        if !layout.separate_folder && layout.style == OutputStyle::Normal {
            out.push_str("■\"");
        } else {
            out.push_str("◎\"");
        }
        out.push_str(paths.base_folder);
        out.push('"');
        out.push_str(EOL);
        state.base_folder_written = true;
    }
    if !state.folder_written && layout.separate_folder {
        if paths.folder.is_empty() {
            out.push('■');
        } else {
            out.push_str("■\"");
            out.push_str(paths.folder);
            out.push('"');
        }
        out.push_str(EOL);
        state.folder_written = true;
    }
    // The normal style writes the name with each line; the per-file style once, as a heading.
    if layout.style == OutputStyle::PerFile && !state.file_written {
        if layout.separate_folder {
            // not to be taken for the heading of a folder
            out.push_str("◆\"");
        } else {
            out.push_str("■\"");
        }
        out.push_str(paths.shown(layout));
        out.push('"');
        out.push_str(code_name);
        out.push_str(EOL);
        state.file_written = true;
    }
}

/// What a hit writes after the position.
#[derive(Clone, Copy, Debug)]
pub enum Body<'a> {
    /// The line without its line break (a hit line, or a line without a hit).
    Line(&'a str),
    /// The matched text, which may contain line breaks.
    Part(&'a str),
}

/// The most UTF-16 code units of a line or a match that are written, so that one huge line does
/// not fill the window (`nMaxOutStr`).
fn max_body_units(style: OutputStyle) -> usize {
    match style {
        OutputStyle::Normal => 2000,
        OutputStyle::PerFile | OutputStyle::ResultOnly => 2500,
    }
}

/// `text` cut to at most `max` UTF-16 code units, not in the middle of a character.
pub fn truncate_utf16(text: &str, max: usize) -> &str {
    let mut units = 0;
    for (i, c) in text.char_indices() {
        units += c.len_utf16();
        if units > max {
            return &text[..i];
        }
    }
    text
}

/// The number of UTF-16 code units of `text`: Sakura counts columns with them.
pub fn utf16_len(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

/// `SetGrepResult`: one hit. `line` and `column` count from 1, the column in UTF-16 code units.
pub fn push_result(
    out: &mut String,
    layout: &Layout,
    shown_path: &str,
    code_name: &str,
    line: u64,
    column: usize,
    body: Body<'_>,
) {
    match layout.style {
        OutputStyle::Normal => {
            if layout.base_folder || layout.separate_folder {
                out.push('・');
            }
            out.push_str(shown_path);
            out.push_str(&format!("({line},{column})"));
            out.push_str(code_name);
            out.push_str(": ");
        }
        OutputStyle::PerFile => {
            out.push_str(&format!("・({line:>6},{column:<5}): "));
        }
        OutputStyle::ResultOnly => {}
    }
    let max = max_body_units(layout.style);
    let mut add_eol = true;
    let text = match body {
        Body::Line(text) => truncate_utf16(text, max),
        Body::Part(text) => {
            let cut = truncate_utf16(text, max);
            // A match that ends with a line break brings its own (no empty line after it).
            if cut.ends_with(['\r', '\n']) {
                add_eol = false;
            }
            cut
        }
    };
    out.push_str(text);
    if add_eol {
        out.push_str(EOL);
    }
}

/// Which text the heading shows for the search conditions.
#[derive(Clone, Copy, Debug)]
pub struct HeaderConditions<'a> {
    pub key: &'a str,
    /// 英大文字と小文字を区別.
    pub match_case: bool,
    /// 単語単位 (not written for a regular expression? it is, as Sakura writes the option).
    pub whole_word: bool,
    pub regex: bool,
    /// The folders without their last backslash (`ChopYen`).
    pub folders: &'a [String],
    pub sub_folder: bool,
    pub except_file_regex: bool,
    /// `None` is the automatic detection.
    pub charset: Option<Encoding>,
    pub layout: Layout,
    pub file_only: bool,
    /// The version text of the regular expression library, for `(正規表現:` and `)`.
    pub regex_version: &'a str,
    /// Set for Grep 置換.
    pub replace: Option<ReplaceHeader<'a>>,
}

/// What the heading of a Grep 置換 says about the replacement.
#[derive(Clone, Copy, Debug)]
pub struct ReplaceHeader<'a> {
    /// The replacement text (the clipboard text when `paste`).
    pub text: &'a str,
    /// クリップボードから貼り付け.
    pub paste: bool,
}

/// `FormatPathList`: the patterns joined with `;`, one with a `;` in quotation marks.
pub fn format_path_list<'a>(items: impl IntoIterator<Item = &'a str>) -> String {
    let mut out = String::new();
    for (i, item) in items.into_iter().enumerate() {
        if i > 0 {
            out.push(';');
        }
        if item.contains(';') {
            out.push('"');
            out.push_str(item);
            out.push('"');
        } else {
            out.push_str(item);
        }
    }
    out
}

/// `EscapeStringLiteral` for the default type (`STRING_LITERAL_CPP`): the key is written as the
/// literal of a C string, so that a quotation mark in it does not end the line of the heading.
pub fn escape_key(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    for c in key.chars() {
        if matches!(c, '\\' | '\'' | '"') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// The name of the encoding in the line `(文字コードセット：SJIS)` (`CCodePage::GetNameNormal`).
fn charset_name(encoding: Encoding) -> &'static str {
    match encoding {
        Encoding::CodePage(_) | Encoding::Utf32Le | Encoding::Utf32Be => encoding.combo_name(),
        other => other.short_name(),
    }
}

/// The heading of the result (the `cmemMessage` that `DoGrep` writes before the search).
pub fn header(c: &HeaderConditions<'_>, keys: &GrepKeys) -> String {
    let mut out = String::new();
    out.push_str(EOL);
    out.push_str("□検索条件  ");
    if c.key.is_empty() {
        out.push_str("「ファイル検索」");
        out.push_str(EOL);
    } else {
        out.push('"');
        out.push_str(&escape_key(c.key));
        out.push('"');
        out.push_str(EOL);
    }
    if let Some(replace) = c.replace {
        // `STR_GREP_REPLACE_TO`, then the text in quotation marks. Sakura writes
        // the string for 貼り付け without a line break, which joins the next line to it; the
        // break is added here.
        out.push_str("置換後         ");
        if replace.paste {
            out.push_str("(クリップボードから貼り付け)");
        } else {
            out.push('"');
            out.push_str(&escape_key(replace.text));
            out.push('"');
        }
        out.push_str(EOL);
    }
    out.push_str("検索対象       ");
    out.push_str(&format_path_list(
        keys.search_files.iter().map(String::as_str),
    ));
    out.push_str(EOL);
    out.push_str("フォルダー       ");
    out.push_str(&format_path_list(c.folders.iter().map(String::as_str)));
    out.push_str(EOL);
    out.push_str("除外ファイル   ");
    out.push_str(&format_path_list(keys.exclude_files()));
    out.push_str(EOL);
    out.push_str("除外フォルダー   ");
    out.push_str(&format_path_list(keys.exclude_folders()));
    out.push_str(EOL);
    out.push_str(if c.sub_folder {
        "    (サブフォルダーも検索)"
    } else {
        "    (サブフォルダーを検索しない)"
    });
    out.push_str(EOL);
    if c.except_file_regex {
        out.push_str("    (除外ファイルは正規表現)");
        out.push_str(EOL);
    }
    // The options of the pattern are not written for a search of the names only.
    if !c.key.is_empty() {
        if c.whole_word {
            out.push_str("    (単語単位で探す)");
            out.push_str(EOL);
        }
        out.push_str(if c.match_case {
            "    (英大文字小文字を区別する)"
        } else {
            "    (英大文字小文字を区別しない)"
        });
        out.push_str(EOL);
        if c.regex {
            out.push_str("    (正規表現:");
            out.push_str(c.regex_version);
            out.push(')');
            out.push_str(EOL);
        }
    }
    match c.charset {
        None => {
            out.push_str("    (文字コードセットの自動判別)");
            out.push_str(EOL);
        }
        Some(encoding) => {
            out.push_str("    (文字コードセット：");
            out.push_str(charset_name(encoding));
            out.push(')');
            out.push_str(EOL);
        }
    }
    if !c.key.is_empty() {
        out.push_str(match c.layout.line_type {
            LineType::Line => "    (一致した行を出力)",
            LineType::NoHitLine => "    (一致しなかった行を出力)",
            // `STR_GREP_SHOW_FIRST_LINE`: a replacement with a regular expression changes a
            // line at once, so only its first match is shown.
            LineType::MatchPart if c.regex && c.replace.is_some_and(|replace| !replace.paste) => {
                "    (一致した行の最初の箇所のみ出力)"
            }
            LineType::MatchPart => "    (一致した箇所のみ出力)",
        });
        out.push_str(EOL);
        if c.file_only {
            out.push_str("    (ファイル毎最初のみ検索)");
            out.push_str(EOL);
        }
    }
    out.push_str(EOL);
    out.push_str(EOL);
    out
}

/// The line that ends a stopped search (`STR_GREP_SUSPENDED`).
pub const SUSPENDED: &str = "中断しました。\r\n";

/// The line with the count (`STR_GREP_MATCH_COUNT`).
pub fn count_line(hits: usize) -> String {
    format!("{hits} 個が検索されました。{EOL}")
}

/// `STR_GREP_REPLACE_COUNT`: the line with the number of replacements.
pub fn replace_count_line(replaced: usize) -> String {
    format!("{replaced} 個を置換しました。{EOL}")
}

/// `STR_GREP_REP_ERR_DELETE`: the old file could not be removed.
pub fn replace_delete_error(path: &str) -> String {
    format!("ファイルの削除に失敗しました。[{path}]{EOL}")
}

/// `STR_GREP_REP_ERR_REPLACE`: the new file could not take the place of the old one.
pub fn replace_replace_error(path: &str) -> String {
    format!("ファイルの置き換えに失敗しました。[{path}]{EOL}")
}

/// `STR_GREP_ERR_FILEWRITE`: the new content could not be written.
pub fn file_write_error(path: &str) -> String {
    format!("書き込めませんでした。 [{path}]{EOL}")
}

/// `STR_GREP_ERR_FILEOPEN`: a file that could not be opened.
pub fn file_open_error(path: &str) -> String {
    format!("file open error [{path}]{EOL}")
}

/// `STR_GREP_ERR_FILEREAD`: a file that failed while it was read.
pub fn file_read_error(path: &str) -> String {
    format!("ファイルの読み込み中にエラーが発生しました。[{path}]{EOL}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(style: OutputStyle, line_type: LineType) -> Layout {
        Layout {
            style,
            line_type,
            ..Layout::default()
        }
    }

    fn paths<'a>() -> FilePaths<'a> {
        FilePaths {
            full: r"C:\work\sub\a.txt",
            base_folder: r"C:\work",
            folder: r"C:\work\sub",
            relative: r"sub\a.txt",
        }
    }

    #[test]
    fn the_normal_style_has_the_path_the_position_and_the_text() {
        let mut out = String::new();
        let l = layout(OutputStyle::Normal, LineType::Line);
        push_result(
            &mut out,
            &l,
            r"C:\work\a.txt",
            "",
            12,
            3,
            Body::Line("hello"),
        );
        push_result(
            &mut out,
            &l,
            r"C:\work\a.txt",
            "  [SJIS]",
            13,
            1,
            Body::Line("世界"),
        );
        assert_eq!(
            out,
            "C:\\work\\a.txt(12,3): hello\r\nC:\\work\\a.txt(13,1)  [SJIS]: 世界\r\n"
        );
    }

    #[test]
    fn the_normal_style_with_folders_marks_the_lines() {
        let mut out = String::new();
        let l = Layout {
            base_folder: true,
            ..layout(OutputStyle::Normal, LineType::Line)
        };
        push_result(&mut out, &l, r"sub\a.txt", "", 1, 1, Body::Line("x"));
        assert_eq!(out, "・sub\\a.txt(1,1): x\r\n");
    }

    #[test]
    fn the_per_file_style_pads_the_numbers() {
        let mut out = String::new();
        let l = layout(OutputStyle::PerFile, LineType::Line);
        push_result(&mut out, &l, "ignored", "", 7, 4, Body::Line("text"));
        push_result(&mut out, &l, "ignored", "", 123456, 12345, Body::Line("t"));
        assert_eq!(out, "・(     7,4    ): text\r\n・(123456,12345): t\r\n");
    }

    #[test]
    fn the_result_only_style_has_only_the_text() {
        let mut out = String::new();
        let l = layout(OutputStyle::ResultOnly, LineType::MatchPart);
        push_result(&mut out, &l, "p", "", 1, 1, Body::Part("abc"));
        assert_eq!(out, "abc\r\n");
    }

    #[test]
    fn a_match_with_a_line_break_brings_its_own() {
        let mut out = String::new();
        let l = layout(OutputStyle::ResultOnly, LineType::MatchPart);
        push_result(&mut out, &l, "p", "", 1, 1, Body::Part("a\r\n"));
        push_result(&mut out, &l, "p", "", 1, 1, Body::Part("b\n"));
        push_result(&mut out, &l, "p", "", 1, 1, Body::Part(""));
        assert_eq!(out, "a\r\nb\n\r\n");
    }

    #[test]
    fn a_long_line_is_cut_at_the_limit_of_the_style() {
        let long = "あ".repeat(3000);
        let mut out = String::new();
        push_result(
            &mut out,
            &layout(OutputStyle::Normal, LineType::Line),
            "p",
            "",
            1,
            1,
            Body::Line(&long),
        );
        assert_eq!(out.chars().filter(|c| *c == 'あ').count(), 2000);
        let mut out = String::new();
        push_result(
            &mut out,
            &layout(OutputStyle::PerFile, LineType::Line),
            "p",
            "",
            1,
            1,
            Body::Line(&long),
        );
        assert_eq!(out.chars().filter(|c| *c == 'あ').count(), 2500);
        // the cut counts UTF-16 code units and does not split a pair
        let astral = "𠮷".repeat(5);
        assert_eq!(truncate_utf16(&astral, 5), "𠮷𠮷");
        assert_eq!(truncate_utf16("abc", 10), "abc");
        assert_eq!(utf16_len("a𠮷あ"), 4);
    }

    #[test]
    fn the_path_lines_of_the_normal_style() {
        let mut state = PathInfoState::default();
        let mut out = String::new();
        // nothing for the plain form
        let l = layout(OutputStyle::Normal, LineType::Line);
        push_path_info(&mut out, &l, &paths(), "", &mut state);
        assert_eq!(out, "");
        // the base folder is written once
        let l = Layout {
            base_folder: true,
            ..l
        };
        push_path_info(&mut out, &l, &paths(), "", &mut state);
        push_path_info(&mut out, &l, &paths(), "", &mut state);
        assert_eq!(out, "■\"C:\\work\"\r\n");
    }

    #[test]
    fn the_path_lines_with_separate_folders() {
        let l = Layout {
            base_folder: true,
            separate_folder: true,
            ..layout(OutputStyle::Normal, LineType::Line)
        };
        let mut state = PathInfoState::default();
        let mut out = String::new();
        let p = FilePaths {
            folder: "sub",
            relative: "a.txt",
            ..paths()
        };
        push_path_info(&mut out, &l, &p, "", &mut state);
        assert_eq!(out, "◎\"C:\\work\"\r\n■\"sub\"\r\n");
        // the base folder itself has an empty folder name
        let mut state = PathInfoState::default();
        let mut out = String::new();
        let p = FilePaths { folder: "", ..p };
        push_path_info(&mut out, &l, &p, "", &mut state);
        assert_eq!(out, "◎\"C:\\work\"\r\n■\r\n");
    }

    #[test]
    fn the_per_file_style_writes_a_heading_for_each_file() {
        let l = layout(OutputStyle::PerFile, LineType::Line);
        let mut state = PathInfoState::default();
        let mut out = String::new();
        push_path_info(&mut out, &l, &paths(), "  [UTF-8]", &mut state);
        push_path_info(&mut out, &l, &paths(), "  [UTF-8]", &mut state);
        assert_eq!(out, "■\"C:\\work\\sub\\a.txt\"  [UTF-8]\r\n");
        // with the base folder: the heading of the base is ◎ and the file is shown relative
        let l = Layout {
            base_folder: true,
            ..l
        };
        let mut state = PathInfoState::default();
        let mut out = String::new();
        push_path_info(&mut out, &l, &paths(), "", &mut state);
        assert_eq!(out, "◎\"C:\\work\"\r\n■\"sub\\a.txt\"\r\n");
        // with separate folders the file is ◆
        let l = Layout {
            separate_folder: true,
            ..l
        };
        let mut state = PathInfoState::default();
        let mut out = String::new();
        let p = FilePaths {
            folder: "sub",
            relative: "a.txt",
            ..paths()
        };
        push_path_info(&mut out, &l, &p, "", &mut state);
        assert_eq!(out, "◎\"C:\\work\"\r\n■\"sub\"\r\n◆\"a.txt\"\r\n");
    }

    #[test]
    fn the_result_only_style_has_no_path_lines() {
        let l = Layout {
            base_folder: true,
            separate_folder: true,
            ..layout(OutputStyle::ResultOnly, LineType::Line)
        };
        let mut out = String::new();
        push_path_info(&mut out, &l, &paths(), "", &mut PathInfoState::default());
        assert_eq!(out, "");
    }

    #[test]
    fn path_lists_quote_a_semicolon() {
        assert_eq!(format_path_list(["a", "b"]), "a;b");
        assert_eq!(format_path_list(["a;b", "c"]), "\"a;b\";c");
        assert_eq!(format_path_list(Vec::<&str>::new()), "");
    }

    #[test]
    fn the_key_is_written_as_a_literal() {
        assert_eq!(escape_key(r#"a"b\c'd"#), r#"a\"b\\c\'d"#);
        assert_eq!(escape_key("日本語"), "日本語");
    }

    fn conditions<'a>(folders: &'a [String]) -> HeaderConditions<'a> {
        HeaderConditions {
            key: "foo",
            match_case: false,
            whole_word: false,
            regex: false,
            folders,
            sub_folder: true,
            except_file_regex: false,
            charset: None,
            layout: layout(OutputStyle::Normal, LineType::Line),
            file_only: false,
            regex_version: "Onigmo",
            replace: None,
        }
    }

    #[test]
    fn the_heading_has_the_conditions() {
        let keys = GrepKeys::parse("*.txt;*.md;!*.bak;#.git", false).unwrap();
        let folders = vec![r"C:\work".to_owned()];
        let text = header(&conditions(&folders), &keys);
        assert_eq!(
            text,
            "\r\n□検索条件  \"foo\"\r\n\
             検索対象       *.txt;*.md\r\n\
             フォルダー       C:\\work\r\n\
             除外ファイル   *.bak\r\n\
             除外フォルダー   .git\r\n\
             \x20   (サブフォルダーも検索)\r\n\
             \x20   (英大文字小文字を区別しない)\r\n\
             \x20   (文字コードセットの自動判別)\r\n\
             \x20   (一致した行を出力)\r\n\
             \r\n\r\n"
        );
    }

    #[test]
    fn the_heading_of_a_replacement_has_the_new_text() {
        let keys = GrepKeys::parse("*.txt", false).unwrap();
        let folders = vec![r"C:\work".to_owned()];
        let c = HeaderConditions {
            replace: Some(ReplaceHeader {
                text: "a\"b",
                paste: false,
            }),
            ..conditions(&folders)
        };
        let text = header(&c, &keys);
        // Sakura (CGrepAgent.cpp): STR_GREP_REPLACE_TO, then the text as a literal
        assert!(text.contains("□検索条件  \"foo\"\r\n置換後         \"a\\\"b\"\r\n検索対象"));
        let c = HeaderConditions {
            replace: Some(ReplaceHeader {
                text: "clip",
                paste: true,
            }),
            ..c
        };
        let text = header(&c, &keys);
        assert!(text.contains("置換後         (クリップボードから貼り付け)\r\n検索対象"));
        assert!(!text.contains("clip"));
    }

    #[test]
    fn a_regex_replacement_shows_only_the_first_part_of_a_line() {
        let keys = GrepKeys::parse("*.txt", false).unwrap();
        let folders = vec![r"C:\work".to_owned()];
        let c = HeaderConditions {
            regex: true,
            layout: layout(OutputStyle::Normal, LineType::MatchPart),
            replace: Some(ReplaceHeader {
                text: "x",
                paste: false,
            }),
            ..conditions(&folders)
        };
        assert!(header(&c, &keys).contains("(一致した行の最初の箇所のみ出力)"));
        // pasting replaces every match one by one
        let c = HeaderConditions {
            replace: Some(ReplaceHeader {
                text: "x",
                paste: true,
            }),
            ..c
        };
        assert!(header(&c, &keys).contains("    (一致した箇所のみ出力)"));
    }

    #[test]
    fn the_heading_for_other_options() {
        let keys = GrepKeys::parse("", false).unwrap();
        let folders = vec![r"C:\a".to_owned(), r"D:\b;c".to_owned()];
        let c = HeaderConditions {
            match_case: true,
            whole_word: true,
            regex: true,
            sub_folder: false,
            charset: Some(Encoding::Utf8),
            layout: layout(OutputStyle::PerFile, LineType::NoHitLine),
            file_only: true,
            except_file_regex: true,
            ..conditions(&folders)
        };
        let text = header(&c, &keys);
        assert_eq!(
            text,
            "\r\n□検索条件  \"foo\"\r\n\
             検索対象       *.*\r\n\
             フォルダー       C:\\a;\"D:\\b;c\"\r\n\
             除外ファイル   \r\n\
             除外フォルダー   \r\n\
             \x20   (サブフォルダーを検索しない)\r\n\
             \x20   (除外ファイルは正規表現)\r\n\
             \x20   (単語単位で探す)\r\n\
             \x20   (英大文字小文字を区別する)\r\n\
             \x20   (正規表現:Onigmo)\r\n\
             \x20   (文字コードセット：UTF-8)\r\n\
             \x20   (一致しなかった行を出力)\r\n\
             \x20   (ファイル毎最初のみ検索)\r\n\
             \r\n\r\n"
        );
        let c = HeaderConditions {
            layout: layout(OutputStyle::Normal, LineType::MatchPart),
            charset: Some(Encoding::ShiftJis),
            ..c
        };
        let text = header(&c, &keys);
        assert!(text.contains("(一致した箇所のみ出力)"));
        assert!(text.contains("(文字コードセット：SJIS)"));
    }

    #[test]
    fn the_heading_of_a_search_of_the_names_has_no_pattern_options() {
        let keys = GrepKeys::parse("*.rs", false).unwrap();
        let folders = vec![r"C:\a".to_owned()];
        let c = HeaderConditions {
            key: "",
            whole_word: true,
            regex: true,
            file_only: true,
            ..conditions(&folders)
        };
        let text = header(&c, &keys);
        assert!(text.starts_with("\r\n□検索条件  「ファイル検索」\r\n検索対象       *.rs\r\n"));
        assert!(!text.contains("単語単位"));
        assert!(!text.contains("英大文字"));
        assert!(!text.contains("正規表現"));
        assert!(!text.contains("一致した"));
        assert!(!text.contains("ファイル毎最初のみ"));
        assert!(text.contains("(文字コードセットの自動判別)"));
        assert!(text.ends_with("\r\n\r\n"));
    }

    #[test]
    fn a_key_with_quotes_is_escaped_in_the_heading() {
        let keys = GrepKeys::parse("", false).unwrap();
        let folders = vec![];
        let c = HeaderConditions {
            key: r#"say "hi""#,
            ..conditions(&folders)
        };
        assert!(header(&c, &keys).contains(r#"□検索条件  "say \"hi\"""#));
    }

    #[test]
    fn the_last_lines() {
        assert_eq!(count_line(0), "0 個が検索されました。\r\n");
        assert_eq!(count_line(12), "12 個が検索されました。\r\n");
        assert_eq!(SUSPENDED, "中断しました。\r\n");
        assert_eq!(replace_count_line(3), "3 個を置換しました。\r\n");
        assert_eq!(file_open_error("a.txt"), "file open error [a.txt]\r\n");
        assert!(file_read_error("a.txt").ends_with("[a.txt]\r\n"));
    }

    #[test]
    fn the_numbers_are_sakuras() {
        assert_eq!(OutputStyle::from_number(2), OutputStyle::PerFile);
        assert_eq!(OutputStyle::from_number(9), OutputStyle::Normal);
        assert_eq!(OutputStyle::ResultOnly.number(), 3);
        assert_eq!(LineType::from_number(2), LineType::NoHitLine);
        assert_eq!(LineType::from_number(0), LineType::MatchPart);
        assert_eq!(LineType::Line.number(), 1);
    }
}
