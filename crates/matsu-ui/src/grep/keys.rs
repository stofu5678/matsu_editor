// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/grep/CGrepEnumKeys.h
//
// sakura_core/grep/CGrepEnumKeys.h:
//   Copyright (C) 2008, wakura
//   Copyright (C) 2011, Moca
//   Copyright (C) 2018-2026, Sakura Editor Organization
//
// This file is derived from the file above and was modified: the C++ code (UTF-16 buffers,
// `std::vector<std::wstring>` members and the status codes returned as numbers) was rewritten in
// Rust on `&str`, with an error type instead of the numbers, and the tests are new.
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

//! The file patterns of Grep (the "ファイル", "除外ファイル" and "除外フォルダー" boxes of the dialog).
//!
//! Sakura: `CGrepEnumKeys` (`sakura_core/grep/CGrepEnumKeys.h`). The three boxes are packed into one
//! string, `file;#folder;!file` (`CDlgGrep::GetPackedGFileString`), and this parses it: the
//! elements are separated by a space, a semicolon or a comma (not inside quotation marks), a
//! leading `!` makes the element an excluded file and a leading `#` an excluded folder.

/// The pattern that stands for everything (`WILDCARD_ANY`). It is the file pattern when none is
/// given, and the only folder pattern.
pub const WILDCARD_ANY: &str = "*.*";

/// The elements are separated by these (`WILDCARD_DELIMITER`).
const DELIMITERS: [char; 3] = [' ', ';', ','];

/// Why a pattern was refused (the return values 1 and 2 of `SetFileKeys`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeysError {
    /// A wildcard in the folder part of the pattern, such as `*\file.exe`
    /// (`STR_GREP_ERR_ENUMKEYS1`).
    WildcardInFolder,
    /// A full path as the pattern to search (`STR_GREP_ERR_ENUMKEYS2`).
    AbsolutePath,
}

impl KeysError {
    /// The text of the message box (`STR_GREP_ERR_ENUMKEYS1`, `STR_GREP_ERR_ENUMKEYS2` in
    /// `sakura_core/sakura_rc.rc`).
    pub fn message(self) -> &'static str {
        match self {
            KeysError::WildcardInFolder => {
                "ファイル指定のフォルダー部分にはワイルドカードは使えません"
            }
            KeysError::AbsolutePath => "ファイル指定にはフルパスは使えません",
        }
    }
}

/// The parsed patterns (the members of `CGrepEnumKeys`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GrepKeys {
    /// The files to search (`m_vecSearchFileKeys`).
    pub search_files: Vec<String>,
    /// The folders to go into (`m_vecSearchFolderKeys`): always [`WILDCARD_ANY`].
    pub search_folders: Vec<String>,
    /// `!pattern` with a relative path (`m_vecExceptFileKeys`).
    pub except_files: Vec<String>,
    /// `#pattern` with a relative path (`m_vecExceptFolderKeys`).
    pub except_folders: Vec<String>,
    /// `!pattern` with a full path (`m_vecExceptAbsFileKeys`).
    pub except_abs_files: Vec<String>,
    /// `#pattern` with a full path (`m_vecExceptAbsFolderKeys`).
    pub except_abs_folders: Vec<String>,
    /// `!pattern` as a regular expression on the full path, when the option is on
    /// (`m_vecExceptFileRegexKeys`).
    pub except_file_regexes: Vec<String>,
}

impl GrepKeys {
    /// Parses the packed patterns (`SetFileKeys`). With `except_file_regex` the excluded files
    /// are regular expressions and are not checked as paths.
    pub fn parse(keys: &str, except_file_regex: bool) -> Result<GrepKeys, KeysError> {
        let mut parsed = GrepKeys::default();
        for element in split_pattern(keys) {
            parsed.add_file_key(&element, except_file_regex)?;
        }
        if parsed.search_files.is_empty() {
            push_unique(&mut parsed.search_files, WILDCARD_ANY);
        }
        if parsed.search_folders.is_empty() {
            push_unique(&mut parsed.search_folders, WILDCARD_ANY);
        }
        Ok(parsed)
    }

    /// The excluded files of every kind, for the header of the result (`GetExcludeFiles`).
    pub fn exclude_files(&self) -> Vec<&str> {
        self.except_files
            .iter()
            .chain(&self.except_abs_files)
            .chain(&self.except_file_regexes)
            .map(String::as_str)
            .collect()
    }

    /// The excluded folders of every kind, for the header of the result (`GetExcludeFolders`).
    pub fn exclude_folders(&self) -> Vec<&str> {
        self.except_folders
            .iter()
            .chain(&self.except_abs_folders)
            .map(String::as_str)
            .collect()
    }

