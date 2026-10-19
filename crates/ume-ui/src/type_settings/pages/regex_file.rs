//! インポート and エクスポート of the regular expression keywords: `CPropTypesRegex::Import` and `Export`,
//! `CImpExpRegex` (`sakura_core/typeprop/CImpExpManager.cpp`), the `.rkw` file.
//!
//! The file is text: the line `// 正規表現キーワード Ver1`, then a line `RxKey[NNN]=color,keyword` for each
//! keyword, the color by its name (`TXT` ...; `GetColorNameByIndex`). Importing replaces the list of the
//! page; nothing is written to the settings file until OK.
//!
//! Not built: the limit on the size of the whole list (`MAX_REGEX_KEYWORDLISTLEN`, `STR_IMPEXP_REGEX2`).

use std::path::Path;

use gpui::{Context, Window};
use ume_config::settings::ColorSet;
use ume_config::settings::type_rules::MAX_REGEX_KEYWORDS;

use super::super::ids::REGEX_LIST;
use super::color::COLOR_ITEMS;
use super::regex::{color_label, color_name, color_options};
use crate::common_settings::CommonSettingsDialog;
use crate::common_settings::table::Row;
use crate::common_settings::transfer::{cannot_export, cannot_open, exported, say};
use crate::file_dialog::Ask;

/// `GetDefaultExtension` of `CImpExpRegex`.
pub(in crate::type_settings) const WILDCARD: &str = "*.rkw";
/// `WSTR_REGEXKW_HEAD`
const HEAD: &str = "// 正規表現キーワード Ver1";
/// `MAX_REGEX_KEYWORDLEN`
const MAX_KEYWORD_LEN: usize = 1000;
/// `STR_IMPEXP_REGEX1`
const TOO_MANY: &str = "キーワード数が上限に達したため切り捨てました。";
/// `STR_IMPEXP_REGEX3`
const INVALID: &str = "不正なキーワードを無視しました。";
/// `STR_IMPEXP_REGEX4`
const TOO_LONG: &str = "キーワードが長過ぎるため切り捨てました。";

/// What `CImpExpRegex::Import` reads: the keywords, and the message it leaves (the last one; Sakura
/// shows it after a successful import).
#[derive(Debug, PartialEq, Eq)]
pub(in crate::type_settings) struct Imported {
    pub rows: Vec<Row>,
    pub message: Option<&'static str>,
}

/// The label of the list for a color written in the file: by the name of the entry (`GetColorIndexByName`),
/// else by the name on the list (`m_szName`). A color that is not one a keyword may have is
/// 正規表現キーワード1, as `GetData` does.
fn label_of(color: &str) -> Option<&'static str> {
    let item = ColorSet::all_names()
        .zip(COLOR_ITEMS)
        .find(|(name, item)| *name == color || item.0 == color)
        .map(|(_, item)| item.0)?;
    Some(if color_options().iter().any(|(label, _)| *label == item) {
        item
    } else {
        color_label("RK1")
    })
}

/// Decodes the file as `CTextInputStream` does for the files of Sakura (like the `.kwd` files): UTF-8 when
/// it starts with a BOM, else Shift_JIS.
fn decode(bytes: &[u8]) -> String {
    match bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        Some(rest) => String::from_utf8_lossy(rest).into_owned(),
        None => encoding_rs::SHIFT_JIS
            .decode_without_bom_handling(bytes)
            .0
            .into_owned(),
    }
}

/// `CImpExpRegex::Import`.
pub(in crate::type_settings) fn parse(bytes: &[u8]) -> Imported {
    let mut rows = Vec::new();
    let mut message = None;
    // `CTextInputStream::ReadLineW`: CR LF, LF and CR all end a line.
    let text = decode(bytes).replace("\r\n", "\n").replace('\r', "\n");
    for line in text.split('\n') {
        // `RxKey[999]=ColorName,RegexKeyword`
        let chars: Vec<char> = line.chars().collect();
        if chars.len() < 12
            || chars[..6].iter().collect::<String>() != "RxKey["
            || chars[9..11].iter().collect::<String>() != "]="
        {
            continue;
        }
        let Some(sep) = chars[11..].iter().position(|&c| c == ',').map(|p| p + 11) else {
            continue;
        };
        let color: String = chars[11..sep].iter().collect();
        let keyword: String = chars[sep + 1..].iter().collect();
        if keyword.is_empty() || !ume_coloring::has_keyword_form(&keyword) {
            message = Some(INVALID);
            continue;
        }
        let Some(label) = label_of(&color) else {
            continue;
        };
        if rows.len() >= MAX_REGEX_KEYWORDS {
            message = Some(TOO_MANY);
            break;
        }
        if keyword.chars().count() >= MAX_KEYWORD_LEN {
            message = Some(TOO_LONG);
            continue;
        }
        rows.push(vec![keyword, label.to_owned()]);
    }
    Imported { rows, message }
}

/// `CImpExpRegex::Export`: the bytes of the file. The list ends at the first empty keyword.
pub(in crate::type_settings) fn render(rows: &[Row]) -> Vec<u8> {
    let mut text = format!("{HEAD}\n");
    for (i, row) in rows
        .iter()
        .take(MAX_REGEX_KEYWORDS)
        .take_while(|row| !row[0].is_empty())
        .enumerate()
    {
        text.push_str(&format!(
            "RxKey[{i:03}]={},{}\n",
            color_name(&row[1]),
            row[0]
        ));
    }
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.replace('\n', "\r\n").as_bytes());
    bytes
}

