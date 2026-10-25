// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/typeprop/CImpExpManager.cpp (CImpExpKeyWord::Import and CImpExpKeyWord::Export: the format of
//     the keyword definition file)
//   sakura_core/io/CTextStream.cpp (CTextInputStream, CTextOutputStream: the encodings of the file)
//
// Original copyright notices of those files:
//   Copyright (C) 2010, Uchi, Moca
//   Copyright (C) 2018-2026, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// zlib License
//
// Copyright (C) 2018-2022, Sakura Editor Organization
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

//! The keyword definition file (`.kwd`) that the page 強調キーワード imports and exports.
//!
//! A line that starts with `//` is a comment, except that `// CASE=True` and `// CASE=False` say whether the
//! set ignores case; an empty line is skipped; every other line is a keyword. The file is read as UTF-8 when
//! it starts with a BOM, else as Shift_JIS (`CTextInputStream`), and written as UTF-8 with a BOM and CR LF
//! line ends (`CTextOutputStream`).

use super::keywords::KeywordSet;

/// `WSTR_KEYWORD_HEAD`: what follows the name of the set in the first line.
const HEAD: &str = " キーワード定義ファイル";
/// `WSTR_KEYWORD_CASE`
const CASE: &str = "// CASE=";

/// What `CImpExpKeyWord::Import` reads from a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordFile {
    /// The last `// CASE=` line: `None` when the file has none.
    pub case_sensitive: Option<bool>,
    /// The keywords, one for each line that is neither empty nor a comment, as they are written (no limit
    /// of length or count, no check for duplicates: the page does that when it adds them).
    pub words: Vec<String>,
}

/// Reads the bytes of a keyword definition file.
pub fn parse(bytes: &[u8]) -> KeywordFile {
    let text = match bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        Some(rest) => String::from_utf8_lossy(rest),
        None => encoding_rs::SHIFT_JIS.decode_without_bom_handling(bytes).0,
    };
    let mut case_sensitive = None;
    let mut words = Vec::new();
    // `CTextInputStream::ReadLineW`: CR LF, LF and CR all end a line.
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    for line in normalized.split('\n') {
        if line.is_empty() {
            continue;
        }
        if line.starts_with("//") {
            match line {
                "// CASE=True" => case_sensitive = Some(true),
                "// CASE=False" => case_sensitive = Some(false),
                _ => {}
            }
            continue;
        }
        words.push(line.to_owned());
    }
    KeywordFile {
        case_sensitive,
        words,
    }
}

/// The bytes of the file `CImpExpKeyWord::Export` writes: the name of the set, the case rule, and the words
/// sorted the way the set is searched.
pub fn render(name: &str, case_sensitive: bool, words: &[String]) -> Vec<u8> {
    let sorted = KeywordSet::new(name, case_sensitive, words.iter().cloned());
    let mut text = format!(
        "// {name}{HEAD}\n{CASE}{}\n\n",
        if case_sensitive { "True" } else { "False" }
    );
    for word in sorted.words() {
        text.push_str(word);
        text.push('\n');
    }
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.replace('\n', "\r\n").as_bytes());
    bytes
}

/// The file name the export offers (`MakeExportFileName`): the characters a file name cannot have become
/// `_`, a `/` the full-width `／`, and the extension `kwd` follows.
pub fn export_file_name(set_name: &str) -> String {
    export_file_name_with(set_name, "kwd")
}

/// [`export_file_name`] for another kind of file (`ini` for a type, `col` for colors).
pub fn export_file_name_with(name: &str, extension: &str) -> String {
    let name: String = name
        .chars()
        .map(|c| match c {
            '\t' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            '/' => '／',
            c => c,
        })
        .collect();
    format!("{name}.{extension}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_and_empty_lines_are_skipped_and_the_case_line_is_read() {
        let file = parse(
            "\u{feff}// Mine\r\n// CASE=True\r\n\r\nalpha\r\n// other\r\nBeta\nあ\rgamma\r\n"
                .as_bytes(),
        );
        assert_eq!(file.case_sensitive, Some(true));
        assert_eq!(file.words, ["alpha", "Beta", "あ", "gamma"]);
        // The last case line wins; a line that only looks like one is a comment.
        let file = parse(b"// CASE=True\n// CASE=False\n// CASE=false\nx\n");
        assert_eq!(file.case_sensitive, Some(false));
        assert_eq!(file.words, ["x"]);
    }

    #[test]
    fn a_file_without_a_bom_is_shift_jis() {
        let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("// CASE=False\r\n関数\r\nint\r\n");
        let file = parse(&bytes);
        assert_eq!(file.case_sensitive, Some(false));
        assert_eq!(file.words, ["関数", "int"]);
    }

    #[test]
    fn the_exported_file_has_the_head_the_case_and_the_sorted_words() {
        let words = ["b", "A", "a"].map(String::from);
        let bytes = render("My set", true, &words);
        assert_eq!(&bytes[..3], [0xEF, 0xBB, 0xBF]);
        let text = String::from_utf8(bytes[3..].to_vec()).unwrap();
        assert_eq!(
            text,
            "// My set キーワード定義ファイル\r\n// CASE=True\r\n\r\nA\r\na\r\nb\r\n"
        );
        // What was written is read back.
        let back = parse(&bytes);
        assert_eq!(back.case_sensitive, Some(true));
        assert_eq!(back.words, ["A", "a", "b"]);
        let text = String::from_utf8(render("x", false, &words)[3..].to_vec()).unwrap();
        assert!(text.starts_with("// x キーワード定義ファイル\r\n// CASE=False\r\n\r\n"));
    }

    #[test]
    fn the_name_of_the_export_file_has_no_character_a_file_name_cannot_have() {
        assert_eq!(export_file_name("C/C++"), "C／C++.kwd");
        assert_eq!(
            export_file_name("a:b*c?\"d<e>f|g\\h\ti"),
            "a_b_c__d_e_f_g_h_i.kwd"
        );
        assert_eq!(export_file_name("Perl2"), "Perl2.kwd");
    }
}