    /// One element (`AddFileKey`).
    fn add_file_key(&mut self, pattern: &str, except_file_regex: bool) -> Result<(), KeysError> {
        enum Kind {
            Search,
            ExceptFile,
            ExceptFolder,
        }
        let (kind, token) = if let Some(rest) = pattern.strip_prefix('!') {
            (Kind::ExceptFile, rest)
        } else if let Some(rest) = pattern.strip_prefix('#') {
            (Kind::ExceptFolder, rest)
        } else {
            (Kind::Search, pattern)
        };
        // An excluded file that is a regular expression is not a path: no check of the path.
        if except_file_regex && matches!(kind, Kind::ExceptFile) {
            if !token.is_empty() {
                push_unique(&mut self.except_file_regexes, token);
            }
            return Ok(());
        }
        let relative = is_relative(token);
        validate_key(token)?;
        match kind {
            Kind::Search => {
                if !relative {
                    return Err(KeysError::AbsolutePath);
                }
                push_unique(&mut self.search_files, token);
            }
            Kind::ExceptFile => push_unique(
                if relative {
                    &mut self.except_files
                } else {
                    &mut self.except_abs_files
                },
                token,
            ),
            Kind::ExceptFolder => push_unique(
                if relative {
                    &mut self.except_folders
                } else {
                    &mut self.except_abs_folders
                },
                token,
            ),
        }
        Ok(())
    }
}

fn push_unique(list: &mut Vec<String>, item: &str) {
    if !list.iter().any(|existing| existing == item) {
        list.push(item.to_owned());
    }
}

/// A wildcard may not be in the folder part (`ValidateKey`): after the first `*` or `?`, a
/// separator is an error.
fn validate_key(key: &str) -> Result<(), KeysError> {
    let mut wildcard = false;
    for c in key.chars() {
        if !wildcard && (c == '*' || c == '?') {
            wildcard = true;
        } else if wildcard && (c == '\\' || c == '/') {
            return Err(KeysError::WildcardInFolder);
        }
    }
    Ok(())
}

/// Whether a path is relative (`_IS_REL_PATH` in `sakura_core/util/file.h`: a drive letter and a
/// backslash, or two backslashes, make it absolute). A path that is absolute for the OS this runs
/// on is absolute too.
pub fn is_relative(path: &str) -> bool {
    let c: Vec<char> = path.chars().take(3).collect();
    let drive = c.len() == 3 && c[0].is_ascii_alphabetic() && c[1] == ':' && c[2] == '\\';
    let unc = c.len() >= 2 && c[0] == '\\' && c[1] == '\\';
    !(drive || unc || std::path::Path::new(path).is_absolute())
}

/// The elements without their quotation marks (`SplitPattern`).
pub fn split_pattern(keys: &str) -> Vec<String> {
    split_pattern_keep_quotes(keys)
        .into_iter()
        .map(|quoted| quoted.replace('"', ""))
        .collect()
}

/// The elements with their quotation marks (`SplitPatternKeepQuotes`): separated by a space, a
/// semicolon or a comma outside quotation marks. A quotation mark switches inside and outside
/// wherever it is in the element. Empty elements are left out.
pub fn split_pattern_keep_quotes(keys: &str) -> Vec<&str> {
    let mut patterns = Vec::new();
    let mut in_quote = false;
    let mut begin = 0;
    for (i, c) in keys.char_indices() {
        if c == '"' {
            in_quote = !in_quote;
        } else if !in_quote && DELIMITERS.contains(&c) {
            if begin < i {
                patterns.push(&keys[begin..i]);
            }
            begin = i + c.len_utf8();
        }
    }
    if begin < keys.len() {
        patterns.push(&keys[begin..]);
    }
    patterns
}

/// Whether a quotation mark is not closed: an odd number of them (`HasUnclosedQuote`).
pub fn has_unclosed_quote(keys: &str) -> bool {
    keys.chars().filter(|c| *c == '"').count() % 2 != 0
}

