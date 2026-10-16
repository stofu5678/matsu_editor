//! The text of the file property dialog (`F_PROPERTY_FILE`).
//!
//! Sakura: `CDlgProperty::SetData` (`sakura_core/dlg/CDlgProperty.cpp`) fills a read-only edit box;
//! the strings are `STR_DLGFLPROP_*` in `sakura_core/sakura_rc.rc`. Left out: the type of the
//! document ("設定のタイプ"), the number of commands run, the lock of the file, the MS-DOS name and
//! the attributes that only Windows has (archive, hidden, system, ...). The read-only attribute
//! is shown on every OS.

use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Datelike, Local, Timelike};

/// What the file has on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DiskInfo {
    pub size: u64,
    pub read_only: bool,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub accessed: Option<String>,
}

/// Everything the dialog shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PropertyInfo {
    /// The path of the file, or `None` for a document without one.
    pub path: Option<String>,
    pub encoding: String,
    pub bom: bool,
    /// The line ending the document writes, by name (`CRLF`).
    pub line_ending: String,
    pub lines: usize,
    /// The number of rows on the screen; `None` while a large document is still being laid out.
    pub rows: Option<usize>,
    pub view_mode: bool,
    pub modified: bool,
    pub disk: Option<DiskInfo>,
}

/// `STR_DLGFLPROP_YMDHMS`: `2026年10月16日 09:05:03`.
pub(crate) fn format_time(t: &DateTime<Local>) -> String {
    format!(
        "{}年{}月{}日 {:02}:{:02}:{:02}",
        t.year(),
        t.month(),
        t.day(),
        t.hour(),
        t.minute(),
        t.second()
    )
}

fn local(t: SystemTime) -> String {
    format_time(&DateTime::<Local>::from(t))
}

/// Reads what the disk has about `path`. `None` if there is no such file (Sakura shows no file
/// information then either: `FindFirstFile` failed).
pub(crate) fn disk_info(path: &Path) -> Option<DiskInfo> {
    let meta = std::fs::metadata(path).ok()?;
    Some(DiskInfo {
        size: meta.len(),
        read_only: meta.permissions().readonly(),
        created: meta.created().ok().map(local),
        modified: meta.modified().ok().map(local),
        accessed: meta.accessed().ok().map(local),
    })
}

/// The text of the dialog. The line breaks are `\n`.
pub(crate) fn property_text(info: &PropertyInfo) -> String {
    let mut text = String::new();
    // STR_DLGFLPROP_FILENAME
    text.push_str(&format!(
        "ファイル名  {}\n",
        info.path.as_deref().unwrap_or("")
    ));
    // STR_DLGFLPROP_ENCODING and STR_DLGFLPROP_WITH_BOM
    text.push_str(&format!(
        "文字コード  {}{}\n",
        info.encoding,
        if info.bom { " BOM 付" } else { "" }
    ));
    // Not in Sakura's dialog (it shows the encoding only); the line ending is on the status bar.
    text.push_str(&format!("改行コード  {}\n", info.line_ending));
    // STR_DLGFLPROP_LINE_COUNT and STR_DLGFLPROP_LAYOUT_LINE
    text.push_str(&format!("行数  {}行\n", info.lines));
    if let Some(rows) = info.rows {
        text.push_str(&format!("レイアウト行数  {rows}行\n"));
    }
    if info.view_mode {
        text.push_str("ビューモードで開いています。\n");
    }
    text.push_str(if info.modified {
        "変更されています。\n"
    } else {
        "変更されていません。\n"
    });
    if let Some(disk) = &info.disk {
        // STR_DLGFLPROP_FILE_INFO
        text.push_str("\n--ファイル情報-----------------\n");
        text.push_str("ファイル属性  ");
        if disk.read_only {
            text.push_str("/読み取り専用");
        }
        text.push('\n');
        for (label, time) in [
            ("作成日時", &disk.created),
            ("更新日時", &disk.modified),
            ("アクセス日時", &disk.accessed),
        ] {
            if let Some(time) = time {
                text.push_str(&format!("{label}  {time}\n"));
            }
        }
        text.push_str(&format!("ファイルサイズ  {} バイト\n", disk.size));
    }
    text
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn info() -> PropertyInfo {
        PropertyInfo {
            path: Some("/p/日本語.txt".to_owned()),
            encoding: "SJIS".to_owned(),
            bom: false,
            line_ending: "CRLF".to_owned(),
            lines: 3,
            rows: Some(4),
            view_mode: false,
            modified: false,
            disk: None,
        }
    }

    #[test]
    fn the_text_follows_sakuras_dialog() {
        assert_eq!(
            property_text(&info()),
            "ファイル名  /p/日本語.txt\n文字コード  SJIS\n改行コード  CRLF\n行数  3行\nレイアウト行数  4行\n変更されていません。\n"
        );
    }

    #[test]
    fn bom_view_mode_and_modified_are_told() {
        let text = property_text(&PropertyInfo {
            encoding: "UTF-8".to_owned(),
            bom: true,
            view_mode: true,
            modified: true,
            rows: None,
            ..info()
        });
        assert!(text.contains("文字コード  UTF-8 BOM 付\n"));
        assert!(text.contains("ビューモードで開いています。\n変更されています。\n"));
        assert!(!text.contains("レイアウト行数"));
    }

    #[test]
    fn a_document_without_a_file_has_an_empty_name_and_no_file_information() {
        let text = property_text(&PropertyInfo {
            path: None,
            ..info()
        });
        assert!(text.starts_with("ファイル名  \n"));
        assert!(!text.contains("ファイル情報"));
    }

    #[test]
    fn the_file_information_has_the_times_and_the_size() {
        let text = property_text(&PropertyInfo {
            disk: Some(DiskInfo {
                size: 1234,
                read_only: true,
                created: Some("2026年1月2日 03:04:05".to_owned()),
                modified: Some("2026年10月16日 23:59:59".to_owned()),
                accessed: None,
            }),
            ..info()
        });
        assert!(text.ends_with(
            "\n--ファイル情報-----------------\nファイル属性  /読み取り専用\n作成日時  2026年1月2日 03:04:05\n更新日時  2026年10月16日 23:59:59\nファイルサイズ  1234 バイト\n"
        ));
    }

    #[test]
    fn a_time_has_two_digit_clock_fields_and_plain_date_fields() {
        let t = Local.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap();
        assert_eq!(format_time(&t), "2026年1月2日 03:04:05");
    }

    #[test]
    fn the_disk_information_comes_from_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, "あい").unwrap();
        let disk = disk_info(&path).unwrap();
        assert_eq!(disk.size, 6);
        assert!(!disk.read_only);
        assert!(disk.modified.is_some());
        assert_eq!(disk_info(&dir.path().join("none.txt")), None);
    }
}
