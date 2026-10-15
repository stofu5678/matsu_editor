//! The text of the status bar and the title of the window.

/// What the status bar shows about the editor.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    if info.modified {
        text.push_str("  |  変更あり");
    }
    if let Some(notice) = &info.notice {
        text.push_str("  |  ");
        text.push_str(notice);
    }
    text
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
            notice: None,
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
            notice: None,
        };
        assert_eq!(status_text(&info), "1 行  1 桁  |  上書  |  全 1 行");
    }

    #[test]
    fn the_title_names_the_file_and_the_program() {
        assert_eq!(window_title(Some("a.txt"), false), "a.txt - UME Editor");
        assert_eq!(window_title(Some("a.txt"), true), "a.txt * - UME Editor");
        assert_eq!(window_title(None, false), "(無題) - UME Editor");
    }
}
