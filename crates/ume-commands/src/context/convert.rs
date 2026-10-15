//! Converting the selected text in place: what the commands of Sakura's conversion group share.
//!
//! Sakura: `CEditView::ConvSelectedArea` in `sakura_core/view/CEditView.cpp`. Every conversion
//! command (`Command_TOLOWER`, `Command_TABTOSPACE` and the others in
//! `sakura_core/cmd/CViewCommander_Convert.cpp`) only calls it with its function code.

use ume_core::buffer::Pos;
use ume_core::cursor::SelectionMode;

use super::CommandContext;
use crate::command::Result;

/// The most tabs that [`CommandContext::tab_width`] looks at.
const MAX_TABS_SEEN: usize = 16;
/// How many lines around the selection [`CommandContext::tab_width`] searches for tabs.
const TAB_SEARCH_LINES: usize = 200;
/// The largest tab width that Sakura allows (`MAXLINEKETAS` is 64).
const MAX_TAB_WIDTH: usize = 64;
/// Sakura's default tab width (`m_nTabSpace`, `InitTypeConfigDefault` in
/// `sakura_core/types/CType.cpp`).
const DEFAULT_TAB_WIDTH: usize = 4;

impl CommandContext<'_> {
    /// Replaces the selected text by `convert(selected text)`.
    ///
    /// * Without a selection nothing happens (Sakura returns at once, without a beep). A
    ///   rectangular selection beeps and does nothing: it is not implemented yet.
    /// * The conversion gets the text with its line endings as they are.
    /// * Afterwards the converted text is selected, from where the selection began to the end of
    ///   the new text, with the caret at the end, even if the selection was made backwards
    ///   (`SetSelectArea(ptFrom, caret)` and `MoveCursor(GetTo())` in `ConvSelectedArea`).
    /// * Difference: a conversion that gives the same text makes no edit (and no undo step), where
    ///   Sakura replaces the text by itself.
    pub fn convert_selection(&mut self, convert: impl FnOnce(&str) -> String) -> Result<()> {
        self.convert_selection_with_columns(|text, _tab_width, _start_col| convert(text))
    }

    /// As [`convert_selection`](Self::convert_selection), for the conversions that depend on
    /// display columns. `convert` gets the selected text, the tab width (see
    /// [`tab_width`](Self::tab_width)) and the display column where the selection starts
    /// (`nStartColumn`, `GetFrom().GetX2() / GetLayoutXDefault()` in Sakura).
    pub fn convert_selection_with_columns(
        &mut self,
        convert: impl FnOnce(&str, usize, usize) -> String,
    ) -> Result<()> {
        if self.cursor.mode() == SelectionMode::Rectangular {
            self.beep();
            return Ok(());
        }
        let Some((from, to)) = self.selection() else {
            return Ok(());
        };
        let old = self.doc().text_between(from, to)?;
        let tab_width = self.tab_width(from, to)?;
        let start_col = self.layout.display_col(self.doc(), from)?;
        let new = convert(&old, tab_width, start_col);
        let end = if new == old {
            to
        } else {
            self.replace(from, to, &new)?.end
        };
        self.select_range(from, end)
    }

    /// The tab width of the layout, found from the display columns of the tabs near `from..to`.
    ///
    /// The [`Layout`](crate::layout::Layout) trait has no tab width, so it is derived: a tab at
    /// display column `c` that ends at column `e` means the width `w` with `(c / w + 1) * w == e`.
    /// The largest width that fits all the tabs seen is taken (a lone tab at column 3 that ends
    /// at 4 fits 1, 2 and 4, and the answer is 4). With no tab in the text around the selection,
    /// Sakura's default 4 is used. Only the lines near the selection are searched, because a
    /// document may be huge.
    pub fn tab_width(&self, from: Pos, to: Pos) -> Result<usize> {
        let first = from.line.saturating_sub(TAB_SEARCH_LINES);
        let last = (to.line + TAB_SEARCH_LINES + 1).min(self.doc().line_count());
        // the lines of the selection first, then the ones before it
        let order = (from.line..last).chain((first..from.line).rev());
        let mut seen: Vec<(usize, usize)> = Vec::new();
        'lines: for line in order {
            let Some(l) = self.doc().line(line)? else {
                continue;
            };
            for (col, _) in l.text.match_indices('\t') {
                let before = self.layout.display_col(self.doc(), Pos::new(line, col))?;
                let after = self
                    .layout
                    .display_col(self.doc(), Pos::new(line, col + 1))?;
                seen.push((before, after));
                if seen.len() >= MAX_TABS_SEEN {
                    break 'lines;
                }
            }
        }
        Ok((1..=MAX_TAB_WIDTH)
            .rev()
            .find(|&w| !seen.is_empty() && seen.iter().all(|&(c, e)| (c / w + 1) * w == e))
            .unwrap_or(DEFAULT_TAB_WIDTH))
    }
}

#[cfg(test)]
mod tests {
    use ume_core::buffer::Document;
    use ume_core::cursor::Cursor;
    use ume_core::model::DocumentModel;

    use super::*;
    use crate::layout::PlainLayout;

    fn tab_width_of(text: &str, layout_tab: usize) -> usize {
        let mut model = DocumentModel::new(Document::from_utf8(text.as_bytes().to_vec()).unwrap());
        let mut cursor = Cursor::default();
        let mut layout = PlainLayout {
            tab_width: layout_tab,
        };
        let cx = CommandContext::new(&mut model, &mut cursor, &mut layout, None);
        cx.tab_width(Pos::new(0, 0), Pos::new(0, 0)).unwrap()
    }

    #[test]
    fn the_tab_width_is_found_from_the_tabs_in_the_text() {
        assert_eq!(tab_width_of("\tx", 8), 8);
        assert_eq!(tab_width_of("\tx", 2), 2);
        assert_eq!(tab_width_of("ab\tx", 4), 4);
        // a tab at column 3 that ends at 4 fits 1, 2 and 4; the largest is taken
        assert_eq!(tab_width_of("abc\tx", 4), 4);
        // two tabs settle what one cannot: columns 3 -> 6 and 6 -> 9 mean 3
        assert_eq!(tab_width_of("abc\t\tx", 3), 3);
        // no tab: Sakura's default
        assert_eq!(tab_width_of("abc", 8), 4);
        assert_eq!(tab_width_of("", 8), 4);
    }
}