impl CommonSettingsDialog {
    /// 「インポート」: asks for a file and replaces the list with its keywords.
    pub(in crate::type_settings) fn regex_import(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pick_settings_file(
            Ask::open(false).titled("正規表現キーワードのインポート", "インポート"),
            WILDCARD,
            window,
            cx,
            |this, path, window, cx| {
                this.regex_import_from(path, window, cx);
            },
        );
    }

    /// `CImpExpRegex::Import` of `path`. Returns whether the file was read.
    pub(in crate::type_settings) fn regex_import_from(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Ok(bytes) = std::fs::read(path) else {
            say(&cannot_open(path), window, cx);
            return false;
        };
        let imported = parse(&bytes);
        self.replace_table_rows(REGEX_LIST, imported.rows, window, cx);
        if let Some(message) = imported.message {
            say(message, window, cx);
        }
        true
    }

    /// 「エクスポート」: asks for a name and writes the list.
    pub(in crate::type_settings) fn regex_export(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pick_settings_file(
            Ask::save().titled("正規表現キーワードのエクスポート", "保存"),
            WILDCARD,
            window,
            cx,
            |this, path, window, cx| {
                this.regex_export_to(path, window, cx);
            },
        );
    }

    /// `CImpExpRegex::Export` to `path`. Returns whether the file was written.
    pub(in crate::type_settings) fn regex_export_to(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let written = std::fs::write(path, render(self.values().rows(REGEX_LIST))).is_ok();
        say(
            &if written {
                exported(path)
            } else {
                cannot_export(path)
            },
            window,
            cx,
        );
        written
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file as Sakura writes it: UTF-8 with a BOM.
    fn utf8(text: &str) -> Vec<u8> {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        bytes
    }

    fn rows(list: &[(&str, &str)]) -> Vec<Row> {
        list.iter()
            .map(|(k, c)| vec![(*k).to_owned(), (*c).to_owned()])
            .collect()
    }

    #[test]
    fn a_file_is_read_line_by_line() {
        let text =
            "// 正規表現キーワード Ver1\r\nRxKey[000]=RK1,/a+/k\r\nRxKey[001]=RK2,/日本語/k\r\n";
        let imported = parse(&utf8(text));
        assert_eq!(imported.message, None);
        assert_eq!(imported.rows.len(), 2);
        assert_eq!(imported.rows[0][0], "/a+/k");
        assert_eq!(imported.rows[1][0], "/日本語/k");
        assert_eq!(imported.rows[0][1], color_label("RK1"));
        assert_eq!(imported.rows[1][1], color_label("RK2"));
    }

    #[test]
    fn a_keyword_that_is_not_framed_is_left_out_with_a_message() {
        let text = "RxKey[000]=RK1,abc\nRxKey[001]=RK1,/ok/k\nRxKey[002]=RK1,\n";
        let imported = parse(&utf8(text));
        assert_eq!(imported.rows, rows(&[("/ok/k", color_label("RK1"))]));
        assert_eq!(imported.message, Some(INVALID));
    }

    #[test]
    fn lines_that_are_not_keywords_and_unknown_colors_are_skipped() {
        let text =
            "x\nRxKey[0]=RK1,/a/k\nRxKey[000]=NOPE,/a/k\nRxKey[000] RK1,/a/k\nRxKey[000]=RK1/a/k\n";
        let imported = parse(&utf8(text));
        assert!(imported.rows.is_empty());
        assert_eq!(imported.message, None);
    }

    #[test]
    fn the_color_may_be_the_name_on_the_list() {
        let imported = parse(&utf8("RxKey[000]=正規表現キーワード3,/a/k\n"));
        assert_eq!(imported.rows[0][1], color_label("RK3"));
        // A color a keyword may not have becomes 正規表現キーワード1.
        let imported = parse(&utf8("RxKey[000]=CAR,/a/k\n"));
        assert_eq!(imported.rows[0][1], color_label("RK1"));
    }

    #[test]
    fn the_list_is_cut_at_the_limit_and_at_the_length() {
        let mut text = String::new();
        for i in 0..MAX_REGEX_KEYWORDS + 3 {
            text.push_str(&format!("RxKey[{i:03}]=RK1,/{i}/k\n"));
        }
        let imported = parse(&utf8(&text));
        assert_eq!(imported.rows.len(), MAX_REGEX_KEYWORDS);
        assert_eq!(imported.message, Some(TOO_MANY));
        let long = format!(
            "RxKey[000]=RK1,/{}/k\nRxKey[001]=RK1,/b/k\n",
            "a".repeat(1000)
        );
        let imported = parse(&utf8(&long));
        assert_eq!(imported.rows, rows(&[("/b/k", color_label("RK1"))]));
        assert_eq!(imported.message, Some(TOO_LONG));
    }

    #[test]
    fn shift_jis_files_are_read() {
        let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("RxKey[000]=RK1,/あ/k\r\n");
        assert_eq!(parse(&bytes).rows[0][0], "/あ/k");
    }

    #[test]
    fn export_then_import_gives_the_same_list() {
        let list = rows(&[
            ("/a+/k", color_label("RK1")),
            ("m#b/c#k", color_label("RK2")),
        ]);
        let bytes = render(&list);
        assert!(bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(text.contains("// 正規表現キーワード Ver1\r\n"));
        assert!(text.contains("RxKey[000]=RK1,/a+/k\r\n"));
        assert!(text.contains("RxKey[001]=RK2,m#b/c#k\r\n"));
        assert_eq!(parse(&bytes).rows, list);
        // The list ends at the first empty keyword.
        let with_gap = rows(&[("/a/k", color_label("RK1")), ("", ""), ("/b/k", "")]);
        assert_eq!(parse(&render(&with_gap)).rows.len(), 1);
    }
}
