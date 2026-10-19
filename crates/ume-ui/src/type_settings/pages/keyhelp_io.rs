// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/typeprop/CImpExpManager.cpp (CImpExpKeyHelp::Import, CImpExpKeyHelp::Export,
//   CImpExpManager::ImportUI, CImpExpManager::ExportUI)
//
// Original copyright notices of those files:
//   Copyright (C) 2006, ryoji
//   Copyright (C) 2007, genta, kobake
//   Copyright (C) 2008, nasukoji
//   Copyright (C) 2010, Moca
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

//! インポート and エクスポート of the list of the keyword help dictionaries (`CImpExpKeyHelp`).
//!
//! The file is a text file of lines `KDct[NN]=use,about,path` after the first line
//! `// キーワード辞書設定 Ver1`. It is written in UTF-8 with the BOM and read as the text streams of
//! Sakura read (UTF-8 with the BOM, else Shift_JIS). This module has the format; the buttons are in
//! [`super::keyhelp`].

use std::path::Path;

use ume_config::settings::KeyHelpDict;
use ume_config::settings::type_rules::{DICT_ABOUT_LEN, MAX_KEYHELP_FILES};

/// `WSTR_KEYHELP_HEAD`: the first line of an exported file.
const HEAD: &str = "// キーワード辞書設定 Ver1";
/// `_MAX_PATH`: the buffer of the path of a dictionary.
const PATH_BUFFER: usize = 260;
/// `STR_IMPEXP_DIC_NOTFOUND`: the description of a dictionary that is not there.
pub(super) const NOT_FOUND: &str = "【辞書ファイルが見つかりません】";

/// What a file gave.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Imported {
    /// The dictionaries, up to `MAX_KEYHELP_FILE`.
    pub dicts: Vec<KeyHelpDict>,
    /// The message to show (`sErrMsg`): empty when all went well.
    pub message: String,
}

/// `_wtoi`: the number at the start of the text (white space and a sign are allowed), else 0.
fn atoi(text: &str) -> i64 {
    let text = text.trim_start();
    let (sign, digits) = match text.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, text.strip_prefix('+').unwrap_or(text)),
    };
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    sign * digits[..end].parse::<i64>().unwrap_or(0)
}

/// The lines of a file as `CTextInputStream::ReadLineW` gives them: UTF-8 with the BOM, else
/// Shift_JIS.
pub(super) fn lines_of(bytes: &[u8]) -> Vec<String> {
    ume_commands::text::complete::word_file_lines(bytes)
}

/// `CImpExpKeyHelp::Import` over the lines of the file. `exists` says whether a dictionary file
/// opens (a relative path starts at the folder of the settings file). A line that is not a
/// dictionary is counted and skipped; the reading stops at `MAX_KEYHELP_FILE` dictionaries.
pub(super) fn import(lines: &[String], exists: impl Fn(&str) -> bool) -> Imported {
    let mut dicts: Vec<KeyHelpDict> = Vec::new();
    let mut message = String::new();
    let mut invalid = 0usize;
    for line in lines {
        if dicts.len() >= MAX_KEYHELP_FILES {
            break;
        }
        let chars: Vec<char> = line.chars().collect();
        // An empty line, `#`, `;` and `//` are comments.
        if chars.is_empty()
            || matches!(chars[0], '#' | ';')
            || (chars[0] == '/' && chars.get(1) == Some(&'/'))
        {
            continue;
        }
        // `KDct[99]=use,about,path`
        if chars.len() < 10
            || chars[..5].iter().collect::<String>() != "KDct["
            || chars[7] != ']'
            || chars[8] != '='
        {
            invalid += 1;
            continue;
        }
        let rest: String = chars[9..].iter().collect();
        // The path may have commas: only the first two commas split.
        let mut parts = rest.splitn(3, ',');
        let (Some(flag), Some(about), Some(path)) = (parts.next(), parts.next(), parts.next())
        else {
            invalid += 1;
            continue;
        };
        // Not 0 is 1 (`b_enable_flag > 1`, a negative number is a large unsigned one).
        let mut used = atoi(flag) != 0;
        // The length is the one of the text in the file, also for a dictionary that is not there.
        let about_len = about.chars().count();
        let mut about = about.to_owned();
        if !exists(path) {
            // A dictionary that is not there is taken, with a note, and not used.
            about = NOT_FOUND.to_owned();
            used = false;
        }
        if about_len >= DICT_ABOUT_LEN {
            // `STR_IMPEXP_DIC_LENGTH`
            message = format!(
                "辞書の説明は{}文字以内にしてください。\n",
                DICT_ABOUT_LEN - 1
            );
            invalid += 1;
            continue;
        }
        if path.chars().count() >= PATH_BUFFER {
            message = format!("辞書の説明は{}文字以内にしてください。\n", PATH_BUFFER - 1);
            invalid += 1;
            continue;
        }
        dicts.push(KeyHelpDict {
            used,
            about,
            path: path.to_owned(),
        });
    }
    if invalid > 0 {
        // `STR_IMPEXP_DIC_RECORD`
        message = format!("一部のデータが読み込めませんでした\n不正な行数: {invalid}");
    }
    Imported { dicts, message }
}

