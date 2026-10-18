//! The text of the status bar and the title of the window.

use ume_core::buffer::Eol;
use ume_core::encoding::Encoding;

/// What the status bar shows about the editor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatusInfo {
    /// The line of the caret, counted from 1 (the logical line, not the screen row).
    pub line: usize,
    /// The display column of the caret on its row, counted from 1.
    pub column: u32,
    /// The number of lines of the document.
    pub lines: usize,
    pub modified: bool,
    /// Overwrite mode (the Insert key); else insert mode.
    pub overwrite: bool,
    /// The encoding (with BOM) and the line ending of the file, such as `UTF-8 BOM付  CRLF`
    /// ([`file_format_label`]). Empty when there is nothing to show.
    pub file_format: String,
    /// The code of the character at the caret, or the name of the line ending when the caret is at
    /// it (`EditorView::caret_char_code`). Empty at the end of the file.
    pub char_code: String,
    /// What Sakura prints in the first part while text is selected (`PrintSelectionInfoMsg`).
    pub selection: Option<String>,
    /// A message about the last command, such as "not implemented yet".
    pub notice: Option<String>,
}

/// Sakura's status bar shows the line and the column ("1 行 1 桁") and the typing mode, "挿入" or
/// "上書" (`STR_INS_MODE_INS` and `STR_INS_MODE_OVR` in `sakura_core/sakura_rc.rc`, shown by
/// `CCaret::ShowCaretPosInfo`). The rest is ours.
pub fn status_text(info: &StatusInfo) -> String {
    let mut text = format!(
        "{} 行  {} 桁  |  {}  |  全 {} 行",
        info.line,
        info.column,
        if info.overwrite { "上書" } else { "挿入" },
        info.lines
    );
    if !info.char_code.is_empty() {
        text.push_str("  |  ");
        text.push_str(&info.char_code);
    }
    if !info.file_format.is_empty() {
        text.push_str("  |  ");
        text.push_str(&info.file_format);
    }
    if let Some(selection) = &info.selection {
        text.push_str("  |  ");
        text.push_str(selection);
    }
    if info.modified {
        text.push_str("  |  変更あり");
    }
    if let Some(notice) = &info.notice {
        text.push_str("  |  ");
        text.push_str(notice);
    }
    text
}

/// The encoding, the BOM and the line ending as the status bar shows them. Sakura shows the name of
/// the encoding with " BOM付" when the document has a BOM, and the name of the line ending
/// (`CCaret::ShowCaretPosInfo`, `sakura_core/view/CCaret.cpp`; `STR_CARET_WITHBOM` in
/// `sakura_core/sakura_rc.rc`; `CEol::GetName`, `sakura_core/basis/CEol.cpp`).
pub fn file_format_label(encoding: Encoding, bom: bool, eol: Eol) -> String {
    format!(
        "{}{}  {}",
        encoding.combo_name(),
        if bom { " BOM付" } else { "" },
        eol_name(eol)
    )
}

/// The name of a line ending (`CEol::GetName`).
pub fn eol_name(eol: Eol) -> &'static str {
    match eol {
        Eol::CrLf => "CRLF",
        Eol::Lf => "LF",
        Eol::Cr => "CR",
        Eol::Nel => "NEL",
        Eol::Ls => "LS",
        Eol::Ps => "PS",
    }
}

/// The title of the window: the file name (or 無題 for a new document), a mark when the document
/// has unsaved changes, and the name of the program.
pub fn window_title(file_name: Option<&str>, modified: bool) -> String {
    format!(
        "{}{} - UME Editor",
        file_name.unwrap_or("(無題)"),
        if modified { " *" } else { "" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_status_shows_line_and_column() {
        let info = StatusInfo {
            line: 3,
            column: 12,
            lines: 100,
            modified: false,
            overwrite: false,
            ..StatusInfo::default()
        };
        assert_eq!(status_text(&info), "3 行  12 桁  |  挿入  |  全 100 行");
        let info = StatusInfo {
            modified: true,
            notice: Some("未実装".into()),
            ..info
        };
        assert_eq!(
            status_text(&info),
            "3 行  12 桁  |  挿入  |  全 100 行  |  変更あり  |  未実装"
        );
    }

    #[test]
    fn the_status_shows_the_typing_mode() {
        // Sakura: STR_INS_MODE_OVR in sakura_core/sakura_rc.rc
        let info = StatusInfo {
            line: 1,
            column: 1,
            lines: 1,
            modified: false,
            overwrite: true,
            ..StatusInfo::default()
        };
        assert_eq!(status_text(&info), "1 行  1 桁  |  上書  |  全 1 行");
    }

    #[test]
    fn the_status_shows_the_encoding_and_the_line_ending() {
        let info = StatusInfo {
            line: 1,
            column: 1,
            lines: 2,
            modified: false,
            overwrite: false,
            file_format: file_format_label(Encoding::Utf8, true, Eol::CrLf),
            ..StatusInfo::default()
        };
        assert_eq!(
            status_text(&info),
            "1 行  1 桁  |  挿入  |  全 2 行  |  UTF-8 BOM付  CRLF"
        );
        assert_eq!(
            file_format_label(Encoding::ShiftJis, false, Eol::Lf),
            "SJIS  LF"
        );
    }

    #[test]
    fn the_title_names_the_file_and_the_program() {
        assert_eq!(window_title(Some("a.txt"), false), "a.txt - UME Editor");
        assert_eq!(window_title(Some("a.txt"), true), "a.txt * - UME Editor");
        assert_eq!(window_title(None, false), "(無題) - UME Editor");
    }
}
