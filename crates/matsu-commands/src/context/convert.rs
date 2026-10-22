//! Converting the selected text in place: what the commands of Sakura's conversion group share.
//!
//! Sakura: `CEditView::ConvSelectedArea` in `sakura_core/view/CEditView.cpp`. Every conversion
//! command (`Command_TOLOWER`, `Command_TABTOSPACE` and the others in
//! `sakura_core/cmd/CViewCommander_Convert.cpp`) only calls it with its function code.

use super::CommandContext;
use crate::command::Result;

impl CommandContext<'_> {
    /// Replaces the selected text by `convert(selected text)`.
    ///
    /// * Without a selection nothing happens (Sakura returns at once, without a beep).
    /// * A box selection is converted row by row (`convert_box`): the text
    ///   of each row of the box, which is then no longer selected.
    /// * The conversion gets the text with its line endings as they are.
    /// * Afterwards the converted text is selected, from where the selection began to the end of
    ///   the new text, with the caret at the end, even if the selection was made backwards
    ///   (`SetSelectArea(ptFrom, caret)` and `MoveCursor(GetTo())` in `ConvSelectedArea`).
    /// * Difference: a conversion that gives the same text makes no edit (and no undo step), where
    ///   Sakura replaces the text by itself.
    pub fn convert_selection(&mut self, mut convert: impl FnMut(&str) -> String) -> Result<()> {
        self.convert_selection_with_columns(|text, _tab_width, _start_col| convert(text))
    }

    /// As [`convert_selection`](Self::convert_selection), for the conversions that depend on
    /// display columns. `convert` gets the selected text, the tab width (see
    /// [`tab_width`](Self::tab_width)) and the display column where the selection starts
    /// (`nStartColumn`, `GetFrom().GetX2() / GetLayoutXDefault()` in Sakura).
    pub fn convert_selection_with_columns(
        &mut self,
        mut convert: impl FnMut(&str, usize, usize) -> String,
    ) -> Result<()> {
        let Some((from, to)) = self.selection() else {
            return Ok(());
        };
        if self.cursor.is_box() {
            return self.convert_box(convert);
        }
        let old = self.doc().text_between(from, to)?;
        let tab_width = self.tab_width();
        let start_col = self.layout.display_col(self.doc(), from)?;
        let new = convert(&old, tab_width, start_col);
        let end = if new == old {
            to
        } else {
            self.replace(from, to, &new)?.end
        };
        self.select_range(from, end)
    }

    /// The tab width of the layout ([`Layout::tab_width`](crate::layout::Layout::tab_width)).
    ///
    /// Sakura gives the conversions `m_cLayoutMgr.GetTabSpaceKetas()`, the tab width of the
    /// layout (`CEditView::ConvSelectedArea` in `sakura_core/view/CEditView.cpp`).
    pub fn tab_width(&self) -> usize {
        self.layout.tab_width()
    }
}

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Document;
    use matsu_core::cursor::Cursor;
    use matsu_core::model::DocumentModel;

    use super::*;
    use crate::layout::PlainLayout;

    #[test]
    fn the_tab_width_is_the_one_of_the_layout() {
        for width in [1, 2, 4, 8] {
            let mut model = DocumentModel::new(Document::from_utf8(b"\tx".to_vec()).unwrap());
            let mut cursor = Cursor::default();
            let mut layout = PlainLayout { tab_width: width };
            let cx = CommandContext::new(&mut model, &mut cursor, &mut layout, None);
            assert_eq!(cx.tab_width(), width);
        }
    }
}