/// `CImpExpKeyHelp::Export`: the bytes of the file (UTF-8 with the BOM, lines end with a line
/// feed).
pub(super) fn export(dicts: &[KeyHelpDict]) -> Vec<u8> {
    let mut text = format!("{HEAD}\n");
    for (i, dict) in dicts.iter().enumerate() {
        text.push_str(&format!(
            "KDct[{i:02}]={},{},{}\n",
            u8::from(dict.used),
            dict.about,
            dict.path
        ));
    }
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

/// Whether the dictionary file opens: a relative path starts at the folder of the settings file.
pub(super) fn opens(path: &str, base: Option<&Path>) -> bool {
    let typed = Path::new(path);
    let full = match base {
        Some(dir) if typed.is_relative() => dir.join(typed),
        _ => typed.to_owned(),
    };
    std::fs::File::open(full).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_owned).collect()
    }

    fn exists_all(_: &str) -> bool {
        true
    }

    fn want(used: bool, about: &str, path: &str) -> KeyHelpDict {
        KeyHelpDict {
            used,
            about: about.to_owned(),
            path: path.to_owned(),
        }
    }

    #[test]
    fn dictionaries_are_read_and_comments_are_skipped() {
        let got = import(
            &lines(
                "// header\n# c\n; c\n\nKDct[00]=1,Memo,a.txt\nKDct[01]=0,,dir/b,c.txt\nKDct[02]=7,x,z.txt",
            ),
            exists_all,
        );
        assert_eq!(got.message, "");
        assert_eq!(
            got.dicts,
            [
                want(true, "Memo", "a.txt"),
                // the path keeps its commas
                want(false, "", "dir/b,c.txt"),
                // not 0 is used
                want(true, "x", "z.txt"),
            ]
        );
        // a negative number is used too (`(unsigned int)_wtoi`)
        let got = import(&lines("KDct[00]=-1,x,a.txt"), exists_all);
        assert!(got.dicts[0].used);
    }

    #[test]
    fn a_line_with_the_wrong_form_is_counted() {
        let got = import(
            &lines(
                "KDct[00]=1,Memo,a.txt\nnonsense line\nKDct[01]=1,only\nKDct[02]=1\nKDct[1]=1,a,b",
            ),
            exists_all,
        );
        assert_eq!(got.dicts.len(), 1);
        assert_eq!(
            got.message,
            "一部のデータが読み込めませんでした\n不正な行数: 4"
        );
    }

    #[test]
    fn a_dictionary_that_is_missing_is_taken_unused_with_a_note() {
        let got = import(&lines("KDct[00]=1,Memo,gone.txt"), |_| false);
        assert_eq!(got.dicts[0].about, NOT_FOUND);
        assert!(!got.dicts[0].used);
        assert_eq!(got.message, "");
    }

    #[test]
    fn a_long_description_or_path_is_refused() {
        let long = "あ".repeat(DICT_ABOUT_LEN);
        let got = import(&lines(&format!("KDct[00]=1,{long},a.txt")), exists_all);
        assert!(got.dicts.is_empty());
        // the count of the lines that could not be read replaces the message of the limit
        assert_eq!(
            got.message,
            "一部のデータが読み込めませんでした\n不正な行数: 1"
        );
        let ok = "い".repeat(DICT_ABOUT_LEN - 1);
        let got = import(&lines(&format!("KDct[00]=1,{ok},a.txt")), exists_all);
        assert_eq!(got.dicts.len(), 1);
        let path = "p".repeat(PATH_BUFFER);
        let got = import(&lines(&format!("KDct[00]=1,a,{path}")), exists_all);
        assert!(got.dicts.is_empty());
    }

    #[test]
    fn at_most_twenty_dictionaries_are_read() {
        let text: String = (0..25)
            .map(|i| format!("KDct[{i:02}]=1,a,{i}.txt\n"))
            .collect();
        assert_eq!(
            import(&lines(&text), exists_all).dicts.len(),
            MAX_KEYHELP_FILES
        );
    }

    #[test]
    fn export_writes_the_head_and_one_line_for_each() {
        let dicts = [want(true, "辞書", "a.txt"), want(false, "", "b,c.txt")];
        let bytes = export(&dicts);
        assert_eq!(&bytes[..3], [0xEF, 0xBB, 0xBF]);
        let text = String::from_utf8(bytes[3..].to_vec()).unwrap();
        assert_eq!(
            text,
            "// キーワード辞書設定 Ver1\nKDct[00]=1,辞書,a.txt\nKDct[01]=0,,b,c.txt\n"
        );
        // what is written is read back
        let back = import(&lines_of(&bytes), exists_all);
        assert_eq!(back.dicts, dicts);
    }
}
