//! What the status bar says about the caret and the selection: the line and the column, the code
//! of the character at the caret and the size of the selection.
//!
//! Sakura: `CCaret::ShowCaretPosInfo` (`sakura_core/view/CCaret.cpp`) and
//! `CViewSelect::PrintSelectionInfoMsg` (`sakura_core/view/CViewSelect.cpp`).

use std::cell::RefCell;

use gpui::App;
use matsu_core::buffer::{Document, Pos};
use matsu_core::encoding::{Encoding, char_code, sakura_utf16_len};
use matsu_core::model::eol_text;

use super::EditorView;
use crate::status::eol_name;
use crate::status_options::StatusbarOptions;

/// The size of the selection that was counted last, so that a selection that grows or shrinks at
/// one end is counted from the difference only (Sakura: `m_nLastSelectedByteLen`,
/// `m_sSelectOld`). The status is made on every frame; a selection of the whole file would be
/// read again each time.
#[derive(Default)]
pub(crate) struct SelectionCountCache(RefCell<Option<Counted>>);

struct Counted {
    version: u64,
    encoding: Encoding,
    by_byte: bool,
    from: Pos,
    to: Pos,
    size: usize,
}

/// The size of the text between `from` and `to` (`from` first): UTF-16 units, which is what
/// Sakura counts as characters (a surrogate pair is two, a line ending is its length), or the
/// bytes of the text in `encoding` (`UnicodeToCode`). The text is read a line at a time.
pub(crate) fn measure(
    doc: &Document,
    encoding: Encoding,
    from: Pos,
    to: Pos,
    by_byte: bool,
) -> usize {
    let codec = encoding.codec();
    let mut bytes = Vec::new();
    let mut size = 0;
    // (the count of the very long lines, which are not read)
    let mut extra = 0;
    let mut add = |text: &str| {
        if by_byte {
            bytes.clear();
            // Characters that cannot be written are written as Sakura writes them (`?`).
            let _ = codec.encode(text, &mut bytes);
            size += bytes.len();
        } else {
            size += sakura_utf16_len(text);
        }
    };
    for index in from.line..to.line.saturating_add(1).min(doc.line_count()) {
        let first = index == from.line;
        let last = index == to.line;
        let start = if first { from.col } else { 0 };
        // A very long line (ADR-0020, 9.10) is not read for a count: its bytes stand for its
        // characters, which is right for narrow ones.
        if doc.is_long_line(index) {
            let end = if last {
                to.col
            } else {
                doc.line_len(index).ok().flatten().unwrap_or(start)
            };
            extra += end.saturating_sub(start);
            if !last && let Ok(Some(eol)) = doc.line_eol(index) {
                add(eol_text(eol));
            }
            continue;
        }
        let Ok(Some(line)) = doc.line(index) else {
            break;
        };
        let end = if last { to.col } else { line.text.len() };
        add(line.text.get(start..end.max(start)).unwrap_or(""));
        if !last && let Some(eol) = line.eol {
            add(eol_text(eol));
        }
    }
    size + extra
}

impl EditorView {
    /// The line and the column the status bar shows, counted from 1.
    ///
    /// Sakura (`ShowCaretPosInfo`): with the lines numbered by the text (`bLineNumIsCRLF`, the
    /// default) the line is the line of the text, else the row of the screen. The column is in
    /// the units of the ruler (the display column; across the rows of a wrapped line when the
    /// lines are numbered by the text, without the indent of the wrapped rows), or, with
    /// `DispColByChar`, the number of characters (UTF-16 units) before the caret in the line (in
    /// the row, when the rows are numbered).
    pub(crate) fn status_position(&self, options: &StatusbarOptions, cx: &App) -> (usize, u32) {
        let doc = self.doc(cx);
        let caret = self.cursor.caret();
        let by_text = self.prefs.settings.outline.line_num_is_crlf;
        let at = self.caret_row(cx);
        let info = self.layout.row(doc, at.row).ok();
        let line = if by_text { caret.line } else { at.row } + 1;
        let Some(info) = info else {
            return (line, self.caret_column(cx) + 1);
        };
        // In a very long line (ADR-0020, 9.10) the part of the line before the row is not read:
        // its bytes stand for its columns, which is right for narrow characters.
        let long = by_text && doc.is_long_line(caret.line);
        let column = if options.column_by_char {
            let from = if by_text && !long {
                Pos::new(caret.line, 0)
            } else {
                info.start
            };
            let before = doc.text_between(from, caret).unwrap_or_default();
            let skipped = if long { info.start.col } else { 0 };
            u32::try_from(skipped + sakura_utf16_len(&before)).unwrap_or(u32::MAX)
        } else if by_text {
            let wide: u32 = if long {
                u32::try_from(info.start.col).unwrap_or(u32::MAX)
            } else {
                // The rows before this one in the line, each without its indent.
                let before = info.row_in_line;
                let rows = self
                    .layout
                    .rows(doc, at.row - before, before)
                    .unwrap_or_default();
                rows.iter()
                    .map(|r| r.end_column.saturating_sub(r.start_column))
                    .sum()
            };
            wide + self.caret_column(cx).saturating_sub(info.start_column)
        } else {
            self.caret_column(cx)
        };
        (line, column + 1)
    }