/// Splits the folder box at `;` outside quotation marks and removes the marks
/// (`CGrepAgent::CreateFolders`, which uses `my_strtok` of `sakura_core/util/string_ex.cpp`).
/// Empty parts are left out.
pub fn split_folders(folders: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut in_quote = false;
    let mut current = String::new();
    for c in folders.chars() {
        if c == '"' {
            in_quote = !in_quote;
        }
        if c == ';' && !in_quote {
            parts.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    parts.push(current);
    parts
        .into_iter()
        .map(|part| part.replace('"', ""))
        .filter(|part| !part.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn elements_are_separated_by_space_semicolon_and_comma() {
        assert_eq!(
            split_pattern("*.txt;*.md *.rs,*.toml"),
            strings(&["*.txt", "*.md", "*.rs", "*.toml"])
        );
        // empty elements are not returned
        assert_eq!(split_pattern(" ;; a ,, b "), strings(&["a", "b"]));
        assert!(split_pattern("").is_empty());
    }

    #[test]
    fn a_quoted_element_keeps_its_separators() {
        assert_eq!(
            split_pattern(r#""my docs\*.txt";*.md"#),
            strings(&["my docs\\*.txt", "*.md"])
        );
        // the mark switches inside and outside at any place of the element
        assert_eq!(split_pattern(r#"a"b c"d;e"#), strings(&["ab cd", "e"]));
        // the marks stay when asked (the dialog keeps them in the excluded patterns)
        assert_eq!(
            split_pattern_keep_quotes(r#""a,b.txt";c"#),
            vec![r#""a,b.txt""#, "c"]
        );
    }

    #[test]
    fn an_unclosed_quote_is_found() {
        assert!(has_unclosed_quote(r#"*.txt;"a b"#));
        assert!(!has_unclosed_quote(r#""a b";*.txt"#));
        assert!(!has_unclosed_quote("*.txt"));
    }

    #[test]
    fn the_patterns_go_to_their_kinds() {
        let keys = GrepKeys::parse(
            r"*.rs;sub\*.toml;!*.bak;!C:\tmp\*.log;#target;#C:\skip",
            false,
        )
        .unwrap();
        assert_eq!(keys.search_files, strings(&["*.rs", r"sub\*.toml"]));
        assert_eq!(keys.except_files, strings(&["*.bak"]));
        assert_eq!(keys.except_abs_files, strings(&[r"C:\tmp\*.log"]));
        assert_eq!(keys.except_folders, strings(&["target"]));
        assert_eq!(keys.except_abs_folders, strings(&[r"C:\skip"]));
        assert_eq!(keys.search_folders, strings(&["*.*"]));
        assert_eq!(keys.exclude_files(), vec!["*.bak", r"C:\tmp\*.log"]);
        assert_eq!(keys.exclude_folders(), vec!["target", r"C:\skip"]);
    }

    #[test]
    fn no_pattern_means_everything_and_a_pattern_is_listed_once() {
        let keys = GrepKeys::parse("", false).unwrap();
        assert_eq!(keys.search_files, strings(&["*.*"]));
        let keys = GrepKeys::parse("!*.exe", false).unwrap();
        assert_eq!(keys.search_files, strings(&["*.*"]));
        let keys = GrepKeys::parse("*.rs;*.rs;!a;!a", false).unwrap();
        assert_eq!(keys.search_files, strings(&["*.rs"]));
        assert_eq!(keys.except_files, strings(&["a"]));
    }

    #[test]
    fn japanese_patterns_are_kept() {
        let keys = GrepKeys::parse("議事録*.txt;!下書き?.txt;#旧版", false).unwrap();
        assert_eq!(keys.search_files, strings(&["議事録*.txt"]));
        assert_eq!(keys.except_files, strings(&["下書き?.txt"]));
        assert_eq!(keys.except_folders, strings(&["旧版"]));
    }

    #[test]
    fn a_wildcard_in_the_folder_part_is_an_error() {
        assert_eq!(
            GrepKeys::parse(r"*\file.exe", false),
            Err(KeysError::WildcardInFolder)
        );
        assert_eq!(
            GrepKeys::parse("a?/b.txt", false),
            Err(KeysError::WildcardInFolder)
        );
        assert_eq!(
            GrepKeys::parse(r"!x*\y", false),
            Err(KeysError::WildcardInFolder)
        );
        // a wildcard only in the last part is fine
        assert!(GrepKeys::parse(r"sub\dir\*.txt", false).is_ok());
    }

    #[test]
    fn a_full_path_cannot_be_searched() {
        assert_eq!(
            GrepKeys::parse(r"C:\data\*.txt", false),
            Err(KeysError::AbsolutePath)
        );
        assert_eq!(
            GrepKeys::parse(r"\\server\share\*.txt", false),
            Err(KeysError::AbsolutePath)
        );
        assert!(KeysError::AbsolutePath.message().contains("フルパス"));
    }

    #[test]
    fn excluded_files_can_be_regular_expressions() {
        let keys = GrepKeys::parse(r"*.rs;!\.git[\\/].*;!(a|b)\.txt$", true).unwrap();
        assert_eq!(keys.search_files, strings(&["*.rs"]));
        assert_eq!(
            keys.except_file_regexes,
            strings(&[r"\.git[\\/].*", r"(a|b)\.txt$"])
        );
        assert!(keys.except_files.is_empty());
        // a wildcard before a separator is no error for an expression
        assert!(GrepKeys::parse(r"!.*\.log", true).is_ok());
        // but `#` is still a folder
        let keys = GrepKeys::parse("#obj", true).unwrap();
        assert_eq!(keys.except_folders, strings(&["obj"]));
    }

    #[test]
    fn the_folder_box_splits_at_semicolons_outside_quotes() {
        assert_eq!(
            split_folders(r#"C:\a;"D:\b;c";;E:\d"#),
            strings(&[r"C:\a", r"D:\b;c", r"E:\d"])
        );
        assert!(split_folders("").is_empty());
    }

    #[test]
    fn relative_and_absolute_paths() {
        assert!(is_relative("a.txt"));
        assert!(is_relative(r"sub\a.txt"));
        assert!(is_relative("C:a.txt"));
        assert!(!is_relative(r"C:\a.txt"));
        assert!(!is_relative(r"\\host\share"));
    }
}
