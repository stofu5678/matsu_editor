//! What the status bar says, part by part.
//!
//! Sakura's status bar has eight parts (`CEditWnd::OnSize`, `pszLabel`, in
//! `sakura_core/window/CEditWnd.cpp`; the texts are made by `CCaret::ShowCaretPosInfo`):
//! the message, the line and column, the line ending, the code at the caret, the encoding, REC,
//! the typing mode and the zoom of the font. `crate::status_bar` draws them.

use ume_core::buffer::Eol;
use ume_core::encoding::Encoding;

/// What the status bar shows about the editor.
#[derive(Clone, Debug, PartialEq)]
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
    /// The name of the encoding, with " BOM付" when the document has a BOM ([`charset_label`]).
    pub charset: String,
    /// The name of the line ending that Enter inserts ([`eol_name`]).
    pub eol: String,
    /// The code of the character at the caret, or the name of the line ending when the caret is at
    /// it (`EditorView::caret_char_code`). Empty at the end of the file.
    pub char_code: String,
    /// What Sakura prints in the first part while text is selected (`PrintSelectionInfoMsg`).
    pub selection: Option<String>,
    /// A message about the last command, such as "not implemented yet".
    pub notice: Option<String>,
    /// Whether this window is recording a key macro (REC is drawn dark then, gray otherwise).
    pub recording: bool,
    /// The zoom of the font, 1.0 for 100 % (`CEditWnd::GetFontZoom`).
    pub zoom: f64,
}

impl Default for StatusInfo {
    fn default() -> Self {
        Self {
            line: 0,
            column: 0,
            lines: 0,
            modified: false,
            overwrite: false,
            charset: String::new(),
            eol: String::new(),
            char_code: String::new(),
            selection: None,
            notice: None,
            recording: false,
            zoom: 1.0,
        }
    }
}

/// The parts of the status bar, left to right (the index is `dwItemSpec` of the click).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Message,
    Position,
    Eol,
    CharCode,
    Charset,
    Rec,
    Mode,
    Zoom,
}

impl Part {
    pub const ALL: [Part; 8] = [
        Part::Message,
        Part::Position,
        Part::Eol,
        Part::CharCode,
        Part::Charset,
        Part::Rec,
        Part::Mode,
        Part::Zoom,
    ];

    /// The widest text of the part, which its width is made from (`pszLabel`). The message part
    /// takes what is left.
    pub fn width_label(self) -> &'static str {
        match self {
            Part::Message => "",
            Part::Position => "99999 行 9999 列",
            Part::Eol => "CRLF",
            Part::CharCode => "AAAAAAAAAAAA",
            Part::Charset => "UTF-16 BOM付",
            Part::Rec => "REC",
            Part::Mode => "上書",
            Part::Zoom => "9999 %",
        }
    }
}

impl StatusInfo {
    /// The text of `part`. REC is always "REC"; whether it is lit is [`recording`](Self::recording).
    pub fn text(&self, part: Part) -> String {
        match part {
            Part::Message => self.message(),
            Part::Position => self.position_text(),
            Part::Eol => self.eol.clone(),
            Part::CharCode => self.char_code.clone(),
            Part::Charset => self.charset.clone(),
            Part::Rec => "REC".to_owned(),
            Part::Mode => self.mode_text().to_owned(),
            Part::Zoom => self.zoom_text(),
        }
    }

    /// The first part: the size of the selection (`PrintSelectionInfoMsg`) and the message about
    /// the last command, such as the pattern of the incremental search.
    pub fn message(&self) -> String {
        match (&self.selection, &self.notice) {
            (Some(selection), Some(notice)) => format!("{selection}  {notice}"),
            (Some(text), None) | (None, Some(text)) => text.clone(),
            (None, None) => String::new(),
        }
    }

    /// `STR_STATUS_ROW_COL`: "%5d 行 %4d 桁".
    pub fn position_text(&self) -> String {
        format!("{:>5} 行 {:>4} 桁", self.line, self.column)
    }