    /// The code of the character at the caret, the name of the line ending when the caret is at
    /// it, and nothing at the end of the file (`ShowCaretPosInfo`: `szCaretChar`).
    pub(crate) fn caret_char_code(&self, options: &StatusbarOptions, cx: &App) -> String {
        let shared = self.document.read(cx);
        let caret = self.cursor.caret();
        let Ok(Some((rest, eol))) = shared.doc().rest_of_line(caret.line, caret.col, 64) else {
            return String::new();
        };
        match Some(rest.as_str()).filter(|rest| !rest.is_empty()) {
            Some(rest) => char_code(shared.file().encoding, rest, &options.char_code),
            None => eol.map(eol_name).unwrap_or_default().to_owned(),
        }
    }

    /// The text of the line from the caret to the end of the line, without the line ending; `None`
    /// when the caret is at the end of the line or of the file (`ShowCodeBox` shows nothing then).
    pub(crate) fn caret_char_rest(&self, cx: &App) -> Option<String> {
        let shared = self.document.read(cx);
        let caret = self.cursor.caret();
        // (a very long line is cut after 4 KiB)
        let (rest, _) = shared
            .doc()
            .rest_of_line(caret.line, caret.col, 4096)
            .ok()??;
        Some(rest).filter(|rest| !rest.is_empty())
    }

    /// The message of the selection (`PrintSelectionInfoMsg`), or `None` when it is empty.
    pub(crate) fn selection_message(&self, options: &StatusbarOptions, cx: &App) -> Option<String> {
        let doc = self.doc(cx);
        let cursor = &self.cursor;
        let rows = |a: Pos, b: Pos| {
            let row = |p: Pos| self.layout.pos_to_row(doc, p).map_or(0, |r| r.row);
            row(a).abs_diff(row(b)) + 1
        };
        if cursor.is_box() {
            let Some(((a, a_col), (c, c_col))) = cursor.box_corners() else {
                return Some("box selecting".to_owned());
            };
            // The width in columns, and in the pixels of the view.
            let cols = a_col.abs_diff(c_col);
            let px = (cols as f32 * self.metrics.cell_width).round() as i64;
            return Some(format!(
                "{cols} col ({px}px) * {} lines selected.",
                rows(a, c)
            ));
        }
        let Some((from, to)) = cursor.selection() else {
            return cursor.selecting_lock().then(|| "selecting".to_owned());
        };
        let shared = self.document.read(cx);
        let encoding = shared.file().encoding;
        let by_byte = self
            .prefs
            .select_count
            .by_byte(options.select_count_by_byte);
        let size = self.count_selection(doc, shared.version(), encoding, from, to, by_byte);
        // The last line is not counted when the selection ends at the start of its row.
        let mut lines = rows(from, to);
        let ends_at_row_start = self
            .layout
            .pos_to_row(doc, to)
            .ok()
            .and_then(|r| self.layout.row(doc, r.row).ok())
            .is_none_or(|row| row.start == to);
        if ends_at_row_start {
            lines -= 1;
        }
        Some(format!(
            "{size} {} ({lines} lines) selected.",
            if by_byte { "bytes" } else { "chars" }
        ))
    }

    fn count_selection(
        &self,
        doc: &Document,
        version: u64,
        encoding: Encoding,
        from: Pos,
        to: Pos,
        by_byte: bool,
    ) -> usize {
        let mut cache = self.selection_count.0.borrow_mut();
        let incremental = encoding.codec().is_stateless();
        if let Some(c) = cache.as_ref()
            && c.version == version
            && c.encoding == encoding
            && c.by_byte == by_byte
            && c.from == from
            && c.to == to
        {
            return c.size;
        }
        let size = match cache.as_ref() {
            Some(c)
                if incremental
                    && c.version == version
                    && c.encoding == encoding
                    && c.by_byte == by_byte
                    && c.from == from =>
            {
                if to >= c.to {
                    c.size + measure(doc, encoding, c.to, to, by_byte)
                } else {
                    c.size
                        .saturating_sub(measure(doc, encoding, to, c.to, by_byte))
                }
            }
            Some(c)
                if incremental
                    && c.version == version
                    && c.encoding == encoding
                    && c.by_byte == by_byte
                    && c.to == to =>
            {
                if from <= c.from {
                    c.size + measure(doc, encoding, from, c.from, by_byte)
                } else {
                    c.size
                        .saturating_sub(measure(doc, encoding, c.from, from, by_byte))
                }
            }
            _ => measure(doc, encoding, from, to, by_byte),
        };
        *cache = Some(Counted {
            version,
            encoding,
            by_byte,
            from,
            to,
            size,
        });
        size
    }
}
