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
    /// A message about the last command, such as "not implemented yet".
    pub notice: Option<String>,
}

/// Sakura's status bar shows the line and the column ("1 行 1 桁"). The rest is ours.
/// (`sakura_core/window/CMainStatusBar.cpp` fills the parts; this keeps the line and column and
/// the length of the document.)
pub fn status_text(info: &StatusInfo) -> String {
    let mut text = format!(
        "{} 行  {} 桁  |  全 {} 行",
        info.line, info.column, info.lines
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
            notice: None,
        };
        assert_eq!(status_text(&info), "3 行  12 桁  |  全 100 行");
        let info = StatusInfo {
            modified: true,
            notice: Some("未実装".into()),
            ..info
        };
        assert_eq!(
            status_text(&info),
            "3 行  12 桁  |  全 100 行  |  変更あり  |  未実装"
        );
    }

    #[test]
    fn the_title_names_the_file_and_the_program() {
        assert_eq!(window_title(Some("a.txt"), false), "a.txt - UME Editor");
        assert_eq!(window_title(Some("a.txt"), true), "a.txt * - UME Editor");
        assert_eq!(window_title(None, false), "(無題) - UME Editor");
    }
}