    /// `STR_INS_MODE_INS` and `STR_INS_MODE_OVR`.
    pub fn mode_text(&self) -> &'static str {
        if self.overwrite { "上書" } else { "挿入" }
    }

    /// `STR_STATUS_FONTZOOM_0` ("%4.0f %%"), and `STR_STATUS_FONTZOOM_1` ("%4.1f %%") below 5 %.
    pub fn zoom_text(&self) -> String {
        let percent = self.zoom * 100.0;
        if percent < 5.0 {
            format!("{percent:>4.1} %")
        } else {
            format!("{percent:>4.0} %")
        }
    }

    /// The encoding and the line ending together, such as `UTF-8 BOM付  CRLF`.
    pub fn file_format(&self) -> String {
        format!("{}  {}", self.charset, self.eol)
    }
}

/// The name of the encoding with " BOM付" when the document has a BOM
/// (`CCaret::ShowCaretPosInfo`, `sakura_core/view/CCaret.cpp`; `STR_CARET_WITHBOM` in
/// `sakura_core/sakura_rc.rc`).
pub fn charset_label(encoding: Encoding, bom: bool) -> String {
    format!(
        "{}{}",
        encoding.combo_name(),
        if bom { " BOM付" } else { "" }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> StatusInfo {
        StatusInfo {
            line: 3,
            column: 12,
            lines: 100,
            charset: charset_label(Encoding::Utf8, true),
            eol: "CRLF".to_owned(),
            char_code: "U+3042".to_owned(),
            ..StatusInfo::default()
        }
    }

    #[test]
    fn the_parts_have_sakuras_texts() {
        let i = info();
        // STR_STATUS_ROW_COL "%5d 行 %4d 桁"
        assert_eq!(i.text(Part::Position), "    3 行   12 桁");
        assert_eq!(i.text(Part::Eol), "CRLF");
        assert_eq!(i.text(Part::CharCode), "U+3042");
        assert_eq!(i.text(Part::Charset), "UTF-8 BOM付");
        assert_eq!(i.text(Part::Rec), "REC");
        assert_eq!(i.text(Part::Mode), "挿入");
        assert_eq!(i.text(Part::Zoom), " 100 %");
        assert_eq!(i.text(Part::Message), "");
        assert_eq!(i.file_format(), "UTF-8 BOM付  CRLF");
    }

    #[test]
    fn overwrite_mode_says_so() {
        // STR_INS_MODE_OVR
        let i = StatusInfo {
            overwrite: true,
            ..info()
        };
        assert_eq!(i.text(Part::Mode), "上書");
    }

    #[test]
    fn the_zoom_has_a_decimal_below_five_percent() {
        let zoom = |z: f64| StatusInfo { zoom: z, ..info() }.text(Part::Zoom);
        assert_eq!(zoom(1.5), " 150 %");
        assert_eq!(zoom(0.1), "  10 %");
        assert_eq!(zoom(0.04), " 4.0 %");
        assert_eq!(zoom(0.011), " 1.1 %");
        assert_eq!(zoom(0.05), "   5 %");
        assert_eq!(zoom(90.0), "9000 %");
    }

    #[test]
    fn the_message_part_holds_the_selection_and_the_notice() {
        let i = StatusInfo {
            selection: Some("3 chars (1 lines) selected.".to_owned()),
            ..info()
        };
        assert_eq!(i.text(Part::Message), "3 chars (1 lines) selected.");
        let i = StatusInfo {
            selection: None,
            notice: Some("I-Search: abc".to_owned()),
            ..i
        };
        assert_eq!(i.text(Part::Message), "I-Search: abc");
        let i = StatusInfo {
            selection: Some("1 chars (1 lines) selected.".to_owned()),
            ..i
        };
        assert_eq!(
            i.text(Part::Message),
            "1 chars (1 lines) selected.  I-Search: abc"
        );
    }

    #[test]
    fn the_labels_are_sakuras_widest_texts() {
        let labels: Vec<_> = Part::ALL.iter().map(|p| p.width_label()).collect();
        assert_eq!(
            labels,
            [
                "",
                "99999 行 9999 列",
                "CRLF",
                "AAAAAAAAAAAA",
                "UTF-16 BOM付",
                "REC",
                "上書",
                "9999 %"
            ]
        );
    }

    #[test]
    fn the_encoding_and_the_line_ending_are_named_as_sakura_does() {
        assert_eq!(charset_label(Encoding::ShiftJis, false), "SJIS");
        assert_eq!(charset_label(Encoding::Utf16Le, true), "UTF-16 BOM付");
        assert_eq!(eol_name(Eol::Nel), "NEL");
    }
}
